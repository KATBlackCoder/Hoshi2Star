//! The game's language setting (`System.json` → `locale`, MV 1.6+ / MZ).
//!
//! RPG Maker reads it through `Game_System.isJapanese()`, `isChinese()`,
//! `isKorean()`, `isCJK()` and `isRussian()`; plugins may read it too. Switching
//! it to the target language is optional: this module lists, from the game's
//! own engine and plugin code, exactly what reads it so the user can decide.

use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

static RE_LOCALE_USE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:isJapanese|isChinese|isKorean|isCJK|isRussian)\s*\(|\$dataSystem\.locale\b")
        .expect("RE_LOCALE_USE must compile")
});

/// The core's own `Game_System.prototype.isJapanese = function() {` style
/// definitions: their bodies read the setting but are not uses.
static RE_LOCALE_DEFINITION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^Game_System\.prototype\.is(?:Japanese|Chinese|Korean|CJK|Russian)\s*=")
        .expect("RE_LOCALE_DEFINITION must compile")
});

/// `ja` → `ja_JP` style locale written for a project target language.
pub fn target_locale(language: &str) -> String {
    match language {
        "en" => "en_US",
        "fr" => "fr_FR",
        "es" => "es_ES",
        "de" => "de_DE",
        "it" => "it_IT",
        "pt" => "pt_BR",
        "ru" => "ru_RU",
        "zh" => "zh_CN",
        "ko" => "ko_KR",
        "ja" => "ja_JP",
        other => other,
    }
    .to_string()
}

/// One line of engine or plugin code that reads the language setting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocaleUse {
    pub location: String,
    pub code: String,
}

/// Lines (outside comments) of `sources` that read the language setting.
/// The core's own definitions (`Game_System.prototype.isJapanese = …`) are
/// not uses and are skipped with their bodies.
pub fn locale_uses(sources: &[(String, String)]) -> Vec<LocaleUse> {
    let mut uses = Vec::new();
    for (name, text) in sources {
        let mut in_block_comment = false;
        let mut in_definition = false;
        for (index, line) in text.lines().enumerate() {
            let trimmed = line.trim_start();
            if in_block_comment {
                in_block_comment = !trimmed.contains("*/");
                continue;
            }
            if in_definition {
                in_definition = !trimmed.starts_with('}');
                continue;
            }
            if trimmed.starts_with("/*") {
                in_block_comment = !trimmed.contains("*/");
                continue;
            }
            if trimmed.starts_with("//") || trimmed.starts_with('*') {
                continue;
            }
            if RE_LOCALE_DEFINITION.is_match(trimmed) {
                in_definition = !trimmed.contains('}');
                continue;
            }
            if RE_LOCALE_USE.is_match(line) {
                uses.push(LocaleUse {
                    location: format!("{name}:{}", index + 1),
                    code: trimmed.chars().take(140).collect(),
                });
            }
        }
    }
    uses
}

