//! Line breaks of a Wolf RPG message sent to the LLM (plan WR-E6).
//!
//! Most breaks of a message only lay its text out in the window. Sent as
//! mandatory placeholders, they are dropped by the model at sentence ends,
//! sometimes with a word (plan WR, K6). These soft breaks are sent as a plain
//! join instead; only structural breaks stay mandatory placeholders. After the
//! response, each block is broken again into as many lines as the source had,
//! as evenly as possible. The window width stays the export layout's business.

use super::layout::fit::is_label;
use super::layout::text::{split_face_prefix, tokenize};
use crate::engines::mv_mz::layout::font::{is_wide, Metrics};
use crate::engines::mv_mz::layout::text::{raw_of, TokenKind};
use crate::engines::mv_mz::layout::wrap::{pieces, wrap};

/// Source lines of each block of a message (blocks are separated by its
/// structural breaks).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinePlan {
    lines: Vec<usize>,
}

/// The soft breaks of a source message, and how to break its translation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoftBreaks {
    /// Byte offset of each soft `\n` in the text, with what replaces it.
    joins: Vec<(usize, &'static str)>,
    pub plan: LinePlan,
}

impl SoftBreaks {
    /// What replaces the `\n` at byte `offset`; `None` for a structural break.
    pub fn join_at(&self, offset: usize) -> Option<&'static str> {
        self.joins
            .binary_search_by_key(&offset, |(at, _)| *at)
            .ok()
            .map(|index| self.joins[index].1)
    }
}

/// A line starting with one of these opens a note or a list entry.
fn is_item_marker(c: char) -> bool {
    matches!(
        c,
        '※' | '・' | '●' | '○' | '■' | '□' | '◆' | '◇' | '★' | '☆' | '\u{2460}'..='\u{2473}'
    )
}

/// The printed text of a line, without its codes: `\c[2]魚淫さん：\c[0]`
/// reads `魚淫さん：`.
fn visible(line: &str) -> String {
    tokenize(line)
        .tokens
        .into_iter()
        .filter(|token| token.kind == TokenKind::Text)
        .map(|token| token.raw)
        .collect()
}

/// The break after `lines[index]` is structural: it touches a blank line,
/// ends a leading speaker label, or opens a note or a list entry. A line
/// indented with `　` or opening with `「` is not: in the games measured,
/// these continue the sentence (plan WR-E6, E6-K7).
fn is_structural(lines: &[&str], index: usize) -> bool {
    let (before, after) = (lines[index], lines[index + 1]);
    before.trim().is_empty()
        || after.trim().is_empty()
        || (index == 0 && is_label(&visible(before)))
        || visible(after)
            .trim_start()
            .chars()
            .next()
            .is_some_and(is_item_marker)
}

/// Nothing when either side is full-width (Japanese runs on without spaces)
/// or already spaced, one space otherwise.
fn join_for(before: &str, after: &str) -> &'static str {
    match (before.chars().last(), after.chars().next()) {
        (Some(last), Some(first))
            if !is_wide(last)
                && !is_wide(first)
                && !last.is_whitespace()
                && !first.is_whitespace() =>
        {
            " "
        }
        _ => "",
    }
}

/// Soft breaks of a message `text` (with its `@N\n` prefix, if any). `None`
/// when every break is structural, and for a positioned message: alignment
/// and position codes are laid out by hand, so all its breaks stay.
pub fn soft_breaks(text: &str) -> Option<SoftBreaks> {
    let (prefix, body) = split_face_prefix(text);
    if !body.contains('\n') || tokenize(body).positioned {
        return None;
    }
    let lines: Vec<&str> = body.split('\n').collect();
    let mut joins = Vec::new();
    let mut plan = vec![1usize];
    let mut offset = prefix.len();
    for (index, pair) in lines.windows(2).enumerate() {
        offset += pair[0].len();
        if is_structural(&lines, index) {
            plan.push(1);
        } else {
            joins.push((offset, join_for(&visible(pair[0]), &visible(pair[1]))));
            if let Some(count) = plan.last_mut() {
                *count += 1;
            }
        }
        offset += 1;
    }
    if joins.is_empty() {
        return None;
    }
    Some(SoftBreaks {
        joins,
        plan: LinePlan { lines: plan },
    })
}

