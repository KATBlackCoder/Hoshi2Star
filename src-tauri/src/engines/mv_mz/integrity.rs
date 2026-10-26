//! Export integrity gate for RPG Maker MV/MZ.
//!
//! Every file an export produces is compared with the pristine original it was
//! derived from. The only differences allowed are the edits the export
//! *planned* (a JSON pointer → final string value). Anything else — a changed
//! number, a vanished key, a string the plan never mentioned — means an
//! injector bug and aborts the export before the archive is written.

use std::collections::HashMap;
use std::fmt;

use serde::Serialize;
use serde_json::Value;

/// Issues listed per file before the gate stops collecting (the export is
/// rejected either way; this only bounds the error message).
const MAX_ISSUES_PER_FILE: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum IntegrityIssue {
    /// The output (or original) does not parse.
    Unparseable { file: String, reason: String },
    /// A key, array length, type or non-string value differs.
    StructureChanged {
        file: String,
        pointer: String,
        reason: String,
    },
    /// A string changed that no planned edit covers.
    UnplannedChange { file: String, pointer: String },
    /// A planned edit is absent from the output.
    PlannedValueMissing { file: String, pointer: String },
    /// The original value no longer matches what was extracted (or a fill-in
    /// target is no longer empty): the edit would overwrite unknown text.
    OriginalChanged { file: String, pointer: String },
    /// The original value at a translated pointer is no longer the source
    /// recorded at extraction: the game changed since (an update that
    /// inserted a command shifts every later pointer of the page). Exporting
    /// would write the translation over unrelated text. `found` is `None`
    /// when the pointer no longer exists.
    SourceChanged {
        file: String,
        pointer: String,
        expected: String,
        found: Option<String>,
    },
    /// A cross-file reference (e.g. a battle-command tag looking up a skill
    /// type by name) would resolve differently after export.
    LinkedReferenceChanged {
        file: String,
        pointer: String,
        detail: String,
    },
}

impl fmt::Display for IntegrityIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unparseable { file, reason } => {
                write!(f, "{file}: output does not parse ({reason})")
            }
            Self::StructureChanged {
                file,
                pointer,
                reason,
            } => write!(f, "{file} {pointer}: structure changed ({reason})"),
            Self::UnplannedChange { file, pointer } => {
                write!(f, "{file} {pointer}: unplanned text change")
            }
            Self::OriginalChanged { file, pointer } => write!(
                f,
                "{file} {pointer}: the original text no longer matches what was extracted"
            ),
            // Strings are JSON-quoted so the line stays one line and the
            // frontend (`parseSourceChange`) can read them back exactly.
            Self::SourceChanged {
                file,
                pointer,
                expected,
                found,
            } => {
                let quote = |text: &str| Value::String(text.to_string()).to_string();
                let found = found
                    .as_deref()
                    .map_or_else(|| "nothing".to_string(), quote);
                write!(
                    f,
                    "{file} {pointer}: source changed since extraction; expected {}, found {found}",
                    quote(expected)
                )
            }
            Self::LinkedReferenceChanged {
                file,
                pointer,
                detail,
            } => write!(f, "{file} {pointer}: {detail}"),
            Self::PlannedValueMissing { file, pointer } => {
                write!(
                    f,
                    "{file} {pointer}: planned translation missing from output"
                )
            }
        }
    }
}

/// Check, before injecting, that every pointer about to be translated still
/// holds in `original` the source text recorded at extraction (`sources`:
/// pointer → source). Any other value means the game changed since: the
/// translation belongs elsewhere, so the export must not write it.
pub fn verify_sources<'a>(
    file: &str,
    original: &Value,
    sources: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Vec<IntegrityIssue> {
    sources
        .into_iter()
        .filter_map(|(pointer, source)| {
            let found = original.pointer(pointer);
            if found.and_then(Value::as_str) == Some(source) {
                return None;
            }
            Some(IntegrityIssue::SourceChanged {
                file: file.to_string(),
                pointer: pointer.to_string(),
                expected: source.to_string(),
                found: found.map(|value| match value {
                    Value::String(text) => text.clone(),
                    other => other.to_string(),
                }),
            })
        })
        .collect()
}

