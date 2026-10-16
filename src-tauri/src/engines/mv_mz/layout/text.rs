//! Message text as the MZ engine reads it.
//!
//! Mirrors `Window_Base.convertEscapeCharacters` (inserted text: `\V[n]`,
//! `\N[n]`, `\P[n]`, `\G`, plus plugin codes), `obtainEscapeCode` (a code is
//! one of `$.|^!><{}\` or a run of ASCII letters, case-insensitive) and
//! `processEscapeCharacter` (`\C[n]`, `\I[n]`, `\PX[n]`, `\PY[n]`, `\FS[n]`,
//! `\{`, `\}`). Every token keeps its raw text, so tokens re-assemble into the
//! exact original string.

use std::collections::HashMap;

use super::font::Metrics;

/// `ImageManager.iconWidth + 4` (`processDrawIcon`).
pub const ICON_ADVANCE: f32 = 36.0;
/// U+000C: `Window_Message.processNewPage`.
pub const PAGE_BREAK: char = '\u{000C}';

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// Printable characters (no control characters).
    Text,
    Newline,
    PageBreak,
    /// `\C[n]` (colour index; `None` when the parameter is not a number).
    Color(Option<u32>),
    /// `\I[n]`.
    Icon,
    /// `\PX[n]` / `\PY[n]`: manual positioning.
    Position,
    /// `\FS[n]`.
    FontSize(u32),
    /// `\{`.
    Bigger,
    /// `\}`.
    Smaller,
    /// Text inserted by `convertEscapeCharacters`, measured as `sample`.
    Inserted {
        sample: String,
    },
    /// `\\`: prints a backslash.
    Backslash,
    /// `\$ \. \| \! \> \< \^`: no width; `code` is the character.
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
    fn new(kind: TokenKind, raw: impl Into<String>) -> Self {
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

/// What inserted-text codes print, as widest plausible samples.
#[derive(Debug, Clone, Default)]
pub struct Inserts {
    /// Actor id → translated name (`\N[n]`).
    pub actor_names: HashMap<u32, String>,
    /// Translated currency unit (`\G`).
    pub currency: String,
    /// Extra codes from plugins (`S`, `SS`, `SV`…), all printing a value.
    pub plugin_codes: Vec<String>,
}

impl Inserts {
    /// `\V[n]`: variables are assumed to print up to four digits.
    const VARIABLE_SAMPLE: &'static str = "0000";
    /// Plugin codes print a switch (`true`/`false`) or a number.
    const PLUGIN_SAMPLE: &'static str = "false";

    fn actor(&self, id: u32) -> String {
        self.actor_names.get(&id).cloned().unwrap_or_default()
    }

    /// `\P[n]` can be any party member: the longest actor name.
    fn widest_actor(&self) -> String {
        self.actor_names
            .values()
            .max_by_key(|name| name.chars().count())
            .cloned()
            .unwrap_or_default()
    }
}

/// Split raw message text into tokens.
pub fn tokenize(text: &str, inserts: &Inserts) -> Vec<Token> {
    let chars: Vec<char> = text.chars().collect();
    let mut tokens: Vec<Token> = Vec::new();
    let mut buffer = String::new();
    let flush = |buffer: &mut String, tokens: &mut Vec<Token>| {
        if !buffer.is_empty() {
            tokens.push(Token::new(TokenKind::Text, std::mem::take(buffer)));
        }
    };
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\n' => {
                flush(&mut buffer, &mut tokens);
                tokens.push(Token::new(TokenKind::Newline, "\n"));
                i += 1;
            }
            PAGE_BREAK => {
                flush(&mut buffer, &mut tokens);
                tokens.push(Token::new(TokenKind::PageBreak, PAGE_BREAK.to_string()));
                i += 1;
            }
            '\\' => {
                flush(&mut buffer, &mut tokens);
                let (token, used) = escape(&chars[i..], inserts);
                tokens.push(token);
                i += used;
            }
            _ if (c as u32) < 0x20 => {
                // Other control characters are flushed and ignored by the engine.
                flush(&mut buffer, &mut tokens);
                tokens.push(Token::new(TokenKind::Unknown(String::new()), c.to_string()));
                i += 1;
            }
            _ => {
                buffer.push(c);
                i += 1;
            }
        }
    }
    flush(&mut buffer, &mut tokens);
    tokens
}

