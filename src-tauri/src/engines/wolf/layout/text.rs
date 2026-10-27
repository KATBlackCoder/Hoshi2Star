//! Wolf RPG message text as layout tokens (plan WR, rule R1).
//!
//! Produces the MZ layout [`Token`]s so the shared line breaking
//! ([`crate::engines::mv_mz::layout::wrap`]) applies unchanged. Every token
//! keeps its raw text: tokens re-assemble into the exact original string.

use std::sync::LazyLock;

use regex::Regex;

use crate::engines::mv_mz::layout::text::{Token, TokenKind};

/// `\v[n]`, `\self[n]`… print a number: assumed up to four digits.
const NUMBER_SAMPLE: &str = "0000";
/// `\s[n]`, `\sysS[n]`, database references… print a string (often a name):
/// six full-width characters, a deliberately wide estimate.
const STRING_SAMPLE: &str = "\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}";
/// `\i[n]`: an icon is 20 px at the default size of the calibrated game
/// (≈ 0.78 em, plan WR, WR-1); one full-width space (1 em) is a safe estimate.
const ICON_SAMPLE: &str = "\u{3000}";

/// Leading `@N\n`: face / speaker graphic of the base system.
static RE_FACE_PREFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^@\d+\n").expect("RE_FACE_PREFIX must compile"));

/// One escape code at the start of the text: `\name[param]` or `\x`.
static RE_CODE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)^\\(?:
            r\[(?P<base>[^\[\],]+),[^\[\]]*\]
          | (?P<db>udb|cdb|sdb)\[\d+:\d+:\d+\]
          | (?P<name>sysS|cself|self|sys|space|sp|mx|my|ax|ay|font|[vVcCsSfFiImM])(?P<q>\?)?\[(?P<num>\d+)\]
          | -\[\d+\]
          | A[+\-]
          | (?P<ctrl>[EN!.^><])
        )",
    )
    .expect("RE_CODE must compile")
});

/// Split a message into its `@N\n` prefix (possibly empty) and body.
pub fn split_face_prefix(text: &str) -> (&str, &str) {
    match RE_FACE_PREFIX.find(text) {
        Some(m) => (m.as_str(), &text[m.end()..]),
        None => ("", text),
    }
}

/// A tokenized message body.
#[derive(Debug, Clone, PartialEq)]
pub struct WolfText {
    pub tokens: Vec<Token>,
    /// Holds a positioning or alignment code (`\ax`, `\ay`, `\mx`, `\my`,
    /// `\space`, `\-`, `<L>`, `<C>`, `<R>`): left as translated (R1).
    pub positioned: bool,
}

fn token(kind: TokenKind, raw: impl Into<String>) -> Token {
    Token {
        kind,
        raw: raw.into(),
    }
}

fn inserted(sample: &str, raw: &str) -> Token {
    token(
        TokenKind::Inserted {
            sample: sample.to_string(),
        },
        raw,
    )
}

/// Token for one matched code (`raw` = the whole match).
fn code_token(caps: &regex::Captures<'_>, raw: &str) -> (Token, bool) {
    if let Some(base) = caps.name("base") {
        // Ruby: drawn as its base text.
        return (inserted(base.as_str(), raw), false);
    }
    if caps.name("db").is_some() {
        return (inserted(STRING_SAMPLE, raw), false);
    }
    if let Some(ctrl) = caps.name("ctrl") {
        let c = ctrl.as_str().chars().next().unwrap_or('.');
        return (token(TokenKind::Control(c), raw), false);
    }
    let Some(name) = caps.name("name") else {
        // `\-[n]` (spacing) and `\A±` (anti-aliasing).
        let positioned = raw.starts_with("\\-");
        let kind = if positioned {
            TokenKind::Position
        } else {
            TokenKind::Control('A')
        };
        return (token(kind, raw), positioned);
    };
    let num: u32 = caps
        .name("num")
        .and_then(|m| m.as_str().parse().ok())
        .unwrap_or(0);
    match name.as_str() {
        "c" | "C" => (token(TokenKind::Color(Some(num)), raw), false),
        "f" | "F" => (token(TokenKind::FontSize(num), raw), false),
        "i" | "I" => (inserted(ICON_SAMPLE, raw), false),
        "s" | "S" | "sysS" => (inserted(STRING_SAMPLE, raw), false),
        // Common-event self variables 5–9 hold strings.
        "cself" if (5..=9).contains(&num) => (inserted(STRING_SAMPLE, raw), false),
        "v" | "V" | "self" | "cself" | "sys" => (inserted(NUMBER_SAMPLE, raw), false),
        "ax" | "ay" | "mx" | "my" | "space" => (token(TokenKind::Position, raw), true),
        // `\sp` (speed), `\font` (sub-font), `\m`: no width of their own.
        other => (token(TokenKind::Unknown(other.to_string()), raw), false),
    }
}

