//! Wolf RPG extraction rules (plan WF): which texts are translatable.
//!
//! A text these rules reject is **skipped** at extraction (decision WF-D1): no
//! segment is created, so it never reaches the editor nor the LLM, and the
//! export leaves its original value in the game. Every Wolf-specific rule lives
//! here; `extractor.rs` only calls them. The rules shared by all engines stay in
//! [`crate::engines::filter`].
//!
//! Adding a rule: write it here with its unit tests, call it from the matching
//! extraction step (map, common event, message, database field), and measure it
//! on real games first: only rules that skip no displayed text belong here.

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;

use super::dat_parser::{DatField, DatType};
use crate::engines::filter::{is_pure_number, needs_translation};
use crate::llm::tokenizer::Engine as TokEngine;

// ---------------------------------------------------------------------------
// Texts
// ---------------------------------------------------------------------------

/// `true` when `text` holds something to translate: shared rules of
/// [`needs_translation`], minus resource paths ([`is_resource_path`]).
pub(crate) fn is_translatable(text: &str) -> bool {
    needs_translation(text, TokEngine::Wolf) && !is_resource_path(text.trim())
}

/// Matches a trailing resource file extension (images, audio, video, fonts)
/// commonly used in Wolf RPG database/event fields for graphic, BGM/SE, and
/// font references — these are file paths, not translatable text.
static RE_RESOURCE_EXTENSION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\.(png|jpe?g|bmp|gif|ogg|wav|mp3|mid|midi|ttf|otf|avi|webm|mp4)$")
        .expect("RE_RESOURCE_EXTENSION must compile")
});

/// Sentence punctuation, never part of a resource path.
const SENTENCE_PUNCTUATION: [char; 12] = [
    '。', '、', '！', '？', '「', '」', '『', '』', '（', '）', '…', '：',
];

/// Returns true if `text` is a resource path, possibly followed by numeric
/// lines (plan UW, UW-4: `bg/スラム.png⏎0⏎0⏎0`, a `SysDatabase` background
/// field): every line, trailing `\r` and spaces removed, is empty, a number or
/// a lone path ([`is_lone_path`]), and at least one is a lone path.
fn is_resource_path(text: &str) -> bool {
    let mut has_path = false;
    for line in text.split('\n').map(str::trim_end) {
        let unsigned = line.strip_prefix('-').unwrap_or(line);
        if line.is_empty() || is_pure_number(unsigned) {
            continue;
        }
        if !is_lone_path(line) {
            return false;
        }
        has_path = true;
    }
    has_path
}

/// A line that is a resource path and nothing else: it ends with a known
/// extension, holds no full-width space nor sentence punctuation, and either
/// contains `/` or `\` without any space, or is entirely ASCII (`icon.png`,
/// `MS Gothic.ttf`). `ファイル名はicon.png` is a sentence, not a path.
fn is_lone_path(line: &str) -> bool {
    RE_RESOURCE_EXTENSION.is_match(line)
        && !line.contains('\u{3000}')
        && !line.contains(SENTENCE_PUNCTUATION)
        && (line.is_ascii() || (line.contains(['/', '\\']) && !line.contains(' ')))
}

/// Returns true if `text` contains at least one Japanese character
/// (hiragana, katakana, or CJK unified ideograph).
fn contains_japanese(text: &str) -> bool {
    text.chars().any(|c| {
        matches!(c,
            '\u{3041}'..='\u{3096}'   // hiragana
            | '\u{30A0}'..='\u{30FF}' // katakana
            | '\u{4E00}'..='\u{9FFF}' // CJK unified ideographs
        )
    })
}

/// An error message shown by a game's own checks, never by its story
/// (WF-D5): `ｴﾗｰ`, or `エラー` / `警告` followed by optional spaces and `!`,
/// `！`, `:` or `：`. `警告、彼女の…` (a story line) does not match.
static RE_ERROR_MESSAGE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"ｴﾗｰ|(?:エラー|警告)[ \t\u{3000}]*[!！:：]").expect("RE_ERROR_MESSAGE must compile")
});

/// `true` when a message or a choice of a map or a common event is skipped:
/// nothing to translate ([`is_translatable`]) or an error message
/// ([`RE_ERROR_MESSAGE`]).
pub(crate) fn skip_message(text: &str) -> bool {
    !is_translatable(text) || RE_ERROR_MESSAGE.is_match(text)
}

// ---------------------------------------------------------------------------
// Maps and common events
// ---------------------------------------------------------------------------

