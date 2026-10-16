//! Fitting a Show Text message or a description to its box.
//!
//! A message is the run of 401 lines after a 101. Lines that hold translated
//! text are *writable*; other lines (`……`, code-only) are kept verbatim. Each
//! block of consecutive writable lines is laid out and written back only
//! into its own 401 commands (embedded `\n`), so the event structure never
//! changes. Messages that already fit are left as they are.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::font::Metrics;
use super::geometry::Geometry;
use super::simulate::{simulate, BoxSpec, PageView};
use super::text::{apply_font_change, raw_of, tokenize, Inserts, Token, TokenKind, PAGE_BREAK};
use super::wrap::{pieces, size_after, wrap, Piece};

/// How line breaks are chosen inside a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineBreaks {
    /// Join the lines of each block and rewrap them evenly.
    Reflow,
    /// Keep each translated line; split only the ones that are too wide.
    Keep,
}

/// What happens when a message needs more lines than the box holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Overflow {
    /// Continue on a new page at a sentence boundary.
    NewPage,
    /// Shrink the font (down to the minimum); a new page if still too long.
    Shrink,
    /// Leave the text as translated and report it.
    Leave,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FitSettings {
    pub line_breaks: LineBreaks,
    pub overflow: Overflow,
    pub min_font_size: u32,
    /// Dialogue font size emitted at every page start (`None` = the game's).
    pub dialogue_font_size: Option<u32>,
    /// Drop a `\FS[n]` stored at the start of a translated line (an older
    /// export's prefix; the export's "replace existing" choice). Otherwise it
    /// is the translator's code and is kept.
    pub strip_stored_font_prefix: bool,
}

pub struct FitContext<'a> {
    pub metrics: &'a Metrics,
    pub geometry: &'a Geometry,
    pub inserts: &'a Inserts,
    /// The target language writes without spaces (zh/ja/ko): lines join directly.
    pub no_space_join: bool,
    /// A plugin draws the first line of item descriptions as plain text.
    pub plain_description_first_line: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageLine {
    pub text: String,
    /// Holds translated text (a segment): may be rewritten.
    pub writable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageInput {
    pub lines: Vec<MessageLine>,
    /// A portrait is shown (`101` parameter 0 set).
    pub face: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FitOutcome {
    /// Already fits as translated.
    Fits,
    /// Line breaks changed; same number of pages.
    Rewrapped,
    /// Continued on extra pages.
    NewPage { pages: usize },
    /// Font reduced to `size` px.
    Shrunk { size: u32 },
    /// Still does not fit with these settings (left as translated).
    TooLong { reason: String },
    /// Not fitted at all (left as translated).
    Skipped { reason: String },
}

impl FitOutcome {
    pub fn changed_layout(&self) -> bool {
        matches!(
            self,
            FitOutcome::Rewrapped | FitOutcome::NewPage { .. } | FitOutcome::Shrunk { .. }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Fitted {
    pub outcome: FitOutcome,
    /// New text per input line (`None` = unchanged).
    pub values: Vec<Option<String>>,
    /// How the result is drawn (preview).
    pub pages: Vec<PageView>,
}

/// `\FS[n]` that an older export stored at the start of targets.
static RE_STORED_FONT_PREFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\\FS\[\d+\]").expect("RE_STORED_FONT_PREFIX must compile"));

/// A line as fitting reads it: without a stored export prefix when
/// [`FitSettings::strip_stored_font_prefix`] says so, as translated otherwise.
pub(crate) fn effective_text<'a>(line: &'a MessageLine, settings: &FitSettings) -> Cow<'a, str> {
    if line.writable && settings.strip_stored_font_prefix {
        RE_STORED_FONT_PREFIX.replace(&line.text, "")
    } else {
        Cow::Borrowed(&line.text)
    }
}

/// A bracketed speaker label on its own line (`【恵】`, `[Megumi]`).
static RE_NAME_LABEL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?:【[^】]{1,24}】|\[[^\]]{1,24}\]|［[^］]{1,24}］|<[^>]{1,24}>|＜[^＞]{1,24}＞)$",
    )
    .expect("RE_NAME_LABEL must compile")
});

fn is_rtl(text: &str) -> bool {
    text.chars()
        .any(|c| ('\u{0600}'..='\u{06FF}').contains(&c) || ('\u{0750}'..='\u{077F}').contains(&c))
}

/// The Show Text box of a message (portrait or not).
pub(crate) fn message_box(ctx: &FitContext, face: bool) -> BoxSpec {
    let message = &ctx.geometry.message;
    let text_x = message.text_x(face);
    BoxSpec {
        text_x,
        limit: text_x + message.text_width(face),
        height: message.inner_height,
        main_font: ctx.geometry.font_size,
        line_spacing: ctx.geometry.line_spacing(),
        paging: true,
    }
}

/// The help box of descriptions and profiles.
pub(crate) fn help_box(ctx: &FitContext) -> BoxSpec {
    let help = &ctx.geometry.help;
    BoxSpec {
        text_x: help.text_x,
        limit: help.text_x + help.text_width,
        height: help.inner_height,
        main_font: ctx.geometry.font_size,
        line_spacing: ctx.geometry.line_spacing(),
        paging: false,
    }
}

