//! RPG Maker MZ message and help box geometry, from the engine's own formulas
//! (`rmmz_scenes.js` / `rmmz_windows.js`), plus a check of enabled plugins
//! that could change it.

use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;
use serde_json::Value;

use crate::engines::mv_mz::profiles::{ProfileSet, Verification};

/// `Game_System.windowPadding()`.
const WINDOW_PADDING: f32 = 12.0;
/// `Window_Base.lineHeight()`.
const LINE_HEIGHT: f32 = 36.0;
/// `Window_Base.itemPadding()` (help text starts at this x).
const ITEM_PADDING: f32 = 8.0;
/// `ImageManager.faceWidth`.
const FACE_WIDTH: f32 = 144.0;
/// `Window_Message.newLineX`: spacing after a portrait / margin without one.
const FACE_SPACING: f32 = 20.0;
const PLAIN_MARGIN: f32 = 4.0;
/// `Scene_Boot.adjustBoxSize`: `boxWidth = uiAreaWidth − boxMargin × 2`.
const BOX_MARGIN: f32 = 4.0;
/// Visual margin kept before the right edge (the engine itself clips at the
/// contents edge; this mirrors the 4 px left margin of a portrait-less box).
pub const RIGHT_MARGIN: f32 = 4.0;
/// `Scene_Message.messageWindowRect`: `calcWindowHeight(4) + 8`.
const MESSAGE_ROWS: f32 = 4.0;
const MESSAGE_EXTRA_HEIGHT: f32 = 8.0;
/// `Scene_MenuBase.helpAreaHeight`: `calcWindowHeight(2)`.
const HELP_ROWS: f32 = 2.0;

/// The Show Text window.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageBox {
    pub inner_width: f32,
    /// Contents height: a line that would end below it starts a new page.
    pub inner_height: f32,
}

impl MessageBox {
    /// `Window_Message.newLineX`.
    pub fn text_x(&self, face: bool) -> f32 {
        if face {
            FACE_WIDTH + FACE_SPACING
        } else {
            PLAIN_MARGIN
        }
    }

    /// Usable line width.
    pub fn text_width(&self, face: bool) -> f32 {
        self.inner_width - self.text_x(face) - RIGHT_MARGIN
    }
}

/// `Window_Help` (item/skill/weapon/armor descriptions, actor profile).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HelpBox {
    /// `baseTextRect().x`.
    pub text_x: f32,
    /// `baseTextRect().width`.
    pub text_width: f32,
    pub inner_height: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Geometry {
    /// `$gameSystem.mainFontSize()`.
    pub font_size: f32,
    /// `advanced.mainFontFilename` (in `fonts/`).
    pub font_file: Option<String>,
    /// `Graphics.boxWidth`.
    pub box_width: f32,
    pub line_height: f32,
    pub message: MessageBox,
    pub help: HelpBox,
}

impl Geometry {
    /// Geometry of an MZ game from its `System.json`.
    pub fn from_system(system: &Value) -> Self {
        let advanced = system.get("advanced");
        let number = |key: &str, default: f32| {
            advanced
                .and_then(|a| a.get(key))
                .and_then(Value::as_f64)
                .map_or(default, |n| n as f32)
        };
        let font_size = number("fontSize", 26.0);
        let box_width = number("uiAreaWidth", 816.0) - BOX_MARGIN * 2.0;
        let inner_width = box_width - WINDOW_PADDING * 2.0;
        let fitting = |rows: f32| rows * LINE_HEIGHT + WINDOW_PADDING * 2.0;
        Self {
            font_size,
            font_file: advanced
                .and_then(|a| a.get("mainFontFilename"))
                .and_then(Value::as_str)
                .filter(|name| !name.trim().is_empty())
                .map(str::to_string),
            box_width,
            line_height: LINE_HEIGHT,
            message: MessageBox {
                inner_width,
                inner_height: fitting(MESSAGE_ROWS) + MESSAGE_EXTRA_HEIGHT - WINDOW_PADDING * 2.0,
            },
            help: HelpBox {
                text_x: ITEM_PADDING,
                text_width: inner_width - ITEM_PADDING * 2.0,
                inner_height: fitting(HELP_ROWS) - WINDOW_PADDING * 2.0,
            },
        }
    }

