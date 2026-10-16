//! Keke_VariableActorCommand note-tag grammar.
//!
//! The plugin builds each actor's battle commands from note tags:
//!
//! ```text
//! <コマンド:TYPE,order,label,icon,help>     (also <cmd:…>, and <command:…> on classes)
//! ```
//!
//! Reproduced from the reviewed plugin source (`makeCommandListFree`,
//! `totalAllMetaArray`, `metaAll`):
//! - tags match `<word:([^>]*)>` case-insensitively;
//! - **all** JavaScript `\s` whitespace is removed from the content first;
//! - the content is split on `,`;
//! - `TYPE` picks a symbol by substring (`攻撃`/`attack`, `防御`/`guard`,
//!   `スキル`/`skill`, `アイテム`/`item`, `逃げる`/`escape`, `オート`/`auto`;
//!   later checks win), a leading `-` removes the command;
//! - for skills, `TYPE.split("/")[1]` containing an ASCII digit anywhere is a
//!   skill **id**; otherwise it is a skill-type **name** resolved with
//!   `$dataSystem.skillTypes.indexOf(name) || 1` — an unknown name silently
//!   drops the command.
//!
//! Hoshi2Star uses this to (a) offer the visible labels for translation,
//! (b) rewrite skill-type names inside the tags when skill types are
//! translated, and (c) prove after export that every tag still resolves to
//! the same command.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::LazyLock;

use regex::Regex;

/// Family name used in segment keys and profiles.
pub const FAMILY: &str = "keke_command";
/// Note-tag key prefix for label segments.
pub const NOTETAG_KEY_PREFIX: &str = "notetag:v1:keke-cmd:";
/// Index of the visible label inside the tag.
pub const LABEL_FIELD: usize = 2;
/// Data files whose notes the plugin reads for battle commands.
pub const READ_FILES: &[&str] = &[
    "Actors.json",
    "Classes.json",
    "Weapons.json",
    "Armors.json",
    "States.json",
    "Enemies.json",
];

static RE_TAG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)<(?:コマンド|cmd|command):([^>]*)>").expect("RE_TAG must compile")
});
static RE_ASCII_DIGIT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[0-9]").expect("RE_ASCII_DIGIT must compile"));

/// JavaScript `\s` (ECMAScript WhiteSpace + LineTerminator).
pub fn is_js_whitespace(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r' | ' ' | '\u{A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

pub fn strip_js_whitespace(text: &str) -> String {
    text.chars().filter(|c| !is_js_whitespace(*c)).collect()
}

/// One command tag inside a note, with raw byte spans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    /// 0-based position among command tags of the note.
    pub occurrence: usize,
    /// Raw spans of each `,`-separated field (commas are not whitespace, so
    /// raw and stripped field boundaries coincide).
    pub fields: Vec<Range<usize>>,
}

pub fn parse_tags(note: &str) -> Vec<Tag> {
    RE_TAG
        .captures_iter(note)
        .enumerate()
        .filter_map(|(occurrence, caps)| {
            let content = caps.get(1)?;
            let mut fields = Vec::new();
            let mut start = content.start();
            for (offset, ch) in content.as_str().char_indices() {
                if ch == ',' {
                    fields.push(start..content.start() + offset);
                    start = content.start() + offset + 1;
                }
            }
            fields.push(start..content.end());
            Some(Tag { occurrence, fields })
        })
        .collect()
}

/// Stripped text of field `index`, if present.
pub fn field_text(note: &str, tag: &Tag, index: usize) -> Option<String> {
    tag.fields
        .get(index)
        .map(|span| strip_js_whitespace(&note[span.clone()]))
}

/// A skill-type name referenced by a tag's TYPE field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillTypeRef {
    pub occurrence: usize,
    /// Stripped name, as compared with `skillTypes`.
    pub name: String,
    /// Raw span of the name inside the note.
    pub span: Range<usize>,
}

