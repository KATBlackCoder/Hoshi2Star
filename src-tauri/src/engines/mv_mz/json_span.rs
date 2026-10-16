//! Span-preserving JSON reader/editor for RPG Maker MV/MZ plugin data.
//!
//! `serde_json` round-trips values, not bytes. Plugin parameters are stored as
//! JSON text nested inside JSON strings (struct/array parameters are
//! `JSON.stringify`'d by the editor, recursively), so a value-level round trip
//! would re-encode every sibling of an edited leaf. This module instead records
//! the byte span of every node and edits by *surgery*: only the string literal
//! of the edited leaf — and the literals of the strings that transitively
//! contain it — are rewritten. Every other byte is copied verbatim.
//!
//! Paths are RFC 6901 tokens resolved against the node type: object → member
//! name, array → index. When a path continues past a string node, that string
//! is parsed as JSON text and the walk continues inside it ("JSON-in-string").

use std::ops::Range;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum JsonSpanError {
    #[error("invalid JSON at byte {offset}: {reason}")]
    Syntax { offset: usize, reason: &'static str },
    #[error("path not found: {0}")]
    PathNotFound(String),
    #[error("path does not end at a string: {0}")]
    NotAString(String),
    #[error("conflicting edits for path: {0}")]
    ConflictingEdits(String),
}

/// One parsed JSON node with its byte span in the text it was parsed from.
#[derive(Debug, Clone, PartialEq)]
pub enum SpanNode {
    Object {
        members: Vec<(String, SpanNode)>,
        span: Range<usize>,
    },
    Array {
        items: Vec<SpanNode>,
        span: Range<usize>,
    },
    String {
        value: String,
        span: Range<usize>,
    },
    /// Number, `true`, `false` or `null` — never edited, only skipped.
    Scalar {
        span: Range<usize>,
    },
}

impl SpanNode {
    pub fn span(&self) -> Range<usize> {
        match self {
            Self::Object { span, .. }
            | Self::Array { span, .. }
            | Self::String { span, .. }
            | Self::Scalar { span } => span.clone(),
        }
    }

    /// Member lookup with `JSON.parse` semantics: the last duplicate wins.
    pub fn member(&self, key: &str) -> Option<&SpanNode> {
        match self {
            Self::Object { members, .. } => members
                .iter()
                .rev()
                .find(|(name, _)| name == key)
                .map(|(_, node)| node),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String { value, .. } => Some(value),
            _ => None,
        }
    }
}

/// Parse `text`, which must contain exactly one JSON value (surrounding
/// whitespace allowed).
pub fn parse(text: &str) -> Result<SpanNode, JsonSpanError> {
    let mut parser = Parser::new(text, 0);
    parser.skip_ws();
    let node = parser.value()?;
    parser.skip_ws();
    if parser.pos != text.len() {
        return Err(parser.error("trailing characters after JSON value"));
    }
    Ok(node)
}

/// Parse one JSON value starting at byte `start` (leading whitespace allowed);
/// returns the node and the byte offset just past it.
pub fn parse_at(text: &str, start: usize) -> Result<(SpanNode, usize), JsonSpanError> {
    let mut parser = Parser::new(text, start);
    parser.skip_ws();
    let node = parser.value()?;
    Ok((node, parser.pos))
}

/// Encode `value` as a JSON string literal exactly like JavaScript's
/// `JSON.stringify` (which is what the RPG Maker editor uses): escapes `"`,
/// `\`, and U+0000–U+001F (`\b \t \n \f \r`, others as lowercase `\u00xx`);
/// everything else, including non-ASCII, is written raw.
pub fn encode_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{0C}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Split an RFC 6901 pointer (`/a/b~1c`) into unescaped tokens.
pub fn pointer_tokens(pointer: &str) -> Result<Vec<String>, JsonSpanError> {
    if pointer.is_empty() {
        return Ok(Vec::new());
    }
    let Some(rest) = pointer.strip_prefix('/') else {
        return Err(JsonSpanError::PathNotFound(pointer.to_string()));
    };
    Ok(rest
        .split('/')
        .map(|token| token.replace("~1", "/").replace("~0", "~"))
        .collect())
}

/// Escape one token for an RFC 6901 pointer.
pub fn escape_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

/// Resolve `path` inside `text` (descending into JSON-in-string as needed) and
/// return the string value it ends at.
pub fn get_string(text: &str, path: &[String]) -> Result<String, JsonSpanError> {
    let root = parse(text)?;
    get_in(&root, path, path)
}

fn get_in(node: &SpanNode, path: &[String], full: &[String]) -> Result<String, JsonSpanError> {
    let Some((head, tail)) = path.split_first() else {
        return node
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| JsonSpanError::NotAString(display_path(full)));
    };
    match node {
        SpanNode::String { value, .. } => {
            let inner = parse(value)?;
            get_in(&inner, path, full)
        }
        _ => {
            let child =
                child(node, head).ok_or_else(|| JsonSpanError::PathNotFound(display_path(full)))?;
            get_in(child, tail, full)
        }
    }
}

