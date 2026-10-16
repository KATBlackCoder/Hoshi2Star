//! Plugin text categories (`plugins.js` parameters and MZ plugin-command
//! arguments) driven by verified [`profiles`](crate::engines::mv_mz::profiles),
//! plus the report of plugin text no verified profile covers.

use std::collections::{BTreeSet, HashSet};

use serde::Serialize;
use serde_json::{json, Value};

use super::{category_context, Candidate, CategoryGroup, CategoryState, Constraint, TextCategory};
use crate::engines::filter;
use crate::engines::mv_mz::extractor::SegmentKind;
use crate::engines::mv_mz::json_span::escape_token;
use crate::engines::mv_mz::plugins_js::{plugin_key, PluginEntry, PluginsJs};
use crate::engines::mv_mz::profiles::{
    CommandArgRule, DisplayRule, FillInRule, LeafRole, PluginProfile, ProfileSet, Verification,
};
use crate::llm::tokenizer::{Engine as TokEngine, PLACEHOLDER_SYNTAX_FORMAT};

/// `source_files.file_name` of `js/plugins.js`.
pub const PLUGINS_FILE_NAME: &str = "plugins.js";
/// Not-covered rows kept in the report.
const MAX_NOT_COVERED: usize = 300;

/// Plugin text left untranslated because no verified profile vouches for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotCovered {
    pub plugin: String,
    pub field: String,
    pub sample: String,
    pub reason: String,
    /// Static guess from the plugin source — never used to enable anything.
    pub hint: Option<String>,
}

/// `(plugin, field)` of identifier sites that a verified profile proved to be
/// display text or editor memos. Field = parameter path joined with `.`, or
/// `<command>.<arg>` for plugin-command arguments.
pub type SiteExclusions = HashSet<(String, String)>;

#[derive(Debug, Default)]
pub struct PluginOutcome {
    /// Display categories (classified for references by the caller).
    pub categories: Vec<TextCategory>,
    /// Profile-documented dangerous parameters (always blocked).
    pub blocked: Vec<TextCategory>,
    pub not_covered: Vec<NotCovered>,
    pub excluded_sites: SiteExclusions,
    /// `(note-tag family, plugin)` of verified profiles whose note-tag
    /// grammar Hoshi2Star implements (e.g. Keke command tags).
    pub note_tag_owners: Vec<(String, String)>,
}

pub fn plugin_categories(
    plugins: &PluginsJs,
    plugins_rel: &str,
    sources: &[(String, String)],
    data_files: &[(String, String, Value)],
    profiles: &ProfileSet,
) -> PluginOutcome {
    let mut outcome = PluginOutcome::default();
    for entry in plugins.entries().iter().filter(|entry| entry.status) {
        let source = sources
            .iter()
            .find(|(name, _)| *name == entry.name)
            .map(|(_, text)| text.as_str());
        let reason = match (profiles.find(&entry.name), source) {
            (Some(profile), Some(text)) => match profile.verify(text) {
                Verification::Verified => {
                    profiled(entry, profile, plugins_rel, data_files, &mut outcome);
                    continue;
                }
                Verification::Unverified { found_version } => format!(
                    "A profile exists for reviewed version(s) {}, but the installed plugin \
                     ({}) differs; its settings are not offered.",
                    if profile.plugin.verified_versions.is_empty() {
                        "pinned by checksum".to_string()
                    } else {
                        profile.plugin.verified_versions.join(", ")
                    },
                    found_version.unwrap_or_else(|| "no version header".into())
                ),
            },
            (Some(_), None) => {
                "The plugin source file was not found, so its profile cannot be verified.".into()
            }
            (None, _) => "No verified profile for this plugin yet.".into(),
        };
        report_uncovered(entry, &reason, source, None, &mut outcome.not_covered);
    }
    outcome
}

