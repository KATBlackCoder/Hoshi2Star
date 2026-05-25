//! QA engine — checks each segment before it is saved to DB.
//!
//! ## Checks (in priority order)
//! 1. **Placeholders** — every escape code present in the source (`\V[n]`, `\C[n]`, …)
//!    must also appear in the target.  Uses the tokenizer to enumerate them.
//! 2. **Line length** — MV/MZ message boxes display at most `MAX_CHARS_PER_LINE` chars
//!    on `MAX_LINES` lines.  Lines beyond `MAX_LINES` are ignored (they typically
//!    overflow to the next message).
//! 3. **BOM** — a UTF-8 BOM (`\u{FEFF}`) at the start of the target causes
//!    mojibake in the game engine.
//!
//! ## Score
//! 100 if 0 errors.  Per error:
//! - `MissingPlaceholder` → −25
//! - `LineTooLong`        → −10
//! - `BomDetected`        → −15
//!
//! Minimum score: 0.

use crate::llm::tokenizer::{Engine as TokEngine, Tokenizer};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum characters per line for MV/MZ message boxes.
pub const MAX_CHARS_PER_LINE: usize = 50;

/// Maximum lines checked in one segment (excess lines are not checked).
pub const MAX_LINES: usize = 4;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QaError {
    MissingPlaceholder {
        placeholder: String,
    },
    LineTooLong {
        line: usize,
        length: usize,
        max: usize,
    },
    BomDetected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QaResult {
    pub score: u8,
    pub errors: Vec<QaError>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run all QA checks on a (source, target) pair.
///
/// `source` is the original Japanese text; `target` is the translation.
/// Returns a `QaResult` containing the score and the list of errors found.
pub fn check(source: &str, target: &str) -> QaResult {
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

    // 3. Line length check (up to MAX_LINES lines)
    for (i, line) in target.lines().take(MAX_LINES).enumerate() {
        let len = line.chars().count();
        if len > MAX_CHARS_PER_LINE {
            errors.push(QaError::LineTooLong {
                line: i + 1,
                length: len,
                max: MAX_CHARS_PER_LINE,
            });
        }
    }

    // Score calculation
    let penalty: i32 = errors
        .iter()
        .map(|e| match e {
            QaError::MissingPlaceholder { .. } => 25,
            QaError::LineTooLong { .. } => 10,
            QaError::BomDetected => 15,
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

    #[test]
    fn test_clean_segment_score_100() {
        let result = check(r"\V[12] pièces", r"\V[12] coins");
        assert_eq!(result.score, 100);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_missing_placeholder() {
        let result = check(r"\V[12] pièces", "coins");
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(
            &result.errors[0],
            QaError::MissingPlaceholder { placeholder } if placeholder == r"\V[12]"
        ));
        assert_eq!(result.score, 75);
    }

    #[test]
    fn test_multiple_missing_placeholders() {
        let result = check(r"\V[12] et \N[1]", "coins");
        assert_eq!(result.errors.len(), 2);
        assert_eq!(result.score, 50); // 100 - 25 - 25
    }

    #[test]
    fn test_line_too_long() {
        let long_line = "A".repeat(51);
        let result = check("hello", &long_line);
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(
            &result.errors[0],
            QaError::LineTooLong {
                line: 1,
                length: 51,
                max: 50
            }
        ));
        assert_eq!(result.score, 90); // 100 - 10
    }

    #[test]
    fn test_bom_detected() {
        let result = check("hello", "\u{FEFF}hello");
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(&result.errors[0], QaError::BomDetected));
        assert_eq!(result.score, 85); // 100 - 15
    }

    #[test]
    fn test_cumulative_score() {
        // BOM + missing placeholder + long line = -15 -25 -10 = 50
        let long_line = format!("\u{FEFF}{}", "A".repeat(51));
        let result = check(r"\V[12]", &long_line);
        // BomDetected + MissingPlaceholder + LineTooLong = 15+25+10 = 50
        assert_eq!(result.score, 50);
        assert_eq!(result.errors.len(), 3);
    }

    #[test]
    fn test_score_floor_zero() {
        // 4 missing placeholders = -100 → floor to 0
        let result = check(r"\V[1]\V[2]\V[3]\V[4]", "no placeholders here");
        assert_eq!(result.score, 0);
    }

    #[test]
    fn test_no_source_placeholders_passes() {
        // No placeholders in source → check does not fail
        let result = check("こんにちは", "Hello");
        assert_eq!(result.score, 100);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_lines_beyond_max_ignored() {
        // 5 long lines — only first 4 are checked
        let lines: Vec<String> = (0..5).map(|_| "A".repeat(51)).collect();
        let text = lines.join("\n");
        let result = check("hello", &text);
        // Only 4 errors (MAX_LINES = 4)
        let long_errors: Vec<_> = result
            .errors
            .iter()
            .filter(|e| matches!(e, QaError::LineTooLong { .. }))
            .collect();
        assert_eq!(long_errors.len(), 4);
    }
}
