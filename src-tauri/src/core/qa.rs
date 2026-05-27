//! QA engine — checks each segment before it is saved to DB.
//!
//! ## Checks (in priority order)
//! 1. **Placeholders** — every escape code present in the source (`\V[n]`, `\C[n]`, …)
//!    must also appear in the target.  Uses the tokenizer to enumerate them.
//! 2. **Line width** — MV/MZ message boxes have ~720 px of usable width.
//!    Full-width characters (CJK, hiragana, katakana) count as 2 half-width units;
//!    ASCII and other half-width characters count as 1 unit.
//!    The configurable limit is ~55.38 units (720 px / 13 px per half-width char).
//!    Lines beyond `LineWidthConfig::max_lines` are ignored.
//!    Empty lines are skipped.
//! 3. **BOM** — a UTF-8 BOM (`\u{FEFF}`) at the start of the target causes
//!    mojibake in the game engine.
//!
//! ## Score
//! 100 if 0 errors.  Per error:
//! - `MissingPlaceholder`  → −25
//! - `LineTooLong`         → −10
//! - `BomDetected`         → −15
//! - `GlossaryMismatch`    → −15
//!
//! Minimum score: 0.

use crate::llm::tokenizer::{Engine as TokEngine, Tokenizer};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Layout parameters for RPG Maker MV/MZ message boxes.
///
/// Default values are calibrated for the standard MV/MZ 816 px window
/// with ~48 px margins → ~720 px usable, using the default game font
/// (DejaVu Sans Mono-like metrics: 26 px full-width, 13 px half-width).
///
/// In F3, `LineWidthConfig` will be exposed as a per-project setting.
#[derive(Debug, Clone)]
pub struct LineWidthConfig {
    /// Usable box width in pixels (default: 720).
    pub box_width_px: f32,
    /// Width of one full-width character in pixels (default: 26.0).
    pub fullwidth_char_px: f32,
    /// Width of one half-width character in pixels (default: 13.0).
    pub halfwidth_char_px: f32,
    /// Maximum number of lines to check per segment (default: 4).
    pub max_lines: usize,
}

impl Default for LineWidthConfig {
    fn default() -> Self {
        Self {
            box_width_px: 720.0,
            fullwidth_char_px: 26.0,
            halfwidth_char_px: 13.0,
            max_lines: 4,
        }
    }
}