    /// `lineHeight() − mainFontSize()`, added to a line's largest font size.
    pub fn line_spacing(&self) -> f32 {
        self.line_height - self.font_size
    }
}

// ---------------------------------------------------------------------------
// Plugins
// ---------------------------------------------------------------------------

/// Functions whose override can change where text is drawn, how wide it is
/// or where a page breaks.
static RE_LAYOUT_HOOK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"\b(?:Window_Message|Window_Base|Window_Help|Scene_Message|Scene_MenuBase|Scene_Battle|Scene_Status|Scene_Boot|Game_System)\.prototype\.(messageWindowRect|newLineX|numVisibleRows|lineHeight|windowPadding|mainFontSize|mainFontFace|resetFontSettings|calcTextHeight|maxFontSizeInLine|processEscapeCharacter|convertEscapeCharacters|obtainEscapeCode|helpWindowRect|helpAreaHeight|profileWindowRect|refresh|setItem|newPage|processNewLine|processCharacter|processControlCharacter|adjustBoxSize|flushTextState|textWidth|createTextState)\s*=|\bImageManager\.faceWidth\s*=",
    )
    .expect("RE_LAYOUT_HOOK must compile")
});

static RE_WORD_WRAP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)word\s*wrap").expect("RE_WORD_WRAP must compile"));

/// An enabled plugin that may change text layout and is not covered by a
/// reviewed profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutWarning {
    pub plugin: String,
    /// Overridden functions, or `word wrap` when the plugin wraps text itself.
    pub hooks: Vec<String>,
}

/// What enabled plugins change about text layout.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginLayout {
    pub warnings: Vec<LayoutWarning>,
    /// Notes from reviewed profiles, prefixed with the plugin title.
    pub notes: Vec<String>,
    /// Extra escape codes that print a value.
    pub inserted_codes: Vec<String>,
    /// A plugin draws the first line of item descriptions as plain text.
    pub plain_description_first_line: bool,
}

/// Check enabled plugins (`(name, source)`) against reviewed profiles.
pub fn plugin_layout(sources: &[(String, String)], profiles: &ProfileSet) -> PluginLayout {
    let mut out = PluginLayout::default();
    for (name, source) in sources {
        let reviewed = profiles
            .find(name)
            .filter(|profile| profile.verify(source) == Verification::Verified)
            .and_then(|profile| profile.layout.as_ref().map(|layout| (profile, layout)));
        if let Some((profile, layout)) = reviewed {
            for note in &layout.notes {
                out.notes.push(format!("{}: {note}", profile.plugin.title));
            }
            for code in &layout.inserted_codes {
                if !out.inserted_codes.contains(code) {
                    out.inserted_codes.push(code.clone());
                }
            }
            out.plain_description_first_line |= layout.plain_description_first_line;
            if layout.neutral {
                continue;
            }
        }
        let hooks = layout_hooks(source);
        if !hooks.is_empty() {
            out.warnings.push(LayoutWarning {
                plugin: name.clone(),
                hooks,
            });
        }
    }
    out
}