/// `[digits]` right after a code: `(value, characters used)`.
fn digit_param(chars: &[char]) -> Option<(u32, usize)> {
    if chars.first() != Some(&'[') {
        return None;
    }
    let digits: String = chars[1..]
        .iter()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if digits.is_empty() || chars.get(1 + digits.len()) != Some(&']') {
        return None;
    }
    digits.parse().ok().map(|value| (value, digits.len() + 2))
}

/// `[word]` (letters, digits, `_`) right after a code.
fn word_param(chars: &[char]) -> Option<usize> {
    if chars.first() != Some(&'[') {
        return None;
    }
    let len = chars[1..]
        .iter()
        .take_while(|c| c.is_ascii_alphanumeric() || **c == '_')
        .count();
    (len > 0 && chars.get(1 + len) == Some(&']')).then_some(len + 2)
}

/// One escape sequence starting at a backslash: `(token, characters used)`.
fn escape(chars: &[char], inserts: &Inserts) -> (Token, usize) {
    let raw = |n: usize| chars[..n].iter().collect::<String>();
    let Some(&next) = chars.get(1) else {
        return (Token::new(TokenKind::Unknown(String::new()), "\\"), 1);
    };
    if next == '\\' {
        return (Token::new(TokenKind::Backslash, "\\\\"), 2);
    }
    if "$.|^!><".contains(next) {
        return (Token::new(TokenKind::Control(next), raw(2)), 2);
    }
    if next == '{' {
        return (Token::new(TokenKind::Bigger, "\\{"), 2);
    }
    if next == '}' {
        return (Token::new(TokenKind::Smaller, "\\}"), 2);
    }

    // convertEscapeCharacters runs before code parsing: V/N/P need `[digits]`,
    // G needs nothing (`\Gold` is the currency followed by "old").
    let upper = next.to_ascii_uppercase();
    let after = &chars[2..];
    match upper {
        'V' | 'N' | 'P' => {
            if let Some((id, used)) = digit_param(after) {
                let sample = match upper {
                    'V' => Inserts::VARIABLE_SAMPLE.to_string(),
                    'N' => inserts.actor(id),
                    _ => inserts.widest_actor(),
                };
                return (
                    Token::new(TokenKind::Inserted { sample }, raw(2 + used)),
                    2 + used,
                );
            }
        }
        'G' => {
            return (
                Token::new(
                    TokenKind::Inserted {
                        sample: inserts.currency.clone(),
                    },
                    raw(2),
                ),
                2,
            );
        }
        _ => {}
    }
    if next.is_ascii_alphabetic() {
        let letters: String = chars[1..]
            .iter()
            .take_while(|c| c.is_ascii_alphabetic())
            .collect();
        let code = letters.to_ascii_uppercase();
        let after = &chars[1 + letters.len()..];
        let base = 1 + letters.len();
        // Plugin codes (PluginCommonBase & co.) are expanded before parsing.
        if inserts.plugin_codes.contains(&code) {
            if let Some(used) = word_param(after) {
                return (
                    Token::new(
                        TokenKind::Inserted {
                            sample: Inserts::PLUGIN_SAMPLE.to_string(),
                        },
                        raw(base + used),
                    ),
                    base + used,
                );
            }
        }
        let param = digit_param(after);
        let with_param = |kind: TokenKind| match param {
            Some((_, used)) => (Token::new(kind, raw(base + used)), base + used),
            None => (Token::new(kind, raw(base)), base),
        };
        return match code.as_str() {
            "C" => with_param(TokenKind::Color(param.map(|(v, _)| v))),
            "I" => with_param(TokenKind::Icon),
            "PX" | "PY" => with_param(TokenKind::Position),
            "FS" => match param {
                Some((size, used)) => (
                    Token::new(TokenKind::FontSize(size), raw(base + used)),
                    base + used,
                ),
                None => (
                    Token::new(TokenKind::Unknown(code.clone()), raw(base)),
                    base,
                ),
            },
            _ => (
                Token::new(TokenKind::Unknown(code.clone()), raw(base)),
                base,
            ),
        };
    }
    // A backslash before anything else is dropped by the engine.
    (Token::new(TokenKind::Unknown(String::new()), raw(1)), 1)
}