fn profiled(
    entry: &PluginEntry,
    profile: &PluginProfile,
    plugins_rel: &str,
    data_files: &[(String, String, Value)],
    outcome: &mut PluginOutcome,
) {
    for family in &profile.plugin.note_tag_families {
        outcome
            .note_tag_owners
            .push((family.clone(), entry.name.clone()));
    }
    let leaves = entry.parameter_leaves();
    let mut candidates = Vec::new();
    let mut blocked_candidates = Vec::new();
    let mut blocked_reasons = BTreeSet::new();
    let mut visible_in = BTreeSet::new();
    let mut fill_ins = 0usize;
    let mut unreviewed = Vec::new();
    let category_key = format!("plugin.{}", entry.name);

    for (path, value) in &leaves {
        let field = path.join(".");
        match profile.classify_leaf(path) {
            LeafRole::Memo => {
                outcome.excluded_sites.insert((entry.name.clone(), field));
            }
            LeafRole::Blocked(rule) => {
                if filter::needs_translation(value, TokEngine::MvMz) {
                    blocked_reasons.insert(format!("{field}: {}", rule.reason));
                    blocked_candidates.push(param_candidate(
                        entry,
                        profile,
                        plugins_rel,
                        &category_key,
                        path,
                        value,
                        None,
                        None,
                    ));
                }
            }
            LeafRole::Display(rule) if !value.trim().is_empty() => {
                outcome.excluded_sites.insert((entry.name.clone(), field));
                if filter::needs_translation(value, TokEngine::MvMz) {
                    visible_in.insert(rule.visible_in.clone());
                    candidates.push(param_candidate(
                        entry,
                        profile,
                        plugins_rel,
                        &category_key,
                        path,
                        value,
                        Some(rule),
                        None,
                    ));
                }
            }
            LeafRole::Display(_) | LeafRole::FillIn(_) => {
                outcome.excluded_sites.insert((entry.name.clone(), field));
                if let Some((rule, default)) = fill_in_default(profile, path, &leaves) {
                    visible_in.insert(rule.visible_in.clone());
                    fill_ins += 1;
                    candidates.push(param_candidate(
                        entry,
                        profile,
                        plugins_rel,
                        &category_key,
                        path,
                        &default,
                        None,
                        Some(rule),
                    ));
                }
            }
            LeafRole::Unreviewed => {
                if looks_like_text(value) {
                    unreviewed.push((field, value.clone()));
                }
            }
        }
    }

    if !candidates.is_empty() {
        let constraints = union_constraints(&candidates);
        outcome.categories.push(TextCategory {
            key: category_key.clone(),
            title: profile.plugin.title.clone(),
            group: CategoryGroup::Plugin,
            state: CategoryState::Verified,
            default_enabled: true,
            visible_in: visible_in.into_iter().collect::<Vec<_>>().join("; "),
            consequences: plugin_consequences(profile, candidates.len(), fill_ins),
            constraints,
            candidates,
            references: Vec::new(),
            reference_count: 0,
            blocked_reason: None,
            owner_plugin: Some(entry.name.clone()),
        });
    }
    if !blocked_candidates.is_empty() {
        outcome.blocked.push(TextCategory {
            key: format!("{category_key}.blocked"),
            title: format!(
                "{} — settings that must stay unchanged",
                profile.plugin.title
            ),
            group: CategoryGroup::Plugin,
            state: CategoryState::Blocked,
            default_enabled: false,
            visible_in: "Not displayed: these values are read as keys or options".into(),
            consequences: blocked_reasons.iter().cloned().collect(),
            constraints: Vec::new(),
            candidates: blocked_candidates,
            references: Vec::new(),
            reference_count: 0,
            blocked_reason: Some(
                "The plugin uses these values as identifiers (note-tag keys, option values or \
                 key names). Translating them would change how the game behaves."
                    .into(),
            ),
            owner_plugin: Some(entry.name.clone()),
        });
    }
    for (field, value) in unreviewed {
        if outcome.not_covered.len() < MAX_NOT_COVERED {
            outcome.not_covered.push(NotCovered {
                plugin: entry.name.clone(),
                field,
                sample: value,
                reason: "This setting is not reviewed by the plugin's profile.".into(),
                hint: None,
            });
        }
    }

    for rule in &profile.command_arg {
        outcome
            .excluded_sites
            .insert((entry.name.clone(), format!("{}.{}", rule.command, rule.arg)));
        if let Some(category) = command_arg_category(entry, profile, rule, data_files) {
            outcome.categories.push(category);
        }
    }
}

