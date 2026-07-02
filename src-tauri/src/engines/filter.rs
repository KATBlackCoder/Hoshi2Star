//! Common text-filter utilities shared across all engine extractors.
//!
//! `needs_translation()` is the single gate deciding whether a segment is
//! worth extracting. Engine extractors call it with their specific `TokEngine`
//! variant so that the placeholder check uses the correct tokenizer.

use crate::llm::tokenizer::{Engine as TokEngine, Tokenizer};

/// Returns `true` if `file_name` is a numbered map file (`Map001{ext}` …
/// `MapNNN{ext}`), but not `MapInfos{ext}`. `ext` includes the leading dot
/// (e.g. `".json"` for MV/MZ, `".rvdata2"` for VX Ace).
///
/// Shared by the MV/MZ and VX Ace classifiers/dispatchers, which each spelled
/// this test out identically. The numeric-suffix check alone excludes
/// `MapInfos{ext}` (`"Infos"` is not a `u32`), so no explicit guard is needed.
pub fn is_map_file(file_name: &str, ext: &str) -> bool {
    file_name.starts_with("Map")
        && file_name
            .trim_start_matches("Map")
            .trim_end_matches(ext)
            .parse::<u32>()
            .is_ok()
}

/// Broad engine class of a `source_files.file_type`, used to route export and
/// injection without scattering `starts_with("wolf_")` / `starts_with("vx_")`
/// checks across the export layer. Adding an engine adds one variant here plus
/// one arm at each routing `match`.
pub enum FileClass {
    /// Wolf RPG binary (`wolf_map`, `wolf_database`, `wolf_common_events`).
    Wolf,
    /// RPG Maker VX Ace marshalled data (`vx_*`).
    VxAce,
    /// RPG Maker MV/MZ JSON (`map`, `actors`, … — the default).
    Json,
}

/// Classify a `source_files.file_type` into its [`FileClass`].
pub fn classify_file_type(file_type: &str) -> FileClass {
    if file_type.starts_with("wolf_") {
        FileClass::Wolf
    } else if file_type.starts_with("vx_") {
        FileClass::VxAce
    } else {
        FileClass::Json
    }
}

/// Returns `true` if `text` consists entirely of ASCII or fullwidth digits.
pub fn is_pure_number(text: &str) -> bool {
    let t = text.trim();
    !t.is_empty()
        && t.chars()
            .all(|c| c.is_ascii_digit() || ('０'..='９').contains(&c))
}

/// Returns `true` if `text` consists entirely of punctuation / symbols with no
/// Japanese characters or Latin words — nothing a translator can act on.
///
/// Catches: `…`, `-`, `？？？`, `！！！！`, `・・・`, `…？`, etc.
pub fn is_pure_symbol(text: &str) -> bool {
    let t = text.trim();
    !t.is_empty()
        && t.chars().all(|c| {
            matches!(
                c,
                '…' | '‥'
                    | '．'
                    | '。'
                    | '、'
                    | '！'
                    | '？'
                    | '!'
                    | '?'
                    | '.'
                    | '-'
                    | '―'
                    | '─'
                    | '＿'
                    | '_'
                    | '・'
                    | '★'
                    | '☆'
                    | '◆'
                    | '◇'
                    | '■'
                    | '□'
                    | '●'
                    | '○'
                    | '×'
                    | '＊'
                    | '*'
                    | '♪'
                    | '♫'
                    | '♦'
                    | '～'
                    | '~'
                    | '＝'
                    | '='
                    | '「'
                    | '」'
                    | '『'
                    | '』'
                    | '【'
                    | '】'
                    | '（'
                    | '）'
                    | '('
                    | ')'
                    | '／'
                    | '/'
                    | '|'
                    | '｜'
            ) || c.is_whitespace()
        })
}

/// Single filter gate for all engine extractors.
///
/// Returns `true` only when `text` contains content worth sending to a translator:
/// - not empty / whitespace-only
/// - not pure digits (`5`, `100`)
/// - not pure punctuation/symbols (`…`, `-`, `？？？`, `！！！！`)
/// - not exclusively engine escape codes (tokenized per `engine`)
pub fn needs_translation(text: &str, engine: TokEngine) -> bool {
    let t = text.trim();
    if t.is_empty() || is_pure_number(t) || is_pure_symbol(t) {
        return false;
    }
    // Tokenize with the engine-specific placeholder syntax and check whether
    // any real content remains after stripping all placeholder tokens.
    let tok = Tokenizer::tokenize(t, engine);
    if tok.map.is_empty() {
        return true; // No placeholders — content is real
    }
    let bare = tok
        .map
        .keys()
        .fold(tok.text.clone(), |s, k| s.replace(k.as_str(), ""));
    !bare.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_pure_number() {
        assert!(!is_pure_number(""));
        assert!(is_pure_number("5"));
        assert!(is_pure_number("100"));
        assert!(is_pure_number("０"));
        assert!(is_pure_number("１２３"));
        assert!(!is_pure_number("5a"));
        assert!(!is_pure_number("レベル5"));
        assert!(!is_pure_number("HP"));
    }

    #[test]
    fn test_is_pure_symbol() {
        assert!(!is_pure_symbol(""));
        assert!(is_pure_symbol("…"));
        assert!(is_pure_symbol("・・・"));
        assert!(is_pure_symbol("-"));
        assert!(is_pure_symbol("？？？"));
        assert!(is_pure_symbol("！！！！"));
        assert!(is_pure_symbol("…？"));
        assert!(is_pure_symbol("………"));
        assert!(!is_pure_symbol("反応なし…")); // Japanese present
        assert!(!is_pure_symbol("やった！！")); // Japanese present
        assert!(!is_pure_symbol("HP")); // Latin letters
    }

    #[test]
    fn test_needs_translation_empty_and_trivial() {
        assert!(!needs_translation("", TokEngine::MvMz));
        assert!(!needs_translation("   ", TokEngine::MvMz));
        assert!(!needs_translation("5", TokEngine::MvMz));
        assert!(!needs_translation("100", TokEngine::MvMz));
        assert!(!needs_translation("…", TokEngine::MvMz));
        assert!(!needs_translation("-", TokEngine::MvMz));
        assert!(!needs_translation("？？？", TokEngine::MvMz));
        assert!(!needs_translation("！！！！", TokEngine::MvMz));
        assert!(!needs_translation("・・・", TokEngine::MvMz));
    }

    #[test]
    fn test_needs_translation_placeholders_mvmz() {
        assert!(!needs_translation(r"\V[12]", TokEngine::MvMz));
        assert!(!needs_translation(r"\C[2]\N[4]", TokEngine::MvMz));
        assert!(needs_translation(r"\C[2]勇者", TokEngine::MvMz));
    }

    #[test]
    fn test_needs_translation_placeholders_wolf() {
        assert!(!needs_translation(r"\cdb[0:1:0]", TokEngine::Wolf));
        assert!(needs_translation("勇者の村", TokEngine::Wolf));
    }

    #[test]
    fn test_needs_translation_real_content() {
        assert!(needs_translation("反応なし…", TokEngine::MvMz));
        assert!(needs_translation("こんにちは！", TokEngine::MvMz));
        assert!(needs_translation("HP", TokEngine::MvMz)); // abbreviation, not pure symbol
    }
}
