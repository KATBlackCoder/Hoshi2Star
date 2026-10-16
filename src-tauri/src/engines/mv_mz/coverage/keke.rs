//! Keke_VariableActorCommand integration for the coverage scan: battle-command
//! labels inside note tags, and skill-type names referenced by those tags
//! (rewritten consistently when skill types are translated → "linked").

use serde_json::{json, Value};

use super::sites::{excerpt, Site, SiteKind};
use super::{
    category_context, Candidate, CategoryGroup, CategoryState, Constraint, Reference, TextCategory,
};
use crate::engines::filter;
use crate::engines::mv_mz::extractor::SegmentKind;
use crate::engines::mv_mz::keke::{self, LABEL_FIELD};
use crate::llm::tokenizer::Engine as TokEngine;

pub const LABELS_CATEGORY: &str = "notetag.keke_command.labels";
pub const SKILL_TYPES_CATEGORY: &str = "system.skill_types";

/// A skill-type name referenced inside a command tag of some note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeReference {
    pub name: String,
    pub location: String,
    pub excerpt: String,
}

/// Blank out, in every note site, the spans Keke reads as labels (display
/// text) or as skill-type names (handled by the linked rewrite), and return
/// the skill-type references found. Everything else in the note — including
/// the help-name field — stays a generic identifier site.
pub fn mask_note_sites(sites: &mut [Site]) -> Vec<TypeReference> {
    let mut references = Vec::new();
    for site in sites.iter_mut().filter(|site| site.kind == SiteKind::Note) {
        let note = site.text.clone();
        let tags = keke::parse_tags(&note);
        if tags.is_empty() {
            continue;
        }
        let mut masked: Vec<char> = Vec::new();
        let mut spans = Vec::new();
        for tag in &tags {
            if let Some(span) = tag.fields.get(LABEL_FIELD) {
                spans.push(span.clone());
            }
            if let Some(reference) = keke::skill_type_ref(&note, tag) {
                references.push(TypeReference {
                    name: reference.name.clone(),
                    location: site.location.clone(),
                    excerpt: excerpt(&note, reference.span.start, reference.span.len()),
                });
                spans.push(reference.span);
            }
        }
        for (offset, ch) in note.char_indices() {
            let hidden = spans.iter().any(|span| span.contains(&offset));
            masked.push(if hidden { ' ' } else { ch });
        }
        site.text = masked.into_iter().collect();
    }
    references
}

/// Turn a Verified skill-type category whose only references are command-tag
/// names into a Linked one (opt-in, rewritten at export).
pub fn link_skill_types(category: &mut TextCategory, references: &[TypeReference]) {
    if category.key != SKILL_TYPES_CATEGORY || category.state != CategoryState::Verified {
        return;
    }
    let names: Vec<&str> = category
        .candidates
        .iter()
        .map(|c| c.source.as_str())
        .collect();
    let linked: Vec<&TypeReference> = references
        .iter()
        .filter(|reference| names.contains(&reference.name.as_str()))
        .collect();
    if linked.is_empty() {
        return;
    }
    let notes: std::collections::BTreeSet<&str> = linked
        .iter()
        .filter_map(|reference| reference.location.split(' ').next())
        .collect();
    category.state = CategoryState::Linked;
    category.default_enabled = false;
    category.reference_count = linked.len();
    category.references = linked
        .iter()
        .take(50)
        .map(|reference| Reference {
            text: reference.name.clone(),
            site: SiteKind::Note,
            location: reference.location.clone(),
            excerpt: reference.excerpt.clone(),
        })
        .collect();
    category.consequences.push(format!(
        "Keke_VariableActorCommand looks these names up from {} battle-command note tag(s) \
         (in {}). Hoshi2Star rewrites every one of those tags to the translated name in the \
         same export, then re-runs the plugin's lookup on the result: if any command would \
         resolve differently, the export is refused.",
        linked.len(),
        notes.into_iter().collect::<Vec<_>>().join(", ")
    ));
    category.consequences.push(
        "Because the plugin removes spaces and splits on `,` `/` `>` inside tags, and treats any \
         ASCII digit as a skill id, translated skill-type names must be single words without \
         spaces, separators or digits (e.g. \"Whip\", \"MartialArts\"), and all different. QA \
         blocks the export otherwise."
            .to_string(),
    );
    if !category
        .constraints
        .contains(&Constraint::KekeSkillTypeName)
    {
        category.constraints.push(Constraint::KekeSkillTypeName);
    }
    for candidate in &mut category.candidates {
        candidate.context_json =
            with_constraint(&candidate.context_json, Constraint::KekeSkillTypeName);
    }
}