/// Plain-language consequences of switching `current` → `target`, limited to
/// what the listed code lines actually decide.
pub fn consequences(
    current: &str,
    target: &str,
    uses: &[LocaleUse],
    name_input_commands: usize,
) -> Vec<String> {
    let mut lines = vec![format!(
        "Sets System.json `locale` from {current} to {target}. Nothing else in the game data changes."
    )];
    let reads = |call: &str| uses.iter().any(|u| u.code.contains(call));

    let is_cjk = |locale: &str| ["ja", "zh", "ko"].iter().any(|p| locale.starts_with(p));
    if is_cjk(current) != is_cjk(target) && reads("isCJK") {
        lines.push(if is_cjk(target) {
            "Battle enemy letters switch from half-width (A, B…) to full-width (Ａ, Ｂ…)."
                .to_string()
        } else {
            "Battle enemy letters switch from full-width (Ａ, Ｂ…) to half-width (A, B…)."
                .to_string()
        });
    }

    let keyboard = |locale: &str| {
        if locale.starts_with("ja") {
            "Japanese kana"
        } else if locale.starts_with("ru") {
            "Cyrillic"
        } else {
            "Latin"
        }
    };
    let keyboard_code = uses.iter().any(|u| {
        u.location.contains("windows")
            && (u.code.contains("isJapanese") || u.code.contains("isRussian"))
    });
    if keyboard(current) != keyboard(target) && keyboard_code {
        let (from, to) = (keyboard(current), keyboard(target));
        lines.push(if name_input_commands == 0 {
            format!(
                "The name-input keyboard would switch from {from} to {to} letters; this game has no name-input event, so nothing visible changes there."
            )
        } else {
            format!(
                "The name-input keyboard switches from {from} to {to} letters ({name_input_commands} name-input event(s) in this game)."
            )
        });
    }

    let font = |locale: &str| {
        if locale.starts_with("zh") {
            Some("Chinese")
        } else if locale.starts_with("ko") {
            Some("Korean")
        } else {
            None
        }
    };
    if font(current) != font(target) && (reads("isChinese") || reads("isKorean")) {
        lines.push(
            "Code that picks a font with isChinese()/isKorean() now answers differently, so the window font may change."
                .to_string(),
        );
    }

    let mut plugin_files: Vec<&str> = uses
        .iter()
        .filter(|u| u.location.starts_with("js/plugins/"))
        .filter_map(|u| u.location.rsplit_once(':').map(|(file, _)| file))
        .collect();
    let plugin_lines = plugin_files.len();
    plugin_files.dedup();
    if plugin_lines > 0 {
        lines.push(format!(
            "{plugin_lines} plugin code line(s) read the setting ({}): those plugins switch to their non-{} texts and behaviour (often built-in English labels). Review the lines below.",
            plugin_files.join(", "),
            language_name(current)
        ));
    }
    lines
}

fn language_name(locale: &str) -> &'static str {
    match locale.get(..2) {
        Some("ja") => "Japanese",
        Some("zh") => "Chinese",
        Some("ko") => "Korean",
        Some("ru") => "Russian",
        Some("en") => "English",
        _ => "original-language",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_reads_but_not_definitions_or_comments() {
        let sources = vec![
            (
                "js/rmmz_objects.js".to_string(),
                "Game_System.prototype.isCJK = function() {\n    return $dataSystem.locale.match(/^(ja|zh|ko)/);\n};\nGame_Troop.prototype.letterTable = function() {\n    return $gameSystem.isCJK() ? Game_Troop.LETTER_TABLE_FULL : Game_Troop.LETTER_TABLE_HALF;\n};".to_string(),
            ),
            (
                "js/rmmz_windows.js".to_string(),
                "Window_NameInput.prototype.table = function() {\n    if ($gameSystem.isJapanese()) {".to_string(),
            ),
            (
                "js/plugins/nuun/NUUN_EnemyBook.js".to_string(),
                "/*:\n * isJapanese() in docs\n */\nthis.language_Jp = $gameSystem.isJapanese();\n// $gameSystem.isJapanese() in a comment".to_string(),
            ),
        ];
        let uses = locale_uses(&sources);
        let locations: Vec<_> = uses.iter().map(|u| u.location.as_str()).collect();
        assert_eq!(
            locations,
            vec![
                "js/rmmz_objects.js:5",
                "js/rmmz_windows.js:2",
                "js/plugins/nuun/NUUN_EnemyBook.js:4"
            ]
        );
        let lines = consequences("ja_JP", "en_US", &uses, 0);
        assert!(lines.iter().any(|l| l.contains("to half-width")));
        assert!(lines
            .iter()
            .any(|l| l.contains("from Japanese kana to Latin")));
        assert!(lines.iter().any(|l| l.contains("NUUN_EnemyBook")));
        assert!(!lines.iter().any(|l| l.contains("font")));
    }

    #[test]
    fn only_lists_what_actually_changes() {
        let uses = vec![LocaleUse {
            location: "js/rmmz_objects.js:9".into(),
            code: "return $gameSystem.isCJK() ? A : B;".into(),
        }];
        // Japanese → Chinese stays CJK: enemy letters do not change.
        assert_eq!(consequences("ja_JP", "zh_CN", &uses, 3).len(), 1);
        assert_eq!(consequences("ja_JP", "en_US", &uses, 3).len(), 2);
    }

    #[test]
    fn maps_languages_to_locales() {
        assert_eq!(target_locale("en"), "en_US");
        assert_eq!(target_locale("fr"), "fr_FR");
        assert_eq!(target_locale("xx"), "xx");
    }
}