/// Skill-type name referenced by `tag`, if TYPE is a skill command whose
/// second `/` part has no ASCII digit.
pub fn skill_type_ref(note: &str, tag: &Tag) -> Option<SkillTypeRef> {
    let field = tag.fields.first()?;
    let raw = &note[field.clone()];
    let stripped = strip_js_whitespace(raw);
    if symbol_of(&stripped) != Some("skill") {
        return None;
    }
    // Raw span of the second `/`-separated part (slashes are not whitespace).
    let first_slash = raw.find('/')?;
    let rest = &raw[first_slash + 1..];
    let end = rest.find('/').unwrap_or(rest.len());
    let span = field.start + first_slash + 1..field.start + first_slash + 1 + end;
    let name = strip_js_whitespace(&note[span.clone()]);
    if name.is_empty() || RE_ASCII_DIGIT.is_match(&name) {
        return None;
    }
    Some(SkillTypeRef {
        occurrence: tag.occurrence,
        name,
        span,
    })
}

/// Every skill-type reference in `note`.
pub fn skill_type_refs(note: &str) -> Vec<SkillTypeRef> {
    parse_tags(note)
        .iter()
        .filter_map(|tag| skill_type_ref(note, tag))
        .collect()
}

fn symbol_of(stripped_type: &str) -> Option<&'static str> {
    let mut symbol = None;
    for (needles, name) in [
        (["攻撃", "attack"], "attack"),
        (["防御", "guard"], "guard"),
        (["スキル", "skill"], "skill"),
        (["アイテム", "item"], "item"),
        (["逃げる", "escape"], "escape"),
        (["オート", "auto"], "auto"),
    ] {
        if needles.iter().any(|needle| stripped_type.contains(needle)) {
            symbol = Some(name);
        }
    }
    symbol
}

/// How the plugin resolves one tag (label excluded: it may be translated).
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub occurrence: usize,
    pub symbol: Option<&'static str>,
    /// Skill-type index, skill id or item id; `None` for `NaN`.
    pub id: Option<f64>,
    pub delete: bool,
}

/// Simulate the plugin's parsing of every command tag in `note`.
pub fn resolve(note: &str, skill_types: &[String]) -> Vec<Resolved> {
    parse_tags(note)
        .iter()
        .map(|tag| {
            let stripped_type = field_text(note, tag, 0).unwrap_or_default();
            let symbol = symbol_of(&stripped_type);
            let second = stripped_type
                .split('/')
                .nth(1)
                .filter(|part| !part.is_empty());
            let id = match (symbol, second) {
                (Some("skill"), Some(part)) if RE_ASCII_DIGIT.is_match(part) => {
                    part.parse::<f64>().ok()
                }
                (Some("skill"), Some(part)) => {
                    let index = skill_types
                        .iter()
                        .position(|name| name == part)
                        .map_or(-1.0, |i| i as f64);
                    Some(if index == 0.0 { 1.0 } else { index })
                }
                (Some("item"), Some(part)) => part.parse::<f64>().ok(),
                _ => Some(1.0),
            };
            Resolved {
                occurrence: tag.occurrence,
                symbol,
                id,
                delete: stripped_type.starts_with('-'),
            }
        })
        .collect()
}

/// A label replacement: field [`LABEL_FIELD`] of tag `occurrence`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelEdit {
    pub occurrence: usize,
    /// Stripped value the field must currently hold.
    pub expected: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KekeEditError {
    #[error("command tag #{0} not found in the note")]
    MissingTag(usize),
    #[error("command tag #{occurrence} label is {found:?}, expected {expected:?}")]
    LabelChanged {
        occurrence: usize,
        expected: String,
        found: String,
    },
}

