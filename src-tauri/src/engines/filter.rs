//! Common text-filter utilities shared across all engine extractors.
//!
//! `needs_translation()` is the single gate deciding whether a segment is
//! worth extracting. Engine extractors call it with their specific `TokEngine`
//! variant so that the placeholder check uses the correct tokenizer.

use crate::llm::tokenizer::{Engine as TokEngine, Tokenizer};

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