/// For an empty fill-in leaf, the hard-coded default the plugin shows.
fn fill_in_default<'p>(
    profile: &'p PluginProfile,
    path: &[String],
    leaves: &[(Vec<String>, String)],
) -> Option<(&'p FillInRule, String)> {
    let rule = profile.fill_in.iter().find(|rule| {
        crate::engines::mv_mz::profiles::PathPattern::parse(&rule.path)
            .is_ok_and(|p| p.matches(path))
    })?;
    let (_, parent) = path.split_last()?;
    let mut key_path = parent.to_vec();
    key_path.push(rule.key_field.clone());
    let key = leaves
        .iter()
        .find(|(leaf, _)| *leaf == key_path)
        .map(|(_, value)| value.trim().to_string())?;
    rule.defaults
        .get(&key)
        .map(|default| (rule, default.clone()))
}

#[allow(clippy::too_many_arguments)]
fn param_candidate(
    entry: &PluginEntry,
    profile: &PluginProfile,
    plugins_rel: &str,
    category_key: &str,
    path: &[String],
    source: &str,
    display: Option<&DisplayRule>,
    fill_in: Option<&FillInRule>,
) -> Candidate {
    let (message, template, escape_codes, visible) = match (display, fill_in) {
        (Some(rule), _) => (
            rule.message,
            rule.template,
            rule.escape_codes,
            rule.visible_in.as_str(),
        ),
        (None, Some(rule)) => (false, false, false, rule.visible_in.as_str()),
        (None, None) => (false, false, false, ""),
    };
    let constraints = constraints_for(profile, template, escape_codes);
    let mut extra = json!({
        "plugin": entry.name,
        "field": path.join("."),
        "visibleIn": visible,
        "escapeCodes": escape_codes,
    });
    if template {
        extra["placeholderSyntax"] = json!(PLACEHOLDER_SYNTAX_FORMAT);
    }
    if let Some(rule) = fill_in {
        extra["fillIn"] = json!(true);
        extra["keyField"] = json!(rule.key_field);
    }
    Candidate {
        file_name: PLUGINS_FILE_NAME.to_string(),
        file_rel: plugins_rel.to_string(),
        key: plugin_key(&entry.name, entry.occurrence, path),
        source: source.to_string(),
        kind: if message || template {
            SegmentKind::PluginMessage
        } else {
            SegmentKind::PluginUiText
        },
        context_json: category_context(category_key, &constraints, extra),
        // Fill-in defaults are not game strings (they live in the plugin's own
        // code), so they are never looked up as identifiers.
        reference_check: fill_in.is_none(),
    }
}

fn constraints_for(profile: &PluginProfile, template: bool, escape_codes: bool) -> Vec<Constraint> {
    let mut constraints = Vec::new();
    if template {
        constraints.push(Constraint::KeepFormatSpecifiers);
    }
    if !escape_codes {
        constraints.push(Constraint::NoEscapeCodes);
    }
    if profile.plugin.coerces_values {
        constraints.push(Constraint::PlainTextValue);
    }
    constraints
}

fn union_constraints(candidates: &[Candidate]) -> Vec<Constraint> {
    let mut all: Vec<Constraint> = Vec::new();
    for candidate in candidates {
        for rule in Constraint::from_context(&candidate.context_json) {
            if !all.contains(&rule) {
                all.push(rule);
            }
        }
    }
    all
}

