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

use std::sync::LazyLock;

use regex::Regex;

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

    /// `skip_message` keeps the rules of `is_translatable`.
    #[test]
    fn test_skip_message_keeps_the_translatable_rules() {
        assert!(skip_message(""));
        assert!(skip_message("123"));
        assert!(skip_message("Picture/title.png"));
        assert!(!skip_message("ファイル名はicon.png"));
    }
}