/// Verify that `output` differs from `original` exactly by `planned` edits.
pub fn verify_json_edit(
    file: &str,
    original: &str,
    output: &[u8],
    planned: &HashMap<String, String>,
) -> Vec<IntegrityIssue> {
    let parse = |bytes: &[u8]| serde_json::from_slice::<Value>(bytes);
    let before = match parse(original.as_bytes()) {
        Ok(value) => value,
        Err(error) => {
            return vec![IntegrityIssue::Unparseable {
                file: file.to_string(),
                reason: format!("original: {error}"),
            }]
        }
    };
    let after = match parse(output) {
        Ok(value) => value,
        Err(error) => {
            return vec![IntegrityIssue::Unparseable {
                file: file.to_string(),
                reason: error.to_string(),
            }]
        }
    };

    let mut walker = Walker {
        file,
        planned,
        issues: Vec::new(),
    };
    walker.walk(&before, &after, &mut String::new());
    for (pointer, value) in planned {
        if walker.issues.len() >= MAX_ISSUES_PER_FILE {
            break;
        }
        if after.pointer(pointer).and_then(Value::as_str) != Some(value.as_str()) {
            walker.issues.push(IntegrityIssue::PlannedValueMissing {
                file: file.to_string(),
                pointer: pointer.clone(),
            });
        }
    }
    walker.issues
}

struct Walker<'a> {
    file: &'a str,
    planned: &'a HashMap<String, String>,
    issues: Vec<IntegrityIssue>,
}

impl Walker<'_> {
    fn structure(&mut self, pointer: &str, reason: impl Into<String>) {
        if self.issues.len() < MAX_ISSUES_PER_FILE {
            self.issues.push(IntegrityIssue::StructureChanged {
                file: self.file.to_string(),
                pointer: pointer.to_string(),
                reason: reason.into(),
            });
        }
    }

    fn walk(&mut self, before: &Value, after: &Value, pointer: &mut String) {
        if self.issues.len() >= MAX_ISSUES_PER_FILE {
            return;
        }
        match (before, after) {
            (Value::Object(a), Value::Object(b)) => {
                if a.len() != b.len() || a.keys().any(|key| !b.contains_key(key)) {
                    self.structure(pointer, "object keys differ");
                    return;
                }
                for (key, value) in a {
                    let len = pointer.len();
                    pointer.push('/');
                    pointer.push_str(&key.replace('~', "~0").replace('/', "~1"));
                    self.walk(value, &b[key], pointer);
                    pointer.truncate(len);
                }
            }
            (Value::Array(a), Value::Array(b)) => {
                if a.len() != b.len() {
                    self.structure(pointer, format!("array length {} → {}", a.len(), b.len()));
                    return;
                }
                for (index, (left, right)) in a.iter().zip(b).enumerate() {
                    let len = pointer.len();
                    pointer.push('/');
                    pointer.push_str(&index.to_string());
                    self.walk(left, right, pointer);
                    pointer.truncate(len);
                }
            }
            (Value::String(a), Value::String(b)) => {
                if a != b && self.planned.get(pointer.as_str()) != Some(b) {
                    self.issues.push(IntegrityIssue::UnplannedChange {
                        file: self.file.to_string(),
                        pointer: pointer.clone(),
                    });
                }
            }
            (a, b) if std::mem::discriminant(a) == std::mem::discriminant(b) => {
                if a != b {
                    self.structure(pointer, "non-text value changed");
                }
            }
            _ => self.structure(pointer, "value type changed"),
        }
    }
}

/// One planned `plugins.js` edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedPluginEdit {
    pub plugin: String,
    pub occurrence: u32,
    pub path: Vec<String>,
    pub value: String,
    /// What the original must hold for the edit to be safe (the extracted
    /// source, or `""` for a fill-in).
    pub expected_original: String,
}

impl PlannedPluginEdit {
    fn pointer(&self) -> String {
        crate::engines::mv_mz::plugins_js::plugin_key(&self.plugin, self.occurrence, &self.path)
    }
}

