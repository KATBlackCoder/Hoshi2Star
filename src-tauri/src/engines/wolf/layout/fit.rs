//! Fitting one Wolf RPG message to its window (plan WR, rules R2–R5).
//!
//! The width budget is measured in em on the game's own message lines
//! ([`measure_budget`]): the window size in pixels never has to be known, and
//! without a font size code the game's font size cancels out. A message is one
//! segment; it is cleaned of stray line breaks, then rewrapped. Event
//! structure never changes: a message too long for its window is shrunk with
//! a leading `\f[m]` when allowed, or left and reported.

use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use super::text::{split_face_prefix, tokenize};
use crate::engines::mv_mz::layout::fit::{FitOutcome, LineBreaks, Overflow};
use crate::engines::mv_mz::layout::font::Metrics;
use crate::engines::mv_mz::layout::text::{
    apply_font_change, raw_of, token_width, Token, TokenKind,
};
use crate::engines::mv_mz::layout::wrap::{pieces, size_after, wrap};

/// Fewer original lines than this: no reliable measurement.
pub const MIN_SAMPLE_LINES: usize = 20;
/// Share of original lines that fit the measured width (the longest 1 % are
/// taken as overflowing in the original game too, plan WR, K10).
const WIDTH_PERCENTILE: f32 = 0.99;
/// Floating-point tolerance on widths.
const EPSILON: f32 = 0.001;
/// A speaker label line (`魚淫さん：`, `Gyoin:`) is at most this long.
const LABEL_MAX_CHARS: usize = 24;

/// Leading `\f[n]` of a message body (the export prefix, or a shrink).
static RE_LEADING_SIZE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\\[fF]\[\d+\]").expect("RE_LEADING_SIZE must compile"));

/// What a message window holds, measured on the game's original messages.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WolfBudget {
    /// Line width in em of the game's default font size.
    pub width_em: f32,
    pub max_lines: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WolfFitSettings {
    pub line_breaks: LineBreaks,
    /// `NewPage` does not exist in Wolf: treated as `Leave`.
    pub overflow: Overflow,
    pub min_font_size: u32,
    /// The game's default font size (`\f[n]` units). Needed to measure a
    /// text that holds a font size code, and to shrink (R2).
    pub base_font_size: Option<u32>,
    /// The target language writes without spaces (zh/ja/ko).
    pub no_space_join: bool,
    /// The target language puts a space before `! ? ; :` (French).
    pub space_before_marks: bool,
}

/// Width of one line drawn from font size `start`.
fn line_width(tokens: &[Token], metrics: &Metrics, start: f32) -> f32 {
    let mut size = start;
    let mut width = 0.0;
    for token in tokens {
        width += token_width(token, metrics, size);
        size = apply_font_change(&token.kind, size);
    }
    width
}

fn split_lines(tokens: Vec<Token>) -> Vec<Vec<Token>> {
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

fn has_size_code(tokens: &[Token]) -> bool {
    tokens
        .iter()
        .any(|t| matches!(t.kind, TokenKind::FontSize(_)))
}

/// Width (p99 of the line widths, in em) and line count (the most lines one
/// message uses) of the game's original messages. `None` below
/// [`MIN_SAMPLE_LINES`] measurable lines. Messages with positioning or font
/// size codes are not measured.
pub fn measure_budget<'a>(
    sources: impl IntoIterator<Item = &'a str>,
    metrics: &Metrics,
) -> Option<WolfBudget> {
    let mut widths: Vec<f32> = Vec::new();
    let mut max_lines = 0;
    for source in sources {
        let (_, body) = split_face_prefix(source);
        let parsed = tokenize(body);
        if parsed.positioned || has_size_code(&parsed.tokens) {
            continue;
        }
        let lines = split_lines(parsed.tokens);
        max_lines = max_lines.max(lines.len());
        widths.extend(lines.iter().map(|line| line_width(line, metrics, 1.0)));
    }
    if widths.len() < MIN_SAMPLE_LINES {
        return None;
    }
    widths.sort_by(f32::total_cmp);
    let index = ((widths.len() as f32 * WIDTH_PERCENTILE) as usize).min(widths.len() - 1);
    Some(WolfBudget {
        width_em: widths[index],
        max_lines,
    })
}

/// A speaker label alone on its line (`魚淫さん：`, `Gyoin:`).
fn is_label(line: &str) -> bool {
    let line = line.trim();
    !line.is_empty()
        && line.chars().count() <= LABEL_MAX_CHARS
        && (line.ends_with('：') || line.ends_with(':'))
}

fn is_punctuation_only(line: &str) -> bool {
    let line = line.trim();
    !line.is_empty() && line.chars().all(|c| !c.is_alphanumeric())
}