/// Apply label edits and skill-type renames (`old name → new name`) to a
/// note by span surgery; everything else in the note is kept byte-for-byte.
pub fn rewrite_note(
    note: &str,
    labels: &[LabelEdit],
    renames: &HashMap<String, String>,
) -> Result<String, KekeEditError> {
    let tags = parse_tags(note);
    let mut replacements: Vec<(Range<usize>, String)> = Vec::new();
    for edit in labels {
        let tag = tags
            .iter()
            .find(|tag| tag.occurrence == edit.occurrence)
            .ok_or(KekeEditError::MissingTag(edit.occurrence))?;
        let span = tag
            .fields
            .get(LABEL_FIELD)
            .ok_or(KekeEditError::MissingTag(edit.occurrence))?;
        let found = strip_js_whitespace(&note[span.clone()]);
        if found != edit.expected {
            return Err(KekeEditError::LabelChanged {
                occurrence: edit.occurrence,
                expected: edit.expected.clone(),
                found,
            });
        }
        replacements.push((span.clone(), edit.value.clone()));
    }
    for tag in &tags {
        if let Some(reference) = skill_type_ref(note, tag) {
            if let Some(new_name) = renames.get(&reference.name) {
                replacements.push((reference.span, new_name.clone()));
            }
        }
    }
    replacements.sort_by_key(|(span, _)| span.start);
    let mut out = String::with_capacity(note.len());
    let mut cursor = 0;
    for (span, value) in replacements {
        out.push_str(&note[cursor..span.start]);
        out.push_str(&value);
        cursor = span.end;
    }
    out.push_str(&note[cursor..]);
    Ok(out)
}

/// One translated label segment: `(segment key, target, source)`.
pub type LabelRow = (String, String, String);

/// New note texts for one data file: label edits from `label_rows` plus
/// skill-type `renames` applied to every command tag of every note.
/// Returns `(note pointer, new note)` for notes that actually change.
pub fn note_rewrites(
    json: &serde_json::Value,
    label_rows: &[LabelRow],
    renames: &HashMap<String, String>,
) -> Result<Vec<(String, String)>, String> {
    let mut labels_by_note: HashMap<String, Vec<LabelEdit>> = HashMap::new();
    for (key, target, source) in label_rows {
        let (occurrence, field, pointer) =
            parse_label_key(key).ok_or_else(|| format!("invalid note-tag segment key: {key}"))?;
        if field != LABEL_FIELD {
            return Err(format!("unsupported note-tag field in {key}"));
        }
        labels_by_note.entry(pointer).or_default().push(LabelEdit {
            occurrence,
            expected: source.clone(),
            value: target.clone(),
        });
    }
    let mut pointers: Vec<String> = labels_by_note.keys().cloned().collect();
    if !renames.is_empty() {
        for (index, entry) in json.as_array().into_iter().flatten().enumerate() {
            if let Some(note) = entry.get("note").and_then(serde_json::Value::as_str) {
                let pointer = format!("/{index}/note");
                if !pointers.contains(&pointer)
                    && skill_type_refs(note)
                        .iter()
                        .any(|r| renames.contains_key(&r.name))
                {
                    pointers.push(pointer);
                }
            }
        }
    }
    pointers.sort();
    let mut out = Vec::new();
    for pointer in pointers {
        let note = json
            .pointer(&pointer)
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("note {pointer} not found"))?;
        let edits = labels_by_note
            .get(&pointer)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let rewritten =
            rewrite_note(note, edits, renames).map_err(|error| format!("{pointer}: {error}"))?;
        if rewritten != note {
            out.push((pointer, rewritten));
        }
    }
    Ok(out)
}

/// Undo previously exported label edits and skill-type renames in one note
/// (snapshot reconstruction). Only fields holding exactly the exported value
/// are reverted; returns `None` when nothing changes.
pub fn reverse_note_edits(
    note: &str,
    labels: &[(usize, String, String)],
    reverse_renames: &HashMap<String, String>,
) -> Option<String> {
    let tags = parse_tags(note);
    let edits: Vec<LabelEdit> = labels
        .iter()
        .filter_map(|(occurrence, target, source)| {
            let tag = tags.iter().find(|tag| tag.occurrence == *occurrence)?;
            let current = field_text(note, tag, LABEL_FIELD)?;
            (current == *target && target != source).then(|| LabelEdit {
                occurrence: *occurrence,
                expected: target.clone(),
                value: source.clone(),
            })
        })
        .collect();
    let restored = rewrite_note(note, &edits, reverse_renames).ok()?;
    (restored != note).then_some(restored)
}