/// Verify an edited `plugins.js` against its original: text outside the
/// `$plugins` array identical, same plugins in the same order with the same
/// status, untouched entries byte-identical, and in edited entries every
/// parameter equal to the original except the planned leaves.
pub fn verify_plugins_edit(
    file: &str,
    original: &str,
    output: &str,
    planned: &[PlannedPluginEdit],
) -> Vec<IntegrityIssue> {
    use crate::engines::mv_mz::plugins_js::PluginsJs;

    let structure = |pointer: &str, reason: &str| IntegrityIssue::StructureChanged {
        file: file.to_string(),
        pointer: pointer.to_string(),
        reason: reason.to_string(),
    };
    let before = match PluginsJs::parse(original) {
        Ok(parsed) => parsed,
        Err(error) => {
            return vec![IntegrityIssue::Unparseable {
                file: file.to_string(),
                reason: format!("original: {error}"),
            }]
        }
    };
    let after = match PluginsJs::parse(output) {
        Ok(parsed) => parsed,
        Err(error) => {
            return vec![IntegrityIssue::Unparseable {
                file: file.to_string(),
                reason: error.to_string(),
            }]
        }
    };

    let mut issues = Vec::new();
    if before.prefix() != after.prefix() || before.suffix() != after.suffix() {
        issues.push(structure(
            "$plugins",
            "text outside the plugin list changed",
        ));
    }
    if before.entries().len() != after.entries().len() {
        issues.push(structure("$plugins", "plugin count changed"));
        return issues;
    }
    for (old, new) in before.entries().iter().zip(after.entries()) {
        let pointer = format!("plugin #{} {}", old.index, old.name);
        if old.name != new.name || old.status != new.status || old.occurrence != new.occurrence {
            issues.push(structure(&pointer, "plugin name, order or status changed"));
            continue;
        }
        let edits: Vec<&PlannedPluginEdit> = planned
            .iter()
            .filter(|edit| edit.plugin == old.name && edit.occurrence == old.occurrence)
            .collect();
        if edits.is_empty() {
            if before.entry_text(old) != after.entry_text(new) {
                issues.push(IntegrityIssue::UnplannedChange {
                    file: file.to_string(),
                    pointer,
                });
            }
            continue;
        }
        let old_leaves = old.parameter_leaves();
        let new_leaves = new.parameter_leaves();
        if old_leaves.len() != new_leaves.len() {
            issues.push(structure(&pointer, "parameter structure changed"));
            continue;
        }
        for ((old_path, old_value), (new_path, new_value)) in old_leaves.iter().zip(&new_leaves) {
            if old_path != new_path {
                issues.push(structure(&pointer, "parameter structure changed"));
                break;
            }
            match edits.iter().find(|edit| edit.path == *old_path) {
                Some(edit) => {
                    if *old_value != edit.expected_original {
                        issues.push(IntegrityIssue::OriginalChanged {
                            file: file.to_string(),
                            pointer: edit.pointer(),
                        });
                    }
                    if *new_value != edit.value {
                        issues.push(IntegrityIssue::PlannedValueMissing {
                            file: file.to_string(),
                            pointer: edit.pointer(),
                        });
                    }
                }
                None if old_value != new_value => issues.push(IntegrityIssue::UnplannedChange {
                    file: file.to_string(),
                    pointer: format!("{pointer} {}", old_path.join(".")),
                }),
                None => {}
            }
        }
        for edit in &edits {
            if !old_leaves.iter().any(|(path, _)| *path == edit.path) {
                issues.push(IntegrityIssue::PlannedValueMissing {
                    file: file.to_string(),
                    pointer: edit.pointer(),
                });
            }
        }
    }
    issues
}

/// Re-run Keke_VariableActorCommand's tag resolution on the original and the
/// exported data: every command tag must resolve to the same command (symbol,
/// skill-type index / skill id / item id, removal flag). `files` holds
/// `(file label, original JSON, output JSON)` of every data file with notes.
pub fn verify_keke_resolution(
    original_skill_types: &[String],
    output_skill_types: &[String],
    files: &[(String, Value, Value)],
) -> Vec<IntegrityIssue> {
    use crate::engines::mv_mz::keke::resolve;

    let mut issues = Vec::new();
    for (file, before, after) in files {
        let entries = before.as_array().into_iter().flatten().enumerate();
        for (index, entry) in entries {
            let Some(note) = entry.get("note").and_then(Value::as_str) else {
                continue;
            };
            let pointer = format!("/{index}/note");
            let output_note = after
                .pointer(&pointer)
                .and_then(Value::as_str)
                .unwrap_or("");
            let expected = resolve(note, original_skill_types);
            let actual = resolve(output_note, output_skill_types);
            if expected != actual {
                issues.push(IntegrityIssue::LinkedReferenceChanged {
                    file: file.clone(),
                    pointer,
                    detail: format!(
                        "{} command tag(s) would resolve differently",
                        expected.len().max(actual.len())
                    ),
                });
            }
        }
    }
    issues
}

