//! Compile-time embedded prompt templates for LLM calls.
//!
//! Each template lives in `src-tauri/prompts/<task>/<target_lang>.toml`
//! (or `default.toml` as fallback). Files are embedded via `include_str!()` —
//! no runtime file access, no Tauri bundle config needed.
//!
//! Adding a new target language: create the `.toml`, add a `match` arm in
//! `translate_for()` / `glossary_for()`, add the code in `lang_code_to_name()`.

use serde::Deserialize;
use std::sync::LazyLock;

// ---------------------------------------------------------------------------
// PromptTemplate
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct PromptTemplate {
    pub system: String,
    pub user: String,
}

impl PromptTemplate {
    /// Substitute `{{key}}` placeholders in `part` with the provided values.
    pub fn render(&self, part: &str, vars: &[(&str, &str)]) -> String {
        let mut out = part.to_string();
        for (key, value) in vars {
            out = out.replace(&format!("{{{{{key}}}}}"), value);
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Language code → full name
// ---------------------------------------------------------------------------

/// Map a BCP-47 language code to the full English name sent to the LLM.
/// Unknown codes are returned as-is so the LLM still gets something useful.
pub fn lang_code_to_name(code: &str) -> &str {
    match code {
        "ja" => "Japanese",
        "en" => "English",
        "fr" => "French",
        "es" => "Spanish",
        "de" => "German",
        "ko" => "Korean",
        "zh" => "Chinese",
        other => other,
    }
}

// ---------------------------------------------------------------------------
// Embedded templates (parsed once via LazyLock)
// ---------------------------------------------------------------------------

static TRANSLATE_DEFAULT: LazyLock<PromptTemplate> = LazyLock::new(|| {
    toml::from_str(include_str!("../../prompts/translate/default.toml"))
        .expect("prompts/translate/default.toml is malformed")
});

static GLOSSARY_DEFAULT: LazyLock<PromptTemplate> = LazyLock::new(|| {
    toml::from_str(include_str!("../../prompts/glossary/default.toml"))
        .expect("prompts/glossary/default.toml is malformed")
});

// ---------------------------------------------------------------------------
// Public accessors with per-language routing
// ---------------------------------------------------------------------------

/// Return the translation prompt template for the given target language code.
/// Falls back to `default.toml` for any language without a dedicated file.
///
/// To add a language: embed a new static, then match on the code here.
/// Example: `"fr" => &TRANSLATE_FR` once `prompts/translate/fr.toml` exists.
pub fn translate_for(_target_lang: &str) -> &'static PromptTemplate {
    &TRANSLATE_DEFAULT
}

/// Return the glossary extraction prompt template for the given target language code.
/// Falls back to `default.toml` for any language without a dedicated file.
///
/// To add a language: embed a new static, then match on the code here.
/// Example: `"fr" => &GLOSSARY_FR` once `prompts/glossary/fr.toml` exists.
pub fn glossary_for(_target_lang: &str) -> &'static PromptTemplate {
    &GLOSSARY_DEFAULT
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_default_loads() {
        let tmpl = translate_for("en");
        assert!(tmpl.system.contains("CRITICAL RULE"));
        assert!(tmpl.user.contains("/no_think"));
    }

    #[test]
    fn test_glossary_default_loads() {
        let tmpl = glossary_for("en");
        assert!(tmpl.system.contains("JSON array"));
        assert!(tmpl.user.contains("{{target_lang}}"));
    }

    #[test]
    fn test_render_substitutes_vars() {
        let tmpl = translate_for("en");
        let out = tmpl.render(
            &tmpl.system,
            &[
                ("source_lang", "Japanese"),
                ("target_lang", "English"),
                ("glossary", ""),
            ],
        );
        assert!(out.contains("Japanese"));
        assert!(out.contains("English"));
        assert!(!out.contains("{{source_lang}}"));
        assert!(!out.contains("{{target_lang}}"));
    }

    #[test]
    fn test_render_glossary_hint_injected() {
        let tmpl = translate_for("en");
        let hint = "\nGlossary:\n勇者 → Hero";
        let out = tmpl.render(
            &tmpl.system,
            &[
                ("source_lang", "Japanese"),
                ("target_lang", "English"),
                ("glossary", hint),
            ],
        );
        assert!(out.contains("勇者 → Hero"));
    }

    #[test]
    fn test_render_empty_glossary() {
        let tmpl = translate_for("en");
        let out = tmpl.render(
            &tmpl.system,
            &[
                ("source_lang", "Japanese"),
                ("target_lang", "English"),
                ("glossary", ""),
            ],
        );
        assert!(!out.contains("{{glossary}}"));
    }

    #[test]
    fn test_lang_code_to_name() {
        assert_eq!(lang_code_to_name("ja"), "Japanese");
        assert_eq!(lang_code_to_name("en"), "English");
        assert_eq!(lang_code_to_name("fr"), "French");
        assert_eq!(lang_code_to_name("xx"), "xx"); // unknown → passthrough
    }

    #[test]
    fn test_render_segments_user() {
        let tmpl = translate_for("en");
        let segments = "[1] 主人公\n[2] 剣";
        let out = tmpl.render(&tmpl.user, &[("segments", segments)]);
        assert!(out.contains("[1] 主人公"));
        assert!(!out.contains("{{segments}}"));
    }
}