/// Break each block of a restored translation into its source's number of
/// lines, as evenly as possible (estimated widths); a block too short for
/// that many lines takes fewer. A translation whose blocks do not match the
/// plan (a raw line break in a structured response) is returned unchanged.
pub fn rebreak(target: &str, plan: &LinePlan) -> String {
    let (prefix, body) = split_face_prefix(target);
    let blocks: Vec<&str> = body.split('\n').collect();
    if blocks.len() != plan.lines.len() {
        return target.to_string();
    }
    let lines: Vec<String> = blocks
        .iter()
        .zip(&plan.lines)
        .map(|(block, &count)| balance(block, count))
        .collect();
    format!("{prefix}{}", lines.join("\n"))
}

fn balance(block: &str, count: usize) -> String {
    let text = block.trim();
    if count < 2 || text.is_empty() {
        return block.to_string();
    }
    // A `\f[n]` sets an absolute size: measured from 1 em, the text after it
    // would weigh n times more. Only the raw text is laid out here.
    let tokens: Vec<_> = tokenize(text)
        .tokens
        .into_iter()
        .map(|mut token| {
            if matches!(token.kind, TokenKind::FontSize(_)) {
                token.kind = TokenKind::Unknown(String::new());
            }
            token
        })
        .collect();
    let parts = pieces(&tokens, &Metrics::Estimated, 1.0);
    let width: f32 = parts.iter().map(|part| part.width + part.space).sum();
    (2..=count)
        .rev()
        .find_map(|lines| wrap(&parts, width + 1.0, Some(lines)))
        .map(|lines| {
            lines
                .iter()
                .map(|line| raw_of(line))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_else(|| text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan_of(text: &str) -> Vec<usize> {
        soft_breaks(text).expect("soft breaks").plan.lines
    }

    #[test]
    fn soft_breaks_point_at_the_line_breaks_of_the_text() {
        let source = "@4\n釣りを行いたい場合は水場に面して\n「決定ボタン」を押す事で竿を振れるぞ！";
        let soft = soft_breaks(source).unwrap();
        assert_eq!(soft.plan.lines, vec![2]);
        assert_eq!(soft.joins.len(), 1);
        let (offset, join) = soft.joins[0];
        assert_eq!(&source[offset..offset + 1], "\n");
        assert!(offset > 3, "the face prefix break is not a soft break");
        assert_eq!(join, "");
        assert_eq!(soft.join_at(offset), Some(""));
        assert_eq!(soft.join_at(2), None);
    }

    #[test]
    fn blank_lines_labels_and_notes_stay_structural() {
        assert!(soft_breaks("あいう\n\nえお").is_none());
        assert!(
            soft_breaks("ゲームを終了しますか？\n※セーブされていないデータは失われます。")
                .is_none()
        );
        assert!(soft_breaks("持ち物：\n・釣り竿\n・エサ").is_none());
        assert_eq!(
            plan_of("魚淫さん：\n俺君、本格的に行動に移す前に、\n確か…この海岸沿いの道を進んだ"),
            vec![1, 2]
        );
        assert_eq!(plan_of("一行目\n二行目\n\n三行目\n四行目"), vec![2, 1, 2]);
    }

    #[test]
    fn indents_and_quotes_continue_the_sentence() {
        assert_eq!(
            plan_of("（でも…まだまだ元気\n\u{3000}いっぱいなので大丈夫そうだ…）"),
            vec![2]
        );
        assert_eq!(
            plan_of("ねぇ～お願い～、おかあさ～ん\n「新しくできた水族館」に連れてって！"),
            vec![2]
        );
    }

    #[test]
    fn positioned_or_single_line_messages_have_no_soft_break() {
        assert!(soft_breaks("<C>タイトル\n本文").is_none());
        assert!(soft_breaks("\\ax[10]一行目\n二行目").is_none());
        assert!(soft_breaks("一行だけ").is_none());
        assert!(soft_breaks("@2\n一行だけ").is_none());
    }

    #[test]
    fn codes_around_labels_notes_and_joins_are_ignored() {
        assert_eq!(
            plan_of("\\c[2]魚淫さん：\\c[0]\n俺君、本格的に\n確か…"),
            vec![1, 2]
        );
        assert!(soft_breaks("終了しますか？\n\\c[2]※セーブされていません。").is_none());
        let soft = soft_breaks("あいう\\c[0]\n\\c[2]えお").unwrap();
        assert_eq!(soft.joins[0].1, "");
    }

    #[test]
    fn a_font_size_code_does_not_unbalance_the_lines() {
        let soft = soft_breaks("一行目\n二行目").unwrap();
        assert_eq!(
            rebreak("\\f[24]aaaa bbbb cccc dddd", &soft.plan),
            "\\f[24]aaaa bbbb\ncccc dddd"
        );
    }

    #[test]
    fn joins_add_a_space_between_half_width_text_only() {
        assert_eq!(join_for("abc", "def"), " ");
        assert_eq!(join_for("あ", "b"), "");
        assert_eq!(join_for("a", "い"), "");
        assert_eq!(join_for("a ", "b"), "");
        assert_eq!(join_for("a", "\u{3000}b"), "");
    }

    #[test]
    fn a_translation_takes_the_source_line_count_without_stray_punctuation() {
        // Real case of plan WR, K6: « …the work ⏎. ».
        let soft = soft_breaks(
            "短い内容となりますが、作品全体の雰囲気等を\nつかむ一助にお役立ていただければ幸いです。",
        )
        .unwrap();
        let target = "This is a short piece, but we hope it will be of some help in grasping the \
                      overall atmosphere of the work.";
        let out = rebreak(target, &soft.plan);
        let lines: Vec<&str> = out.split('\n').collect();
        assert_eq!(lines.len(), 2);
        assert!(lines
            .iter()
            .all(|line| line.chars().any(char::is_alphanumeric)));
        assert_eq!(lines.join(" "), target);
        let (short, long) = (
            lines[0].len().min(lines[1].len()),
            lines[0].len().max(lines[1].len()),
        );
        assert!(long - short < 20, "balanced: {lines:?}");
    }

    #[test]
    fn rebreak_keeps_the_face_prefix_and_structural_breaks() {
        let soft = soft_breaks("@2\n魚淫さん：\n俺君、本格的に行動に移す前に、\n確か…").unwrap();
        let out = rebreak(
            "@2\nGyoin:\nKid, before you really get moving, if I remember right…",
            &soft.plan,
        );
        let lines: Vec<&str> = out.split('\n').collect();
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0], "@2");
        assert_eq!(lines[1], "Gyoin:");
    }

    #[test]
    fn short_or_mismatched_translations_are_left_as_they_are() {
        let three = soft_breaks("一\n二\n三").unwrap();
        assert_eq!(three.plan.lines, vec![3]);
        assert_eq!(rebreak("Yes.", &three.plan), "Yes.");
        assert_eq!(rebreak("Oh no", &three.plan), "Oh\nno");
        let two = soft_breaks("一\n二").unwrap();
        assert_eq!(rebreak("a\n\nb", &two.plan), "a\n\nb");
    }

    #[test]
    fn languages_without_spaces_are_broken_too() {
        let soft = soft_breaks("一行目の文章です\n二行目の文章です").unwrap();
        let out = rebreak("这是第一句话。这是第二句话。", &soft.plan);
        assert_eq!(out.split('\n').count(), 2);
        assert_eq!(out.replace('\n', ""), "这是第一句话。这是第二句话。");
    }
}