fn child<'a>(node: &'a SpanNode, token: &str) -> Option<&'a SpanNode> {
    match node {
        SpanNode::Object { .. } => node.member(token),
        SpanNode::Array { items, .. } => token.parse::<usize>().ok().and_then(|i| items.get(i)),
        _ => None,
    }
}

/// Apply leaf replacements to `text` by span surgery and return the new text.
///
/// Each edit is `(path, new_value)`; the path must end at a string. Bytes not
/// on the path of an edit are preserved exactly.
pub fn apply_edits(text: &str, edits: &[(Vec<String>, String)]) -> Result<String, JsonSpanError> {
    let root = parse(text)?;
    let borrowed: Vec<(&[String], &str)> = edits
        .iter()
        .map(|(path, value)| (path.as_slice(), value.as_str()))
        .collect();
    edit_node(text, &root, &borrowed, &[])
}

/// One pending leaf edit: remaining path (relative to the current node) and
/// the new string value.
type PendingEdit<'a> = (&'a [String], &'a str);

/// Rewrite `text` (whose parsed root is `node`) applying `edits`, with paths
/// relative to `node`. `prefix` is only used for error messages.
fn edit_node(
    text: &str,
    node: &SpanNode,
    edits: &[PendingEdit<'_>],
    prefix: &[String],
) -> Result<String, JsonSpanError> {
    let mut replacements = collect_replacements(node, edits, prefix)?;
    replacements.sort_by_key(|(span, _)| span.start);
    for pair in replacements.windows(2) {
        if pair[0].0.end > pair[1].0.start {
            return Err(JsonSpanError::ConflictingEdits(display_path(prefix)));
        }
    }
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for (span, replacement) in replacements {
        out.push_str(&text[cursor..span.start]);
        out.push_str(&replacement);
        cursor = span.end;
    }
    out.push_str(&text[cursor..]);
    Ok(out)
}

fn collect_replacements(
    node: &SpanNode,
    edits: &[PendingEdit<'_>],
    prefix: &[String],
) -> Result<Vec<(Range<usize>, String)>, JsonSpanError> {
    if edits.is_empty() {
        return Ok(Vec::new());
    }
    if let SpanNode::String { value, span } = node {
        let (leaf, nested): (Vec<_>, Vec<_>) = edits.iter().partition(|(path, _)| path.is_empty());
        if !leaf.is_empty() {
            let distinct = leaf
                .iter()
                .map(|(_, v)| *v)
                .collect::<std::collections::HashSet<_>>();
            if !nested.is_empty() || distinct.len() > 1 {
                return Err(JsonSpanError::ConflictingEdits(display_path(prefix)));
            }
            return Ok(vec![(span.clone(), encode_string(leaf[0].1))]);
        }
        // Path continues inside JSON text stored in this string.
        let inner = parse(value)?;
        let rewritten = edit_node(value, &inner, &nested, prefix)?;
        return Ok(vec![(span.clone(), encode_string(&rewritten))]);
    }

    let mut out = Vec::new();
    let mut groups: Vec<(&str, Vec<PendingEdit<'_>>)> = Vec::new();
    for (path, value) in edits {
        let Some((head, tail)) = path.split_first() else {
            return Err(JsonSpanError::NotAString(display_path(prefix)));
        };
        match groups.iter_mut().find(|(name, _)| *name == head.as_str()) {
            Some((_, group)) => group.push((tail, value)),
            None => groups.push((head.as_str(), vec![(tail, value)])),
        }
    }
    for (head, group) in groups {
        let mut next_prefix = prefix.to_vec();
        next_prefix.push(head.to_string());
        let target = child(node, head)
            .ok_or_else(|| JsonSpanError::PathNotFound(display_path(&next_prefix)))?;
        out.extend(collect_replacements(target, &group, &next_prefix)?);
    }
    Ok(out)
}

fn display_path(path: &[String]) -> String {
    path.iter()
        .map(|t| format!("/{}", escape_token(t)))
        .collect()
}

/// Visit every string leaf reachable from `node`, descending into strings that
/// contain a JSON object or array. `visit(path, value)` receives the full path.
pub fn visit_string_leaves(node: &SpanNode, visit: &mut dyn FnMut(&[String], &str)) {
    let mut path = Vec::new();
    walk_leaves(node, &mut path, visit);
}

fn walk_leaves(node: &SpanNode, path: &mut Vec<String>, visit: &mut dyn FnMut(&[String], &str)) {
    match node {
        SpanNode::Object { members, .. } => {
            for (name, child) in members {
                path.push(name.clone());
                walk_leaves(child, path, visit);
                path.pop();
            }
        }
        SpanNode::Array { items, .. } => {
            for (index, child) in items.iter().enumerate() {
                path.push(index.to_string());
                walk_leaves(child, path, visit);
                path.pop();
            }
        }
        SpanNode::String { value, .. } => {
            let trimmed = value.trim_start();
            if trimmed.starts_with('{') || trimmed.starts_with('[') {
                if let Ok(inner) = parse(value) {
                    walk_leaves(&inner, path, visit);
                    return;
                }
            }
            visit(path, value);
        }
        SpanNode::Scalar { .. } => {}
    }
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

/// Deepest object/array nesting accepted in one JSON text (the same limit as
/// `serde_json`): `plugins.js` comes from the game folder, and unbounded
/// nesting would overflow the stack. JSON-in-string levels need no limit of
/// their own: each level doubles the escaping, so input size bounds them.
const MAX_DEPTH: usize = 128;

struct Parser<'a> {
    text: &'a str,
    bytes: &'a [u8],
    pos: usize,
    depth: usize,
}

impl<'a> Parser<'a> {
    fn new(text: &'a str, start: usize) -> Self {
        Self {
            text,
            bytes: text.as_bytes(),
            pos: start,
            depth: 0,
        }
    }

    fn nested(
        &mut self,
        container: fn(&mut Self) -> Result<SpanNode, JsonSpanError>,
    ) -> Result<SpanNode, JsonSpanError> {
        if self.depth == MAX_DEPTH {
            return Err(self.error("nesting too deep"));
        }
        self.depth += 1;
        let node = container(self);
        self.depth -= 1;
        node
    }

    fn error(&self, reason: &'static str) -> JsonSpanError {
        JsonSpanError::Syntax {
            offset: self.pos,
            reason,
        }
    }

    fn skip_ws(&mut self) {
        while let Some(b' ' | b'\t' | b'\n' | b'\r') = self.bytes.get(self.pos) {
            self.pos += 1;
        }
    }

    fn value(&mut self) -> Result<SpanNode, JsonSpanError> {
        match self.bytes.get(self.pos) {
            Some(b'{') => self.nested(Self::object),
            Some(b'[') => self.nested(Self::array),
            Some(b'"') => {
                let start = self.pos;
                let value = self.string()?;
                Ok(SpanNode::String {
                    value,
                    span: start..self.pos,
                })
            }
            Some(b't') => self.literal("true"),
            Some(b'f') => self.literal("false"),
            Some(b'n') => self.literal("null"),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => Err(self.error("unexpected character")),
            None => Err(self.error("unexpected end of input")),
        }
    }

    fn object(&mut self) -> Result<SpanNode, JsonSpanError> {
        let start = self.pos;
        self.pos += 1;
        let mut members = Vec::new();
        self.skip_ws();
        if self.bytes.get(self.pos) == Some(&b'}') {
            self.pos += 1;
            return Ok(SpanNode::Object {
                members,
                span: start..self.pos,
            });
        }
        loop {
            self.skip_ws();
            if self.bytes.get(self.pos) != Some(&b'"') {
                return Err(self.error("expected object key"));
            }
            let key = self.string()?;
            self.skip_ws();
            if self.bytes.get(self.pos) != Some(&b':') {
                return Err(self.error("expected ':'"));
            }
            self.pos += 1;
            self.skip_ws();
            let value = self.value()?;
            members.push((key, value));
            self.skip_ws();
            match self.bytes.get(self.pos) {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(SpanNode::Object {
                        members,
                        span: start..self.pos,
                    });
                }
                _ => return Err(self.error("expected ',' or '}'")),
            }
        }
    }

    fn array(&mut self) -> Result<SpanNode, JsonSpanError> {
        let start = self.pos;
        self.pos += 1;
        let mut items = Vec::new();
        self.skip_ws();
        if self.bytes.get(self.pos) == Some(&b']') {
            self.pos += 1;
            return Ok(SpanNode::Array {
                items,
                span: start..self.pos,
            });
        }
        loop {
            self.skip_ws();
            items.push(self.value()?);
            self.skip_ws();
            match self.bytes.get(self.pos) {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(SpanNode::Array {
                        items,
                        span: start..self.pos,
                    });
                }
                _ => return Err(self.error("expected ',' or ']'")),
            }
        }
    }

    fn literal(&mut self, word: &'static str) -> Result<SpanNode, JsonSpanError> {
        let start = self.pos;
        if self.text[self.pos..].starts_with(word) {
            self.pos += word.len();
            Ok(SpanNode::Scalar {
                span: start..self.pos,
            })
        } else {
            Err(self.error("invalid literal"))
        }
    }

    fn number(&mut self) -> Result<SpanNode, JsonSpanError> {
        let start = self.pos;
        if self.bytes.get(self.pos) == Some(&b'-') {
            self.pos += 1;
        }
        match self.bytes.get(self.pos) {
            Some(b'0') => self.pos += 1,
            Some(b'1'..=b'9') => self.digits(),
            _ => return Err(self.error("invalid number")),
        }
        if self.bytes.get(self.pos) == Some(&b'.') {
            self.pos += 1;
            if !matches!(self.bytes.get(self.pos), Some(b'0'..=b'9')) {
                return Err(self.error("invalid fraction"));
            }
            self.digits();
        }
        if let Some(b'e' | b'E') = self.bytes.get(self.pos) {
            self.pos += 1;
            if let Some(b'+' | b'-') = self.bytes.get(self.pos) {
                self.pos += 1;
            }
            if !matches!(self.bytes.get(self.pos), Some(b'0'..=b'9')) {
                return Err(self.error("invalid exponent"));
            }
            self.digits();
        }
        Ok(SpanNode::Scalar {
            span: start..self.pos,
        })
    }

    fn digits(&mut self) {
        while let Some(b'0'..=b'9') = self.bytes.get(self.pos) {
            self.pos += 1;
        }
    }

    /// Decode a string literal starting at the opening quote.
    fn string(&mut self) -> Result<String, JsonSpanError> {
        self.pos += 1; // opening quote
        let mut out = String::new();
        loop {
            let run_start = self.pos;
            while let Some(&b) = self.bytes.get(self.pos) {
                if b == b'"' || b == b'\\' || b < 0x20 {
                    break;
                }
                self.pos += 1;
            }
            out.push_str(&self.text[run_start..self.pos]);
            match self.bytes.get(self.pos) {
                Some(b'"') => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.pos += 1;
                    let escaped = *self
                        .bytes
                        .get(self.pos)
                        .ok_or_else(|| self.error("unterminated escape"))?;
                    self.pos += 1;
                    match escaped {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{08}'),
                        b'f' => out.push('\u{0C}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => out.push(self.unicode_escape()?),
                        _ => return Err(self.error("invalid escape")),
                    }
                }
                Some(_) => return Err(self.error("control character in string")),
                None => return Err(self.error("unterminated string")),
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, JsonSpanError> {
        let digits = self
            .text
            .get(self.pos..self.pos + 4)
            .ok_or_else(|| self.error("truncated \\u escape"))?;
        let value =
            u32::from_str_radix(digits, 16).map_err(|_| self.error("invalid \\u escape"))?;
        self.pos += 4;
        Ok(value)
    }

    fn unicode_escape(&mut self) -> Result<char, JsonSpanError> {
        let first = self.hex4()?;
        let code = match first {
            0xD800..=0xDBFF => {
                if !self.text[self.pos..].starts_with("\\u") {
                    return Err(self.error("lone leading surrogate"));
                }
                self.pos += 2;
                let second = self.hex4()?;
                if !(0xDC00..=0xDFFF).contains(&second) {
                    return Err(self.error("invalid trailing surrogate"));
                }
                0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00)
            }
            0xDC00..=0xDFFF => return Err(self.error("lone trailing surrogate")),
            other => other,
        };
        char::from_u32(code).ok_or_else(|| self.error("invalid code point"))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn path(pointer: &str) -> Vec<String> {
        pointer_tokens(pointer).unwrap()
    }

    #[test]
    fn encode_string_matches_json_stringify() {
        assert_eq!(encode_string("a\"b\\c"), r#""a\"b\\c""#);
        assert_eq!(encode_string("x\ny\tz\r"), r#""x\ny\tz\r""#);
        assert_eq!(encode_string("\u{1f}\u{08}\u{0c}"), r#""\u001f\b\f""#);
        assert_eq!(encode_string("エネミー図鑑/é"), "\"エネミー図鑑/é\"");
    }

    #[test]
    fn encode_then_parse_round_trips_value() {
        for value in [
            "",
            "plain",
            "quote\"back\\slash",
            "ctrl\u{1}\u{7f}",
            "改行\n含む",
        ] {
            let node = parse(&encode_string(value)).unwrap();
            assert_eq!(node.as_str(), Some(value));
        }
    }

    #[test]
    fn decodes_escapes_and_surrogate_pairs() {
        let node = parse(r#""\u3042\/\ud83d\ude00""#).unwrap();
        assert_eq!(node.as_str(), Some("あ/😀"));
    }

    #[test]
    fn lone_surrogates_are_rejected() {
        assert!(parse(r#""\ud800""#).is_err());
        assert!(parse(r#""\udc00x""#).is_err());
    }

    #[test]
    fn nesting_is_bounded_instead_of_overflowing_the_stack() {
        let nested = |depth: usize| format!("{}{}", "[".repeat(depth), "]".repeat(depth));
        assert!(parse(&nested(MAX_DEPTH)).is_ok());
        assert!(matches!(
            parse(&nested(MAX_DEPTH + 1)),
            Err(JsonSpanError::Syntax {
                reason: "nesting too deep",
                ..
            })
        ));
        // Far past the limit: an error, not a crash.
        assert!(parse(&nested(1_000_000)).is_err());
    }

    #[test]
    fn rejects_trailing_commas_and_garbage() {
        assert!(parse("[1,]").is_err());
        assert!(parse("{\"a\":1,}").is_err());
        assert!(parse("[1] x").is_err());
        assert!(parse("01").is_err());
    }

    #[test]
    fn spans_cover_the_exact_literal() {
        let text = r#"{ "a" : [ 1, "x\"y" ] }"#;
        let root = parse(text).unwrap();
        let item = &match root.member("a").unwrap() {
            SpanNode::Array { items, .. } => items.clone(),
            _ => unreachable!(),
        }[1];
        assert_eq!(&text[item.span()], r#""x\"y""#);
    }

    #[test]
    fn duplicate_keys_resolve_to_last_like_json_parse() {
        let root = parse(r#"{"k":"first","k":"last"}"#).unwrap();
        assert_eq!(root.member("k").and_then(SpanNode::as_str), Some("last"));
    }

    #[test]
    fn get_descends_into_json_in_string_levels() {
        // Two nesting levels, exactly as the MZ editor stores struct arrays.
        let inner = r#"{"PageCategoryName":"基本情報","Img":""}"#;
        let array = serde_json::to_string(&vec![inner]).unwrap();
        let text = format!(r#"{{"PageSetting":{}}}"#, encode_string(&array));
        assert_eq!(
            get_string(&text, &path("/PageSetting/0/PageCategoryName")).unwrap(),
            "基本情報"
        );
    }

    #[test]
    fn edit_rewrites_only_the_leaf_and_its_containers() {
        let inner0 = r#"{"PageCategoryName":"基本情報","Img":"a\\b"}"#;
        let inner1 = r#"{"PageCategoryName":"説明","Img":""}"#;
        let array = serde_json::to_string(&vec![inner0, inner1]).unwrap();
        let text = format!(
            r#"{{"Other":"keep \"me\"","PageSetting":{},"Tail":1}}"#,
            encode_string(&array)
        );

        let edited = apply_edits(
            &text,
            &[(
                path("/PageSetting/1/PageCategoryName"),
                "Description".into(),
            )],
        )
        .unwrap();

        assert_eq!(
            get_string(&edited, &path("/PageSetting/1/PageCategoryName")).unwrap(),
            "Description"
        );
        assert_eq!(
            get_string(&edited, &path("/PageSetting/0/PageCategoryName")).unwrap(),
            "基本情報"
        );
        assert_eq!(
            get_string(&edited, &path("/PageSetting/0/Img")).unwrap(),
            "a\\b"
        );
        // Bytes outside the edited parameter are untouched.
        assert!(edited.starts_with(r#"{"Other":"keep \"me\"","PageSetting":"#));
        assert!(edited.ends_with(r#","Tail":1}"#));
        // The sibling element's literal inside the decoded array is byte-identical.
        let decoded: Vec<String> =
            serde_json::from_str(&get_raw_param(&edited, "PageSetting")).unwrap();
        assert_eq!(decoded[0], inner0);
    }

    fn get_raw_param(text: &str, key: &str) -> String {
        parse(text)
            .unwrap()
            .member(key)
            .and_then(SpanNode::as_str)
            .unwrap()
            .to_string()
    }

    #[test]
    fn no_edits_is_byte_identical() {
        let text = "{ \"a\" : \"\\u3042\" ,\r\n \"b\":[1.50, -0, 2e+3] }";
        assert_eq!(apply_edits(text, &[]).unwrap(), text);
    }

    #[test]
    fn multiple_edits_in_one_nested_string_are_composed() {
        let inner = r#"[{"n":"一"},{"n":"二"}]"#;
        let text = format!(r#"{{"L":{}}}"#, encode_string(inner));
        let edited = apply_edits(
            &text,
            &[
                (path("/L/0/n"), "One".into()),
                (path("/L/1/n"), "Two".into()),
            ],
        )
        .unwrap();
        assert_eq!(get_string(&edited, &path("/L/0/n")).unwrap(), "One");
        assert_eq!(get_string(&edited, &path("/L/1/n")).unwrap(), "Two");
    }

    #[test]
    fn conflicting_edits_are_rejected() {
        let text = r#"{"a":"x"}"#;
        let result = apply_edits(text, &[(path("/a"), "1".into()), (path("/a"), "2".into())]);
        assert!(matches!(result, Err(JsonSpanError::ConflictingEdits(_))));
    }

    #[test]
    fn missing_paths_and_non_string_targets_error() {
        let text = r#"{"a":1,"b":"x"}"#;
        assert!(matches!(
            apply_edits(text, &[(path("/zzz"), "v".into())]),
            Err(JsonSpanError::PathNotFound(_))
        ));
        assert!(matches!(
            apply_edits(text, &[(path("/a"), "v".into())]),
            Err(JsonSpanError::NotAString(_))
        ));
    }

    #[test]
    fn visit_string_leaves_descends_nested_json() {
        let inner = r#"{"name":"毒","id":"3"}"#;
        let text = format!(
            r#"{{"list":{},"plain":"x"}}"#,
            encode_string(&format!("[{}]", encode_string(inner)))
        );
        let root = parse(&text).unwrap();
        let mut seen = Vec::new();
        visit_string_leaves(&root, &mut |p, v| seen.push((p.join("/"), v.to_string())));
        assert!(seen.contains(&("list/0/name".to_string(), "毒".to_string())));
        assert!(seen.contains(&("list/0/id".to_string(), "3".to_string())));
        assert!(seen.contains(&("plain".to_string(), "x".to_string())));
    }

    #[test]
    fn pointer_token_escaping_round_trips() {
        assert_eq!(
            pointer_tokens("/nuun~1NUUN_EnemyBook/a~0b").unwrap(),
            vec!["nuun/NUUN_EnemyBook", "a~b"]
        );
        assert_eq!(escape_token("nuun/NUUN~x"), "nuun~1NUUN~0x");
    }
}
