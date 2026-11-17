//! Message text as layout tokens, independent of any engine.
//!
//! Each engine reads its own codes into these tokens (MZ:
//! `engines::mv_mz::layout::text`, Wolf: `engines::wolf::layout::text`).
//! Every token keeps its raw text, so tokens re-assemble into the exact
//! original string. What a relative font size code or an icon measures is an
//! engine rule ([`TokenRules`]), not part of the token.

use super::font::Metrics;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// Printable characters (no control characters).
    Text,
    Newline,
    PageBreak,
    /// Colour code (colour index; `None` when the parameter is not a number).
    Color(Option<u32>),
    /// Icon (its advance is an engine rule, [`TokenRules::icon_advance`]).
    Icon,
    /// Manual positioning.
    Position,
    /// Absolute font size.
    FontSize(u32),
    /// Relative font size step up (an engine rule, [`TokenRules::relative_font_size`]).
    Bigger,
    /// Relative font size step down.
    Smaller,
    /// Text inserted by the engine at draw time, measured as `sample`.
    Inserted {
        sample: String,
    },
    /// An escaped backslash: prints a backslash.
    Backslash,
    /// A control code with no width; `code` is the character.
    Control(char),
    /// Any other code: no width (its `[...]`, if any, prints as text).
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub raw: String,
}

impl Token {
    pub fn new(kind: TokenKind, raw: impl Into<String>) -> Self {
        Self {
            kind,
            raw: raw.into(),
        }
    }

    /// Whether the token draws something (text, icon, inserted text).
    pub fn is_visible(&self) -> bool {
        match &self.kind {
            TokenKind::Text => self.raw.chars().any(|c| !c.is_whitespace()),
            TokenKind::Icon | TokenKind::Backslash => true,
            TokenKind::Inserted { sample } => !sample.is_empty(),
            _ => false,
        }
    }
}

/// The engine rules of the tokens whose effect depends on the engine.
pub trait TokenRules {
    /// Font size after a relative size step from `size` (`bigger`: up).
    fn relative_font_size(&self, bigger: bool, size: f32) -> f32;
    /// Horizontal advance of an icon.
    fn icon_advance(&self) -> f32;
}

/// Font size after `kind`, from `size`.
pub fn font_size_after(kind: &TokenKind, size: f32, rules: &impl TokenRules) -> f32 {
    match kind {
        TokenKind::FontSize(n) => *n as f32,
        TokenKind::Bigger => rules.relative_font_size(true, size),
        TokenKind::Smaller => rules.relative_font_size(false, size),
        _ => size,
    }
}

/// Width of one token at font size `size` (per-character advances).
pub fn width_of(token: &Token, metrics: &Metrics, size: f32, rules: &impl TokenRules) -> f32 {
    match &token.kind {
        TokenKind::Text => metrics.text_width(&token.raw, size),
        TokenKind::Icon => rules.icon_advance(),
        TokenKind::Inserted { sample } => metrics.text_width(sample, size),
        TokenKind::Backslash => metrics.text_width("\\", size),
        _ => 0.0,
    }
}

/// Concatenate the raw text of tokens.
pub fn raw_of(tokens: &[Token]) -> String {
    tokens.iter().map(|t| t.raw.as_str()).collect()
}
