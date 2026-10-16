//! Reference sites: places where a game string can act as an *identifier*
//! (looked up, compared or parsed) rather than as display text.
//!
//! Translating a string that a site refers to by value breaks that lookup, so
//! every candidate text is checked against these sites before it may become a
//! translatable segment. Editor-only text (event comments 108/408, plugin
//! command summaries 657 and display names) is never collected: nothing reads
//! it at runtime.

use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;
use serde_json::Value;

use crate::engines::mv_mz::plugins_js::PluginsJs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SiteKind {
    /// Database / map / event `note` field (plugin note tags).
    Note,
    /// JavaScript evaluated by the engine (script commands, conditions,
    /// variable scripts, move-route scripts, damage formulas).
    Script,
    /// MV plugin command line (code 356).
    PluginCommand,
    /// MZ plugin command argument (code 357).
    PluginCommandArg,
    /// `plugins.js` parameter of an enabled plugin.
    PluginParam,
    /// String literal in an enabled plugin's source code.
    PluginSource,
}

/// One identifier site with enough context to explain a finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Site {
    pub kind: SiteKind,
    /// Human-readable location (`Weapons.json /2/note`, `nuun/X::PageList[3].name`…).
    pub location: String,
    /// Plugin that owns the site (plugin params, args, sources), if any.
    pub plugin: Option<String>,
    /// Parameter/argument path inside the plugin, if any.
    pub field: Option<String>,
    pub text: String,
}

/// Collect sites from the parsed data files (`file name`, JSON).
pub fn collect_data_sites(files: &[(String, Value)], sites: &mut Vec<Site>) {
    for (file_name, json) in files {
        match json {
            Value::Array(entries) => {
                for (index, entry) in entries.iter().enumerate() {
                    collect_entry(file_name, &format!("/{index}"), entry, sites);
                }
            }
            Value::Object(_) => collect_map(file_name, json, sites),
            _ => {}
        }
    }
}

fn push(sites: &mut Vec<Site>, kind: SiteKind, location: String, text: &str) {
    if !text.trim().is_empty() {
        sites.push(Site {
            kind,
            location,
            plugin: None,
            field: None,
            text: text.to_string(),
        });
    }
}

static RE_NOTE_TAG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<[^<>\n]*>").expect("RE_NOTE_TAG must compile"));
static RE_CLOSING_TAG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"</([^<>\n]+)>").expect("RE_CLOSING_TAG must compile"));

/// Keep only the parts of a note that code can read as data: single-line
/// tags (`<key>`, `<key:value>`, as parsed by `DataManager.extractMetadata`
/// and plugins) and the bodies of block tags (`<Name>…</Name>`). Free text
/// outside every tag is an editor memo; it is blanked out (one space per
/// character, so boundaries are preserved).
pub fn readable_note_text(note: &str) -> String {
    let mut keep: Vec<std::ops::Range<usize>> =
        RE_NOTE_TAG.find_iter(note).map(|m| m.range()).collect();
    for closing in RE_CLOSING_TAG.captures_iter(note) {
        let (Some(whole), Some(name)) = (closing.get(0), closing.get(1)) else {
            continue;
        };
        let needle = format!("<{}", name.as_str().trim()).to_ascii_lowercase();
        let before = note[..whole.start()].to_ascii_lowercase();
        if let Some(open) = before.rfind(&needle) {
            keep.push(open..whole.end());
        }
    }
    note.char_indices()
        .map(|(offset, ch)| {
            if keep.iter().any(|span| span.contains(&offset)) {
                ch
            } else {
                ' '
            }
        })
        .collect()
}

