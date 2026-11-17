//! Line breaking: Unicode break opportunities (UAX #14) over message tokens
//! and balanced wrapping to a pixel width.
//!
//! Escape codes never split; zero-width codes between two words travel with
//! the next word (`word \C[2]next` → `\C[2]next` starts the line), codes
//! glued to a word stay with it.

use unicode_linebreak::{linebreaks, BreakOpportunity};

use super::font::Metrics;
use super::token::{font_size_after, width_of, Token, TokenKind, TokenRules};

/// A wrappable unit: everything between two break opportunities.
#[derive(Debug, Clone)]
pub struct Piece {
    pub tokens: Vec<Token>,
    /// Width without trailing whitespace.
    pub width: f32,
    /// Width of the trailing whitespace (dropped at a line end).
    pub space: f32,
    /// Ends a sentence (last visible character closes a sentence).
    pub sentence_end: bool,
}

/// Where a piece boundary falls: before char `offset` of token `token`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Cut {
    token: usize,
    offset: usize,
}

/// Font size in effect before each token, starting from `start`.
pub fn sizes_before(tokens: &[Token], start: f32, rules: &impl TokenRules) -> Vec<f32> {
    let mut size = start;
    tokens
        .iter()
        .map(|token| {
            let before = size;
            size = font_size_after(&token.kind, size, rules);
            before
        })
        .collect()
}

/// Font size after all `tokens`.
pub fn size_after(tokens: &[Token], start: f32, rules: &impl TokenRules) -> f32 {
    tokens.iter().fold(start, |size, token| {
        font_size_after(&token.kind, size, rules)
    })
}

fn is_sentence_end(c: char) -> bool {
    matches!(
        c,
        '.' | '!'
            | '?'
            | '…'
            | '。'
            | '！'
            | '？'
            | '」'
            | '』'
            | '”'
            | '"'
            | ')'
            | '）'
            | '♥'
            | '♡'
            | '♪'
            | '~'
            | '～'
    )
}

/// Split one paragraph (no newlines) into pieces at its break opportunities.
pub fn pieces(
    tokens: &[Token],
    metrics: &Metrics,
    start_size: f32,
    rules: &impl TokenRules,
) -> Vec<Piece> {
    // Projection: visible characters only; atomic tokens become one letter.
    let mut projected = String::new();
    let mut origin: Vec<(usize, usize)> = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::Text => {
                for (offset, c) in token.raw.chars().enumerate() {
                    projected.push(c);
                    origin.push((index, offset));
                }
            }
            TokenKind::Icon | TokenKind::Inserted { .. } | TokenKind::Backslash => {
                projected.push('X');
                origin.push((index, 0));
            }
            _ => {}
        }
    }
    // Byte offsets → character positions where a line may end.
    let byte_to_char: std::collections::HashMap<usize, usize> = projected
        .char_indices()
        .enumerate()
        .map(|(char_index, (byte, _))| (byte, char_index))
        .collect();
    let mut cuts = Vec::new();
    for (byte, opportunity) in linebreaks(&projected) {
        if byte == 0 || byte >= projected.len() {
            continue;
        }
        if matches!(
            opportunity,
            BreakOpportunity::Allowed | BreakOpportunity::Mandatory
        ) {
            let Some(&position) = byte_to_char.get(&byte) else {
                continue;
            };
            // Cut right after the previous visible character: codes in
            // between travel with the next word.
            let (token, offset) = origin[position - 1];
            let cut = match tokens[token].kind {
                TokenKind::Text if offset + 1 < tokens[token].raw.chars().count() => Cut {
                    token,
                    offset: offset + 1,
                },
                _ => Cut {
                    token: token + 1,
                    offset: 0,
                },
            };
            cuts.push(cut);
        }
    }
    cuts.dedup();

    let sizes = sizes_before(tokens, start_size, rules);
    let mut out = Vec::new();
    let mut current: Vec<(Token, f32)> = Vec::new();
    let mut cut_iter = cuts.into_iter().peekable();
    for (index, token) in tokens.iter().enumerate() {
        // Cuts before this token.
        while cut_iter
            .peek()
            .is_some_and(|c| c.token == index && c.offset == 0)
        {
            cut_iter.next();
            if !current.is_empty() {
                out.push(make_piece(std::mem::take(&mut current), metrics, rules));
            }
        }
        if token.kind == TokenKind::Text {
            let chars: Vec<char> = token.raw.chars().collect();
            let mut start = 0;
            while let Some(cut) = cut_iter.peek().copied().filter(|c| c.token == index) {
                cut_iter.next();
                let part: String = chars[start..cut.offset].iter().collect();
                current.push((
                    Token {
                        kind: TokenKind::Text,
                        raw: part,
                    },
                    sizes[index],
                ));
                out.push(make_piece(std::mem::take(&mut current), metrics, rules));
                start = cut.offset;
            }
            let rest: String = chars[start..].iter().collect();
            if !rest.is_empty() {
                current.push((
                    Token {
                        kind: TokenKind::Text,
                        raw: rest,
                    },
                    sizes[index],
                ));
            }
        } else {
            current.push((token.clone(), sizes[index]));
        }
    }
    if !current.is_empty() {
        out.push(make_piece(current, metrics, rules));
    }
    out
}