/// Layout-relevant overrides in code (comments skipped).
fn layout_hooks(source: &str) -> Vec<String> {
    let mut hooks = Vec::new();
    let mut in_comment = false;
    for line in source.lines() {
        let trimmed = line.trim_start();
        if in_comment {
            in_comment = !trimmed.contains("*/");
            continue;
        }
        if trimmed.starts_with("/*") {
            in_comment = !trimmed.contains("*/");
            continue;
        }
        if trimmed.starts_with("//") {
            continue;
        }
        for caps in RE_LAYOUT_HOOK.captures_iter(line) {
            let hook = caps.get(1).map_or("faceWidth", |m| m.as_str()).to_string();
            if !hooks.contains(&hook) {
                hooks.push(hook);
            }
        }
        if RE_WORD_WRAP.is_match(line) && !hooks.iter().any(|h| h == "word wrap") {
            hooks.push("word wrap".to_string());
        }
    }
    hooks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mz_geometry_matches_the_engine_formulas() {
        let system = serde_json::json!({
            "advanced": { "uiAreaWidth": 816, "fontSize": 26, "mainFontFilename": "mplus-1m-regular.woff" }
        });
        let geometry = Geometry::from_system(&system);
        assert_eq!(geometry.box_width, 808.0);
        assert_eq!(geometry.message.inner_width, 784.0);
        assert_eq!(geometry.message.inner_height, 152.0);
        assert_eq!(geometry.message.text_width(false), 776.0);
        assert_eq!(geometry.message.text_width(true), 616.0);
        assert_eq!(geometry.help.text_width, 768.0);
        assert_eq!(geometry.help.inner_height, 72.0);
        assert_eq!(geometry.line_spacing(), 10.0);
        assert_eq!(geometry.font_file.as_deref(), Some("mplus-1m-regular.woff"));
    }

    #[test]
    fn follows_ui_area_and_font_size() {
        let system = serde_json::json!({ "advanced": { "uiAreaWidth": 1280, "fontSize": 28 } });
        let geometry = Geometry::from_system(&system);
        assert_eq!(geometry.message.inner_width, 1272.0 - 24.0);
        assert_eq!(geometry.line_spacing(), 8.0);
        assert_eq!(geometry.font_file, None);
    }

    fn profiles() -> ProfileSet {
        ProfileSet::parse_all(&[(
            "neutral.toml",
            r#"
            [plugin]
            name = "Neutral"
            title = "Neutral plugin"
            verified_versions = ["1.0"]
            reference = "test"

            [layout]
            neutral = true
            inserted_codes = ["SV"]
            notes = ["An icon covers the corner."]
            "#,
        )])
        .unwrap()
    }

    #[test]
    fn warns_about_unreviewed_layout_hooks_only() {
        let sources = vec![
            (
                "Neutral".to_string(),
                "/*:\n * @version 1.0\n */\nWindow_Message.prototype.newPage = function() {};"
                    .to_string(),
            ),
            (
                "Wide".to_string(),
                "// Window_Message.prototype.newLineX = commented\nWindow_Message.prototype.newLineX = function() { return 300; };\nImageManager.faceWidth = 200;".to_string(),
            ),
            (
                "Wrapper".to_string(),
                "if (this._wordWrap) { /* Word Wrap */ }".to_string(),
            ),
            ("Harmless".to_string(), "Window_Message.prototype.update = f;".to_string()),
        ];
        let layout = plugin_layout(&sources, &profiles());
        assert_eq!(
            layout.warnings,
            vec![
                LayoutWarning {
                    plugin: "Wide".into(),
                    hooks: vec!["newLineX".into(), "faceWidth".into()]
                },
                LayoutWarning {
                    plugin: "Wrapper".into(),
                    hooks: vec!["word wrap".into()]
                },
            ]
        );
        assert_eq!(layout.inserted_codes, vec!["SV".to_string()]);
        assert_eq!(
            layout.notes,
            vec!["Neutral plugin: An icon covers the corner.".to_string()]
        );
    }

    #[test]
    fn an_unverified_profile_does_not_vouch_for_hooks() {
        let sources = vec![(
            "Neutral".to_string(),
            "/*:\n * @version 2.0\n */\nWindow_Message.prototype.newPage = function() {};"
                .to_string(),
        )];
        let layout = plugin_layout(&sources, &profiles());
        assert_eq!(layout.warnings.len(), 1);
        assert!(layout.inserted_codes.is_empty());
    }

    #[test]
    fn embedded_layout_profiles_load() {
        let embedded = ProfileSet::embedded();
        for name in [
            "PluginCommonBase",
            "Toriacontan/TemplateEvent",
            "nuun/NUUN_Base",
            "Toriacontan/MessageSkip",
            "MokuseiPengin/MPP_ChoiceEX",
            "SASUKE/TinyGetInfoWndMZ",
        ] {
            let profile = embedded.find(name).unwrap_or_else(|| panic!("{name}"));
            assert!(profile.layout.as_ref().is_some_and(|l| l.neutral), "{name}");
        }
        assert!(embedded
            .find("SASUKE/TinyGetInfoWndMZ")
            .and_then(|p| p.layout.as_ref())
            .is_some_and(|l| l.plain_description_first_line));
    }
}