/// Database row (Actors, Skills, …), CommonEvent or Troop.
fn collect_entry(file_name: &str, base: &str, entry: &Value, sites: &mut Vec<Site>) {
    if let Some(note) = entry.get("note").and_then(Value::as_str) {
        push(
            sites,
            SiteKind::Note,
            format!("{file_name} {base}/note"),
            &readable_note_text(note),
        );
    }
    if let Some(formula) = entry.pointer("/damage/formula").and_then(Value::as_str) {
        push(
            sites,
            SiteKind::Script,
            format!("{file_name} {base}/damage/formula"),
            formula,
        );
    }
    if let Some(list) = entry.get("list").and_then(Value::as_array) {
        collect_commands(file_name, &format!("{base}/list"), list, sites);
    }
    if let Some(pages) = entry.get("pages").and_then(Value::as_array) {
        for (page_index, page) in pages.iter().enumerate() {
            let page_base = format!("{base}/pages/{page_index}");
            if let Some(list) = page.get("list").and_then(Value::as_array) {
                collect_commands(file_name, &format!("{page_base}/list"), list, sites);
            }
            if let Some(route) = page.pointer("/moveRoute/list").and_then(Value::as_array) {
                collect_route(
                    file_name,
                    &format!("{page_base}/moveRoute/list"),
                    route,
                    sites,
                );
            }
        }
    }
}

fn collect_map(file_name: &str, map: &Value, sites: &mut Vec<Site>) {
    if let Some(note) = map.get("note").and_then(Value::as_str) {
        push(
            sites,
            SiteKind::Note,
            format!("{file_name} /note"),
            &readable_note_text(note),
        );
    }
    if let Some(events) = map.get("events").and_then(Value::as_array) {
        for (index, event) in events.iter().enumerate() {
            if !event.is_null() {
                collect_entry(file_name, &format!("/events/{index}"), event, sites);
            }
        }
    }
}