fn overflows(pages: &[PageView]) -> bool {
    pages
        .iter()
        .flat_map(|p| &p.lines)
        .any(|l| l.overflow || l.clipped)
}

// ---------------------------------------------------------------------------
// Layout units
// ---------------------------------------------------------------------------

/// A laid-out line: tokens and the block (run) it belongs to (`None` = a
/// verbatim line).
#[derive(Debug, Clone)]
struct OutLine {
    tokens: Vec<Token>,
    run: Option<usize>,
}

#[derive(Debug, Clone)]
enum UnitKind {
    /// Part of a paragraph, wrapped together with neighbouring parts of the
    /// same paragraph on a page.
    Pieces(Vec<Piece>),
    /// One line as is.
    Line(Vec<Token>),
}

#[derive(Debug, Clone)]
struct Unit {
    run: Option<usize>,
    paragraph: usize,
    kind: UnitKind,
}

/// A paragraph or a verbatim line of the message.
#[derive(Debug, Clone)]
struct Block {
    run: Option<usize>,
    tokens: Vec<Token>,
    /// Wrappable text (`false`: kept as one line).
    wrappable: bool,
}

/// Split a message into blocks: paragraphs of writable runs (reflow joins a
/// run's lines; blank lines and a leading name label stay on their own) and
/// verbatim lines.
fn blocks(
    input: &MessageInput,
    settings: &FitSettings,
    ctx: &FitContext,
) -> (Vec<Block>, Vec<Vec<usize>>) {
    let mut out = Vec::new();
    let mut runs: Vec<Vec<usize>> = Vec::new();
    let mut index = 0;
    while index < input.lines.len() {
        let line = &input.lines[index];
        if !line.writable {
            out.push(Block {
                run: None,
                tokens: tokenize(&line.text, ctx.inserts),
                wrappable: false,
            });
            index += 1;
            continue;
        }
        let run_id = runs.len();
        let mut members = Vec::new();
        while index < input.lines.len() && input.lines[index].writable {
            members.push(index);
            index += 1;
        }
        let texts: Vec<String> = members
            .iter()
            .map(|&i| effective_text(&input.lines[i], settings).into_owned())
            .collect();
        let mut paragraph: Vec<String> = Vec::new();
        let flush = |paragraph: &mut Vec<String>, out: &mut Vec<Block>| {
            if paragraph.is_empty() {
                return;
            }
            let joiner = if ctx.no_space_join { "" } else { " " };
            let joined = std::mem::take(paragraph).join(joiner);
            out.push(Block {
                run: Some(run_id),
                tokens: tokenize(&joined, ctx.inserts),
                wrappable: true,
            });
        };
        for (position, text) in texts.iter().enumerate() {
            let cleaned = text
                .trim_start_matches(['\u{3000}', ' ', '\t'])
                .trim_end()
                .to_string();
            if cleaned.is_empty() {
                flush(&mut paragraph, &mut out);
                out.push(Block {
                    run: Some(run_id),
                    tokens: Vec::new(),
                    wrappable: false,
                });
                continue;
            }
            let label = position == 0 && RE_NAME_LABEL.is_match(&cleaned);
            if settings.line_breaks == LineBreaks::Keep || label {
                flush(&mut paragraph, &mut out);
                out.push(Block {
                    run: Some(run_id),
                    tokens: tokenize(&cleaned, ctx.inserts),
                    wrappable: true,
                });
            } else {
                paragraph.push(cleaned);
            }
        }
        flush(&mut paragraph, &mut out);
        runs.push(members);
    }
    (out, runs)
}

/// Units at font size `size`: sentence groups of wrappable blocks (font
/// flows through the text as the engine draws it) and single lines.
fn units(blocks: &[Block], ctx: &FitContext, size: f32, by_line: bool, width: f32) -> Vec<Unit> {
    let mut out = Vec::new();
    let mut font = size;
    for (paragraph, block) in blocks.iter().enumerate() {
        if block.wrappable && !block.tokens.is_empty() {
            let all = pieces(&block.tokens, ctx.metrics, font);
            if by_line {
                // Fallback: each wrapped line becomes its own unit.
                for line in wrap(&all, width, None).unwrap_or_default() {
                    out.push(Unit {
                        run: block.run,
                        paragraph,
                        kind: UnitKind::Line(line),
                    });
                }
            } else {
                let mut group = Vec::new();
                for piece in all {
                    let end = piece.sentence_end;
                    group.push(piece);
                    if end {
                        out.push(Unit {
                            run: block.run,
                            paragraph,
                            kind: UnitKind::Pieces(std::mem::take(&mut group)),
                        });
                    }
                }
                if !group.is_empty() {
                    out.push(Unit {
                        run: block.run,
                        paragraph,
                        kind: UnitKind::Pieces(group),
                    });
                }
            }
        } else {
            out.push(Unit {
                run: block.run,
                paragraph,
                kind: UnitKind::Line(block.tokens.clone()),
            });
        }
        font = size_after(&block.tokens, font);
    }
    out
}

