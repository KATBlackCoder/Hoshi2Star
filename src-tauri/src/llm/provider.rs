//! LLM provider trait and `OllamaProvider` implementation.
//!
//! ## Design
//! The `LlmProvider` trait uses `async fn` (stable since Rust 1.75).
//! Because of object-safety limits, the pipeline is generic over `P: LlmProvider`
//! rather than using `dyn LlmProvider`.
//!
//! ## OllamaProvider
//! Wraps the Ollama REST API (`POST /api/chat`, `GET /api/tags`).
//! Automatically retries on network errors or timeouts up to `MAX_RETRIES` times.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use std::time::Duration;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationContext {
    pub source_lang: String,
    pub target_lang: String,
    /// (source_term, target_term) pairs injected into the system prompt.
    pub glossary_terms: Vec<(String, String)>,
    /// Project engine (`"wolf"`, `"mv_mz"`, `"vx_ace"`, `"bakin"`, …) — selects
    /// which placeholder patterns `Tokenizer::tokenize` uses (ADR-002).
    pub engine: String,
}

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Provider unavailable: {message}")]
    Unavailable { message: String },
    #[error("Translation failed after {attempts} attempt(s): {reason}")]
    TranslationFailed { attempts: u32, reason: String },
    #[error("Response format error: {0}")]
    ResponseFormat(String),
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

pub trait LlmProvider: Send + Sync {
    /// Translate a batch of text segments.
    ///
    /// `segments` contains pre-tokenized texts (placeholders already replaced
    /// by opaque tokens).  Returns one translation per input segment, in order.
    fn translate(
        &self,
        segments: Vec<String>,
        context: TranslationContext,
    ) -> impl std::future::Future<Output = Result<Vec<String>, LlmError>> + Send;

    /// Check that the provider is reachable and ready.
    fn health_check(&self) -> impl std::future::Future<Output = Result<(), LlmError>> + Send;

    /// Send a single system + user message and return the raw response string.
    ///
    /// Used for non-translation tasks such as glossary term extraction.
    fn chat(
        &self,
        system: &str,
        user: &str,
    ) -> impl std::future::Future<Output = Result<String, LlmError>> + Send;
}

// ---------------------------------------------------------------------------
// OllamaProvider
// ---------------------------------------------------------------------------

/// Default Ollama URL when none is provided.
pub const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";
/// Default model — consistent with the hoshi-trans reference pipeline.
pub const DEFAULT_OLLAMA_MODEL: &str = "qwen3:4b-instruct-2507-q4_K_M";
/// Default per-request timeout.
pub const DEFAULT_TIMEOUT_SECS: u64 = 120;

const MAX_RETRIES: u32 = 3;

pub struct OllamaProvider {
    base_url: String,
    model: String,
    client: Client,
}

impl OllamaProvider {
    pub fn new(base_url: &str, model: &str, timeout: Duration) -> Self {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .expect("reqwest client");
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            client,
        }
    }

    /// Convenience constructor using defaults.
    pub fn default_local() -> Self {
        Self::new(
            DEFAULT_OLLAMA_URL,
            DEFAULT_OLLAMA_MODEL,
            Duration::from_secs(DEFAULT_TIMEOUT_SECS),
        )
    }
}

// ---------------------------------------------------------------------------
// Ollama API types (minimal)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct OllamaMessage {
    role: &'static str,
    content: String,
}

#[derive(Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    message: OllamaMessageResponse,
}

#[derive(Deserialize)]
struct OllamaMessageResponse {
    content: String,
}

// ---------------------------------------------------------------------------
// LlmProvider impl for OllamaProvider
// ---------------------------------------------------------------------------