fn make_piece(sized: Vec<(Token, f32)>, metrics: &Metrics, rules: &impl TokenRules) -> Piece {
    let total: f32 = sized
        .iter()
        .map(|(token, size)| width_of(token, metrics, *size, rules))
        .sum();
    // Trailing whitespace of the last text token (codes after it are zero-width).
    // Trailing whitespace: from the end, through zero-width codes and
    // whitespace-only text, up to the last visible character.
    let mut space = 0.0;
    let mut sentence_end = false;
    for (token, size) in sized.iter().rev() {
        match token.kind {
            TokenKind::Text => {
                let trimmed = token.raw.trim_end();
                space += metrics.text_width(&token.raw[trimmed.len()..], *size);
                if !trimmed.is_empty() {
                    sentence_end = trimmed.chars().last().is_some_and(is_sentence_end);
                    break;
                }
            }
            _ if token.is_visible() => break,
            _ => {}
        }
    }
    Piece {
        tokens: sized.into_iter().map(|(token, _)| token).collect(),
        width: total - space,
        space,
        sentence_end,
    }
}

/// Balanced wrapping: the minimum number of lines that fit `width` (a single
/// piece wider than `width` takes a line on its own), then the most even
/// split (least squared slack, last line included, so a message never ends
/// on an orphan word when an even split exists). With `exact_lines`, exactly
/// that many lines (`None` if impossible).
///
/// Cost: O(n·k) for n pieces and at most k pieces per line (times
/// `exact_lines` when given), memory O(n) (times `exact_lines`).
pub fn wrap(pieces: &[Piece], width: f32, exact_lines: Option<usize>) -> Option<Vec<Vec<Token>>> {
    let n = pieces.len();
    if n == 0 {
        return matches!(exact_lines, None | Some(1)).then(|| vec![Vec::new()]);
    }
    // prefix[k]: width of pieces[..k] with their trailing spaces (f64: the
    // sums of long texts must stay exact to well under the 0.001 px margin).
    let mut prefix = Vec::with_capacity(n + 1);
    prefix.push(0.0f64);
    for piece in pieces {
        prefix.push(prefix[prefix.len() - 1] + f64::from(piece.width) + f64::from(piece.space));
    }
    let width = f64::from(width);
    let width_of = |i: usize, j: usize| prefix[j] - prefix[i] - f64::from(pieces[j - 1].space);
    let fits = |i: usize, j: usize| j == i + 1 || width_of(i, j) <= width + 0.001;
    let slack = |i: usize, j: usize| {
        let gap = (width - width_of(i, j)).max(0.0);
        gap * gap
    };
    let back = match exact_lines {
        Some(count) => exact_breaks(n, count, &fits, &slack)?,
        None => fewest_breaks(n, &fits, &slack),
    };
    let mut bounds = Vec::new();
    let mut j = n;
    while j > 0 {
        let i = back[j];
        bounds.push((i, j));
        j = i;
    }
    bounds.reverse();
    Some(
        bounds
            .into_iter()
            .map(|(i, j)| trim_line(pieces[i..j].iter().flat_map(|p| p.tokens.clone()).collect()))
            .collect(),
    )
}

/// Line start for each end `j` of the layout with the fewest lines, then the
/// least slack. Comparing `(lines, slack)` per prefix is enough: in a layout
/// with the fewest lines, every prefix ending at a break also uses its own
/// fewest lines.
fn fewest_breaks(
    n: usize,
    fits: &dyn Fn(usize, usize) -> bool,
    slack: &dyn Fn(usize, usize) -> f64,
) -> Vec<usize> {
    let mut best = vec![(usize::MAX, f64::INFINITY); n + 1];
    let mut back = vec![0usize; n + 1];
    best[0] = (0, 0.0);
    for j in 1..=n {
        for i in (0..j).rev() {
            if !fits(i, j) {
                break;
            }
            let (lines, cost) = best[i];
            if lines == usize::MAX {
                continue;
            }
            let candidate = (lines + 1, cost + slack(i, j));
            if candidate.0 < best[j].0 || (candidate.0 == best[j].0 && candidate.1 < best[j].1) {
                best[j] = candidate;
                back[j] = i;
            }
        }
    }
    back
}

/// Line starts of the least-slack layout in exactly `count` lines (`None` if
/// impossible); `back[j]` is only meaningful along the chosen layout.
fn exact_breaks(
    n: usize,
    count: usize,
    fits: &dyn Fn(usize, usize) -> bool,
    slack: &dyn Fn(usize, usize) -> f64,
) -> Option<Vec<usize>> {
    if count == 0 || count > n {
        return None;
    }
    // best[c][j]: least cost to lay pieces[..j] in c lines.
    let mut best = vec![vec![f64::INFINITY; n + 1]; count + 1];
    let mut starts = vec![vec![0usize; n + 1]; count + 1];
    best[0][0] = 0.0;
    for c in 1..=count {
        for j in c..=n {
            for i in (0..j).rev() {
                if !fits(i, j) {
                    break;
                }
                if best[c - 1][i].is_finite() {
                    let cost = best[c - 1][i] + slack(i, j);
                    if cost < best[c][j] {
                        best[c][j] = cost;
                        starts[c][j] = i;
                    }
                }
            }
        }
    }
    if !best[count][n].is_finite() {
        return None;
    }
    let mut back = vec![0usize; n + 1];
    let mut j = n;
    for c in (1..=count).rev() {
        back[j] = starts[c][j];
        j = starts[c][j];
    }
    Some(back)
}

/// Drop whitespace at the end of a line (the space a break consumed).
pub fn trim_line(mut tokens: Vec<Token>) -> Vec<Token> {
    if let Some(index) = tokens.iter().rposition(|t| t.kind == TokenKind::Text) {
        let trimmed = tokens[index].raw.trim_end().to_string();
        let all_after_zero_width = tokens[index + 1..].iter().all(|t| !t.is_visible());
        if all_after_zero_width {
            if trimmed.is_empty() {
                tokens.remove(index);
            } else {
                tokens[index].raw = trimmed;
            }
        }
    }
    tokens
}