/// Lines of `units`, wrapping consecutive parts of one paragraph together.
fn lay_out(units: &[Unit], width: f32) -> Vec<OutLine> {
    let mut lines = Vec::new();
    let mut index = 0;
    while index < units.len() {
        match &units[index].kind {
            UnitKind::Line(tokens) => {
                lines.push(OutLine {
                    tokens: tokens.clone(),
                    run: units[index].run,
                });
                index += 1;
            }
            UnitKind::Pieces(_) => {
                let paragraph = units[index].paragraph;
                let mut group: Vec<Piece> = Vec::new();
                while index < units.len() && units[index].paragraph == paragraph {
                    match &units[index].kind {
                        UnitKind::Pieces(pieces) => group.extend(pieces.iter().cloned()),
                        UnitKind::Line(_) => break,
                    }
                    index += 1;
                }
                let run = units[index - 1].run;
                for tokens in wrap(&group, width, None).unwrap_or_default() {
                    lines.push(OutLine { tokens, run });
                }
            }
        }
    }
    lines
}

// ---------------------------------------------------------------------------
// Output text
// ---------------------------------------------------------------------------

/// Text of `pages` with the page prefix (`\FS[n]`) and the colour/font in
/// effect re-emitted at every page start, and U+000C ending each page but the
/// last. Returns `(line texts, run per line)`.
fn render(
    pages: &[Vec<OutLine>],
    ctx: &FitContext,
    page_font: Option<u32>,
) -> Vec<(String, Option<usize>)> {
    let base = page_font.map_or(ctx.geometry.font_size, |n| n as f32);
    let mut color = 0u32;
    let mut font = base;
    let mut out = Vec::new();
    for (page_index, page) in pages.iter().enumerate() {
        for (line_index, line) in page.iter().enumerate() {
            let mut text = String::new();
            if line_index == 0 {
                if let Some(size) = page_font {
                    text.push_str(&format!("\\FS[{size}]"));
                }
                if page_index > 0 {
                    if (font - base).abs() > f32::EPSILON {
                        text.push_str(&format!("\\FS[{}]", font.round() as u32));
                    }
                    if color != 0 {
                        text.push_str(&format!("\\C[{color}]"));
                    }
                }
            }
            text.push_str(&raw_of(&line.tokens));
            for token in &line.tokens {
                match &token.kind {
                    TokenKind::Color(n) => color = n.unwrap_or(0),
                    kind => font = apply_font_change(kind, font),
                }
            }
            if line_index + 1 == page.len() && page_index + 1 < pages.len() {
                text.push(PAGE_BREAK);
            }
            out.push((text, line.run));
        }
    }
    out
}