fn collect_commands(file_name: &str, base: &str, list: &[Value], sites: &mut Vec<Site>) {
    for (index, command) in list.iter().enumerate() {
        let code = command.get("code").and_then(Value::as_i64).unwrap_or(0);
        let Some(params) = command.get("parameters").and_then(Value::as_array) else {
            continue;
        };
        let location = |suffix: &str| format!("{file_name} {base}/{index}/parameters{suffix}");
        let text = |i: usize| params.get(i).and_then(Value::as_str);
        match code {
            355 | 655 => {
                if let Some(script) = text(0) {
                    push(sites, SiteKind::Script, location("/0"), script);
                }
            }
            111 if params.first().and_then(Value::as_i64) == Some(12) => {
                if let Some(script) = text(1) {
                    push(sites, SiteKind::Script, location("/1"), script);
                }
            }
            122 if params.get(3).and_then(Value::as_i64) == Some(4) => {
                if let Some(script) = text(4) {
                    push(sites, SiteKind::Script, location("/4"), script);
                }
            }
            205 => {
                if let Some(route) = params
                    .get(1)
                    .and_then(|r| r.get("list"))
                    .and_then(Value::as_array)
                {
                    collect_route(
                        file_name,
                        &format!("{base}/{index}/parameters/1/list"),
                        route,
                        sites,
                    );
                }
            }
            356 => {
                if let Some(line) = text(0) {
                    push(sites, SiteKind::PluginCommand, location("/0"), line);
                }
            }
            357 => {
                let plugin = text(0).unwrap_or_default();
                let command_name = text(1).unwrap_or_default();
                if let Some(args) = params.get(3).and_then(Value::as_object) {
                    for (arg, value) in args {
                        if let Some(value) = value.as_str().filter(|v| !v.trim().is_empty()) {
                            sites.push(Site {
                                kind: SiteKind::PluginCommandArg,
                                location: location(&format!("/3/{arg}")),
                                plugin: Some(plugin.to_string()),
                                field: Some(format!("{command_name}.{arg}")),
                                text: value.to_string(),
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Move-route commands: code 45 ("Script") carries JavaScript.
fn collect_route(file_name: &str, base: &str, route: &[Value], sites: &mut Vec<Site>) {
    for (index, step) in route.iter().enumerate() {
        if step.get("code").and_then(Value::as_i64) == Some(45) {
            if let Some(script) = step.pointer("/parameters/0").and_then(Value::as_str) {
                push(
                    sites,
                    SiteKind::Script,
                    format!("{file_name} {base}/{index}/parameters/0"),
                    script,
                );
            }
        }
    }
}

/// Collect every parameter leaf of the *enabled* plugins.
pub fn collect_plugin_param_sites(plugins: &PluginsJs, sites: &mut Vec<Site>) {
    for entry in plugins.entries().iter().filter(|entry| entry.status) {
        for (path, value) in entry.parameter_leaves() {
            if value.trim().is_empty() {
                continue;
            }
            let field = path.join(".");
            sites.push(Site {
                kind: SiteKind::PluginParam,
                location: format!("{}::{field}", entry.name),
                plugin: Some(entry.name.clone()),
                field: Some(field),
                text: value,
            });
        }
    }
}

static RE_STRING_LITERAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"'(?:[^'\\\n]|\\.)*'|"(?:[^"\\\n]|\\.)*"|`(?:[^`\\]|\\.)*`"#)
        .expect("RE_STRING_LITERAL must compile")
});
static RE_BLOCK_COMMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)/\*.*?\*/").expect("RE_BLOCK_COMMENT must compile"));

/// Collect string literals from an enabled plugin's source (comments — which
/// hold the plugin's own documentation — are skipped).
pub fn collect_plugin_source_sites(plugin: &str, source: &str, sites: &mut Vec<Site>) {
    let without_blocks = RE_BLOCK_COMMENT.replace_all(source, |caps: &regex::Captures| {
        "\n".repeat(caps[0].matches('\n').count())
    });
    for (line_index, line) in without_blocks.lines().enumerate() {
        let code = strip_line_comment(line);
        for literal in RE_STRING_LITERAL.find_iter(code) {
            let inner = &literal.as_str()[1..literal.as_str().len() - 1];
            if inner.trim().is_empty() {
                continue;
            }
            sites.push(Site {
                kind: SiteKind::PluginSource,
                location: format!("{plugin}.js:{}", line_index + 1),
                plugin: Some(plugin.to_string()),
                field: None,
                text: inner.to_string(),
            });
        }
    }
}

/// Drop a `//` comment that is not inside a string literal.
fn strip_line_comment(line: &str) -> &str {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut previous = '\0';
    for (index, ch) in line.char_indices() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == q {
                quote = None;
            }
        } else if ch == '"' || ch == '\'' || ch == '`' {
            quote = Some(ch);
        } else if ch == '/' && previous == '/' {
            return &line[..index - 1];
        }
        previous = ch;
    }
    line
}

/// `true` when `needle` occurs in `hay` with no word character (Unicode
/// alphanumeric or `_`) directly before or after it. Unlike regex `\b`, this
/// treats CJK and kana as word characters, so `鞭` does not match `鞭攻撃`
/// but does match `スキル/鞭,48`.
pub fn token_occurs(needle: &str, hay: &str) -> Option<usize> {
    if needle.is_empty() {
        return None;
    }
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let mut from = 0;
    while let Some(offset) = hay[from..].find(needle) {
        let start = from + offset;
        let end = start + needle.len();
        let before_ok = hay[..start].chars().next_back().is_none_or(|c| !is_word(c));
        let after_ok = hay[end..].chars().next().is_none_or(|c| !is_word(c));
        if before_ok && after_ok {
            return Some(start);
        }
        from = start + hay[start..].chars().next().map_or(1, char::len_utf8);
    }
    None
}

/// Short excerpt of `text` around byte offset `at`.
pub fn excerpt(text: &str, at: usize, needle_len: usize) -> String {
    const RADIUS: usize = 40;
    let mut start = at.saturating_sub(RADIUS);
    while !text.is_char_boundary(start) {
        start -= 1;
    }
    let mut end = (at + needle_len + RADIUS).min(text.len());
    while !text.is_char_boundary(end) {
        end += 1;
    }
    let mut out = String::new();
    if start > 0 {
        out.push('…');
    }
    out.push_str(&text[start..end].replace(['\n', '\r'], " "));
    if end < text.len() {
        out.push('…');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn token_matching_respects_japanese_word_boundaries() {
        assert!(token_occurs("鞭", "<コマンド:スキル/鞭,48,鞭攻撃,76>").is_some());
        assert!(token_occurs("鞭", "鞭攻撃").is_none());
        assert!(token_occurs("炎", "私服\u{3000}炎").is_some());
        assert!(token_occurs("乳首", "乳首←").is_some());
        assert!(token_occurs("恵", "恵み").is_none());
        assert!(token_occurs("Lv", "Lv2").is_none());
        assert!(token_occurs("a", "").is_none());
        // A later, bounded occurrence is still found after an unbounded one.
        assert_eq!(token_occurs("鞭", "鞭攻撃 / 鞭"), Some("鞭攻撃 / ".len()));
    }

    #[test]
    fn data_sites_cover_scripts_notes_and_plugin_commands_but_not_comments() {
        let common = json!([null, {"list": [
            {"code": 108, "parameters": ["鞭 comment"]},
            {"code": 355, "parameters": ["$gameActors.actor(1).name() === '恵'"]},
            {"code": 111, "parameters": [12, "$gameSystem.isJapanese()"]},
            {"code": 122, "parameters": [1, 1, 0, 4, "'炎'.length"]},
            {"code": 357, "parameters": ["P", "cmd", "表示名", {"label": "鞭"}]},
            {"code": 205, "parameters": [0, {"list": [{"code": 45, "parameters": ["this.x = 1"]}]}]},
            {"code": 657, "parameters": ["ラベル = 鞭"]}
        ]}]);
        let weapons =
            json!([null, {"note": "<コマンド:スキル/鞭,48>", "damage": {"formula": "a.atk"}}]);
        let mut sites = Vec::new();
        collect_data_sites(
            &[
                ("CommonEvents.json".into(), common),
                ("Weapons.json".into(), weapons),
            ],
            &mut sites,
        );
        let kinds: Vec<_> = sites.iter().map(|s| (s.kind, s.text.as_str())).collect();
        assert!(kinds.contains(&(SiteKind::Script, "$gameActors.actor(1).name() === '恵'")));
        assert!(kinds.contains(&(SiteKind::Script, "'炎'.length")));
        assert!(kinds.contains(&(SiteKind::Script, "this.x = 1")));
        assert!(kinds.contains(&(SiteKind::PluginCommandArg, "鞭")));
        assert!(kinds.contains(&(SiteKind::Note, "<コマンド:スキル/鞭,48>")));
        assert!(kinds.contains(&(SiteKind::Script, "a.atk")));
        assert!(!sites
            .iter()
            .any(|s| s.text.contains("comment") || s.text.contains("ラベル")));
        assert!(!sites.iter().any(|s| s.text == "表示名"));
    }

    #[test]
    fn note_memos_outside_tags_are_not_readable_data() {
        let note = "チュートリアル用\n<CollapseId:sink2>\n鞭攻撃\n<Custom Text>\n炎の説明\n</Custom Text>\n<NoBook>";
        let readable = readable_note_text(note);
        assert!(!readable.contains("チュートリアル用"));
        assert!(!readable.contains("鞭攻撃"));
        assert!(readable.contains("<CollapseId:sink2>"));
        assert!(readable.contains("炎の説明"));
        assert!(readable.contains("<NoBook>"));
        assert_eq!(readable.chars().count(), note.chars().count());
        assert!(token_occurs("鞭攻撃", &readable).is_none());
    }

    #[test]
    fn plugin_source_literals_skip_comments() {
        let source = "/*:\n * @param X\n * @default '鞭'\n */\nconst a = '攻撃'; // '防御'\nconst url = \"http://x\";\n";
        let mut sites = Vec::new();
        collect_plugin_source_sites("keke/K", source, &mut sites);
        let texts: Vec<_> = sites
            .iter()
            .map(|s| (s.text.as_str(), s.location.as_str()))
            .collect();
        assert_eq!(
            texts,
            vec![("攻撃", "keke/K.js:5"), ("http://x", "keke/K.js:6")]
        );
    }

    #[test]
    fn excerpt_is_char_boundary_safe() {
        let text = "あ".repeat(60) + "鞭" + &"い".repeat(60);
        let at = text.find('鞭').unwrap();
        let out = excerpt(&text, at, '鞭'.len_utf8());
        assert!(out.contains('鞭') && out.starts_with('…') && out.ends_with('…'));
    }
}