/// Append `next` to `into` across a removed line break (R3.2, R3.3): no
/// space before closing punctuation or after an opening one, none for
/// languages written without spaces.
fn join(into: &mut String, next: &str, settings: &WolfFitSettings) {
    let next = next.trim_start();
    let kept = into.trim_end().len();
    into.truncate(kept);
    let first = next.chars().next();
    let tight = into.is_empty()
        || settings.no_space_join
        || into.ends_with(['(', '（', '「', '『', '“', '"'])
        || first.is_some_and(|c| ".,…)）」』。、".contains(c))
        || (first.is_some_and(|c| "!?;:！？".contains(c)) && !settings.space_before_marks);
    if !tight {
        into.push(' ');
    }
    into.push_str(next);
}

/// The blocks of a message body to wrap separately (R3), `""` for a blank
/// line. Kept as breaks: blank lines when the original has some, a line
/// indented with a full-width space, and the line after a speaker label
/// when the original starts with one. Every other break of a translation is
/// joined away (Reflow), or only the ones before punctuation (Keep).
fn blocks(body: &str, source_body: &str, settings: &WolfFitSettings) -> Vec<String> {
    let source_lines: Vec<&str> = source_body.split('\n').collect();
    let source_has_blank = source_lines.iter().any(|line| line.trim().is_empty());
    let lines: Vec<&str> = body.split('\n').collect();
    let label = lines.len() > 1
        && is_label(lines[0])
        && source_lines.first().is_some_and(|line| is_label(line));
    let mut out: Vec<String> = Vec::new();
    let mut open = false;
    for (index, line) in lines.iter().enumerate() {
        if line.trim().is_empty() {
            if source_has_blank {
                out.push(String::new());
                open = false;
            }
            continue;
        }
        let hard = !open || line.starts_with('\u{3000}') || (index == 1 && label);
        let joins = !hard
            && match settings.line_breaks {
                LineBreaks::Reflow => true,
                LineBreaks::Keep => is_punctuation_only(line),
            };
        match out.last_mut() {
            Some(last) if joins => join(last, line, settings),
            _ => out.push(line.trim_end().to_string()),
        }
        open = true;
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

/// Lay blocks out to `width` (each one balanced on its own lines).
fn layout(blocks: &[String], metrics: &Metrics, start: f32, width: f32) -> Vec<Vec<Token>> {
    let mut size = start;
    let mut lines = Vec::new();
    for block in blocks {
        let tokens = tokenize(block).tokens;
        if tokens.is_empty() {
            lines.push(Vec::new());
            continue;
        }
        let wrapped = wrap(&pieces(&tokens, metrics, size), width, None)
            .unwrap_or_else(|| vec![tokens.clone()]);
        size = size_after(&tokens, size);
        lines.extend(wrapped);
    }
    lines
}

/// Widest line, each line starting at the size the previous ones left.
fn widest(lines: &[Vec<Token>], metrics: &Metrics, start: f32) -> f32 {
    let mut size = start;
    let mut widest: f32 = 0.0;
    for line in lines {
        widest = widest.max(line_width(line, metrics, size));
        size = size_after(line, size);
    }
    widest
}

fn render(prefix: &str, lines: &[Vec<Token>]) -> String {
    let body: Vec<String> = lines.iter().map(|line| raw_of(line)).collect();
    format!("{prefix}{}", body.join("\n"))
}

fn fits(lines: &[Vec<Token>], metrics: &Metrics, start: f32, width: f32, max_lines: usize) -> bool {
    lines.len() <= max_lines && widest(lines, metrics, start) <= width + EPSILON
}

/// Shrink with a leading `\f[m]` (replacing a leading size code), from the
/// size in effect down to `min_font_size` (R5). `None` when not allowed or
/// when nothing fits.
fn shrink(
    prefix: &str,
    blocks: &[String],
    metrics: &Metrics,
    base: u32,
    width: f32,
    budget: &WolfBudget,
    settings: &WolfFitSettings,
) -> Option<(String, u32)> {
    let first = blocks.first()?;
    let leading = RE_LEADING_SIZE.find(first);
    let current: u32 = match leading {
        Some(m) => m.as_str()[3..m.as_str().len() - 1].parse().ok()?,
        None => base,
    };
    let rest = &first[leading.map_or(0, |m| m.end())..];
    // A size code further on would override the shrink: left as is.
    let further = blocks[1..]
        .iter()
        .map(String::as_str)
        .chain(std::iter::once(rest))
        .any(|block| has_size_code(&tokenize(block).tokens));
    if further {
        return None;
    }
    for size in (settings.min_font_size..current).rev() {
        let mut shrunk = blocks.to_vec();
        shrunk[0] = format!("\\f[{size}]{rest}");
        let lines = layout(&shrunk, metrics, base as f32, width);
        if fits(&lines, metrics, base as f32, width, budget.max_lines) {
            return Some((render(prefix, &lines), size));
        }
    }
    None
}

/// Fit one translated message (`target`, as exported: font prefix included)
/// whose original is `source`. Returns the text to write and what happened.
pub fn fit_message(
    target: &str,
    source: &str,
    budget: &WolfBudget,
    metrics: &Metrics,
    settings: &WolfFitSettings,
) -> (String, FitOutcome) {
    let skipped = |reason: &str| {
        (
            target.to_string(),
            FitOutcome::Skipped {
                reason: reason.to_string(),
            },
        )
    };
    let (prefix, body) = split_face_prefix(target);
    let parsed = tokenize(body);
    if parsed.positioned {
        return skipped("positioning or alignment codes");
    }
    let (start, width) = match settings.base_font_size {
        Some(base) => (base as f32, budget.width_em * base as f32),
        None if has_size_code(&parsed.tokens) => {
            return skipped("font size code, and the game's font size is not set")
        }
        None => (1.0, budget.width_em),
    };
    let (_, source_body) = split_face_prefix(source);
    let blocks = blocks(body, source_body, settings);
    let lines = layout(&blocks, metrics, start, width);
    if fits(&lines, metrics, start, width, budget.max_lines) {
        let text = render(prefix, &lines);
        let outcome = if text == target {
            FitOutcome::Fits
        } else {
            FitOutcome::Rewrapped
        };
        return (text, outcome);
    }
    let too_many = lines.len() > budget.max_lines;
    if too_many && settings.overflow == Overflow::Shrink {
        if let Some(base) = settings.base_font_size {
            if let Some((text, size)) =
                shrink(prefix, &blocks, metrics, base, width, budget, settings)
            {
                return (text, FitOutcome::Shrunk { size });
            }
        }
    }
    let reason = if too_many {
        format!(
            "{} lines, the window holds {}",
            lines.len(),
            budget.max_lines
        )
    } else {
        "a word is wider than the window".to_string()
    };
    (render(prefix, &lines), FitOutcome::TooLong { reason })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::engines::mv_mz::layout::font::{test_font, GameFont};

    /// Half-width = 0.5 em, full-width = 1 em.
    fn estimated() -> Metrics {
        Metrics::Estimated
    }

    fn settings() -> WolfFitSettings {
        WolfFitSettings {
            line_breaks: LineBreaks::Reflow,
            overflow: Overflow::Leave,
            min_font_size: 8,
            base_font_size: None,
            no_space_join: false,
            space_before_marks: false,
        }
    }

    /// 10 em = 20 half-width characters per line, 4 lines.
    const BUDGET: WolfBudget = WolfBudget {
        width_em: 10.0,
        max_lines: 4,
    };

    fn fit(target: &str, source: &str) -> (String, FitOutcome) {
        fit_message(target, source, &BUDGET, &estimated(), &settings())
    }

    #[test]
    fn budget_is_the_p99_width_and_the_most_lines() {
        let mut sources: Vec<String> = (0..99).map(|_| "ああ".to_string()).collect();
        sources.push("ああああああああああ\nあ\nあ".to_string()); // one very wide line
        sources.push(r"\ax[3]ああああああああああああああ".to_string()); // positioned: ignored
        let budget = measure_budget(sources.iter().map(String::as_str), &estimated()).unwrap();
        assert_eq!(budget.width_em, 2.0);
        assert_eq!(budget.max_lines, 3);
        assert!(measure_budget(["ああ"; 5], &estimated()).is_none());
    }

    #[test]
    fn budget_uses_the_game_font() {
        let font = GameFont::from_bytes(&test_font::ttf(&[('a', 500), ('M', 800)])).unwrap();
        let metrics = Metrics::Font(Arc::new(font));
        let sources = vec!["MMMMM"; 30];
        let budget = measure_budget(sources, &metrics).unwrap();
        assert!((budget.width_em - 4.0).abs() < 0.01, "{}", budget.width_em);
    }

    #[test]
    fn stray_line_breaks_are_joined_and_the_message_rewrapped() {
        // Real LLM output (plan WR, K6): breaks pushed to the end of sentences.
        let (text, outcome) = fit("This is a short piece of work \n.", "短い内容\nです。");
        assert_eq!(text, "This is a short\npiece of work.");
        assert_eq!(outcome, FitOutcome::Rewrapped);
        let (text, _) = fit("Thank you \n \n!", "ありがとう");
        assert_eq!(text, "Thank you!");
    }

    #[test]
    fn a_fitting_single_line_is_left_as_is() {
        assert_eq!(
            fit("Hello there.", "やあ"),
            ("Hello there.".to_string(), FitOutcome::Fits)
        );
    }

    #[test]
    fn face_prefix_label_blank_lines_and_indent_are_kept() {
        let (text, _) = fit("@4\nHello", "@4\nやあ");
        assert_eq!(text, "@4\nHello");
        let (text, _) = fit("Gyoin:\nWell then.", "魚淫さん：\nでは。");
        assert_eq!(text, "Gyoin:\nWell then.");
        let (text, _) = fit("One.\n\nTwo.", "一。\n\n二。");
        assert_eq!(text, "One.\n\nTwo.");
        let (text, _) = fit("One.\n\u{3000}Two.", "一。\n\u{3000}二。");
        assert_eq!(text, "One.\n\u{3000}Two.");
    }

    #[test]
    fn keep_mode_only_joins_punctuation_lines() {
        let keep = WolfFitSettings {
            line_breaks: LineBreaks::Keep,
            ..settings()
        };
        let (text, _) = fit_message("Hi\nthere\n.", "やあ", &BUDGET, &estimated(), &keep);
        assert_eq!(text, "Hi\nthere.");
    }

    #[test]
    fn languages_without_spaces_join_directly() {
        let cjk = WolfFitSettings {
            no_space_join: true,
            ..settings()
        };
        let (text, _) = fit_message("你好\n世界", "やあ\n世界", &BUDGET, &estimated(), &cjk);
        assert_eq!(text, "你好世界");
    }

    #[test]
    fn french_keeps_its_space_before_marks() {
        let fr = WolfFitSettings {
            space_before_marks: true,
            ..settings()
        };
        let (text, _) = fit_message("Bonjour\n!", "やあ", &BUDGET, &estimated(), &fr);
        assert_eq!(text, "Bonjour !");
        assert_eq!(fit("Hello\n!", "やあ").0, "Hello!");
    }

    #[test]
    fn codes_are_never_split_and_ruby_counts_its_base() {
        let (text, _) = fit(r"aaaa \c[2]bbbbbbbbbbbbbbbb\c[0] cc", "あ");
        assert_eq!(text, "aaaa\n\\c[2]bbbbbbbbbbbbbbbb\\c[0] cc");
        // The ruby base (2 em) fits the line; its reading is not measured.
        let (_, outcome) = fit(r"\r[魔法,まほうまほうまほうまほうまほう]", "あ");
        assert_eq!(outcome, FitOutcome::Fits);
    }

    #[test]
    fn positioned_texts_are_skipped() {
        let (text, outcome) = fit("<C>Centered and very long text indeed", "<C>中");
        assert_eq!(text, "<C>Centered and very long text indeed");
        assert!(matches!(outcome, FitOutcome::Skipped { .. }));
    }

    #[test]
    fn a_size_code_needs_the_game_font_size() {
        let (_, outcome) = fit(r"\f[18]Hello", "やあ");
        assert!(matches!(outcome, FitOutcome::Skipped { .. }));
        let known = WolfFitSettings {
            base_font_size: Some(12),
            ..settings()
        };
        // At 18 instead of 12, a 10 em line holds 10 × 12 / 18 × 2 ≈ 13 half-width characters.
        let (text, _) = fit_message(
            r"\f[18]aaaa bbbb cccc dddd",
            "やあ",
            &BUDGET,
            &estimated(),
            &known,
        );
        assert_eq!(text, "\\f[18]aaaa bbbb\ncccc dddd");
    }

    #[test]
    fn too_many_lines_shrink_or_are_reported() {
        // Nine 9-letter words: two per line at the game's size (5 lines),
        // three per line at size 8 (3 lines).
        let long = "aaaaaaaaa ".repeat(9);
        let (text, outcome) = fit(long.trim_end(), "あ");
        assert_eq!(text.lines().count(), 5);
        assert!(matches!(outcome, FitOutcome::TooLong { .. }));

        let shrink = WolfFitSettings {
            overflow: Overflow::Shrink,
            base_font_size: Some(12),
            min_font_size: 8,
            ..settings()
        };
        let (text, outcome) = fit_message(
            &format!("@2\n\\f[12]{}", long.trim_end()),
            "@2\nあ",
            &BUDGET,
            &estimated(),
            &shrink,
        );
        let FitOutcome::Shrunk { size } = outcome else {
            panic!("{outcome:?}");
        };
        assert_eq!(size, 8);
        assert!(text.starts_with(&format!("@2\n\\f[{size}]")), "{text}");
        assert!(
            !text.contains("\\f[12]"),
            "the old prefix is replaced: {text}"
        );
        assert!(text.lines().count() <= 5); // face prefix line + at most 4
    }
}