fn plugin_consequences(profile: &PluginProfile, count: usize, fill_ins: usize) -> Vec<String> {
    let mut lines = vec![format!(
        "Translates {count} setting(s) of {} in js/plugins.js. Only these exact values change; \
         every other byte of plugins.js stays identical (checked at export).",
        profile.plugin.name
    )];
    if fill_ins > 0 {
        lines.push(format!(
            "{fill_ins} of them are empty labels the plugin replaces with a built-in Japanese \
             default; writing the translation into the empty field replaces that default."
        ));
    }
    if profile.plugin.coerces_values {
        lines.push(
            "This plugin converts setting values: a translation that is only a number, true/false \
             or JSON would change type, so QA rejects such translations."
                .into(),
        );
    }
    lines.push(format!("Reviewed: {}", profile.plugin.reference));
    lines
}

fn command_arg_category(
    entry: &PluginEntry,
    profile: &PluginProfile,
    rule: &CommandArgRule,
    data_files: &[(String, String, Value)],
) -> Option<TextCategory> {
    let key = format!(
        "plugin_command.{}.{}.{}",
        entry.name, rule.command, rule.arg
    );
    let constraints = constraints_for(profile, rule.template, rule.escape_codes);
    let mut candidates = Vec::new();
    for_each_command(
        data_files,
        &mut |file_name, file_rel, list_path, index, command| {
            if command.get("code").and_then(Value::as_i64) != Some(357) {
                return;
            }
            let params = command.get("parameters").and_then(Value::as_array);
            let Some(params) = params else { return };
            if params.first().and_then(Value::as_str) != Some(entry.name.as_str())
                || params.get(1).and_then(Value::as_str) != Some(rule.command.as_str())
            {
                return;
            }
            let Some(text) = params
                .get(3)
                .and_then(|args| args.get(&rule.arg))
                .and_then(Value::as_str)
            else {
                return;
            };
            if !filter::needs_translation(text, TokEngine::MvMz) {
                return;
            }
            let mut extra = json!({
                "plugin": entry.name,
                "command": rule.command,
                "field": rule.arg,
                "visibleIn": rule.visible_in,
                "escapeCodes": rule.escape_codes,
                "commandIndex": index,
            });
            if rule.template {
                extra["placeholderSyntax"] = json!(PLACEHOLDER_SYNTAX_FORMAT);
            }
            candidates.push(Candidate {
                file_name: file_name.to_string(),
                file_rel: file_rel.to_string(),
                key: format!(
                    "{list_path}/{index}/parameters/3/{}",
                    escape_token(&rule.arg)
                ),
                source: text.to_string(),
                kind: if rule.message || rule.template {
                    SegmentKind::PluginMessage
                } else {
                    SegmentKind::PluginUiText
                },
                context_json: category_context(&key, &constraints, extra),
                reference_check: true,
            });
        },
    );
    if candidates.is_empty() {
        return None;
    }
    let mut consequences = vec![
        format!(
            "Translates {} `{}` command argument(s) ({}) in maps, common events and troops. Only \
             the argument value changes.",
            candidates.len(),
            rule.command,
            rule.arg
        ),
        "The RPG Maker editor's one-line summary of each command keeps the original text; it is \
         never shown in game."
            .to_string(),
    ];
    if profile.plugin.coerces_values {
        consequences.push(
            "Arguments are converted by the plugin: a translation that is only a number, \
             true/false or JSON would change type, so QA rejects such translations."
                .into(),
        );
    }
    consequences.push(format!("Reviewed: {}", profile.plugin.reference));
    Some(TextCategory {
        key,
        title: rule.title.clone(),
        group: CategoryGroup::PluginCommand,
        state: CategoryState::Verified,
        default_enabled: true,
        visible_in: rule.visible_in.clone(),
        consequences,
        constraints,
        candidates,
        references: Vec::new(),
        reference_count: 0,
        blocked_reason: None,
        owner_plugin: Some(entry.name.clone()),
    })
}