impl LlmProvider for OllamaProvider {
    async fn translate(
        &self,
        segments: Vec<String>,
        context: TranslationContext,
    ) -> Result<Vec<String>, LlmError> {
        if segments.is_empty() {
            return Ok(vec![]);
        }

        let glossary_hint = if context.glossary_terms.is_empty() {
            String::new()
        } else {
            let pairs: Vec<String> = context
                .glossary_terms
                .iter()
                .map(|(s, t)| format!("{s} → {t}"))
                .collect();
            format!("\nGlossary:\n{}", pairs.join("\n"))
        };

        // Escape embedded newlines so they don't break the line-counting protocol.
        // ⏎ (U+23CE) is the round-trip marker; restored after parsing.
        let escaped: Vec<String> = segments.iter().map(|s| s.replace('\n', "⏎")).collect();
        let numbered: Vec<String> = escaped
            .iter()
            .enumerate()
            .map(|(i, s)| format!("[{}] {}", i + 1, s))
            .collect();
        // /no_think désactive le bloc <think> de qwen3 (et des modèles compatibles).
        // Les autres modèles ignorent silencieusement cette directive.
        let prompt_body = format!("/no_think\n{}", numbered.join("\n"));

        let system_prompt = format!(
            "You are a professional game localisation assistant.\n\
             Translate the following numbered lines from {src} to {tgt}.\n\
             CRITICAL RULE: Every ⟦ph_N⟧ token in the source MUST appear identically in your \
             translation. Never translate, remove, paraphrase or modify any ⟦ph_N⟧ token. \
             If you cannot place a token naturally, keep it at the end of the translated sentence.\n\
             Output ONLY the translated lines, one per line, with the same numbering.{glossary}",
            src = context.source_lang,
            tgt = context.target_lang,
            glossary = glossary_hint,
        );

        let mut last_err = String::new();

        for attempt in 0..MAX_RETRIES {
            let request = OllamaChatRequest {
                model: self.model.clone(),
                messages: vec![
                    OllamaMessage {
                        role: "system",
                        content: system_prompt.clone(),
                    },
                    OllamaMessage {
                        role: "user",
                        content: prompt_body.clone(),
                    },
                ],
                stream: false,
            };

            let resp = self
                .client
                .post(format!("{}/api/chat", self.base_url))
                .json(&request)
                .send()
                .await;

            match resp {
                Err(e) if attempt + 1 < MAX_RETRIES => {
                    last_err = e.to_string();
                    continue;
                }
                Err(e) => return Err(LlmError::Http(e)),
                Ok(r) => {
                    let parsed: OllamaChatResponse = r
                        .json()
                        .await
                        .map_err(|e| LlmError::ResponseFormat(e.to_string()))?;

                    let lines = parse_numbered_response(&parsed.message.content, segments.len())?;
                    return Ok(lines.into_iter().map(|l| l.replace('⏎', "\n")).collect());
                }
            }
        }

        Err(LlmError::TranslationFailed {
            attempts: MAX_RETRIES,
            reason: last_err,
        })
    }

    async fn health_check(&self) -> Result<(), LlmError> {
        let resp = self
            .client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await
            .map_err(|e| LlmError::Unavailable {
                message: e.to_string(),
            })?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(LlmError::Unavailable {
                message: format!("HTTP {}", resp.status()),
            })
        }
    }

    async fn chat(&self, system: &str, user: &str) -> Result<String, LlmError> {
        let request = OllamaChatRequest {
            model: self.model.clone(),
            messages: vec![
                OllamaMessage {
                    role: "system",
                    content: system.to_string(),
                },
                OllamaMessage {
                    role: "user",
                    content: user.to_string(),
                },
            ],
            stream: false,
        };

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(LlmError::Http)?;

        let parsed: OllamaChatResponse = resp
            .json()
            .await
            .map_err(|e| LlmError::ResponseFormat(e.to_string()))?;

        Ok(parsed.message.content)
    }
}

// ---------------------------------------------------------------------------
// Response parser
// ---------------------------------------------------------------------------

/// Regex matching qwen3-style `<think>…</think>` blocks (possibly multiline).
static THINK_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?s)<think>.*?</think>").expect("valid regex"));

/// Remove all `<think>…</think>` blocks from an LLM response.
///
/// qwen3 (and other reasoning models) prepend a thinking section before the
/// actual answer.  The parser must not see those lines.
fn strip_think_blocks(raw: &str) -> String {
    THINK_RE.replace_all(raw, "").into_owned()
}

