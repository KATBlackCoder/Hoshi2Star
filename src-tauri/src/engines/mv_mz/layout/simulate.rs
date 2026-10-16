//! Re-run of how MZ draws a message or a help text: `Window_Message`
//! (`startMessage`, `newPage`, `processNewLine`, `processNewPage`,
//! `needsNewPage`) and `Window_Help` (`drawTextEx`, no paging). Used for the
//! preview, the "fits as is" check and the final output check.

use serde::Serialize;

use super::font::Metrics;
use super::text::{apply_font_change, token_width, tokenize, Inserts, Token, TokenKind};

/// The box a text is drawn into.
#[derive(Debug, Clone, PartialEq)]
pub struct BoxSpec {
    /// x where every line starts (`newLineX` / `baseTextRect().x`).
    pub text_x: f32,
    /// Rightmost x a line may reach.
    pub limit: f32,
    /// Contents height.
    pub height: f32,
    /// `$gameSystem.mainFontSize()` (font reset value).
    pub main_font: f32,
    /// `lineHeight() − mainFontSize()`.
    pub line_spacing: f32,
    /// Message window (pages) or help window (no paging, clipped).
    pub paging: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunView {
    pub text: String,
    pub x: f32,
    pub width: f32,
    pub font_size: f32,
    pub color: u32,
    pub icon: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LineView {
    pub runs: Vec<RunView>,
    pub y: f32,
    pub height: f32,
    /// Rightmost x reached by the line.
    pub right: f32,
    /// Wider than the box.
    pub overflow: bool,
    /// Below the box (help window only: the engine clips it).
    pub clipped: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageView {
    pub lines: Vec<LineView>,
}

impl PageView {
    /// Whether anything is drawn on the page.
    pub fn is_visible(&self) -> bool {
        self.lines
            .iter()
            .flat_map(|line| &line.runs)
            .any(|run| run.icon || run.text.chars().any(|c| !c.is_whitespace()))
    }
}

/// `calcTextHeight`: largest font in the line (starting from the current
/// size) plus the line spacing.
fn line_height(tokens: &[Token], from: usize, font: f32, spec: &BoxSpec) -> f32 {
    let mut size = font;
    let mut max = font;
    for token in &tokens[from..] {
        if token.kind == TokenKind::Newline {
            break;
        }
        size = apply_font_change(&token.kind, size);
        max = max.max(size);
    }
    max + spec.line_spacing
}

struct State<'a> {
    spec: &'a BoxSpec,
    pages: Vec<PageView>,
    x: f32,
    y: f32,
    height: f32,
    font: f32,
    color: u32,
    /// A line starts at the current y (created when the next token is
    /// processed, on whatever page it lands).
    pending_line: bool,
}

impl State<'_> {
    fn line(&mut self) -> &mut LineView {
        let (y, height) = (self.y, self.height);
        let page = self.pages.last_mut().expect("a page exists");
        if page.lines.last().is_none_or(|line| line.y != y) {
            page.lines.push(LineView {
                runs: Vec::new(),
                y,
                height,
                right: 0.0,
                overflow: false,
                clipped: false,
            });
        }
        page.lines.last_mut().expect("a line exists")
    }

    fn draw(&mut self, text: String, width: f32, icon: bool) {
        let (x, font, color, limit, bottom) = (
            self.x,
            self.font,
            self.color,
            self.spec.limit,
            self.spec.height,
        );
        let line = self.line();
        line.runs.push(RunView {
            text,
            x,
            width,
            font_size: font,
            color,
            icon,
        });
        line.right = line.right.max(x + width);
        line.overflow = line.right > limit + 0.01;
        line.clipped = line.y + line.height > bottom + 0.01;
        self.x += width;
    }

    fn new_page(&mut self, tokens: &[Token], index: usize) {
        self.pages.push(PageView { lines: Vec::new() });
        self.x = self.spec.text_x;
        self.y = 0.0;
        self.font = self.spec.main_font;
        self.color = 0;
        self.height = line_height(tokens, index, self.font, self.spec);
        self.pending_line = true;
    }
}

/// Simulate drawing `text`.
pub fn simulate(text: &str, spec: &BoxSpec, metrics: &Metrics, inserts: &Inserts) -> Vec<PageView> {
    let tokens = tokenize(text, inserts);
    let mut state = State {
        spec,
        pages: Vec::new(),
        x: spec.text_x,
        y: 0.0,
        height: 0.0,
        font: spec.main_font,
        color: 0,
        pending_line: true,
    };
    state.new_page(&tokens, 0);
    let mut index = 0;
    while index < tokens.len() {
        let at_end = index >= tokens.len();
        if spec.paging && !at_end && state.y + state.height > spec.height + 0.01 {
            state.new_page(&tokens, index);
        }
        if state.pending_line {
            state.pending_line = false;
            state.line();
        }
        let token = &tokens[index];
        match &token.kind {
            TokenKind::Text | TokenKind::Backslash | TokenKind::Inserted { .. } => {
                let shown = match &token.kind {
                    TokenKind::Text => token.raw.clone(),
                    TokenKind::Backslash => "\\".to_string(),
                    TokenKind::Inserted { sample } => sample.clone(),
                    _ => unreachable!(),
                };
                let width = token_width(token, metrics, state.font);
                state.draw(shown, width, false);
            }
            TokenKind::Icon => {
                let width = token_width(token, metrics, state.font);
                state.draw(String::new(), width, true);
            }
            TokenKind::Newline => {
                state.x = spec.text_x;
                state.y += state.height;
                state.height = line_height(&tokens, index + 1, state.font, spec);
                state.pending_line = true;
            }
            TokenKind::PageBreak => {
                if spec.paging {
                    if tokens
                        .get(index + 1)
                        .is_some_and(|t| t.kind == TokenKind::Newline)
                    {
                        index += 1;
                    }
                    state.y = spec.height;
                    // The engine then needs a new page only if text follows.
                    if index + 1 >= tokens.len() {
                        break;
                    }
                    state.height = line_height(&tokens, index + 1, state.font, spec);
                    state.pending_line = false;
                }
            }
            TokenKind::Color(n) => state.color = n.unwrap_or(0),
            TokenKind::FontSize(_) | TokenKind::Bigger | TokenKind::Smaller => {
                state.font = apply_font_change(&token.kind, state.font);
            }
            TokenKind::Position | TokenKind::Control(_) | TokenKind::Unknown(_) => {}
        }
        index += 1;
    }
    // Drop a trailing empty page produced by a final page break.
    if state.pages.len() > 1 && state.pages.last().is_some_and(|p| p.lines.is_empty()) {
        state.pages.pop();
    }
    state.pages
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MZ defaults: no portrait, 776 px usable, 152 px, 26 px font.
    pub(crate) fn message_box() -> BoxSpec {
        BoxSpec {
            text_x: 4.0,
            limit: 780.0,
            height: 152.0,
            main_font: 26.0,
            line_spacing: 10.0,
            paging: true,
        }
    }

    fn page_lines(text: &str) -> Vec<Vec<String>> {
        simulate(
            text,
            &message_box(),
            &Metrics::Estimated,
            &Inserts::default(),
        )
        .iter()
        .map(|page| {
            page.lines
                .iter()
                .map(|line| line.runs.iter().map(|r| r.text.as_str()).collect())
                .collect()
        })
        .collect()
    }

    #[test]
    fn four_lines_per_page_then_an_automatic_page() {
        assert_eq!(
            page_lines("1\n2\n3\n4\n5"),
            vec![vec!["1", "2", "3", "4"], vec!["5"]]
        );
    }

    #[test]
    fn form_feed_forces_a_page_and_swallows_the_newline() {
        assert_eq!(
            page_lines("1\n2\u{000C}\n3"),
            vec![vec!["1", "2"], vec!["3"]]
        );
        // After a full page the auto break and the form feed do not stack.
        assert_eq!(
            page_lines("1\n2\n3\n4\u{000C}\n5"),
            vec![vec!["1", "2", "3", "4"], vec!["5"]]
        );
        // A final form feed opens no page.
        assert_eq!(page_lines("1\u{000C}"), vec![vec!["1"]]);
    }

    #[test]
    fn newline_then_form_feed_after_a_full_page_makes_an_empty_page() {
        // The engine trap the layout must avoid: "\n\f" after a full page.
        let pages = simulate(
            "1\n2\n3\n4\n\u{000C}5",
            &message_box(),
            &Metrics::Estimated,
            &Inserts::default(),
        );
        assert_eq!(pages.len(), 3);
        assert!(!pages[1].is_visible());
    }

    #[test]
    fn font_resets_at_each_page_and_sets_line_heights() {
        let pages = simulate(
            "\\FS[20]a\nb\nc\nd\ne\u{000C}\nf",
            &message_box(),
            &Metrics::Estimated,
            &Inserts::default(),
        );
        // First line ≥ 36 (reset size 26); then 20 + 10 = 30 → 5 lines fit
        // in 152 px only at ≤ 19 px, so line 5 opens page 2.
        let heights: Vec<f32> = pages[0].lines.iter().map(|l| l.height).collect();
        assert_eq!(heights, vec![36.0, 30.0, 30.0, 30.0]);
        assert_eq!(pages[1].lines[0].runs[0].font_size, 26.0);
        assert_eq!(pages[1].lines[0].runs[0].text, "e");
    }

    #[test]
    fn widths_and_overflow() {
        let long = "x".repeat(60);
        let pages = simulate(
            &long,
            &message_box(),
            &Metrics::Estimated,
            &Inserts::default(),
        );
        let line = &pages[0].lines[0];
        assert_eq!(line.right, 4.0 + 60.0 * 13.0);
        assert!(line.overflow);
        let help = BoxSpec {
            text_x: 8.0,
            limit: 776.0,
            height: 72.0,
            paging: false,
            ..message_box()
        };
        let pages = simulate("a\nb\nc", &help, &Metrics::Estimated, &Inserts::default());
        assert_eq!(pages.len(), 1);
        assert_eq!(
            pages[0].lines.iter().map(|l| l.clipped).collect::<Vec<_>>(),
            vec![false, false, true]
        );
    }
}
