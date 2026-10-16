//! Fit check on the final values, run at export before anything is written.
//!
//! A fitted message or description must draw without a line wider than its
//! box, without a blank page, with no control characters besides `\n` and
//! U+000C, and with exactly the translated text: the same visible
//! characters (whitespace aside) and the same escape codes in the same order.
//! The only codes fitting may add are font size and colour codes at the
//! start of a line, where it re-emits the font and colour in effect (page
//! starts, a description's second line).

use serde::Serialize;

use super::font::Metrics;
use super::simulate::{simulate, BoxSpec};
use super::text::{tokenize, Inserts, Token, TokenKind, PAGE_BREAK};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FitProblem {
    /// A line is wider than the box (or below a help box).
    Overflow { page: usize, line: usize },
    /// A page with nothing drawn on it.
    BlankPage { page: usize },
    /// A control character other than `\n` / U+000C.
    ControlCharacter { code: u32 },
    /// The text or its escape codes differ from the translation.
    TextChanged,
}

/// One escape code of a text, in order.
struct Code {
    raw: String,
    /// A font size or colour code at the start of a line: the only kind
    /// fitting may add.
    reemitted: bool,
}

/// What must survive fitting: visible characters without whitespace, and
/// every escape code in order.
fn essence(text: &str, inserts: &Inserts) -> (String, Vec<Code>) {
    let mut visible = String::new();
    let mut codes = Vec::new();
    let mut line_start = true;
    for token in tokenize(text, inserts) {
        match &token.kind {
            TokenKind::Text => {
                visible.extend(token.raw.chars().filter(|c| !c.is_whitespace()));
                line_start = false;
            }
            TokenKind::Newline | TokenKind::PageBreak => line_start = true,
            kind => {
                let font_or_color = matches!(kind, TokenKind::FontSize(_) | TokenKind::Color(_));
                codes.push(Code {
                    raw: normalized_code(&token),
                    reemitted: line_start && font_or_color,
                });
                line_start &= font_or_color;
            }
        }
    }
    (visible, codes)
}

fn normalized_code(token: &Token) -> String {
    token.raw.to_ascii_uppercase()
}

/// `true` when `output` holds `translated`'s codes in the same order, plus
/// only re-emitted font size / colour codes.
fn same_codes(translated: &[Code], output: &[Code]) -> bool {
    // ok[j]: translated[i..] matches output[j..], for the current i
    // (rows filled from the end; O(n·m) time, O(m) memory).
    let m = output.len();
    let mut ok = vec![false; m + 1];
    ok[m] = true;
    for j in (0..m).rev() {
        ok[j] = output[j].reemitted && ok[j + 1];
    }
    for code in translated.iter().rev() {
        let mut row = vec![false; m + 1];
        for j in (0..m).rev() {
            row[j] =
                (output[j].raw == code.raw && ok[j + 1]) || (output[j].reemitted && row[j + 1]);
        }
        ok = row;
    }
    ok[0]
}

/// Check a fitted text (`output`) drawn in `spec` against the translated
/// text it came from. `allow_overflow`: the user chose to leave it.
pub fn check(
    translated: &str,
    output: &str,
    spec: &BoxSpec,
    metrics: &Metrics,
    inserts: &Inserts,
    allow_overflow: bool,
) -> Vec<FitProblem> {
    let mut problems = Vec::new();
    for c in output.chars() {
        if (c as u32) < 0x20 && c != '\n' && c != PAGE_BREAK {
            problems.push(FitProblem::ControlCharacter { code: c as u32 });
        }
    }
    let (visible_in, codes_in) = essence(translated, inserts);
    let (visible_out, codes_out) = essence(output, inserts);
    if visible_in != visible_out || !same_codes(&codes_in, &codes_out) {
        problems.push(FitProblem::TextChanged);
    }
    let pages = simulate(output, spec, metrics, inserts);
    for (page_index, page) in pages.iter().enumerate() {
        if !page.is_visible() && pages.len() > 1 {
            problems.push(FitProblem::BlankPage { page: page_index });
        }
        if allow_overflow {
            continue;
        }
        for (line_index, line) in page.lines.iter().enumerate() {
            if line.overflow || line.clipped {
                problems.push(FitProblem::Overflow {
                    page: page_index,
                    line: line_index,
                });
            }
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> BoxSpec {
        BoxSpec {
            text_x: 4.0,
            limit: 780.0,
            height: 152.0,
            main_font: 26.0,
            line_spacing: 10.0,
            paging: true,
        }
    }

    fn problems(translated: &str, output: &str) -> Vec<FitProblem> {
        check(
            translated,
            output,
            &spec(),
            &Metrics::Estimated,
            &Inserts::default(),
            false,
        )
    }

    #[test]
    fn accepts_rewrapped_text_with_repeated_codes() {
        assert!(problems("a b \\C[2]c d", "a b\n\\C[2]c\u{000C}\n\\FS[26]\\C[2]d").is_empty());
    }

    #[test]
    fn font_and_colour_codes_must_survive_in_order() {
        // A translator's own font code may not be dropped (or moved).
        assert_eq!(
            problems("\\FS[20]small words", "small words"),
            vec![FitProblem::TextChanged]
        );
        assert!(problems("\\FS[20]small words", "\\FS[26]\\FS[20]small\nwords").is_empty());
        // Colour codes in a different order are a different text.
        assert_eq!(
            problems("\\C[2]a\\C[3]b", "\\C[3]a\\C[2]b"),
            vec![FitProblem::TextChanged]
        );
        // Codes fitting adds are only accepted at a line start.
        assert_eq!(problems("a b", "a \\C[2]b"), vec![FitProblem::TextChanged]);
        assert!(problems("a b", "a\n\\C[2]b").is_empty());
        assert_eq!(problems("a b", "a\n\\I[3]b"), vec![FitProblem::TextChanged]);
    }

    #[test]
    fn rejects_lost_or_added_text_and_codes() {
        assert_eq!(problems("a b", "a"), vec![FitProblem::TextChanged]);
        assert_eq!(problems("a \\I[3]", "a"), vec![FitProblem::TextChanged]);
        assert_eq!(problems("a \\C[2]b", "a b"), vec![FitProblem::TextChanged]);
        assert_eq!(
            problems("ab", "a\u{0007}b"),
            vec![
                FitProblem::ControlCharacter { code: 7 },
                FitProblem::TextChanged
            ]
        );
    }

    #[test]
    fn rejects_overflow_and_blank_pages() {
        let long = "x".repeat(70);
        assert_eq!(
            problems(&long, &long),
            vec![FitProblem::Overflow { page: 0, line: 0 }]
        );
        assert!(check(
            &long,
            &long,
            &spec(),
            &Metrics::Estimated,
            &Inserts::default(),
            true
        )
        .is_empty());
        assert_eq!(
            problems("1 2 3 4 5", "1\n2\n3\n4\n\u{000C}5"),
            vec![FitProblem::BlankPage { page: 1 }]
        );
    }
}