/// Fragments of a debug or sample name, matched regardless of case (WF-D5).
const DEBUG_NAME_PARTS: [&str; 8] = [
    "sample",
    "debug",
    "test",
    "ﾃﾞﾊﾞ",
    "デバ",
    "ﾃｽﾄ",
    "テスト",
    "使用禁止",
];

/// Prefixes of a debug name, matched regardless of case (WF-D5).
const DEBUG_NAME_PREFIXES: [&str; 2] = ["[d]", "※"];

/// `true` when `name` (a map or a common event) is a debug or sample one
/// (WF-D5): it contains a [`DEBUG_NAME_PARTS`] fragment or starts with `[D]`
/// or `※`, regardless of case.
fn is_debug_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    DEBUG_NAME_PARTS.iter().any(|part| lower.contains(part))
        || DEBUG_NAME_PREFIXES
            .iter()
            .any(|prefix| lower.starts_with(prefix))
}

/// `true` when the map `name` (its file stem, e.g. `SampleMapA`) is skipped as
/// a whole: a debug or sample map ([`is_debug_name`]).
pub(crate) fn skip_map(name: &str) -> bool {
    is_debug_name(name)
}

/// `true` when the common event `name` is skipped as a whole: engine-internal
/// (`X[`), archived or disabled (`zz`), or debug ([`is_debug_name`]) events.
pub(crate) fn skip_common_event(name: &str) -> bool {
    name.starts_with("X[") || name.starts_with("zz") || is_debug_name(name)
}

// ---------------------------------------------------------------------------
// Databases
// ---------------------------------------------------------------------------

/// Returns true if the field name is in the set of known translatable field names.
fn is_known_translatable_field(name: &str) -> bool {
    matches!(
        name,
        "name" | "名前" | "description" | "説明" | "note" | "備考" | "message" | "text"
    )
}

/// `true` when the `value` of a database string field named `field_name` is
/// skipped: nothing to translate ([`is_translatable`]), a value Wolf RPG
/// replaces at runtime (`自動ｼｽﾃﾑ初期化`, never player-facing), or a value
/// without Japanese in a field not known as translatable.
pub(crate) fn skip_database_value(field_name: &str, value: &str) -> bool {
    !is_translatable(value)
        || value.contains("自動ｼｽﾃﾑ初期化")
        || (!is_known_translatable_field(field_name) && !contains_japanese(value))
}

/// Minimum number of rows for the "system" duplicate rule (WF-D5).
const SYSTEM_DUPLICATE_MIN_ROWS: usize = 3;

/// Fragments of an internal "system" field name (WF-D5).
const SYSTEM_FIELD_PARTS: [&str; 3] = ["ｼｽﾃﾑ", "システム", "Sys"];

/// Separators between the items of a list value (WF-D5).
const LIST_SEPARATORS: [char; 4] = [',', '、', '|', '\n'];

/// The fields of `dat_type` skipped as a whole, as indices into
/// `dat_type.fields` (WF-D5): fields holding common event names
/// ([`holds_common_event_names`]) and "system" duplicates of another field
/// ([`is_system_duplicate`]).
///
/// A segment key names its field, and the injector and the integrity gate
/// resolve it to the first string field of that name: a field named like an
/// earlier skipped field would have its translation written into the skipped
/// one, so it is skipped too (the skipped values stay as in the game, WF-D1).
pub(crate) fn skipped_fields(
    dat_type: &DatType,
    common_event_names: &HashSet<String>,
) -> HashSet<usize> {
    let columns = string_columns(dat_type);
    let mut skipped = HashSet::new();
    let mut skipped_names: HashSet<&str> = HashSet::new();
    for column in &columns {
        let name = column.field.name.as_str();
        if skipped_names.contains(name)
            || holds_common_event_names(&column.values, common_event_names)
            || is_system_duplicate(column, &columns)
        {
            skipped.insert(column.field_idx);
            skipped_names.insert(name);
        }
    }
    skipped
}

/// One string field of a database type and its value in every row.
struct Column<'a> {
    /// Index into `DatType::fields`.
    field_idx: usize,
    field: &'a DatField,
    /// One value per row, empty when the row does not hold it.
    values: Vec<&'a str>,
}

/// The string fields of `dat_type`, in declaration order. Values are stored
/// one per valid string field, in that order (as the extractor reads them).
fn string_columns(dat_type: &DatType) -> Vec<Column<'_>> {
    dat_type
        .fields
        .iter()
        .enumerate()
        .filter(|(_, field)| field.is_valid() && field.is_string())
        .enumerate()
        .map(|(slot, (field_idx, field))| Column {
            field_idx,
            field,
            values: dat_type
                .entries
                .iter()
                .map(|entry| entry.string_values.get(slot).map_or("", String::as_str))
                .collect(),
        })
        .collect()
}