/// Split a message body (without its `@N\n` prefix) into tokens.
pub fn tokenize(body: &str) -> WolfText {
    let mut tokens: Vec<Token> = Vec::new();
    let mut positioned = false;
    let mut buffer = String::new();
    let flush = |buffer: &mut String, tokens: &mut Vec<Token>| {
        if !buffer.is_empty() {
            tokens.push(token(TokenKind::Text, std::mem::take(buffer)));
        }
    };
    let mut rest = body;
    while let Some(c) = rest.chars().next() {
        if c == '\n' {
            flush(&mut buffer, &mut tokens);
            tokens.push(token(TokenKind::Newline, "\n"));
            rest = &rest[1..];
            continue;
        }
        if c == '<'
            && ["<L>", "<C>", "<R>"]
                .iter()
                .any(|tag| rest.starts_with(tag))
        {
            flush(&mut buffer, &mut tokens);
            tokens.push(token(TokenKind::Position, &rest[..3]));
            positioned = true;
            rest = &rest[3..];
            continue;
        }
        if c == '\\' {
            flush(&mut buffer, &mut tokens);
            if rest.starts_with("\\\\") {
                tokens.push(token(TokenKind::Backslash, "\\\\"));
                rest = &rest[2..];
                continue;
            }
            if let Some(caps) = RE_CODE.captures(rest) {
                let raw = caps.get(0).map_or("", |m| m.as_str());
                let (code, moves) = code_token(&caps, raw);
                positioned |= moves;
                tokens.push(code);
                rest = &rest[raw.len()..];
                continue;
            }
            // Unknown code: the backslash and the next character, no width.
            let len = rest
                .char_indices()
                .nth(2)
                .map_or(rest.len(), |(index, _)| index);
            tokens.push(token(TokenKind::Unknown(String::new()), &rest[..len]));
            rest = &rest[len..];
            continue;
        }
        buffer.push(c);
        rest = &rest[c.len_utf8()..];
    }
    flush(&mut buffer, &mut tokens);
    WolfText { tokens, positioned }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::mv_mz::layout::text::raw_of;

    fn kinds(text: &str) -> Vec<TokenKind> {
        tokenize(text).tokens.into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn tokens_reassemble_into_the_original_text() {
        let text = r"\c[2]Gyoin\c[0] : \r[魔法,まほう]\v[3]\s[1]\i[4]\f[18]x\\y\E\!\A+ \zz";
        let parsed = tokenize(text);
        assert_eq!(raw_of(&parsed.tokens), text);
        assert!(!parsed.positioned);
    }

    #[test]
    fn codes_map_to_layout_kinds() {
        assert_eq!(
            kinds(r"\c[2]a\f[12]"),
            vec![
                TokenKind::Color(Some(2)),
                TokenKind::Text,
                TokenKind::FontSize(12)
            ]
        );
        assert_eq!(
            tokenize(r"\r[魔法,まほう]").tokens[0].kind,
            TokenKind::Inserted {
                sample: "魔法".into()
            }
        );
        assert_eq!(
            tokenize(r"\cself[7]").tokens[0].kind,
            TokenKind::Inserted {
                sample: STRING_SAMPLE.into()
            }
        );
        assert_eq!(
            tokenize(r"\v?[30]").tokens[0].kind,
            TokenKind::Inserted {
                sample: NUMBER_SAMPLE.into()
            }
        );
        assert_eq!(kinds("a\nb")[1], TokenKind::Newline);
    }

    #[test]
    fn positioning_codes_mark_the_text() {
        for text in [
            r"\ax[10]a",
            r"\my[3]a",
            r"\space[2]a",
            r"\-[1]a",
            "<C>a",
            "a<R>b",
        ] {
            assert!(tokenize(text).positioned, "{text}");
        }
        assert!(!tokenize("a<b>c").positioned);
    }

    #[test]
    fn face_prefix_is_split_off() {
        assert_eq!(split_face_prefix("@2\nHello"), ("@2\n", "Hello"));
        assert_eq!(split_face_prefix("Hello"), ("", "Hello"));
        assert_eq!(split_face_prefix("a@2\nb"), ("", "a@2\nb"));
    }
}