/// Segment key of a tag label: `notetag:v1:keke-cmd:<occurrence>:<field>:<pointer>`.
pub fn label_key(occurrence: usize, note_pointer: &str) -> String {
    format!("{NOTETAG_KEY_PREFIX}{occurrence}:{LABEL_FIELD}:{note_pointer}")
}

/// Inverse of [`label_key`]: `(occurrence, field, note pointer)`.
pub fn parse_label_key(key: &str) -> Option<(usize, usize, String)> {
    let rest = key.strip_prefix(NOTETAG_KEY_PREFIX)?;
    let mut parts = rest.splitn(3, ':');
    let occurrence = parts.next()?.parse().ok()?;
    let field = parts.next()?.parse().ok()?;
    let pointer = parts.next()?;
    pointer
        .starts_with('/')
        .then(|| (occurrence, field, pointer.to_string()))
}

/// Characters a translated skill-type name may not contain when the plugin
/// looks it up inside tags: whitespace (stripped), tag/field separators and
/// ASCII digits (turn it into a skill id).
pub fn invalid_skill_type_name(name: &str) -> Option<&'static str> {
    if name.is_empty() {
        Some("empty")
    } else if name.chars().any(is_js_whitespace) {
        Some("whitespace")
    } else if name.contains([',', '/', '>', '<', ':']) {
        Some("separator")
    } else if RE_ASCII_DIGIT.is_match(name) {
        Some("digit")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn types(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    const NOTE: &str = "\u{3000}<コマンド:スキル/魔術,49,魔術,76>\n\u{3000}<コマンド:防御,50>\u{3000}\n<cmd:-スキル/ 鞭 ,48,鞭 攻撃,76>\n<Command:スキル/172,10>\n<other:スキル/鞭>";

    #[test]
    fn parses_tags_case_insensitively_with_raw_field_spans() {
        let tags = parse_tags(NOTE);
        assert_eq!(tags.len(), 4);
        assert_eq!(field_text(NOTE, &tags[0], 2).as_deref(), Some("魔術"));
        assert_eq!(field_text(NOTE, &tags[2], 2).as_deref(), Some("鞭攻撃"));
        assert_eq!(field_text(NOTE, &tags[1], 2), None);
        assert_eq!(&NOTE[tags[2].fields[0].clone()], "-スキル/ 鞭 ");
    }

    #[test]
    fn skill_type_refs_skip_ids_and_other_tags() {
        let refs = skill_type_refs(NOTE);
        let names: Vec<_> = refs
            .iter()
            .map(|r| (r.occurrence, r.name.as_str()))
            .collect();
        assert_eq!(names, vec![(0, "魔術"), (2, "鞭")]);
        assert_eq!(&NOTE[refs[1].span.clone()], " 鞭 ");
    }

    #[test]
    fn resolution_matches_the_plugin() {
        let skill_types = types(&["", "魔術", "鞭"]);
        let resolved = resolve(NOTE, &skill_types);
        assert_eq!(resolved[0].symbol, Some("skill"));
        assert_eq!(resolved[0].id, Some(1.0));
        assert_eq!(resolved[1].symbol, Some("guard"));
        assert_eq!(resolved[2].id, Some(2.0));
        assert!(resolved[2].delete);
        assert_eq!(resolved[3].id, Some(172.0));
        // Unknown names resolve to -1: the plugin drops the command.
        let renamed_only_in_system = types(&["", "Magic", "Whip"]);
        assert_eq!(resolve(NOTE, &renamed_only_in_system)[0].id, Some(-1.0));
    }

    #[test]
    fn rewrite_changes_only_targeted_spans() {
        let renames: HashMap<String, String> = [
            ("魔術".to_string(), "Magic".to_string()),
            ("鞭".to_string(), "Whip".to_string()),
        ]
        .into();
        let out = rewrite_note(
            NOTE,
            &[LabelEdit {
                occurrence: 2,
                expected: "鞭攻撃".into(),
                value: "WhipAttack".into(),
            }],
            &renames,
        )
        .unwrap();
        assert_eq!(
            out,
            "\u{3000}<コマンド:スキル/Magic,49,魔術,76>\n\u{3000}<コマンド:防御,50>\u{3000}\n<cmd:-スキル/Whip,48,WhipAttack,76>\n<Command:スキル/172,10>\n<other:スキル/鞭>"
        );
        // Same resolution once skill types are renamed consistently.
        let before = resolve(NOTE, &types(&["", "魔術", "鞭"]));
        let after = resolve(&out, &types(&["", "Magic", "Whip"]));
        assert_eq!(before, after);
    }

    #[test]
    fn rewrite_refuses_a_changed_label() {
        let error = rewrite_note(
            NOTE,
            &[LabelEdit {
                occurrence: 0,
                expected: "別".into(),
                value: "X".into(),
            }],
            &HashMap::new(),
        )
        .unwrap_err();
        assert!(matches!(error, KekeEditError::LabelChanged { .. }));
    }

    #[test]
    fn note_rewrites_cover_labels_and_every_referencing_note_then_reverse() {
        let json = serde_json::json!([
            null,
            {"note": "<コマンド:スキル/鞭,48,鞭攻撃,76>"},
            {"note": "<cmd:-スキル/鞭,48>\n<コマンド:防御,50>"},
            {"note": "no tags"}
        ]);
        let renames: HashMap<String, String> = [("鞭".to_string(), "Whip".to_string())].into();
        let rows = vec![(
            label_key(0, "/1/note"),
            "WhipAttack".to_string(),
            "鞭攻撃".to_string(),
        )];
        let edits = note_rewrites(&json, &rows, &renames).unwrap();
        assert_eq!(
            edits,
            vec![
                (
                    "/1/note".to_string(),
                    "<コマンド:スキル/Whip,48,WhipAttack,76>".to_string()
                ),
                (
                    "/2/note".to_string(),
                    "<cmd:-スキル/Whip,48>\n<コマンド:防御,50>".to_string()
                ),
            ]
        );
        let reverse: HashMap<String, String> = [("Whip".to_string(), "鞭".to_string())].into();
        assert_eq!(
            reverse_note_edits(
                &edits[0].1,
                &[(0, "WhipAttack".into(), "鞭攻撃".into())],
                &reverse
            )
            .as_deref(),
            Some("<コマンド:スキル/鞭,48,鞭攻撃,76>")
        );
        assert_eq!(reverse_note_edits("no tags", &[], &reverse), None);
        // A label that drifted is an error, never a silent overwrite.
        let stale = vec![(label_key(0, "/1/note"), "X".to_string(), "別".to_string())];
        assert!(note_rewrites(&json, &stale, &HashMap::new()).is_err());
    }

    #[test]
    fn label_keys_round_trip() {
        let key = label_key(3, "/28/note");
        assert_eq!(key, "notetag:v1:keke-cmd:3:2:/28/note");
        assert_eq!(parse_label_key(&key), Some((3, 2, "/28/note".to_string())));
        assert_eq!(parse_label_key("/28/note"), None);
    }

    #[test]
    fn skill_type_name_constraints_follow_the_parser() {
        assert_eq!(invalid_skill_type_name("Magic"), None);
        assert_eq!(invalid_skill_type_name("Martial Arts"), Some("whitespace"));
        assert_eq!(invalid_skill_type_name("Arts\u{3000}"), Some("whitespace"));
        assert_eq!(invalid_skill_type_name("Arts\u{A0}X"), Some("whitespace"));
        assert_eq!(invalid_skill_type_name("Sword,Shield"), Some("separator"));
        assert_eq!(invalid_skill_type_name("Tier2"), Some("digit"));
        // Full-width digits do not match the plugin's ASCII /\d+/.
        assert_eq!(invalid_skill_type_name("Tier２"), None);
        assert_eq!(invalid_skill_type_name(""), Some("empty"));
    }
}