/// The items of a list value: split on [`LIST_SEPARATORS`], trimmed, empty
/// items dropped.
fn list_items(value: &str) -> impl Iterator<Item = &str> {
    value
        .split(LIST_SEPARATORS)
        .map(str::trim)
        .filter(|item| !item.is_empty())
}

/// `true` when a field holds common event names (WF-D5): at least half of its
/// non-empty values are lists of common event names or numbers, and at least
/// one value names a common event (a field of numbers alone is not one).
fn holds_common_event_names(values: &[&str], common_event_names: &HashSet<String>) -> bool {
    let filled: Vec<&str> = values
        .iter()
        .copied()
        .filter(|value| !value.trim().is_empty())
        .collect();
    let is_number = |item: &str| is_pure_number(item.strip_prefix('-').unwrap_or(item));
    let listed = filled
        .iter()
        .filter(|value| {
            list_items(value).next().is_some()
                && list_items(value)
                    .all(|item| common_event_names.contains(item) || is_number(item))
        })
        .count();
    let names_an_event = filled
        .iter()
        .any(|value| list_items(value).any(|item| common_event_names.contains(item)));
    names_an_event && listed * 2 >= filled.len()
}

/// `true` when `name` is an internal "system" field name ([`SYSTEM_FIELD_PARTS`]).
fn is_system_field(name: &str) -> bool {
    SYSTEM_FIELD_PARTS.iter().any(|part| name.contains(part))
}