impl LineWidthConfig {
    /// Maximum line width in half-width units (derived: `box_width_px / halfwidth_char_px`).
    ///
    /// With default values: 720 / 13 ≈ 55.38 units.
    pub fn max_halfwidth_units(&self) -> f32 {
        self.box_width_px / self.halfwidth_char_px
    }
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QaError {
    MissingPlaceholder {
        placeholder: String,
    },
    LineTooLong {
        /// 1-based line number.
        line: usize,
        /// Measured width in half-width units (full-width = 2, half-width = 1).
        units: f32,
        /// Configured maximum width in half-width units.
        max_units: f32,
        /// Raw character count of the line.
        char_count: usize,
    },
    BomDetected,
    /// A glossary term whose source was found in the source text but whose
    /// expected target translation is absent from the target text.
    GlossaryMismatch {
        source_term: String,
        expected_target: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QaResult {
    pub score: u8,
    pub errors: Vec<QaError>,
}

// ---------------------------------------------------------------------------
// Width measurement
// ---------------------------------------------------------------------------

/// Returns `true` if `c` is a full-width character (CJK, kana, full-width ASCII/symbols).
fn is_fullwidth(c: char) -> bool {
    matches!(
        c,
        // CJK Unified Ideographs
        '\u{4E00}'..='\u{9FFF}'
        // Hiragana
        | '\u{3040}'..='\u{309F}'
        // Katakana
        | '\u{30A0}'..='\u{30FF}'
        // Full-width ASCII and half-width/full-width forms
        | '\u{FF00}'..='\u{FFEF}'
        // CJK Compatibility Ideographs
        | '\u{F900}'..='\u{FAFF}'
        // CJK Extension A
        | '\u{3400}'..='\u{4DBF}'
        // CJK Symbols and Punctuation
        | '\u{3000}'..='\u{303F}'
    )
}

/// Measures the display width of `line` in half-width units.
///
/// Full-width characters count as 2 units; all other characters count as 1 unit.
pub fn measure_line_units(line: &str) -> f32 {
    line.chars()
        .map(|c| if is_fullwidth(c) { 2.0_f32 } else { 1.0_f32 })
        .sum()
}

// ---------------------------------------------------------------------------
// Internal checks
// ---------------------------------------------------------------------------

fn check_glossary(source: &str, target: &str, terms: &[(String, String)]) -> Vec<QaError> {
    let target_lower = target.to_lowercase();
    terms
        .iter()
        .filter_map(|(source_term, target_term)| {
            if source.contains(source_term.as_str())
                && !target_lower.contains(target_term.to_lowercase().as_str())
            {
                Some(QaError::GlossaryMismatch {
                    source_term: source_term.clone(),
                    expected_target: target_term.clone(),
                })
            } else {
                None
            }
        })
        .collect()
}

fn check_line_length(text: &str, config: &LineWidthConfig) -> Vec<QaError> {
    let max_units = config.max_halfwidth_units();
    text.lines()
        .take(config.max_lines)
        .enumerate()
        .filter_map(|(i, line)| {
            if line.trim().is_empty() {
                return None;
            }
            let units = measure_line_units(line);
            if units > max_units {
                Some(QaError::LineTooLong {
                    line: i + 1,
                    units,
                    max_units,
                    char_count: line.chars().count(),
                })
            } else {
                None
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run all QA checks on a (source, target) pair.
///
/// `source` is the original Japanese text; `target` is the translation.
/// `glossary_terms` is a slice of `(source_term, expected_target)` pairs —
/// pass `&[]` when no glossary is available.
/// Returns a `QaResult` containing the score and the list of errors found.
pub fn check(source: &str, target: &str, glossary_terms: &[(String, String)]) -> QaResult {
    let mut errors: Vec<QaError> = Vec::new();

    // 1. BOM check (cheapest — do first)
    if target.starts_with('\u{FEFF}') {
        errors.push(QaError::BomDetected);
    }

    // 2. Placeholder check
    //    Tokenise the source to enumerate unique original placeholders,
    //    then verify each one is present in the (raw) target text.
    let tokenized = Tokenizer::tokenize(source, TokEngine::MvMz);
    let mut seen: HashSet<&str> = HashSet::new();
    for original in tokenized.map.values() {
        if seen.insert(original.as_str()) && !target.contains(original.as_str()) {
            errors.push(QaError::MissingPlaceholder {
                placeholder: original.clone(),
            });
        }
    }

    // 3. Line width check
    errors.extend(check_line_length(target, &LineWidthConfig::default()));

    // 4. Glossary mismatch check
    if !glossary_terms.is_empty() {
        errors.extend(check_glossary(source, target, glossary_terms));
    }

    // Score calculation
    let penalty: i32 = errors
        .iter()
        .map(|e| match e {
            QaError::MissingPlaceholder { .. } => 25,
            QaError::LineTooLong { .. } => 10,
            QaError::BomDetected => 15,
            QaError::GlossaryMismatch { .. } => 15,
        })
        .sum();

    let score = (100 - penalty).max(0) as u8;

    QaResult { score, errors }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- is_fullwidth ---

    #[test]
    fn test_is_fullwidth_kanji() {
        assert!(is_fullwidth('日'));
        assert!(is_fullwidth('本'));
        assert!(is_fullwidth('語'));
    }

    #[test]
    fn test_is_fullwidth_hiragana() {
        assert!(is_fullwidth('あ'));
        assert!(is_fullwidth('い'));
        assert!(is_fullwidth('う'));
    }

    #[test]
    fn test_is_fullwidth_katakana() {
        assert!(is_fullwidth('ア'));
        assert!(is_fullwidth('カ'));
    }

    #[test]
    fn test_is_fullwidth_ascii_halfwidth() {
        assert!(!is_fullwidth('A'));
        assert!(!is_fullwidth('1'));
        assert!(!is_fullwidth(' '));
        assert!(!is_fullwidth('!'));
    }

    // --- measure_line_units ---

    #[test]
    fn test_measure_line_units_ascii() {
        assert_eq!(measure_line_units("ABC"), 3.0);
    }

    #[test]
    fn test_measure_line_units_japanese() {
        // 3 kanji × 2 = 6
        assert_eq!(measure_line_units("日本語"), 6.0);
    }

    #[test]
    fn test_measure_line_units_mixed() {
        // 'A'=1 + 'B'=1 + '日'=2 = 4
        assert_eq!(measure_line_units("AB日"), 4.0);
    }

    // --- check_line_length ---

    #[test]
    fn test_check_line_length_empty_lines_ignored() {
        let errors = check_line_length("\n   \n", &LineWidthConfig::default());
        assert!(errors.is_empty());
    }

    #[test]
    fn test_check_line_length_jp_long_triggers_error() {
        // 28 kanji × 2 = 56.0 units > 55.38 max
        let line = "日".repeat(28);
        let errors = check_line_length(&line, &LineWidthConfig::default());
        assert_eq!(errors.len(), 1);
        match &errors[0] {
            QaError::LineTooLong {
                line: n,
                units,
                max_units,
                char_count,
            } => {
                assert_eq!(*n, 1);
                assert_eq!(*char_count, 28);
                assert!(
                    *units > *max_units,
                    "units={units} should exceed max_units={max_units}"
                );
            }
            _ => panic!("expected LineTooLong"),
        }
    }

    #[test]
    fn test_check_line_length_en_within_limit() {
        // 55 ASCII chars = 55.0 units < 55.38 max → no error
        let line = "A".repeat(55);
        let errors = check_line_length(&line, &LineWidthConfig::default());
        assert!(errors.is_empty());
    }

    // --- check() integration ---

    #[test]
    fn test_clean_segment_score_100() {
        let result = check(r"\V[12] pièces", r"\V[12] coins", &[]);
        assert_eq!(result.score, 100);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_missing_placeholder() {
        let result = check(r"\V[12] pièces", "coins", &[]);
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(
            &result.errors[0],
            QaError::MissingPlaceholder { placeholder } if placeholder == r"\V[12]"
        ));
        assert_eq!(result.score, 75);
    }

    #[test]
    fn test_multiple_missing_placeholders() {
        let result = check(r"\V[12] et \N[1]", "coins", &[]);
        assert_eq!(result.errors.len(), 2);
        assert_eq!(result.score, 50); // 100 - 25 - 25
    }

    #[test]
    fn test_line_too_long() {
        // 56 ASCII chars = 56.0 units > 55.38 max
        let long_line = "A".repeat(56);
        let result = check("hello", &long_line, &[]);
        assert_eq!(result.errors.len(), 1);
        match &result.errors[0] {
            QaError::LineTooLong {
                line,
                units,
                max_units,
                char_count,
            } => {
                assert_eq!(*line, 1);
                assert_eq!(*char_count, 56);
                assert!(*units > *max_units);
            }
            _ => panic!("expected LineTooLong"),
        }
        assert_eq!(result.score, 90); // 100 - 10
    }

    #[test]
    fn test_bom_detected() {
        let result = check("hello", "\u{FEFF}hello", &[]);
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(&result.errors[0], QaError::BomDetected));
        assert_eq!(result.score, 85); // 100 - 15
    }

    #[test]
    fn test_cumulative_score() {
        // BOM + missing placeholder + long line = -15 -25 -10 = 50
        // '\u{FEFF}' (1 unit) + 56 × 'A' (56 units) = 57.0 > 55.38 → LineTooLong
        let long_line = format!("\u{FEFF}{}", "A".repeat(56));
        let result = check(r"\V[12]", &long_line, &[]);
        assert_eq!(result.score, 50);
        assert_eq!(result.errors.len(), 3);
    }

    #[test]
    fn test_score_floor_zero() {
        // 4 missing placeholders = -100 → floor to 0
        let result = check(r"\V[1]\V[2]\V[3]\V[4]", "no placeholders here", &[]);
        assert_eq!(result.score, 0);
    }

    #[test]
    fn test_no_source_placeholders_passes() {
        let result = check("こんにちは", "Hello", &[]);
        assert_eq!(result.score, 100);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_lines_beyond_max_ignored() {
        // 5 lines of 56 'A' — only first 4 are checked (max_lines = 4)
        let lines: Vec<String> = (0..5).map(|_| "A".repeat(56)).collect();
        let text = lines.join("\n");
        let result = check("hello", &text, &[]);
        let long_errors: Vec<_> = result
            .errors
            .iter()
            .filter(|e| matches!(e, QaError::LineTooLong { .. }))
            .collect();
        assert_eq!(long_errors.len(), 4);
    }

    // --- check_glossary ---

    fn terms(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(s, t)| (s.to_string(), t.to_string()))
            .collect()
    }

    #[test]
    fn test_glossary_mismatch_detected() {
        // "魔法使い" appears in source, "Mage" absent from target → GlossaryMismatch
        let t = terms(&[("魔法使い", "Mage")]);
        let result = check("魔法使い が現れた！", "A sorcerer appeared!", &t);
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(
            &result.errors[0],
            QaError::GlossaryMismatch { source_term, expected_target }
                if source_term == "魔法使い" && expected_target == "Mage"
        ));
        assert_eq!(result.score, 85); // 100 - 15
    }

    #[test]
    fn test_glossary_term_present_no_error() {
        // "Mage" IS in target (case-insensitive) → no error
        let t = terms(&[("魔法使い", "Mage")]);
        let result = check("魔法使い が現れた！", "A mage appeared!", &t);
        assert!(result.errors.is_empty());
        assert_eq!(result.score, 100);
    }

    #[test]
    fn test_glossary_no_source_term_no_error() {
        // "魔法使い" not in source → glossary check is skipped
        let t = terms(&[("魔法使い", "Mage")]);
        let result = check("戦士 が現れた！", "A warrior appeared!", &t);
        assert!(result.errors.is_empty());
        assert_eq!(result.score, 100);
    }

    #[test]
    fn test_glossary_empty_terms_no_error() {
        // Empty glossary → no GlossaryMismatch regardless of target
        let result = check("魔法使い", "something else", &[]);
        assert!(result.errors.is_empty());
        assert_eq!(result.score, 100);
    }

    #[test]
    fn test_glossary_multiple_mismatches_floor_zero() {
        // 7 mismatches = -105 → floor to 0
        let t = terms(&[
            ("term1", "T1"),
            ("term2", "T2"),
            ("term3", "T3"),
            ("term4", "T4"),
            ("term5", "T5"),
            ("term6", "T6"),
            ("term7", "T7"),
        ]);
        let source = "term1 term2 term3 term4 term5 term6 term7";
        let result = check(source, "wrong translation", &t);
        assert_eq!(result.score, 0);
    }
}