/// Visit every event command of maps, common events and troops:
/// `(file_name, file_rel, list pointer, index, command)`.
pub fn for_each_command(
    data_files: &[(String, String, Value)],
    visit: &mut dyn FnMut(&str, &str, &str, usize, &Value),
) {
    let mut walk = |file_name: &str, file_rel: &str, base: String, list: &Value| {
        if let Some(commands) = list.as_array() {
            for (index, command) in commands.iter().enumerate() {
                visit(file_name, file_rel, &base, index, command);
            }
        }
    };
    for (file_name, file_rel, json) in data_files {
        if filter::is_map_file(file_name, ".json") {
            for (event_index, event) in json
                .get("events")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .enumerate()
            {
                for (page_index, page) in event
                    .get("pages")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .enumerate()
                {
                    if let Some(list) = page.get("list") {
                        walk(
                            file_name,
                            file_rel,
                            format!("/events/{event_index}/pages/{page_index}/list"),
                            list,
                        );
                    }
                }
            }
        } else if file_name == "CommonEvents.json" {
            for (index, event) in json.as_array().into_iter().flatten().enumerate() {
                if let Some(list) = event.get("list") {
                    walk(file_name, file_rel, format!("/{index}/list"), list);
                }
            }
        } else if file_name == "Troops.json" {
            for (troop_index, troop) in json.as_array().into_iter().flatten().enumerate() {
                for (page_index, page) in troop
                    .get("pages")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .enumerate()
                {
                    if let Some(list) = page.get("list") {
                        walk(
                            file_name,
                            file_rel,
                            format!("/{troop_index}/pages/{page_index}/list"),
                            list,
                        );
                    }
                }
            }
        }
    }
}

/// Whether a segment's `context_json` marks it as a fill-in (its original
/// value is an empty field, not the source text).
pub fn context_is_fill_in(context_json: Option<&str>) -> bool {
    context_json
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        .and_then(|value| value.get("fillIn").and_then(Value::as_bool))
        .unwrap_or(false)
}

/// Non-ASCII letters: Japanese (or other non-Latin) text worth reporting.
fn looks_like_text(value: &str) -> bool {
    filter::needs_translation(value, TokEngine::MvMz)
        && value.chars().any(|c| !c.is_ascii() && c.is_alphanumeric())
}

fn report_uncovered(
    entry: &PluginEntry,
    reason: &str,
    source: Option<&str>,
    _profile: Option<&PluginProfile>,
    out: &mut Vec<NotCovered>,
) {
    for (path, value) in entry.parameter_leaves() {
        if out.len() >= MAX_NOT_COVERED {
            return;
        }
        if !looks_like_text(&value) {
            continue;
        }
        out.push(NotCovered {
            plugin: entry.name.clone(),
            field: path.join("."),
            sample: value,
            reason: reason.to_string(),
            hint: source.and_then(|text| usage_hint(text, &path)),
        });
    }
}