fn with_constraint(context_json: &str, constraint: Constraint) -> String {
    let mut value: Value = serde_json::from_str(context_json).unwrap_or_else(|_| json!({}));
    if let Some(list) = value.get_mut("constraints").and_then(Value::as_array_mut) {
        if !list
            .iter()
            .any(|rule| rule.as_str() == Some(constraint.as_str()))
        {
            list.push(json!(constraint.as_str()));
        }
    } else if let Value::Object(map) = &mut value {
        map.insert("constraints".into(), json!([constraint.as_str()]));
    }
    value.to_string()
}

/// Visible command labels from the notes the plugin reads.
pub fn labels_category(
    files: &[(String, String, Value)],
    owner_plugin: &str,
) -> Option<TextCategory> {
    let constraints = vec![Constraint::KekeTagField, Constraint::KekeNoSpaces];
    let mut candidates = Vec::new();
    for (file_name, file_rel, json) in files {
        if !keke::READ_FILES.contains(&file_name.as_str()) {
            continue;
        }
        for (index, entry) in json.as_array().into_iter().flatten().enumerate() {
            let Some(note) = entry.get("note").and_then(Value::as_str) else {
                continue;
            };
            let pointer = format!("/{index}/note");
            let owner = entry
                .get("name")
                .and_then(Value::as_str)
                .map(|name| format!("{file_name} #{index} {name}"))
                .unwrap_or_else(|| format!("{file_name} #{index}"));
            for tag in keke::parse_tags(note) {
                let Some(label) = keke::field_text(note, &tag, LABEL_FIELD) else {
                    continue;
                };
                if !filter::needs_translation(&label, TokEngine::MvMz) {
                    continue;
                }
                candidates.push(Candidate {
                    file_name: file_name.clone(),
                    file_rel: file_rel.clone(),
                    key: keke::label_key(tag.occurrence, &pointer),
                    source: label,
                    kind: SegmentKind::PluginUiText,
                    context_json: category_context(
                        LABELS_CATEGORY,
                        &constraints,
                        json!({
                            "plugin": owner_plugin,
                            "family": keke::FAMILY,
                            "field": "label",
                            "owner": owner,
                            "visibleIn": "Battle actor command",
                            "escapeCodes": false,
                        }),
                    ),
                    reference_check: true,
                });
            }
        }
    }
    if candidates.is_empty() {
        return None;
    }
    Some(TextCategory {
        key: LABELS_CATEGORY.to_string(),
        title: "Battle command labels (Keke note tags)".to_string(),
        group: CategoryGroup::NoteTag,
        state: CategoryState::Verified,
        default_enabled: true,
        visible_in: "Battle actor commands".to_string(),
        consequences: vec![
            format!(
                "Translates the label field of {} <コマンド:…> note tag(s). Only that field \
                 changes inside each note; the command type, order, icon and help name stay as \
                 they are (checked at export).",
                candidates.len()
            ),
            "The plugin removes every space from tag contents: \"Whip Attack\" would be shown as \
             \"WhipAttack\". Commas and `>` would break the tag, so QA rejects them."
                .to_string(),
        ],
        constraints,
        candidates,
        references: Vec::new(),
        reference_count: 0,
        blocked_reason: None,
        owner_plugin: Some(owner_plugin.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note_site(text: &str) -> Site {
        Site {
            kind: SiteKind::Note,
            location: "Weapons.json /2/note".into(),
            plugin: None,
            field: None,
            text: text.into(),
        }
    }

    #[test]
    fn masking_hides_labels_and_type_names_but_keeps_help_names() {
        let mut sites = vec![note_site(
            "<コマンド:スキル/鞭,48,鞭攻撃,76,鞭ヘルプ>\n<other:鞭>",
        )];
        let references = mask_note_sites(&mut sites);
        assert_eq!(references.len(), 1);
        assert_eq!(references[0].name, "鞭");
        assert!(!sites[0].text.contains("鞭攻撃"));
        assert!(sites[0].text.contains("鞭ヘルプ"));
        assert!(sites[0].text.contains("<other:鞭>"));
        assert!(sites[0].text.starts_with("<コマンド:スキル/ ,48,"));
    }

    #[test]
    fn labels_come_only_from_notes_the_plugin_reads() {
        let files = vec![
            (
                "Weapons.json".to_string(),
                "data/Weapons.json".to_string(),
                json!([null, {"name": "革の鞭", "note": "<コマンド:スキル/鞭,48,鞭攻撃,76>\n<コマンド:防御,50>"}]),
            ),
            (
                "Skills.json".to_string(),
                "data/Skills.json".to_string(),
                json!([null, {"name": "x", "note": "<コマンド:スキル/鞭,48,読まれない,76>"}]),
            ),
        ];
        let category = labels_category(&files, "keke/Keke_VariableActorCommand").unwrap();
        assert_eq!(category.candidates.len(), 1);
        assert_eq!(category.candidates[0].source, "鞭攻撃");
        assert_eq!(
            category.candidates[0].key,
            "notetag:v1:keke-cmd:0:2:/1/note"
        );
        assert!(category.candidates[0]
            .context_json
            .contains("keke_tag_field"));
    }
}
