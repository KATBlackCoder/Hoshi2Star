//! Line breaking of MZ message text: the shared line breaking
//! ([`crate::core::text_fit::wrap`]) with the MZ token rules
//! ([`MzTokenRules`]). Kept at this path for its callers and tests.

use super::font::Metrics;
use super::text::{MzTokenRules, Token};
use crate::core::text_fit::wrap as shared;

pub use crate::core::text_fit::wrap::{trim_line, wrap, Piece};

/// Font size in effect before each token, starting from `start`.
pub fn sizes_before(tokens: &[Token], start: f32) -> Vec<f32> {
    shared::sizes_before(tokens, start, &MzTokenRules)
}

/// Font size after all `tokens`.
pub fn size_after(tokens: &[Token], start: f32) -> f32 {
    shared::size_after(tokens, start, &MzTokenRules)
}

/// Split one paragraph (no newlines) into pieces at its break opportunities.
pub fn pieces(tokens: &[Token], metrics: &Metrics, start_size: f32) -> Vec<Piece> {
    shared::pieces(tokens, metrics, start_size, &MzTokenRules)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::mv_mz::layout::text::{raw_of, tokenize, Inserts};

    fn lines(text: &str, width: f32) -> Vec<String> {
        let metrics = Metrics::Estimated;
        let tokens = tokenize(text, &Inserts::default());
        let pieces = pieces(&tokens, &metrics, 26.0);
        wrap(&pieces, width, None)
            .unwrap()
            .iter()
            .map(|line| raw_of(line))
            .collect()
    }

    #[test]
    fn breaks_at_spaces_and_balances_lines() {
        // 13 px per Latin character at 26 px (estimate).
        assert_eq!(
            lines("aaa bbb ccc ddd", 13.0 * 15.0),
            vec!["aaa bbb ccc ddd"]
        );
        // Greedy would give "aaa bbb ccc" / "ddd"; balanced splits evenly.
        assert_eq!(
            lines("aaa bbb ccc ddd", 13.0 * 11.0),
            vec!["aaa bbb", "ccc ddd"]
        );
    }

    #[test]
    fn codes_travel_with_the_next_word_or_stay_glued() {
        assert_eq!(
            lines("one two \\C[2]three\\C[0] four", 13.0 * 8.0),
            vec!["one two", "\\C[2]three\\C[0]", "four"]
        );
    }

    #[test]
    fn no_break_before_closing_punctuation_or_ellipsis() {
        let out = lines("Well… you see, it's fine!", 13.0 * 9.0);
        assert!(out
            .iter()
            .all(|line| !line.starts_with('…') && !line.starts_with('!')));
        assert_eq!(out.join(" "), "Well… you see, it's fine!");
    }

    #[test]
    fn exact_line_counts_and_overlong_words() {
        let metrics = Metrics::Estimated;
        let tokens = tokenize("aa bb cc", &Inserts::default());
        let pieces = pieces(&tokens, &metrics, 26.0);
        let three = wrap(&pieces, 1000.0, Some(3)).unwrap();
        assert_eq!(
            three.iter().map(|l| raw_of(l)).collect::<Vec<_>>(),
            vec!["aa", "bb", "cc"]
        );
        assert!(wrap(&pieces, 1000.0, Some(4)).is_none());
        // A word wider than the line takes a line on its own.
        assert_eq!(
            lines("a supercalifragilistic b", 13.0 * 5.0),
            vec!["a", "supercalifragilistic", "b"]
        );
    }

    #[test]
    fn font_changes_affect_following_widths() {
        let metrics = Metrics::Estimated;
        let tokens = tokenize("ab \\{cd ef", &Inserts::default());
        let pieces = pieces(&tokens, &metrics, 26.0);
        // "ab " at 26 px; "cd " and "ef" at 38 px (19 px per Latin char).
        assert_eq!(pieces[0].width, 26.0);
        assert_eq!(pieces[1].width, 38.0);
        assert_eq!(pieces[2].width, 38.0);
        assert!((size_after(&tokens, 26.0) - 38.0).abs() < f32::EPSILON);
    }

    #[test]
    fn sentence_ends_are_marked() {
        let metrics = Metrics::Estimated;
        let tokens = tokenize("Hi. I'm here! Ok", &Inserts::default());
        let ends: Vec<bool> = pieces(&tokens, &metrics, 26.0)
            .iter()
            .map(|p| p.sentence_end)
            .collect();
        assert_eq!(ends, vec![true, false, true, false]);
    }

    /// The previous all-line-counts dynamic program (O(n²·k²) time, O(n²)
    /// memory), kept as the reference for the fewest-lines layout.
    fn reference_wrap(pieces: &[Piece], width: f32) -> Vec<Vec<Token>> {
        let n = pieces.len();
        let line_width = |pieces: &[Piece]| {
            let total: f32 = pieces.iter().map(|p| p.width + p.space).sum();
            total - pieces.last().map_or(0.0, |p| p.space)
        };
        let fits = |i: usize, j: usize| j == i + 1 || line_width(&pieces[i..j]) <= width + 0.001;
        let slack = |i: usize, j: usize| {
            let gap = (width - line_width(&pieces[i..j])).max(0.0) as f64;
            gap * gap
        };
        let mut best = vec![vec![f64::INFINITY; n + 1]; n + 1];
        let mut back = vec![vec![0usize; n + 1]; n + 1];
        best[0][0] = 0.0;
        for c in 1..=n {
            for j in 1..=n {
                for i in (0..j).rev() {
                    if !fits(i, j) {
                        break;
                    }
                    if best[c - 1][i].is_finite() {
                        let cost = best[c - 1][i] + slack(i, j);
                        if cost < best[c][j] {
                            best[c][j] = cost;
                            back[c][j] = i;
                        }
                    }
                }
            }
        }
        let lines = (1..=n).find(|&c| best[c][n].is_finite()).unwrap();
        let mut bounds = Vec::new();
        let mut j = n;
        for c in (1..=lines).rev() {
            bounds.push((back[c][j], j));
            j = back[c][j];
        }
        bounds.reverse();
        bounds
            .into_iter()
            .map(|(i, j)| trim_line(pieces[i..j].iter().flat_map(|p| p.tokens.clone()).collect()))
            .collect()
    }

    #[test]
    fn fewest_lines_layout_matches_the_reference_program() {
        let metrics = Metrics::Estimated;
        let words = [
            "a",
            "tiny",
            "word",
            "considerably",
            "and",
            "…",
            "\\C[2]red\\C[0]",
            "I",
            "extraordinarily",
            "be",
            "it's",
            "ok!",
            "no.",
            "magic",
        ];
        for seed in 0..60usize {
            let text: Vec<&str> = (0..(5 + seed * 3))
                .map(|k| words[(k * 7 + seed * 13 + k * k) % words.len()])
                .collect();
            let tokens = tokenize(&text.join(" "), &Inserts::default());
            let pieces = pieces(&tokens, &metrics, 26.0);
            for width in [13.0 * 6.0, 13.0 * 17.0, 13.0 * 30.0, 776.0] {
                assert_eq!(
                    wrap(&pieces, width, None).unwrap(),
                    reference_wrap(&pieces, width),
                    "seed {seed}, width {width}"
                );
            }
        }
    }

    #[test]
    fn long_texts_wrap_in_linear_memory() {
        let metrics = Metrics::Estimated;
        let text = "lorem ipsum dolor sit amet ".repeat(4_000);
        let tokens = tokenize(&text, &Inserts::default());
        let pieces = pieces(&tokens, &metrics, 26.0);
        assert_eq!(pieces.len(), 20_000);
        // The old program would have needed two 20 001 × 20 001 tables here.
        let lines = wrap(&pieces, 776.0, None).unwrap();
        assert!(lines.len() > 1_000);
    }
}