/// A *guess* from the plugin code about what a parameter is used for. Shown to
/// help prioritise new profiles; never used to enable translation.
pub fn usage_hint(source: &str, path: &[String]) -> Option<String> {
    let field = path
        .iter()
        .rev()
        .find(|token| !token.chars().all(|c| c.is_ascii_digit()))?;
    let needles = [
        format!("'{field}'"),
        format!("\"{field}\""),
        format!(".{field}"),
    ];
    let usage: Vec<&str> = source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with('*') && !trimmed.starts_with("//")
        })
        .filter(|line| needles.iter().any(|needle| line.contains(needle.as_str())))
        .collect();
    if usage.is_empty() {
        return None;
    }
    let joined = usage.join("\n");
    if joined.contains("meta") {
        Some("Read with note-tag metadata: likely a note-tag key (must not be translated).".into())
    } else if ["==", "!=", ".includes(", ".indexOf(", "case "]
        .iter()
        .any(|token| joined.contains(token))
    {
        Some("Compared against other values: likely an identifier or option value.".into())
    } else if ["addCommand(", "drawText", ".format(", "addText", "setText"]
        .iter()
        .any(|token| joined.contains(token))
    {
        Some("Drawn on screen: likely display text (not verified).".into())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROFILE: &str = r#"
        [plugin]
        name = "Test/Book"
        title = "Test book"
        verified_versions = ["1.0"]
        reference = "synthetic"
        coerces_values = true

        [[display]]
        path = "CommandName"
        visible_in = "Main menu"

        [[display]]
        path = "Miss"
        visible_in = "Battle log"
        message = true
        template = true
        escape_codes = true

        [[display]]
        path = "Rows[].label"
        visible_in = "Rows"

        [[fill_in]]
        path = "Rows[].label"
        key_field = "Code"
        visible_in = "Rows (defaults)"
        [fill_in.defaults]
        "32" = "倒した数"

        [[blocked]]
        path = "MetaKey"
        reason = "note-tag key"

        [[memo]]
        path = "Memo[].name"

        [[command_arg]]
        command = "LOG"
        arg = "message"
        title = "Test log"
        visible_in = "Battle log"
        message = true
        escape_codes = true
    "#;

    fn plugins_js() -> String {
        let rows = serde_json::to_string(&vec![
            r#"{"label":"","Code":"32"}"#,
            r#"{"label":"得られる金","Code":"31"}"#,
            r#"{"label":"","Code":"1"}"#,
        ])
        .unwrap();
        let memo = serde_json::to_string(&vec![r#"{"name":"メモ"}"#]).unwrap();
        let params = serde_json::json!({
            "CommandName": "図鑑",
            "Miss": "%1は失敗した",
            "Rows": rows,
            "MetaKey": "HP消費",
            "Memo": memo,
            "Extra": "未確認の設定",
        });
        format!(
            "var $plugins =\n[\n{},\n{}\n];\n",
            serde_json::json!({"name":"Test/Book","status":true,"description":"","parameters":params}),
            serde_json::json!({"name":"Other","status":true,"description":"","parameters":{"Label":"謎の文字","Path":"img/x"}})
        )
    }

    fn sources() -> Vec<(String, String)> {
        vec![
            ("Test/Book".into(), "/*:\n * @version 1.0\n */\n".into()),
            (
                "Other".into(),
                "const label = params.Label; this.addCommand(label, 'x');".into(),
            ),
        ]
    }

    fn data_files() -> Vec<(String, String, Value)> {
        vec![(
            "CommonEvents.json".into(),
            "data/CommonEvents.json".into(),
            serde_json::json!([null, {"list": [
                {"code": 357, "parameters": ["Test/Book", "LOG", "ログ", {"message": "敵が逃げた！"}]},
                {"code": 357, "parameters": ["Test/Book", "OTHER", "x", {"message": "対象外"}]},
                {"code": 357, "parameters": ["Another", "LOG", "x", {"message": "対象外"}]}
            ]}]),
        )]
    }

    #[test]
    fn verified_profile_yields_display_fill_in_blocked_and_arg_categories() {
        let profiles = ProfileSet::parse_all(&[("t.toml", PROFILE)]).unwrap();
        let plugins = PluginsJs::parse(&plugins_js()).unwrap();
        let outcome = plugin_categories(
            &plugins,
            "js/plugins.js",
            &sources(),
            &data_files(),
            &profiles,
        );

        let book = outcome
            .categories
            .iter()
            .find(|c| c.key == "plugin.Test/Book")
            .unwrap();
        let sources: Vec<_> = book.candidates.iter().map(|c| c.source.as_str()).collect();
        assert_eq!(
            sources,
            vec!["図鑑", "%1は失敗した", "倒した数", "得られる金"]
        );
        let fill = book
            .candidates
            .iter()
            .find(|c| c.source == "倒した数")
            .unwrap();
        assert_eq!(fill.key, "plugin:v1:/Test~1Book/1/Rows/0/label");
        assert!(!fill.reference_check);
        assert!(fill.context_json.contains(r#""fillIn":true"#));
        let template = book
            .candidates
            .iter()
            .find(|c| c.source.starts_with("%1"))
            .unwrap();
        assert_eq!(template.kind, SegmentKind::PluginMessage);
        assert!(template
            .context_json
            .contains(r#""placeholderSyntax":"format""#));
        assert!(template.context_json.contains("plain_text_value"));
        assert!(book.constraints.contains(&Constraint::NoEscapeCodes));

        let blocked = outcome
            .blocked
            .iter()
            .find(|c| c.key == "plugin.Test/Book.blocked")
            .unwrap();
        assert_eq!(blocked.state, CategoryState::Blocked);
        assert_eq!(blocked.candidates[0].source, "HP消費");

        let log = outcome
            .categories
            .iter()
            .find(|c| c.key == "plugin_command.Test/Book.LOG.message")
            .unwrap();
        assert_eq!(log.candidates.len(), 1);
        assert_eq!(log.candidates[0].key, "/1/list/0/parameters/3/message");
        assert_eq!(log.candidates[0].file_name, "CommonEvents.json");

        // Display, fill-in and memo leaves are not identifier sites; blocked ones stay sites.
        assert!(outcome
            .excluded_sites
            .contains(&("Test/Book".into(), "CommandName".into())));
        assert!(outcome
            .excluded_sites
            .contains(&("Test/Book".into(), "Memo.0.name".into())));
        assert!(outcome
            .excluded_sites
            .contains(&("Test/Book".into(), "LOG.message".into())));
        assert!(!outcome
            .excluded_sites
            .contains(&("Test/Book".into(), "MetaKey".into())));

        // Unreviewed text of a profiled plugin and unprofiled plugins are reported.
        assert!(outcome
            .not_covered
            .iter()
            .any(|n| n.plugin == "Test/Book" && n.field == "Extra"));
        let other = outcome
            .not_covered
            .iter()
            .find(|n| n.plugin == "Other")
            .unwrap();
        assert_eq!(other.field, "Label");
        assert_eq!(
            other.hint.as_deref(),
            Some("Drawn on screen: likely display text (not verified).")
        );
    }

    #[test]
    fn unverified_plugin_versions_are_report_only() {
        let profiles = ProfileSet::parse_all(&[("t.toml", PROFILE)]).unwrap();
        let plugins = PluginsJs::parse(&plugins_js()).unwrap();
        let sources = vec![("Test/Book".to_string(), " * @version 2.0\n".to_string())];
        let outcome = plugin_categories(
            &plugins,
            "js/plugins.js",
            &sources,
            &data_files(),
            &profiles,
        );
        assert!(outcome.categories.is_empty());
        assert!(outcome.blocked.is_empty());
        assert!(outcome
            .not_covered
            .iter()
            .all(|n| n.plugin != "Test/Book" || n.reason.contains("2.0")));
        assert!(outcome.excluded_sites.is_empty());
    }

    #[test]
    fn usage_hints_classify_meta_comparisons_and_display() {
        let path = |p: &str| vec![p.to_string()];
        assert!(usage_hint(
            "x = skill.meta[p.Key];\nconst k = params.Key;",
            &path("Key")
        )
        .unwrap()
        .contains("note-tag"));
        assert!(usage_hint("if (params.Mode == 'a') {}", &path("Mode"))
            .unwrap()
            .contains("identifier"));
        assert!(usage_hint(" * @param Label\nconst a = 1;", &path("Label")).is_none());
    }
}