/// `makeFontBigger` / `makeFontSmaller` / `\FS[n]` applied to `size`.
pub fn apply_font_change(kind: &TokenKind, size: f32) -> f32 {
    match kind {
        TokenKind::FontSize(n) => *n as f32,
        TokenKind::Bigger if size <= 96.0 => size + 12.0,
        TokenKind::Smaller if size >= 24.0 => size - 12.0,
        _ => size,
    }
}

/// Width of one token at font size `size` (per-character, as normal play draws).
pub fn token_width(token: &Token, metrics: &Metrics, size: f32) -> f32 {
    match &token.kind {
        TokenKind::Text => metrics.text_width(&token.raw, size),
        TokenKind::Icon => ICON_ADVANCE,
        TokenKind::Inserted { sample } => metrics.text_width(sample, size),
        TokenKind::Backslash => metrics.text_width("\\", size),
        _ => 0.0,
    }
}

/// Concatenate the raw text of tokens.
pub fn raw_of(tokens: &[Token]) -> String {
    tokens.iter().map(|t| t.raw.as_str()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inserts() -> Inserts {
        Inserts {
            actor_names: HashMap::from([(1, "Megumi".to_string()), (2, "Al".to_string())]),
            currency: "G".into(),
            plugin_codes: vec!["SV".into()],
        }
    }

    fn kinds(text: &str) -> Vec<TokenKind> {
        tokenize(text, &inserts())
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn tokens_round_trip_exactly() {
        for text in [
            "Hello \\C[2]world\\C[0]!",
            "\\{BIG\\} \\I[64] \\V[12] \\N[1] \\P[3] \\Gold \\\\ \\. \\| \\^",
            "\\FS[22]small\nnext\u{000C}page \\XYZ[3] \\SV[a] \\",
            "",
        ] {
            assert_eq!(raw_of(&tokenize(text, &inserts())), text);
        }
    }

    #[test]
    fn mirrors_engine_code_parsing() {
        assert_eq!(
            kinds("a\\C[2]b"),
            vec![TokenKind::Text, TokenKind::Color(Some(2)), TokenKind::Text]
        );
        // `\Gold` = currency then "old"; `\N[1]` inserts the actor's name.
        let tokens = tokenize("\\Gold \\N[1]", &inserts());
        assert_eq!(tokens[0].kind, TokenKind::Inserted { sample: "G".into() });
        assert_eq!(tokens[1].raw, "old ");
        assert_eq!(
            tokens[2].kind,
            TokenKind::Inserted {
                sample: "Megumi".into()
            }
        );
        // `\P[n]` is the widest actor name, `\V[n]` four digits.
        assert_eq!(
            kinds("\\P[2]\\V[9]"),
            vec![
                TokenKind::Inserted {
                    sample: "Megumi".into()
                },
                TokenKind::Inserted {
                    sample: "0000".into()
                }
            ]
        );
        // Letters run greedily: `\Cabc` is the unknown code CABC.
        assert_eq!(kinds("\\Cabc"), vec![TokenKind::Unknown("CABC".into())]);
        // Unknown code: its parameter prints as text.
        let tokens = tokenize("\\XY[3]", &inserts());
        assert_eq!(tokens[0].kind, TokenKind::Unknown("XY".into()));
        assert_eq!(tokens[1].raw, "[3]");
        assert_eq!(kinds("\\fs[20]"), vec![TokenKind::FontSize(20)]);
        assert_eq!(
            kinds("\\SV[a1]"),
            vec![TokenKind::Inserted {
                sample: "false".into()
            }]
        );
    }

    #[test]
    fn font_changes_follow_engine_thresholds() {
        assert_eq!(apply_font_change(&TokenKind::Smaller, 26.0), 14.0);
        assert_eq!(apply_font_change(&TokenKind::Smaller, 22.0), 22.0);
        assert_eq!(apply_font_change(&TokenKind::Bigger, 96.0), 108.0);
        assert_eq!(apply_font_change(&TokenKind::Bigger, 100.0), 100.0);
        assert_eq!(apply_font_change(&TokenKind::FontSize(18), 26.0), 18.0);
    }

    #[test]
    fn widths_per_token() {
        let metrics = Metrics::Estimated;
        let tokens = tokenize("ab\\I[3]\\N[2]\\C[1]", &inserts());
        let widths: Vec<f32> = tokens
            .iter()
            .map(|t| token_width(t, &metrics, 26.0))
            .collect();
        assert_eq!(widths, vec![26.0, 36.0, 26.0, 0.0]);
    }
}