/// Parse numbered response lines into a plain `Vec<String>`.
///
/// Accepts two formats:
/// - `[1] text`  (preferred — what the system prompt requests)
/// - `1. text`   (fallback — some models ignore the bracket format)
///
/// Returns an error if any line number is missing after all lines are parsed.
fn parse_numbered_response(raw: &str, expected: usize) -> Result<Vec<String>, LlmError> {
    let stripped = strip_think_blocks(raw);
    let raw = stripped.trim();

    let mut out: Vec<Option<String>> = vec![None; expected];

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Pattern 1: [1] text
        if let Some(rest) = line.strip_prefix('[') {
            if let Some(bracket) = rest.find(']') {
                if let Ok(idx) = rest[..bracket].parse::<usize>() {
                    if idx >= 1 && idx <= expected {
                        out[idx - 1] = Some(rest[bracket + 1..].trim().to_string());
                    }
                }
            }
        }
        // Pattern 2: 1. text (sans crochets — fallback si le modèle ignore le format)
        else if let Some(dot) = line.find('.') {
            let num_part = &line[..dot];
            if !num_part.is_empty() && num_part.chars().all(|c| c.is_ascii_digit()) {
                if let Ok(idx) = num_part.parse::<usize>() {
                    let text = line[dot + 1..].trim();
                    if idx >= 1 && idx <= expected && !text.is_empty() {
                        out[idx - 1] = Some(text.to_string());
                    }
                }
            }
        }
    }

    // Fall back: if none matched (model ignored numbering), split by line count
    if out.iter().all(|o| o.is_none()) {
        let plain: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
        if plain.len() == expected {
            return Ok(plain.iter().map(|s| s.trim().to_string()).collect());
        }
        return Err(LlmError::ResponseFormat(format!(
            "expected {expected} lines, got {}",
            plain.len()
        )));
    }

    out.into_iter()
        .enumerate()
        .map(|(i, opt)| {
            opt.ok_or_else(|| {
                LlmError::ResponseFormat(format!("missing translation for line {}", i + 1))
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests (HTTP-level via httpmock)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use serde_json::json;

    fn make_provider(server: &MockServer) -> OllamaProvider {
        OllamaProvider::new(&server.base_url(), "test-model", Duration::from_secs(5))
    }

    fn ctx() -> TranslationContext {
        TranslationContext {
            source_lang: "ja".to_string(),
            target_lang: "en".to_string(),
            glossary_terms: vec![],
            engine: "mv_mz".to_string(),
        }
    }

    #[tokio::test]
    async fn test_health_check_ok() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/tags");
            then.status(200).json_body(json!({ "models": [] }));
        });
        let provider = make_provider(&server);
        assert!(provider.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_health_check_fail() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/tags");
            then.status(503);
        });
        let provider = make_provider(&server);
        assert!(matches!(
            provider.health_check().await,
            Err(LlmError::Unavailable { .. })
        ));
    }

    #[tokio::test]
    async fn test_translate_basic() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/api/chat");
            then.status(200).json_body(json!({
                "message": { "role": "assistant", "content": "[1] Hero" }
            }));
        });
        let provider = make_provider(&server);
        let result = provider
            .translate(vec!["主人公".to_string()], ctx())
            .await
            .expect("translate");
        assert_eq!(result, vec!["Hero"]);
    }

    #[tokio::test]
    async fn test_translate_empty_returns_empty() {
        let server = MockServer::start();
        let provider = make_provider(&server);
        let result = provider
            .translate(vec![], ctx())
            .await
            .expect("translate empty");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_numbered_response_basic() {
        let raw = "[1] Hello\n[2] World";
        let result = parse_numbered_response(raw, 2).unwrap();
        assert_eq!(result, vec!["Hello", "World"]);
    }

    #[test]
    fn test_parse_numbered_response_fallback() {
        // Model ignored numbering but returned correct count
        let raw = "Hello\nWorld";
        let result = parse_numbered_response(raw, 2).unwrap();
        assert_eq!(result, vec!["Hello", "World"]);
    }

    #[test]
    fn test_parse_think_block_stripped() {
        // qwen3 thinking mode: block is ignored, real answer is parsed
        let raw = "<think>\nraisonnement interne\n</think>\n[1] Hero";
        let result = parse_numbered_response(raw, 1).unwrap();
        assert_eq!(result, vec!["Hero"]);
    }

    #[test]
    fn test_parse_dot_format() {
        // Model returns "1. text" instead of "[1] text"
        let raw = "1. Hero\n2. Sword";
        let result = parse_numbered_response(raw, 2).unwrap();
        assert_eq!(result, vec!["Hero", "Sword"]);
    }

    #[tokio::test]
    async fn test_system_prompt_contains_placeholder_instruction() {
        let server = MockServer::start();
        // The mock only matches if the request body contains "CRITICAL RULE".
        // If the system prompt omits it, the mock won't fire and the call fails.
        let m = server.mock(|when, then| {
            when.method(POST)
                .path("/api/chat")
                .body_contains("CRITICAL RULE");
            then.status(200).json_body(json!({
                "message": { "role": "assistant", "content": "[1] Hero" }
            }));
        });
        let provider = make_provider(&server);
        let result = provider
            .translate(vec!["主人公".to_string()], ctx())
            .await
            .expect("translate must succeed");
        m.assert(); // verifies the mock was hit exactly once
        assert_eq!(result, vec!["Hero"]);
    }

    #[tokio::test]
    async fn test_translate_multiline_description_preserved() {
        // Source has an embedded newline (RPG Maker item description pattern).
        // The LLM sees "⏎" in place of the newline and echoes it back.
        // The pipeline must restore "\n" in the final translation.
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/api/chat");
            then.status(200).json_body(json!({
                "message": {
                    "role": "assistant",
                    "content": "[1] A rare injectable ampoule not yet widely available.⏎Fully restores HP and MP."
                }
            }));
        });
        let provider = make_provider(&server);
        let source =
            "まだ表に出回っていない、貴重な注射式アンプル。\nHPとMPを全回復する".to_string();
        let result = provider
            .translate(vec![source], ctx())
            .await
            .expect("translate");
        assert_eq!(
            result,
            vec!["A rare injectable ampoule not yet widely available.\nFully restores HP and MP."]
        );
    }

    #[test]
    fn test_parse_multiline_segment_via_newline_marker() {
        // parse_numbered_response itself is line-based; the ⏎ marker stays opaque
        // through it — the translate() wrapper restores it after.
        let raw = "[1] First line⏎Second line\n[2] Other";
        let result = parse_numbered_response(raw, 2).unwrap();
        assert_eq!(result[0], "First line⏎Second line");
        assert_eq!(result[1], "Other");
    }
}