fn joined(lines: &[(String, Option<usize>)]) -> String {
    lines
        .iter()
        .map(|(text, _)| text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------
// Pagination
// ---------------------------------------------------------------------------

/// Typical lines per page, the target of even page filling.
const PAGE_LINES: f64 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PageCheck {
    Fits,
    TooTall,
    Overflow,
    Blank,
}

/// How `lines` fare on one page (with the page prefix).
fn check_page(
    lines: &[OutLine],
    ctx: &FitContext,
    spec: &BoxSpec,
    page_font: Option<u32>,
) -> PageCheck {
    let rendered = render(&[lines.to_vec()], ctx, page_font);
    let pages = simulate(&joined(&rendered), spec, ctx.metrics, ctx.inserts);
    if pages.len() > 1 {
        PageCheck::TooTall
    } else if overflows(&pages) {
        PageCheck::Overflow
    } else if !pages.first().is_some_and(PageView::is_visible) {
        PageCheck::Blank
    } else {
        PageCheck::Fits
    }
}

/// Group units into the fewest pages, most evenly filled. A page may not end
/// on a verbatim line (it could not carry the page break). `None` if a unit
/// does not fit a page on its own.
fn paginate(
    units: &[Unit],
    width: f32,
    ctx: &FitContext,
    spec: &BoxSpec,
    page_font: Option<u32>,
) -> Option<Vec<Vec<OutLine>>> {
    let n = units.len();
    let mut best: Vec<Option<(usize, f64)>> = vec![None; n + 1];
    let mut back = vec![0usize; n + 1];
    let mut layouts: HashMap<(usize, usize), Vec<OutLine>> = HashMap::new();
    best[0] = Some((0, 0.0));
    for j in 1..=n {
        if j < n && units[j - 1].run.is_none() {
            continue;
        }
        for i in (0..j).rev() {
            let Some((pages, cost)) = best[i] else {
                continue;
            };
            let lines = lay_out(&units[i..j], width);
            match check_page(&lines, ctx, spec, page_font) {
                PageCheck::Fits => {}
                PageCheck::Blank => continue,
                // Longer ranges only get taller / keep the overflowing unit.
                PageCheck::TooTall | PageCheck::Overflow => break,
            }
            let candidate = (pages + 1, cost + (PAGE_LINES - lines.len() as f64).powi(2));
            let better = best[j].is_none_or(|current| {
                candidate.0 < current.0 || (candidate.0 == current.0 && candidate.1 < current.1)
            });
            if better {
                best[j] = Some(candidate);
                back[j] = i;
                layouts.insert((i, j), lines);
            }
        }
    }
    best[n]?;
    let mut pages = Vec::new();
    let mut j = n;
    while j > 0 {
        let i = back[j];
        pages.push(layouts.remove(&(i, j))?);
        j = i;
    }
    pages.reverse();
    Some(pages)
}

/// All blocks in one flow, giving each run at least as many lines as it has
/// 401 commands: its paragraphs are rewrapped into more, shorter lines, and
/// only the message's very last run may be padded with (invisible) blank
/// lines. `None` when a run cannot keep its line count.
fn lay_out_flow(
    blocks: &[Block],
    runs: &[Vec<usize>],
    ctx: &FitContext,
    size: f32,
    width: f32,
) -> Option<Vec<OutLine>> {
    let mut font = size;
    let mut block_pieces: Vec<Option<Vec<Piece>>> = Vec::with_capacity(blocks.len());
    for block in blocks {
        block_pieces.push(
            (block.wrappable && !block.tokens.is_empty())
                .then(|| pieces(&block.tokens, ctx.metrics, font)),
        );
        font = size_after(&block.tokens, font);
    }
    let mut counts: Vec<usize> = block_pieces
        .iter()
        .map(|p| match p {
            Some(pieces) => wrap(pieces, width, None).map_or(1, |lines| lines.len()),
            None => 1,
        })
        .collect();
    let mut padding = vec![0usize; runs.len()];
    for (run_id, members) in runs.iter().enumerate() {
        let run_blocks: Vec<usize> = (0..blocks.len())
            .filter(|&b| blocks[b].run == Some(run_id))
            .collect();
        let have: usize = run_blocks.iter().map(|&b| counts[b]).sum();
        let mut deficit = members.len().saturating_sub(have);
        while deficit > 0 {
            let growable = run_blocks
                .iter()
                .copied()
                .filter(|&b| {
                    block_pieces[b]
                        .as_ref()
                        .is_some_and(|p| p.len() > counts[b])
                })
                .max_by(|&a, &b| {
                    let ratio = |x: usize| {
                        block_pieces[x].as_ref().map_or(0, Vec::len) as f64 / counts[x] as f64
                    };
                    ratio(a).total_cmp(&ratio(b))
                });
            match growable {
                Some(b) => {
                    counts[b] += 1;
                    deficit -= 1;
                }
                None => break,
            }
        }
        padding[run_id] = deficit;
    }
    let mut lines = Vec::new();
    for (b, block) in blocks.iter().enumerate() {
        match &block_pieces[b] {
            Some(pieces) => {
                for tokens in wrap(pieces, width, Some(counts[b]))? {
                    lines.push(OutLine {
                        tokens,
                        run: block.run,
                    });
                }
            }
            None => lines.push(OutLine {
                tokens: block.tokens.clone(),
                run: block.run,
            }),
        }
        if let Some(run_id) = block.run {
            let last_of_run = blocks[b + 1..]
                .iter()
                .all(|later| later.run != Some(run_id));
            if last_of_run && padding[run_id] > 0 {
                if b + 1 != blocks.len() {
                    return None;
                }
                for _ in 0..padding[run_id] {
                    lines.push(OutLine {
                        tokens: Vec::new(),
                        run: block.run,
                    });
                }
            }
        }
    }
    Some(lines)
}

/// Split a flow of lines into the pages the engine draws (lines per page
/// from the simulation; the last page takes any trailing blank lines).
fn split_pages(flow: Vec<OutLine>, drawn: &[PageView]) -> Vec<Vec<OutLine>> {
    let mut pages = Vec::with_capacity(drawn.len());
    let mut rest = flow.into_iter();
    for (index, page) in drawn.iter().enumerate() {
        let lines: Vec<OutLine> = if index + 1 == drawn.len() {
            rest.by_ref().collect()
        } else {
            rest.by_ref().take(page.lines.len()).collect()
        };
        pages.push(lines);
    }
    pages
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

/// Distribute laid-out lines into each run's 401 commands (one line per
/// command, the rest embedded in the run's last command).
fn distribute(
    lines: &[(String, Option<usize>)],
    runs: &[Vec<usize>],
    total: usize,
) -> Option<Vec<Option<String>>> {
    let mut values = vec![None; total];
    for (run_id, members) in runs.iter().enumerate() {
        let run_lines: Vec<&str> = lines
            .iter()
            .filter(|(_, run)| *run == Some(run_id))
            .map(|(text, _)| text.as_str())
            .collect();
        if run_lines.len() < members.len() {
            return None;
        }
        for (position, &line_index) in members.iter().enumerate() {
            let text = if position + 1 == members.len() {
                run_lines[position..].join("\n")
            } else {
                run_lines[position].to_string()
            };
            values[line_index] = Some(text);
        }
    }
    Some(values)
}

fn unchanged(
    input: &MessageInput,
    ctx: &FitContext,
    spec: &BoxSpec,
    outcome: FitOutcome,
) -> Fitted {
    Fitted {
        outcome,
        values: vec![None; input.lines.len()],
        pages: simulate(
            &assembled(input, &vec![None; input.lines.len()]),
            spec,
            ctx.metrics,
            ctx.inserts,
        ),
    }
}

/// Final message text from per-line values.
fn assembled(input: &MessageInput, values: &[Option<String>]) -> String {
    input
        .lines
        .iter()
        .zip(values)
        .map(|(line, value)| value.as_deref().unwrap_or(&line.text))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The as-translated lines (see [`effective_text`]), with the dialogue font
/// size at every page start.
fn as_is(
    input: &MessageInput,
    ctx: &FitContext,
    spec: &BoxSpec,
    settings: &FitSettings,
) -> (Vec<Option<String>>, Vec<PageView>) {
    let base: Vec<Option<String>> = input
        .lines
        .iter()
        .map(|line| match effective_text(line, settings) {
            Cow::Owned(text) => Some(text),
            Cow::Borrowed(_) => None,
        })
        .collect();
    let mut values = base.clone();
    if let Some(size) = settings.dialogue_font_size {
        // Page starts come from the simulation; a smaller font can move
        // them, so look twice (each time from the lines as translated).
        for _ in 0..2 {
            let pages = simulate(&assembled(input, &values), spec, ctx.metrics, ctx.inserts);
            let mut next = base.clone();
            let mut start = 0;
            for page in &pages {
                if let Some(line) = input.lines.get(start).filter(|l| l.writable) {
                    let body = next[start].clone().unwrap_or_else(|| line.text.clone());
                    next[start] = Some(format!("\\FS[{size}]{body}"));
                }
                start += page.lines.len().max(1);
            }
            values = next;
        }
    }
    let pages = simulate(&assembled(input, &values), spec, ctx.metrics, ctx.inserts);
    (values, pages)
}

/// Fit one message.
pub fn fit_message(input: &MessageInput, ctx: &FitContext, settings: &FitSettings) -> Fitted {
    let spec = message_box(ctx, input.face);
    let width = spec.limit - spec.text_x;
    let all_text = assembled(input, &vec![None; input.lines.len()]);
    let tokens = tokenize(&all_text, ctx.inserts);
    let skip = |reason: &str| {
        unchanged(
            input,
            ctx,
            &spec,
            FitOutcome::Skipped {
                reason: reason.to_string(),
            },
        )
    };
    if tokens.iter().any(|t| t.kind == TokenKind::Position) {
        return skip("uses manual text positioning (\\PX/\\PY)");
    }
    if is_rtl(&all_text) {
        return skip("right-to-left text");
    }
    if tokens.iter().any(|t| t.kind == TokenKind::PageBreak) {
        return skip("already contains page breaks");
    }
    // A page adds a key press: never for auto-closing or timed messages.
    let locked = tokens
        .iter()
        .any(|t| matches!(t.kind, TokenKind::Control('^') | TokenKind::Control('|')));

    let (as_is_values, as_is_pages) = as_is(input, ctx, &spec, settings);
    if !overflows(&as_is_pages) {
        return Fitted {
            outcome: FitOutcome::Fits,
            values: as_is_values,
            pages: as_is_pages,
        };
    }
    let too_long = |reason: &str| Fitted {
        outcome: FitOutcome::TooLong {
            reason: reason.to_string(),
        },
        values: as_is_values.clone(),
        pages: as_is_pages.clone(),
    };
    if settings.overflow == Overflow::Leave {
        return too_long("left as translated");
    }
    // The original number of pages (same line count as the source).
    let target_pages = as_is_pages.len().max(1);
    let (blocks, runs) = blocks(input, settings, ctx);
    let base_font = settings.dialogue_font_size;
    let base = base_font.map_or(ctx.geometry.font_size, |n| n as f32);

    let attempt =
        |size: f32, page_font: Option<u32>, allow_pages: bool| -> Option<Vec<Vec<OutLine>>> {
            if let Some(flow) = lay_out_flow(&blocks, &runs, ctx, size, width) {
                let rendered = render(std::slice::from_ref(&flow), ctx, page_font);
                let drawn = simulate(&joined(&rendered), &spec, ctx.metrics, ctx.inserts);
                if !overflows(&drawn)
                    && drawn.iter().all(PageView::is_visible)
                    && drawn.len() <= target_pages
                {
                    return Some(split_pages(flow, &drawn));
                }
            }
            if !allow_pages {
                return None;
            }
            paginate(
                &units(&blocks, ctx, size, false, width),
                width,
                ctx,
                &spec,
                page_font,
            )
            .or_else(|| {
                paginate(
                    &units(&blocks, ctx, size, true, width),
                    width,
                    ctx,
                    &spec,
                    page_font,
                )
            })
        };
    let finish =
        |pages: Vec<Vec<OutLine>>, page_font: Option<u32>, outcome: FitOutcome| -> Option<Fitted> {
            let rendered = render(&pages, ctx, page_font);
            let values = distribute(&rendered, &runs, input.lines.len())?;
            let drawn = simulate(&assembled(input, &values), &spec, ctx.metrics, ctx.inserts);
            if overflows(&drawn) || !drawn.iter().all(PageView::is_visible) {
                return None;
            }
            Some(Fitted {
                outcome,
                values,
                pages: drawn,
            })
        };
    let shrink = || -> Option<Fitted> {
        for size in (settings.min_font_size..base as u32).rev() {
            if let Some(pages) = attempt(size as f32, Some(size), false) {
                if let Some(fitted) = finish(pages, Some(size), FitOutcome::Shrunk { size }) {
                    return Some(fitted);
                }
            }
        }
        None
    };
    let new_page = || -> Option<Fitted> {
        let pages = attempt(base, base_font, true)?;
        let count = pages.len();
        let outcome = if count > target_pages {
            FitOutcome::NewPage { pages: count }
        } else {
            FitOutcome::Rewrapped
        };
        finish(pages, base_font, outcome)
    };

    // Rewrap within the original number of pages first.
    if let Some(fitted) = attempt(base, base_font, false)
        .and_then(|pages| finish(pages, base_font, FitOutcome::Rewrapped))
    {
        return fitted;
    }
    let result = match (settings.overflow, locked) {
        // Locked messages may only shrink.
        (_, true) => shrink(),
        (Overflow::Shrink, false) => shrink().or_else(new_page),
        (Overflow::NewPage | Overflow::Leave, false) => new_page(),
    };
    result.unwrap_or_else(|| {
        too_long(if locked {
            "does not fit and cannot take a new page (\\^ or \\| code)"
        } else {
            "does not fit even after rewrapping"
        })
    })
}

// ---------------------------------------------------------------------------
// Descriptions
// ---------------------------------------------------------------------------

/// Fit an item/skill/weapon/armor description or an actor profile to the
/// 2-line help box. `plain_first_line`: line 1 is also drawn as plain text
/// elsewhere, so no escape code may be added to it.
pub fn fit_description(
    text: &str,
    plain_first_line: bool,
    ctx: &FitContext,
    settings: &FitSettings,
) -> Fitted {
    let spec = help_box(ctx);
    let width = spec.limit - spec.text_x;
    let draw = |value: &str| simulate(value, &spec, ctx.metrics, ctx.inserts);
    let result = |outcome: FitOutcome, value: Option<String>| {
        let pages = draw(value.as_deref().unwrap_or(text));
        Fitted {
            outcome,
            values: vec![value],
            pages,
        }
    };
    if !overflows(&draw(text)) {
        return result(FitOutcome::Fits, None);
    }
    if is_rtl(text) {
        return result(
            FitOutcome::Skipped {
                reason: "right-to-left text".into(),
            },
            None,
        );
    }
    if settings.overflow == Overflow::Leave {
        return result(
            FitOutcome::TooLong {
                reason: "left as translated".into(),
            },
            None,
        );
    }
    let base = ctx.geometry.font_size;
    let wrap_text = |source: &str, size: f32, lines: Option<usize>| -> Option<Vec<String>> {
        let tokens = tokenize(source, ctx.inserts);
        let all = pieces(&tokens, ctx.metrics, size);
        wrap(&all, width, lines).map(|out| out.iter().map(|line| raw_of(line)).collect())
    };
    let source_lines: Vec<String> = text
        .split('\n')
        .map(|l| {
            l.trim_start_matches(['\u{3000}', ' '])
                .trim_end()
                .to_string()
        })
        .collect();
    let fits = |value: &str| !overflows(&draw(value));

    // Keep the translator's lines, wrapping the wide ones.
    if settings.line_breaks == LineBreaks::Keep {
        let mut lines = Vec::new();
        for line in &source_lines {
            lines.extend(wrap_text(line, base, None).unwrap_or_default());
        }
        let value = lines.join("\n");
        if fits(&value) {
            return result(FitOutcome::Rewrapped, Some(value));
        }
    }
    // Reflow into the box.
    let joiner = if ctx.no_space_join { "" } else { " " };
    let joined_text = source_lines
        .iter()
        .filter(|l| !l.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(joiner);
    if let Some(lines) = wrap_text(&joined_text, base, None) {
        let value = lines.join("\n");
        if fits(&value) {
            return result(FitOutcome::Rewrapped, Some(value));
        }
    }
    // Line 1 when it must stay plain: as much as fits at the normal size
    // (pieces are prefixes of the text; only trailing spaces are dropped).
    let plain_first = || -> Option<(String, String)> {
        let tokens = tokenize(&joined_text, ctx.inserts);
        let all = pieces(&tokens, ctx.metrics, base);
        let mut used = 0.0;
        let mut count = 0;
        for piece in &all {
            if count > 0 && used + piece.width > width + 0.001 {
                break;
            }
            used += piece.width + piece.space;
            count += 1;
        }
        let first: String = all[..count].iter().map(|p| raw_of(&p.tokens)).collect();
        let rest: String = all[count..].iter().map(|p| raw_of(&p.tokens)).collect();
        let rest = rest.trim().to_string();
        (!rest.is_empty()).then(|| (first.trim_end().to_string(), rest))
    };
    // Shrink (line 1 stays plain when another window shows it as text).
    let mut size = base as u32;
    while size > settings.min_font_size {
        size -= 1;
        let value = if plain_first_line {
            let Some((first, rest)) = plain_first() else {
                break;
            };
            let Some(rest_lines) = wrap_text(&rest, size as f32, None) else {
                continue;
            };
            format!("{first}\n\\FS[{size}]{}", rest_lines.join("\n"))
        } else {
            let Some(lines) = wrap_text(&joined_text, size as f32, None) else {
                continue;
            };
            format!("\\FS[{size}]{}", lines.join("\n"))
        };
        if fits(&value) {
            return result(FitOutcome::Shrunk { size }, Some(value));
        }
    }
    result(
        FitOutcome::TooLong {
            reason: "longer than the 2-line help box even at the minimum size".into(),
        },
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geometry() -> Geometry {
        Geometry::from_system(&serde_json::json!({
            "advanced": { "uiAreaWidth": 816, "fontSize": 26 }
        }))
    }

    fn settings() -> FitSettings {
        FitSettings {
            line_breaks: LineBreaks::Reflow,
            overflow: Overflow::NewPage,
            min_font_size: 20,
            dialogue_font_size: None,
            strip_stored_font_prefix: false,
        }
    }

    fn fit(lines: &[&str], settings: &FitSettings) -> Fitted {
        let geometry = geometry();
        let inserts = Inserts::default();
        let ctx = FitContext {
            metrics: &Metrics::Estimated,
            geometry: &geometry,
            inserts: &inserts,
            no_space_join: false,
            plain_description_first_line: false,
        };
        let input = MessageInput {
            lines: lines
                .iter()
                .map(|text| MessageLine {
                    text: text.to_string(),
                    writable: !text.starts_with('…'),
                })
                .collect(),
            face: false,
        };
        fit_message(&input, &ctx, settings)
    }

    /// Final text of a fitted message (unchanged lines as given).
    fn output(lines: &[&str], fitted: &Fitted) -> Vec<String> {
        lines
            .iter()
            .zip(&fitted.values)
            .map(|(line, value)| value.clone().unwrap_or_else(|| line.to_string()))
            .collect()
    }

    fn words(text: &str) -> Vec<String> {
        text.replace(PAGE_BREAK, " ")
            .split_whitespace()
            .map(str::to_string)
            .collect()
    }

    /// Words `a00 a01 …` ending with a full stop (3 characters + space each;
    /// 59 Latin characters fit a portrait-less line: 776 px / 13 px).
    fn sentence(count: usize, tag: &str) -> String {
        let mut words: Vec<String> = (0..count).map(|i| format!("{tag}{i:02}")).collect();
        if let Some(last) = words.last_mut() {
            last.push('.');
        }
        words.join(" ")
    }

    #[test]
    fn a_fitting_message_is_left_alone() {
        let fitted = fit(&["Short line.", "Another one."], &settings());
        assert_eq!(fitted.outcome, FitOutcome::Fits);
        assert!(fitted.values.iter().all(Option::is_none));
    }

    #[test]
    fn a_too_wide_line_is_rewrapped_into_the_same_commands() {
        let long = sentence(16, "a");
        let lines = [long.as_str(), "End."];
        let fitted = fit(&lines, &settings());
        assert_eq!(fitted.outcome, FitOutcome::Rewrapped);
        let out = output(&lines, &fitted);
        assert_eq!(out.len(), 2);
        assert_eq!(words(&out.join(" ")), words(&lines.join(" ")));
        assert_eq!(fitted.pages.len(), 1);
        assert!(fitted.pages[0].lines.iter().all(|l| !l.overflow));
    }

    #[test]
    fn a_message_needing_five_lines_continues_on_a_page_starting_a_sentence() {
        // Four 60-character lines (each too wide) reflow into five lines.
        let owned = [
            sentence(15, "a"),
            sentence(15, "b"),
            sentence(15, "c"),
            sentence(15, "d"),
        ];
        let lines: Vec<&str> = owned.iter().map(String::as_str).collect();
        let fitted = fit(&lines, &settings());
        assert_eq!(fitted.outcome, FitOutcome::NewPage { pages: 2 });
        let out = output(&lines, &fitted).join("\n");
        assert_eq!(out.matches(PAGE_BREAK).count(), 1);
        let before = out.split(PAGE_BREAK).next().unwrap();
        assert!(before.trim_end().ends_with('.'));
        assert_eq!(words(&out), words(&lines.join(" ")));
        assert!(fitted.pages.iter().all(PageView::is_visible));
    }

    #[test]
    fn timed_messages_shrink_instead_of_paging() {
        let owned = [
            format!("\\|{}", sentence(15, "a")),
            sentence(15, "b"),
            sentence(15, "c"),
            sentence(14, "d"),
        ];
        let lines: Vec<&str> = owned.iter().map(String::as_str).collect();
        let fitted = fit(&lines, &settings());
        assert!(
            matches!(fitted.outcome, FitOutcome::Shrunk { .. }),
            "{:?}",
            fitted.outcome
        );
        let out = output(&lines, &fitted).join("\n");
        assert!(!out.contains(PAGE_BREAK));
        assert!(out.starts_with("\\FS["));
    }

    #[test]
    fn shrink_setting_reduces_the_font_before_paging() {
        let owned = [
            sentence(15, "a"),
            sentence(15, "b"),
            sentence(15, "c"),
            sentence(14, "d"),
        ];
        let lines: Vec<&str> = owned.iter().map(String::as_str).collect();
        let mut shrink = settings();
        shrink.overflow = Overflow::Shrink;
        let fitted = fit(&lines, &shrink);
        let FitOutcome::Shrunk { size } = fitted.outcome else {
            panic!("{:?}", fitted.outcome);
        };
        assert!((20..26).contains(&size));
        assert_eq!(fitted.pages.len(), 1);
    }

    #[test]
    fn leave_reports_without_changing() {
        let long = sentence(20, "a");
        let mut leave = settings();
        leave.overflow = Overflow::Leave;
        let fitted = fit(&[long.as_str()], &leave);
        assert!(matches!(fitted.outcome, FitOutcome::TooLong { .. }));
        assert!(fitted.values.iter().all(Option::is_none));
    }

    #[test]
    fn verbatim_lines_are_never_written() {
        let long = sentence(16, "a");
        let lines = [long.as_str(), "……", "Fine."];
        let fitted = fit(&lines, &settings());
        assert!(fitted.outcome.changed_layout(), "{:?}", fitted.outcome);
        assert_eq!(fitted.values[1], None);
        assert!(fitted.values[0].is_some() && fitted.values[2].is_some());
    }

    #[test]
    fn a_block_keeps_at_least_its_number_of_commands() {
        // The reflowed text needs two lines but the message has three commands.
        let long = sentence(15, "a");
        let lines = [long.as_str(), "b.", "c."];
        let fitted = fit(&lines, &settings());
        assert_eq!(fitted.outcome, FitOutcome::Rewrapped);
        assert!(fitted
            .values
            .iter()
            .all(|v| v.as_deref().is_some_and(|t| !t.is_empty())));
    }

    #[test]
    fn dialogue_font_size_prefixes_each_page() {
        let mut sized = settings();
        sized.dialogue_font_size = Some(22);
        let fitted = fit(&["Short.", "Fine."], &sized);
        assert_eq!(fitted.outcome, FitOutcome::Fits);
        assert_eq!(fitted.values[0].as_deref(), Some("\\FS[22]Short."));
        assert_eq!(fitted.values[1], None);
    }

    #[test]
    fn colour_carries_over_a_page_break() {
        let owned = [
            format!("\\C[2]{}", sentence(15, "a")),
            sentence(15, "b"),
            sentence(15, "c"),
            sentence(15, "d"),
        ];
        let lines: Vec<&str> = owned.iter().map(String::as_str).collect();
        let fitted = fit(&lines, &settings());
        let out = output(&lines, &fitted).join("\n");
        let second_page = out.split(PAGE_BREAK).nth(1).unwrap();
        assert!(
            second_page.trim_start().starts_with("\\C[2]"),
            "{second_page:?}"
        );
        assert_eq!(fitted.pages[1].lines[0].runs[0].color, 2);
    }

    fn describe(text: &str, plain: bool, settings: &FitSettings) -> Fitted {
        let geometry = geometry();
        let inserts = Inserts::default();
        let ctx = FitContext {
            metrics: &Metrics::Estimated,
            geometry: &geometry,
            inserts: &inserts,
            no_space_join: false,
            plain_description_first_line: plain,
        };
        fit_description(text, plain, &ctx, settings)
    }

    #[test]
    fn descriptions_reflow_shrink_or_report() {
        assert_eq!(
            describe("A potion.", false, &settings()).outcome,
            FitOutcome::Fits
        );
        // 768 px / 13 px ≈ 59 characters per line.
        let reflow = describe(
            &format!("{}\nHeals.", sentence(16, "w")),
            false,
            &settings(),
        );
        assert_eq!(reflow.outcome, FitOutcome::Rewrapped);
        assert_eq!(reflow.values[0].as_deref().unwrap().lines().count(), 2);

        // 128 characters: three lines at 26 px, two at a smaller size.
        let shrunk = describe(&sentence(32, "w"), true, &settings());
        assert!(
            matches!(shrunk.outcome, FitOutcome::Shrunk { .. }),
            "{:?}",
            shrunk.outcome
        );
        let value = shrunk.values[0].clone().unwrap();
        let first = value.lines().next().unwrap();
        assert!(!first.contains('\\'), "line 1 must stay plain: {first:?}");

        let far_too_long = describe(&sentence(60, "w"), false, &settings());
        assert!(matches!(far_too_long.outcome, FitOutcome::TooLong { .. }));
        assert_eq!(far_too_long.values[0], None);
    }
}
