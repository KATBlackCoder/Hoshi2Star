//! A text box measured in em on the game's own texts (plan DD, L1-C).
//!
//! When the engine's window cannot be read from the game data, it can be
//! measured on the game's original texts, in the game's own font: the widest
//! line (a high percentile, the longest lines overflowing in the original
//! too) and the most lines one text uses. Without a font size code the
//! game's font size cancels out, so widths are in em. Wolf RPG messages use
//! it (plan WR); the engine only says how one original text reads as tokens.

use serde::Serialize;

use super::font::Metrics;
use super::token::{font_size_after, width_of, Token, TokenKind, TokenRules};

/// Fewer original lines than this: no reliable measurement.
pub const MIN_SAMPLE_LINES: usize = 20;
/// Share of original lines that fit the measured width (the longest 1 % are
/// taken as overflowing in the original game too, plan WR, K10).
const WIDTH_PERCENTILE: f32 = 0.99;

/// What a text box holds, measured on the game's original texts.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeasuredTextBox {
    /// Line width in em of the game's default font size.
    pub width_em: f32,
    pub max_lines: usize,
}

impl MeasuredTextBox {
    /// Width (p99 of the line widths, in em) and line count (the most lines
    /// one text uses) of the original texts `sources`. `tokens_of` reads one
    /// original as the engine draws it, `None` when it cannot be measured
    /// (positioning or font size codes…). `None` below [`MIN_SAMPLE_LINES`]
    /// measurable lines.
    pub fn measure<'a>(
        sources: impl IntoIterator<Item = &'a str>,
        metrics: &Metrics,
        tokens_of: impl Fn(&str) -> Option<Vec<Token>>,
        rules: &impl TokenRules,
    ) -> Option<Self> {
        let mut widths: Vec<f32> = Vec::new();
        let mut max_lines = 0;
        for source in sources {
            let Some(tokens) = tokens_of(source) else {
                continue;
            };
            let lines = split_lines(tokens);
            max_lines = max_lines.max(lines.len());
            widths.extend(
                lines
                    .iter()
                    .map(|line| line_width(line, metrics, 1.0, rules)),
            );
        }
        if widths.len() < MIN_SAMPLE_LINES {
            return None;
        }
        widths.sort_by(f32::total_cmp);
        let index = ((widths.len() as f32 * WIDTH_PERCENTILE) as usize).min(widths.len() - 1);
        Some(Self {
            width_em: widths[index],
            max_lines,
        })
    }
}

/// Width of one line drawn from font size `start`.
pub fn line_width(tokens: &[Token], metrics: &Metrics, start: f32, rules: &impl TokenRules) -> f32 {
    let mut size = start;
    let mut width = 0.0;
    for token in tokens {
        width += width_of(token, metrics, size, rules);
        size = font_size_after(&token.kind, size, rules);
    }
    width
}

/// Tokens split at their line breaks (an empty text is one empty line).
pub fn split_lines(tokens: Vec<Token>) -> Vec<Vec<Token>> {
    let mut lines = vec![Vec::new()];
    for token in tokens {
        if token.kind == TokenKind::Newline {
            lines.push(Vec::new());
        } else if let Some(line) = lines.last_mut() {
            line.push(token);
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    /// No relative size codes, no icons.
    struct Plain;

    impl TokenRules for Plain {
        fn relative_font_size(&self, _bigger: bool, size: f32) -> f32 {
            size
        }
        fn icon_advance(&self) -> f32 {
            0.0
        }
    }

    /// A stand-in tokenizer: plain text and line breaks; a text starting
    /// with `#` cannot be measured.
    fn tokens_of(text: &str) -> Option<Vec<Token>> {
        if text.starts_with('#') {
            return None;
        }
        let mut tokens = Vec::new();
        for (index, line) in text.split('\n').enumerate() {
            if index > 0 {
                tokens.push(Token::new(TokenKind::Newline, "\n"));
            }
            if !line.is_empty() {
                tokens.push(Token::new(TokenKind::Text, line));
            }
        }
        Some(tokens)
    }

    #[test]
    fn measures_width_and_lines_on_the_originals() {
        // Estimated metrics: half an em per Latin character.
        let mut sources = vec!["aa\nbbbb"; 60];
        sources.push("cccccccccccccccccccccccccccccccccccccccc\nd\ne");
        sources.push("#ffffffffffffffffffffffffffffffffffffffffffffffffffffff");
        let measured =
            MeasuredTextBox::measure(sources, &Metrics::Estimated, tokens_of, &Plain).unwrap();
        // 123 lines: the 99th percentile is the 122nd width (2 em), not the
        // single 20 em line; the unmeasurable text is not counted.
        assert_eq!(
            measured,
            MeasuredTextBox {
                width_em: 2.0,
                max_lines: 3
            }
        );
    }

    #[test]
    fn too_few_lines_measure_nothing() {
        let sources = vec!["aa\nbb"; 9];
        assert!(
            MeasuredTextBox::measure(sources, &Metrics::Estimated, tokens_of, &Plain).is_none()
        );
        let sources = vec!["aa\nbb"; 10];
        assert!(
            MeasuredTextBox::measure(sources, &Metrics::Estimated, tokens_of, &Plain).is_some()
        );
    }

    #[test]
    fn lines_split_at_breaks() {
        let lines = split_lines(tokens_of("ab\n\ncd").unwrap());
        assert_eq!(lines.len(), 3);
        assert!(lines[1].is_empty());
        assert_eq!(line_width(&lines[2], &Metrics::Estimated, 1.0, &Plain), 1.0);
    }
}