/// `true` when `column` is a "system" duplicate (WF-D5): its name is a system
/// one ([`is_system_field`]) and, in at least 80 % of its non-empty rows (and
/// at least [`SYSTEM_DUPLICATE_MIN_ROWS`] of them), it holds the same value as
/// one other field of the same row whose name is not a system one.
fn is_system_duplicate(column: &Column<'_>, columns: &[Column<'_>]) -> bool {
    if !is_system_field(&column.field.name) {
        return false;
    }
    let rows: Vec<usize> = (0..column.values.len())
        .filter(|&row| !column.values[row].trim().is_empty())
        .collect();
    if rows.len() < SYSTEM_DUPLICATE_MIN_ROWS {
        return false;
    }
    columns
        .iter()
        .filter(|other| other.field_idx != column.field_idx && !is_system_field(&other.field.name))
        .any(|other| {
            let same = rows
                .iter()
                .filter(|&&row| other.values.get(row) == column.values.get(row))
                .count();
            same * 5 >= rows.len() * 4
        })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_resource_path() {
        for path in [
            "Picture/title.png",
            "title.PNG",
            "se001.ogg",
            "bgm.mp3",
            "voice.WAV",
            "MS Gothic.ttf",
            "movie.webm",
        ] {
            assert!(is_resource_path(path), "{path:?} should be a resource path");
        }

        for text in ["こんにちは", "Hello, world!", "Lv.5", "Mr. Smith"] {
            assert!(
                !is_resource_path(text),
                "{text:?} should not be a resource path"
            );
        }
    }

    /// Plan UW, UW-4: a path followed by numeric lines is a path; a sentence
    /// that ends with a file name is text.
    #[test]
    fn test_is_resource_path_with_numeric_lines() {
        for path in [
            "bg/スラム.png\n0\n0\n0",
            "bg/スラム.png\r\n0\r\n0\r\n1",
            "icon.png",
            "Picture\\顔\\主人公.png",
            "bg/title.png  \n\n-1\n",
        ] {
            assert!(is_resource_path(path), "{path:?} should be a resource path");
        }

        for text in [
            "ファイル名はicon.png",
            "「…icon031.pngみたいな…",
            "画像はこちら：pic.png",
            "セリス\n見てbg.png",
            "bg/スラム 2.png",
            "bg/スラム　2.png",
            "見て：bg/a.png",
            "0\n0",
            "",
        ] {
            assert!(
                !is_resource_path(text),
                "{text:?} should not be a resource path"
            );
        }
    }

    #[test]
    fn test_is_translatable_skips_resource_paths() {
        assert!(!is_translatable("Picture/title.png"));
        assert!(!is_translatable("se001.ogg"));
        assert!(!is_translatable("bg/スラム.png\n0\n0\n0"));
        assert!(is_translatable("ファイル名はicon.png"));
        assert!(is_translatable("こんにちは"));
    }

    /// WF-D5: every debug fragment and prefix, regardless of case.
    #[test]
    fn test_debug_names_are_skipped() {
        for name in [
            "SampleMapA",
            "SAMPLE",
            "DebugRoom",
            "debug",
            "Test01",
            "マップTEST",
            "ﾃﾞﾊﾞｯｸﾞ用",
            "デバッグ部屋",
            "ﾃｽﾄ",
            "テスト用イベント",
            "使用禁止_旧イベント",
            "[D]全回復",
            "[d]全回復",
            "※旧仕様",
        ] {
            assert!(skip_map(name), "map {name:?} should be skipped");
            assert!(skip_common_event(name), "event {name:?} should be skipped");
        }
        for name in ["Map001", "TitleMap", "町の広場", "戦闘開始", "A[D]", "注※"] {
            assert!(!skip_map(name), "map {name:?} should be kept");
            assert!(!skip_common_event(name), "event {name:?} should be kept");
        }
    }

    /// The engine-internal and archived events stay skipped (moved rule).
    #[test]
    fn test_internal_common_events_are_skipped() {
        assert!(skip_common_event("X[移動]歩行"));
        assert!(skip_common_event("zz旧イベント"));
        assert!(!skip_common_event("イベントX["));
        // The map rule only looks for debug names.
        assert!(!skip_map("X[移動]歩行"));
    }

    /// WF-D5: the strict error pattern.
    #[test]
    fn test_error_messages_are_skipped() {
        for text in [
            "ｴﾗｰ!!!",
            "ｴﾗｰ：ﾃﾞｰﾀがありません",
            "データ読込ｴﾗｰ",
            "エラー!",
            "エラー：番号が範囲外です",
            "エラー　！",
            "警告!",
            "警告 : 変数が不正です",
            "警告：セーブできません",
            "\\c[2]エラー:\\c[0] 値が不正",
        ] {
            assert!(skip_message(text), "{text:?} should be skipped");
        }
    }

    /// The two story lines of とのかじ that the broad pattern (`警告` alone)
    /// would have skipped stay translatable.
    #[test]
    fn test_story_lines_with_warning_are_kept() {
        for text in [
            "警告、彼女の魔力が急激に上昇しています。",
            "…迷いの森の警告柵の奥に向かった人が…",
        ] {
            assert!(!skip_message(text), "{text:?} must stay translatable");
        }
        for text in ["エラーが発生しました", "警告する！", "こんにちは"] {
            assert!(!skip_message(text), "{text:?} must stay translatable");
        }
    }

    /// A database type whose fields are all strings: `rows[i][j]` is the value
    /// of field `j` in row `i`.
    fn string_type(field_names: &[&str], rows: &[&[&str]]) -> DatType {
        use super::super::dat_parser::DatEntry;

        DatType {
            name: "Type".to_string(),
            unknown1: 0,
            unknown1_string: None,
            fields: field_names
                .iter()
                .enumerate()
                .map(|(i, name)| DatField {
                    name: (*name).to_string(),
                    index_info: 0x07D0 + i as u32,
                })
                .collect(),
            entries: rows
                .iter()
                .map(|row| DatEntry {
                    name: String::new(),
                    int_values: Vec::new(),
                    string_values: row.iter().map(|value| (*value).to_string()).collect(),
                })
                .collect(),
        }
    }

    fn names(list: &[&str]) -> HashSet<String> {
        list.iter().map(|name| (*name).to_string()).collect()
    }

    /// WF-D5: a field of common event names (lists and numbers included) is
    /// skipped; the field of displayed names next to it stays.
    #[test]
    fn test_common_event_name_field_is_skipped() {
        let dat_type = string_type(
            &["名前", "[ｺﾓﾝｲﾍﾞﾝﾄ]呼び出しｲﾍﾞﾝﾄ名"],
            &[
                &["回復薬", "アイテム使用"],
                &["毒消し", "状態回復,アイテム使用"],
                &["地図", "0"],
                &["鍵", ""],
                &["手紙", "読む|閉じる"],
            ],
        );
        let ce_names = names(&["アイテム使用", "状態回復", "読む", "閉じる"]);
        assert_eq!(skipped_fields(&dat_type, &ce_names), HashSet::from([1]));
        // Without the common events, nothing is skipped.
        assert!(skipped_fields(&dat_type, &HashSet::new()).is_empty());
    }

    /// WF-D5: at least half of the non-empty values; numbers alone do not
    /// make a field of common event names.
    #[test]
    fn test_common_event_name_field_thresholds() {
        let ce_names = names(&["アイテム使用"]);
        let half = string_type(&["呼び出し"], &[&["アイテム使用"], &["回復する"], &[""]]);
        assert_eq!(skipped_fields(&half, &ce_names), HashSet::from([0]));
        let below = string_type(
            &["呼び出し"],
            &[&["アイテム使用"], &["回復する"], &["毒を消す"]],
        );
        assert!(skipped_fields(&below, &ce_names).is_empty());
        let numbers = string_type(&["番号"], &[&["1"], &["2"], &["-1"]]);
        assert!(skipped_fields(&numbers, &ce_names).is_empty());
    }

    /// WF-D5: `ｼｽﾃﾑ名` repeats `ｴﾝﾁｬﾝﾄ名` in at least 80 % of the rows: only
    /// `ｼｽﾃﾑ名` is skipped.
    #[test]
    fn test_system_duplicate_field_is_skipped() {
        let dat_type = string_type(
            &["ｼｽﾃﾑ名", "ｴﾝﾁｬﾝﾄ名"],
            &[
                &["炎の加護", "炎の加護"],
                &["氷の加護", "氷の加護"],
                &["雷の加護", "雷の加護"],
                &["風の加護", "風の加護"],
                &["内部用", "光の加護"],
                &["", "闇の加護"],
            ],
        );
        assert_eq!(
            skipped_fields(&dat_type, &HashSet::new()),
            HashSet::from([0])
        );
    }

    /// WF-D5: below 80 %, fewer than 3 rows, or a non-system name: kept.
    #[test]
    fn test_system_duplicate_thresholds() {
        let below = string_type(
            &["ｼｽﾃﾑ名", "名前"],
            &[
                &["炎", "炎"],
                &["氷", "氷"],
                &["雷", "雷"],
                &["風", "かぜ"],
                &["光", "ひかり"],
            ],
        );
        assert!(skipped_fields(&below, &HashSet::new()).is_empty());
        let two_rows = string_type(&["Sys名", "名前"], &[&["炎", "炎"], &["氷", "氷"]]);
        assert!(skipped_fields(&two_rows, &HashSet::new()).is_empty());
        // Two system fields repeating each other: neither is a duplicate of a
        // displayed field.
        let both_system = string_type(
            &["ｼｽﾃﾑ名", "システム名"],
            &[&["炎", "炎"], &["氷", "氷"], &["雷", "雷"]],
        );
        assert!(skipped_fields(&both_system, &HashSet::new()).is_empty());
        // A displayed field repeating a system field is not skipped.
        let displayed = string_type(
            &["表示名", "Sys名"],
            &[&["炎", "炎"], &["氷", "氷"], &["雷", "雷"]],
        );
        assert_eq!(
            skipped_fields(&displayed, &HashSet::new()),
            HashSet::from([1])
        );
    }

    /// Only string fields count: the field indices skip the int fields.
    #[test]
    fn test_skipped_fields_use_field_indices() {
        let mut dat_type = string_type(
            &["ｼｽﾃﾑ名", "名前"],
            &[&["炎", "炎"], &["氷", "氷"], &["雷", "雷"]],
        );
        dat_type.fields.insert(
            0,
            DatField {
                name: "ｼｽﾃﾑ番号".to_string(),
                index_info: 0x03E8,
            },
        );
        assert_eq!(
            skipped_fields(&dat_type, &HashSet::new()),
            HashSet::from([1])
        );
    }

    /// A field named like an earlier skipped field is skipped too: its key
    /// would resolve to the skipped field at export. A same-named field placed
    /// before the skipped one keeps its key and stays.
    #[test]
    fn test_field_named_like_an_earlier_skipped_field_is_skipped() {
        let ce_names = names(&["アイテム使用"]);
        let dat_type = string_type(
            &["説明", "呼び出し", "呼び出し"],
            &[
                &["回復する", "アイテム使用", "回復します"],
                &["毒を消す", "アイテム使用", "毒を消します"],
            ],
        );
        assert_eq!(skipped_fields(&dat_type, &ce_names), HashSet::from([1, 2]));

        let kept_first = string_type(
            &["呼び出し", "説明", "呼び出し"],
            &[
                &["回復します", "回復する", "アイテム使用"],
                &["毒を消します", "毒を消す", "アイテム使用"],
            ],
        );
        assert_eq!(skipped_fields(&kept_first, &ce_names), HashSet::from([2]));
    }

    /// `skip_message` keeps the rules of `is_translatable`.
    #[test]
    fn test_skip_message_keeps_the_translatable_rules() {
        assert!(skip_message(""));
        assert!(skip_message("123"));
        assert!(skip_message("Picture/title.png"));
        assert!(!skip_message("ファイル名はicon.png"));
    }
}