/// Render a bounded, human-readable summary for an export error.
pub fn summarize(issues: &[IntegrityIssue]) -> String {
    const SHOWN: usize = 10;
    let mut lines: Vec<String> = issues.iter().take(SHOWN).map(ToString::to_string).collect();
    if issues.len() > SHOWN {
        lines.push(format!("… and {} more", issues.len() - SHOWN));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn unchanged_sources_pass_and_shifted_ones_are_listed() {
        let original: Value = serde_json::from_str(
            r#"{"list":[{"code":101,"parameters":["Actor2",0,0,2]},{"code":401,"parameters":["新しい"]}]}"#,
        )
        .unwrap();
        let issues = verify_sources(
            "www/data/Map001.json",
            &original,
            [
                ("/list/1/parameters/0", "新しい"),
                ("/list/0/parameters/0", "こんばんは"),
                ("/list/1/parameters/4", "村人"),
                ("/list/0/parameters/1", "北へ"),
            ],
        );
        let changed =
            |pointer: &str, expected: &str, found: Option<&str>| IntegrityIssue::SourceChanged {
                file: "www/data/Map001.json".into(),
                pointer: pointer.into(),
                expected: expected.into(),
                found: found.map(Into::into),
            };
        assert_eq!(
            issues,
            vec![
                changed("/list/0/parameters/0", "こんばんは", Some("Actor2")),
                changed("/list/1/parameters/4", "村人", None),
                changed("/list/0/parameters/1", "北へ", Some("0")),
            ]
        );
    }

    #[test]
    fn source_changed_renders_one_quoted_line() {
        let issue = IntegrityIssue::SourceChanged {
            file: "data/Map001.json".into(),
            pointer: "/events/1/pages/0/list/3/parameters/0".into(),
            expected: "「行く」\"よ\"\n".into(),
            found: Some("Actor1".into()),
        };
        assert_eq!(
            issue.to_string(),
            r#"data/Map001.json /events/1/pages/0/list/3/parameters/0: source changed since extraction; expected "「行く」\"よ\"\n", found "Actor1""#
        );
        let gone = IntegrityIssue::SourceChanged {
            file: "data/Map001.json".into(),
            pointer: "/events/1/pages/0/list/2/parameters/4".into(),
            expected: "旅人".into(),
            found: None,
        };
        assert!(gone.to_string().ends_with(", found nothing"));
        assert_eq!(summarize(&[gone]).lines().count(), 1);
    }

    #[test]
    fn planned_edits_pass() {
        let original = r#"{"a":"勇者","list":[{"t":"はい"},{"t":"x"}],"n":1}"#;
        let output = br#"{"a":"Hero","list":[{"t":"Yes"},{"t":"x"}],"n":1}"#;
        let issues = verify_json_edit(
            "Map.json",
            original,
            output,
            &plan(&[("/a", "Hero"), ("/list/0/t", "Yes")]),
        );
        assert!(issues.is_empty(), "{issues:?}");
    }

    #[test]
    fn unplanned_text_and_structure_changes_are_caught() {
        let original = r#"{"a":"勇者","b":"剣","n":1,"arr":[1,2],"k":"x"}"#;
        let output = br#"{"a":"Hero","b":"Sword","n":2,"arr":[1],"k":1}"#;
        let issues = verify_json_edit("X.json", original, output, &plan(&[("/a", "Hero")]));
        assert!(issues.contains(&IntegrityIssue::UnplannedChange {
            file: "X.json".into(),
            pointer: "/b".into()
        }));
        assert!(issues.iter().any(
            |i| matches!(i, IntegrityIssue::StructureChanged { pointer, .. } if pointer == "/n")
        ));
        assert!(issues.iter().any(
            |i| matches!(i, IntegrityIssue::StructureChanged { pointer, .. } if pointer == "/arr")
        ));
        assert!(issues.iter().any(
            |i| matches!(i, IntegrityIssue::StructureChanged { pointer, .. } if pointer == "/k")
        ));
    }

    #[test]
    fn missing_planned_value_and_key_changes_are_caught() {
        let issues = verify_json_edit(
            "X.json",
            r#"{"a":"勇者","b":"剣"}"#,
            r#"{"a":"勇者","c":"剣"}"#.as_bytes(),
            &plan(&[("/a", "Hero")]),
        );
        assert!(issues
            .iter()
            .any(|i| matches!(i, IntegrityIssue::StructureChanged { .. })));
        assert!(issues.contains(&IntegrityIssue::PlannedValueMissing {
            file: "X.json".into(),
            pointer: "/a".into()
        }));
    }

    #[test]
    fn pointer_escaping_matches_rfc6901() {
        let issues = verify_json_edit(
            "X.json",
            r#"{"a/b":{"c~d":"元"}}"#,
            br#"{"a/b":{"c~d":"New"}}"#,
            &plan(&[("/a~1b/c~0d", "New")]),
        );
        assert!(issues.is_empty(), "{issues:?}");
    }

    const PLUGINS: &str = "// header\nvar $plugins =\n[\n{\"name\":\"A\",\"status\":true,\"description\":\"\",\"parameters\":{\"Cmd\":\"図鑑\",\"Key\":\"HP消費\",\"Row\":\"\"}},\n{\"name\":\"B\",\"status\":false,\"description\":\"\",\"parameters\":{\"x\":\"未\"}}\n];\n";

    fn edit(path: &str, value: &str, expected: &str) -> PlannedPluginEdit {
        PlannedPluginEdit {
            plugin: "A".into(),
            occurrence: 1,
            path: vec![path.into()],
            value: value.into(),
            expected_original: expected.into(),
        }
    }

    fn apply(edits: &[(&str, &str)]) -> String {
        let plugins = crate::engines::mv_mz::plugins_js::PluginsJs::parse(PLUGINS).unwrap();
        plugins
            .apply_edits(
                &edits
                    .iter()
                    .map(|(p, v)| ("A".to_string(), 1, vec![p.to_string()], v.to_string()))
                    .collect::<Vec<_>>(),
            )
            .unwrap()
    }

    #[test]
    fn planned_plugin_edits_pass_including_fill_ins() {
        let output = apply(&[("Cmd", "Bestiary"), ("Row", "Kills")]);
        let issues = verify_plugins_edit(
            "js/plugins.js",
            PLUGINS,
            &output,
            &[edit("Cmd", "Bestiary", "図鑑"), edit("Row", "Kills", "")],
        );
        assert!(issues.is_empty(), "{issues:?}");
    }

    #[test]
    fn unplanned_plugin_changes_and_drift_are_caught() {
        // A parameter nobody planned changed (e.g. a note-tag key).
        let output = apply(&[("Cmd", "Bestiary"), ("Key", "HP cost")]);
        let issues = verify_plugins_edit(
            "js/plugins.js",
            PLUGINS,
            &output,
            &[edit("Cmd", "Bestiary", "図鑑")],
        );
        assert!(issues
            .iter()
            .any(|i| matches!(i, IntegrityIssue::UnplannedChange { pointer, .. } if pointer.ends_with("Key"))));

        // The original no longer holds the extracted source.
        let output = apply(&[("Cmd", "Bestiary")]);
        let issues = verify_plugins_edit(
            "js/plugins.js",
            PLUGINS,
            &output,
            &[edit("Cmd", "Bestiary", "別の文字")],
        );
        assert!(issues
            .iter()
            .any(|i| matches!(i, IntegrityIssue::OriginalChanged { .. })));

        // An untouched plugin entry and the header must stay byte-identical.
        let tampered = PLUGINS.replace("\"status\":false", "\"status\":false ");
        let issues = verify_plugins_edit("js/plugins.js", PLUGINS, &tampered, &[]);
        assert!(issues
            .iter()
            .any(|i| matches!(i, IntegrityIssue::UnplannedChange { pointer, .. } if pointer.contains('B'))));
        let header = PLUGINS.replace("// header", "// changed");
        assert!(!verify_plugins_edit("js/plugins.js", PLUGINS, &header, &[]).is_empty());
    }

    #[test]
    fn keke_resolution_catches_a_tag_left_behind_by_a_rename() {
        let types = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        let original = serde_json::json!([null, {"note": "<コマンド:スキル/鞭,48,鞭攻撃,76>"}]);
        let rewritten =
            serde_json::json!([null, {"note": "<コマンド:スキル/Whip,48,WhipAttack,76>"}]);
        let files_ok = vec![("data/Weapons.json".to_string(), original.clone(), rewritten)];
        assert!(
            verify_keke_resolution(&types(&["", "鞭"]), &types(&["", "Whip"]), &files_ok)
                .is_empty()
        );

        // System.json renamed but the tag still says 鞭: the command would vanish.
        let files_stale = vec![("data/Weapons.json".to_string(), original.clone(), original)];
        let issues =
            verify_keke_resolution(&types(&["", "鞭"]), &types(&["", "Whip"]), &files_stale);
        assert!(matches!(
            issues.as_slice(),
            [IntegrityIssue::LinkedReferenceChanged { pointer, .. }] if pointer == "/1/note"
        ));
    }

    #[test]
    fn unparseable_output_is_reported() {
        let issues = verify_json_edit("X.json", "{}", b"{", &HashMap::new());
        assert!(matches!(
            issues.as_slice(),
            [IntegrityIssue::Unparseable { .. }]
        ));
    }
}
