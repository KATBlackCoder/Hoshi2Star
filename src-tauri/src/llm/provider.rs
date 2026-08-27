//! LLM provider trait and OpenAI-compatible implementation.
//!
//! ## Design
//! The `LlmProvider` trait uses `async fn` (stable since Rust 1.75).
//! Because of object-safety limits, the pipeline is generic over `P: LlmProvider`
//! rather than using `dyn LlmProvider`.
//!
//! ## OpenAiCompatibleProvider
//! Wraps the common OpenAI-compatible API exposed by Ollama, LM Studio,
//! Hugging Face Inference Providers and many cloud services.
//! Automatically retries on network errors or timeouts up to `MAX_RETRIES` times.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};
use thiserror::Error;

use crate::core::terminology::types::{Enforcement, PartOfSpeech};
use crate::llm::context::SegmentPromptContext;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Controls whether the translation pipeline may ask the current engine for
/// prompt context. Keeping this separate from `engine` preserves tokenizer and
/// placeholder behavior during context-free comparisons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptContextPolicy {
    Disabled,
    EngineOwned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationContext {
    pub source_lang: String,
    pub target_lang: String,
    /// Terms resolved for this exact provider request from segment occurrences.
    pub terminology_hints: Vec<TerminologyHint>,
    /// Project engine (`"wolf"`, `"mv_mz"`, `"vx_ace"`, `"bakin"`, …) — selects
    /// which placeholder patterns `Tokenizer::tokenize` uses (ADR-002).
    pub engine: String,
    /// Number of segments sent to the provider per LLM call. Clamped to
    /// `[1, 100]` by the pipeline before being passed to `batch::group_segments`.
    pub batch_size: usize,
    /// Sequential pause between completed batches. Zero keeps maximum throughput.
    pub batch_delay_ms: u64,
    /// Whether the pipeline should invoke the selected engine's context
    /// adapter. Normal translations use `EngineOwned`; isolated A/B baselines
    /// use `Disabled` while retaining the real engine for tokenization.
    pub prompt_context_policy: PromptContextPolicy,
    /// Per-input provider-neutral metadata, aligned with the current request.
    /// Empty/`None` preserves the legacy context-free prompt.
    #[serde(default)]
    pub segment_contexts: Vec<Option<SegmentPromptContext>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminologyHint {
    pub source: String,
    pub target: String,
    pub semantic_type: String,
    pub part_of_speech: PartOfSpeech,
    pub enforcement: Enforcement,
    #[serde(default)]
    pub accepted_targets: Vec<String>,
}

/// Render the complete terminology fragment used by translation prompts.
/// Keeping this pure and shared lets the resolver enforce its budget against
/// the exact text that will be sent, including instructions and variants.
pub(crate) fn terminology_prompt_fragment(hints: &[TerminologyHint]) -> String {
    if hints.is_empty() {
        return String::new();
    }
    let pairs = hints
        .iter()
        .map(|hint| {
            let variants = if hint.accepted_targets.is_empty() {
                String::new()
            } else {
                format!("; variants={}", hint.accepted_targets.join("|"))
            };
            format!(
                "[{};{};{}] {}={}{variants}",
                hint.enforcement, hint.semantic_type, hint.part_of_speech, hint.source, hint.target,
            )
        })
        .collect::<Vec<_>>();
    format!(
        "\nTerminology (never insert absent terms; required=must use when sense matches; \
         preferred=consistent; contextual=guidance):\n{}",
        pairs.join("\n")
    )
}

/// Logical operation measured by one provider call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderTask {
    Translate,
    Chat,
}

/// Resource data reported by an OpenAI-compatible endpoint plus local timing.
/// Token counts remain optional because not every compatible API returns them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCallMetrics {
    pub task: ProviderTask,
    pub model: String,
    pub input_units: usize,
    pub prompt_chars: usize,
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub duration_ms: u64,
    pub attempts: u32,
    pub success: bool,
    #[serde(default)]
    pub terminology_hints: usize,
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
    /// Used for non-translation tasks such as terminology candidate translation.
    fn chat(
        &self,
        system: &str,
        user: &str,
    ) -> impl std::future::Future<Output = Result<String, LlmError>> + Send;

    /// Drain metrics recorded since the previous call. Providers without
    /// instrumentation keep the empty default, which also simplifies mocks.
    fn drain_metrics(&self) -> Vec<ProviderCallMetrics> {
        Vec::new()
    }
}

// ---------------------------------------------------------------------------
// OpenAiCompatibleProvider
// ---------------------------------------------------------------------------

/// Default Ollama URL when none is provided.
pub const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434/v1";
/// Default model — MUST match `DEFAULT_OLLAMA_MODEL` in
/// `src/lib/constants.ts` (single user-facing default, mirrored here for
/// backend fallbacks).
pub const DEFAULT_OLLAMA_MODEL: &str = "gemma4:e4b";
/// Default per-request timeout.
pub const DEFAULT_TIMEOUT_SECS: u64 = 120;
/// Default number of segments per LLM call (see `TranslationContext::batch_size`).
pub const DEFAULT_BATCH_SIZE: usize = 20;

const MAX_RETRIES: u32 = 3;

pub struct OpenAiCompatibleProvider {
    base_url: String,
    model: String,
    api_key: Option<String>,
    reasoning_effort: Option<ReasoningEffort>,
    structured_translation_output: bool,
    metrics: Mutex<Vec<ProviderCallMetrics>>,
    client: Client,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
enum ReasoningEffort {
    #[serde(rename = "none")]
    Disabled,
}

impl OpenAiCompatibleProvider {
    pub fn new(base_url: &str, model: &str, api_key: Option<&str>, timeout: Duration) -> Self {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .expect("reqwest client");
        Self {
            base_url: normalize_openai_base_url(base_url),
            model: model.to_string(),
            api_key: api_key
                .map(str::trim)
                .filter(|key| !key.is_empty())
                .map(str::to_string),
            reasoning_effort: None,
            structured_translation_output: false,
            metrics: Mutex::new(Vec::new()),
            client,
        }
    }

    /// Build a provider from a stable UI preset. Ollama's OpenAI-compatible
    /// endpoint supports `reasoning_effort: "none"`; generic endpoints may
    /// reject unknown fields, so they must continue to omit it.
    pub fn new_for_preset(
        provider_id: &str,
        base_url: &str,
        model: &str,
        api_key: Option<&str>,
        timeout: Duration,
    ) -> Self {
        let mut provider = Self::new(base_url, model, api_key, timeout);
        if provider_id == "ollama" {
            provider.reasoning_effort = Some(ReasoningEffort::Disabled);
            provider.structured_translation_output = true;
        }
        provider
    }

    /// Convenience constructor using defaults.
    pub fn default_local() -> Self {
        Self::new_for_preset(
            "ollama",
            DEFAULT_OLLAMA_URL,
            DEFAULT_OLLAMA_MODEL,
            None,
            Duration::from_secs(DEFAULT_TIMEOUT_SECS),
        )
    }

    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let request = self
            .client
            .request(method, format!("{}/{}", self.base_url, path));
        match self.api_key.as_deref() {
            Some(key) => request.bearer_auth(key),
            None => request,
        }
    }

    fn record_metrics(&self, metrics: ProviderCallMetrics) {
        match self.metrics.lock() {
            Ok(mut recorded) => recorded.push(metrics),
            Err(error) => log::warn!("provider metrics lock poisoned: {error}"),
        }
    }

    pub async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        let response = self
            .request(reqwest::Method::GET, "models")
            .send()
            .await
            .map_err(|error| LlmError::Unavailable {
                message: error.to_string(),
            })?;

        if !response.status().is_success() {
            return Err(LlmError::Unavailable {
                message: format!("HTTP {}", response.status()),
            });
        }

        let mut models = response
            .json::<OpenAiModelsResponse>()
            .await
            .map_err(|error| LlmError::ResponseFormat(error.to_string()))?
            .data
            .into_iter()
            .map(|model| model.id)
            .collect::<Vec<_>>();
        models.sort();
        models.dedup();
        Ok(models)
    }
}

fn normalize_openai_base_url(base_url: &str) -> String {
    let base_url = base_url.trim().trim_end_matches('/');
    if base_url.ends_with("/v1") {
        base_url.to_string()
    } else {
        format!("{base_url}/v1")
    }
}

// ---------------------------------------------------------------------------
// Ollama API types (minimal)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct OpenAiMessage {
    role: &'static str,
    content: String,
}

#[derive(Serialize)]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<ReasoningEffort>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct OpenAiChatResponse {
    choices: Vec<OpenAiChoice>,
    #[serde(default)]
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct OpenAiUsage {
    prompt_tokens: Option<u64>,
    completion_tokens: Option<u64>,
    total_tokens: Option<u64>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessageResponse,
}

#[derive(Deserialize)]
struct OpenAiMessageResponse {
    content: String,
}

#[derive(Deserialize)]
struct OpenAiModelsResponse {
    data: Vec<OpenAiModel>,
}

#[derive(Deserialize)]
struct OpenAiModel {
    id: String,
}

// ---------------------------------------------------------------------------
// LlmProvider impl for OpenAiCompatibleProvider
// ---------------------------------------------------------------------------

impl LlmProvider for OpenAiCompatibleProvider {
    async fn translate(
        &self,
        segments: Vec<String>,
        context: TranslationContext,
    ) -> Result<Vec<String>, LlmError> {
        if segments.is_empty() {
            return Ok(vec![]);
        }

        let terminology_hint = terminology_prompt_fragment(&context.terminology_hints);

        let translation_units = render_translation_units(&segments, &context.segment_contexts);
        let tmpl = crate::llm::prompts::translate_for(&context.target_lang);
        let output_protocol = if self.structured_translation_output {
            "Return one JSON object with a `translations` array. Each item must contain exactly \
             the numeric `id` and a non-empty translated `text`. Reuse every input ID exactly \
             once; do not add, omit, duplicate, or reorder IDs. Output no explanation or \
             reference text."
        } else {
            "Output exactly one physical line for every input unit: `[id] translation`.\n\
             Reuse each numeric `id` exactly once. Do not add, omit, duplicate, or reorder IDs.\n\
             Output no JSON, explanation, heading, code fence, blank translation, or reference text."
        };
        let system_prompt = tmpl.render(
            &tmpl.system,
            &[
                (
                    "source_lang",
                    crate::llm::prompts::lang_code_to_name(&context.source_lang),
                ),
                (
                    "target_lang",
                    crate::llm::prompts::lang_code_to_name(&context.target_lang),
                ),
                ("output_protocol", output_protocol),
                ("terminology", &terminology_hint),
            ],
        );
        let prompt_body = tmpl.render(&tmpl.user, &[("segments", &translation_units)]);
        let started = Instant::now();
        let input_units = segments.len();
        let prompt_chars = system_prompt.chars().count() + prompt_body.chars().count();
        let mut last_err = String::new();
        let mut attempts = 0_u32;
        let mut last_usage: Option<OpenAiUsage> = None;
        let outcome: Result<(Vec<String>, Option<OpenAiUsage>), LlmError> = async {
            for attempt in 0..MAX_RETRIES {
                attempts = attempt + 1;
                let request = OpenAiChatRequest {
                    model: self.model.clone(),
                    messages: vec![
                        OpenAiMessage {
                            role: "system",
                            content: system_prompt.clone(),
                        },
                        OpenAiMessage {
                            role: "user",
                            content: prompt_body.clone(),
                        },
                    ],
                    stream: false,
                    reasoning_effort: self.reasoning_effort,
                    response_format: self
                        .structured_translation_output
                        .then(json_translation_response_format),
                };

                let response = self
                    .request(reqwest::Method::POST, "chat/completions")
                    .json(&request)
                    .send()
                    .await;

                match response {
                    Err(error) if attempts < MAX_RETRIES => {
                        last_err = error.to_string();
                    }
                    Err(error) => return Err(LlmError::Http(error)),
                    Ok(response) if !response.status().is_success() => {
                        last_err = format!("HTTP {}", response.status());
                        if attempts >= MAX_RETRIES {
                            return Err(LlmError::TranslationFailed {
                                attempts,
                                reason: last_err.clone(),
                            });
                        }
                    }
                    Ok(response) => {
                        let parsed: OpenAiChatResponse = response
                            .json()
                            .await
                            .map_err(|error| LlmError::ResponseFormat(error.to_string()))?;
                        let content = parsed
                            .choices
                            .first()
                            .ok_or_else(|| LlmError::ResponseFormat("missing choice".to_string()))?
                            .message
                            .content
                            .as_str();
                        last_usage = parsed.usage;
                        let lines = if self.structured_translation_output {
                            parse_structured_response(content, segments.len())?
                        } else {
                            parse_numbered_response(content, segments.len())?
                        };
                        let translations = lines
                            .into_iter()
                            .map(|line| line.replace('⏎', "\n"))
                            .collect();
                        return Ok((translations, parsed.usage));
                    }
                }
            }

            Err(LlmError::TranslationFailed {
                attempts,
                reason: last_err,
            })
        }
        .await;

        let usage = outcome
            .as_ref()
            .ok()
            .and_then(|(_, usage)| usage.as_ref())
            .copied()
            .or(last_usage);
        self.record_metrics(ProviderCallMetrics {
            task: ProviderTask::Translate,
            model: self.model.clone(),
            input_units,
            prompt_chars,
            prompt_tokens: usage.and_then(|value| value.prompt_tokens),
            completion_tokens: usage.and_then(|value| value.completion_tokens),
            total_tokens: usage.and_then(|value| value.total_tokens),
            duration_ms: elapsed_millis(started),
            attempts,
            success: outcome.is_ok(),
            terminology_hints: context.terminology_hints.len(),
        });

        outcome.map(|(translations, _)| translations)
    }

    async fn health_check(&self) -> Result<(), LlmError> {
        self.list_models().await.map(|_| ())
    }

    async fn chat(&self, system: &str, user: &str) -> Result<String, LlmError> {
        let started = Instant::now();
        let prompt_chars = system.chars().count() + user.chars().count();
        let request = OpenAiChatRequest {
            model: self.model.clone(),
            messages: vec![
                OpenAiMessage {
                    role: "system",
                    content: system.to_string(),
                },
                OpenAiMessage {
                    role: "user",
                    content: user.to_string(),
                },
            ],
            stream: false,
            reasoning_effort: self.reasoning_effort,
            response_format: None,
        };

        let mut last_err: Option<LlmError> = None;
        let mut attempts = 0_u32;
        let outcome: Result<(String, Option<OpenAiUsage>), LlmError> = async {
            for attempt in 0..2 {
                attempts = attempt + 1;
                if attempt > 0 {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
                match self
                    .request(reqwest::Method::POST, "chat/completions")
                    .json(&request)
                    .send()
                    .await
                {
                    Err(error) => {
                        last_err = Some(LlmError::Http(error));
                    }
                    Ok(response) if !response.status().is_success() => {
                        last_err = Some(LlmError::Unavailable {
                            message: format!("HTTP {}", response.status()),
                        });
                    }
                    Ok(response) => {
                        let parsed = response
                            .json::<OpenAiChatResponse>()
                            .await
                            .map_err(|error| LlmError::ResponseFormat(error.to_string()))?;
                        let content = parsed
                            .choices
                            .first()
                            .map(|choice| choice.message.content.clone())
                            .ok_or_else(|| {
                                LlmError::ResponseFormat("missing choice".to_string())
                            })?;
                        return Ok((content, parsed.usage));
                    }
                }
            }

            Err(last_err.unwrap_or_else(|| LlmError::Unavailable {
                message: "chat request failed without provider response".to_string(),
            }))
        }
        .await;

        let usage = outcome
            .as_ref()
            .ok()
            .and_then(|(_, usage)| usage.as_ref())
            .copied();
        self.record_metrics(ProviderCallMetrics {
            task: ProviderTask::Chat,
            model: self.model.clone(),
            input_units: 1,
            prompt_chars,
            prompt_tokens: usage.and_then(|value| value.prompt_tokens),
            completion_tokens: usage.and_then(|value| value.completion_tokens),
            total_tokens: usage.and_then(|value| value.total_tokens),
            duration_ms: elapsed_millis(started),
            attempts,
            success: outcome.is_ok(),
            terminology_hints: 0,
        });

        outcome.map(|(content, _)| content)
    }

    fn drain_metrics(&self) -> Vec<ProviderCallMetrics> {
        match self.metrics.lock() {
            Ok(mut recorded) => std::mem::take(&mut *recorded),
            Err(error) => {
                log::warn!("provider metrics lock poisoned while draining: {error}");
                Vec::new()
            }
        }
    }
}

fn elapsed_millis(started: Instant) -> u64 {
    started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}

/// Render each request item as one JSON object on one physical line. Keeping a
/// single input protocol avoids asking smaller models to switch formats inside
/// one batch.
fn render_translation_units(
    segments: &[String],
    contexts: &[Option<SegmentPromptContext>],
) -> String {
    let mut lines = Vec::with_capacity(segments.len());
    for (index, segment) in segments.iter().enumerate() {
        let text = segment.replace('\n', "⏎");
        match contexts.get(index).and_then(Option::as_ref) {
            Some(context) => lines.push(
                serde_json::json!({
                    "id": index + 1,
                    "type": context.segment_kind,
                    "speaker": context.speaker,
                    "branch": context.branch_path,
                    "previous": context.previous,
                    "text": text,
                    "following": context.following,
                })
                .to_string(),
            ),
            None => lines.push(
                serde_json::json!({
                    "id": index + 1,
                    "type": "unknown",
                    "text": text,
                })
                .to_string(),
            ),
        }
    }
    lines.join("\n")
}

/// Ollama supports OpenAI-compatible JSON-schema response formatting. Using it
/// for translations prevents small local models from surrounding the answer
/// with prose or silently changing the requested line protocol.
fn json_translation_response_format() -> serde_json::Value {
    serde_json::json!({
        "type": "json_schema",
        "json_schema": {
            "name": "translation_batch",
            "strict": true,
            "schema": {
                "type": "object",
                "properties": {
                    "translations": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "id": { "type": "integer" },
                                "text": { "type": "string" }
                            },
                            "required": ["id", "text"],
                            "additionalProperties": false
                        }
                    }
                },
                "required": ["translations"],
                "additionalProperties": false
            }
        }
    })
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
/// Accepts only `[N] text`. Duplicate IDs, unknown IDs, empty translations and
/// any non-protocol output are rejected so a response can never silently map a
/// neighbouring line onto the wrong segment.
fn parse_numbered_response(raw: &str, expected: usize) -> Result<Vec<String>, LlmError> {
    let stripped = strip_think_blocks(raw);
    let raw = stripped.trim();

    let mut out: Vec<Option<String>> = vec![None; expected];

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let rest = line
            .strip_prefix('[')
            .ok_or_else(|| LlmError::ResponseFormat(format!("unexpected response line: {line}")))?;
        let bracket = rest
            .find(']')
            .ok_or_else(|| LlmError::ResponseFormat(format!("missing closing bracket: {line}")))?;
        let idx = rest[..bracket]
            .parse::<usize>()
            .map_err(|_| LlmError::ResponseFormat(format!("invalid response id: {line}")))?;
        if !(1..=expected).contains(&idx) {
            return Err(LlmError::ResponseFormat(format!(
                "unknown response id {idx}; expected 1..={expected}"
            )));
        }
        if out[idx - 1].is_some() {
            return Err(LlmError::ResponseFormat(format!(
                "duplicate translation for line {idx}"
            )));
        }
        let text = rest[bracket + 1..].trim();
        if text.is_empty() {
            return Err(LlmError::ResponseFormat(format!(
                "empty translation for line {idx}"
            )));
        }
        out[idx - 1] = Some(text.to_string());
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

#[derive(Deserialize)]
struct StructuredTranslationResponse {
    translations: Vec<StructuredTranslation>,
}

#[derive(Deserialize)]
struct StructuredTranslation {
    id: usize,
    text: String,
}

/// Parse Ollama's schema-constrained response while retaining the same strict
/// ID and non-empty-text guarantees as the portable numbered-line protocol.
fn parse_structured_response(raw: &str, expected: usize) -> Result<Vec<String>, LlmError> {
    let parsed: StructuredTranslationResponse = serde_json::from_str(raw.trim())
        .map_err(|error| LlmError::ResponseFormat(format!("invalid structured JSON: {error}")))?;
    let mut out: Vec<Option<String>> = vec![None; expected];
    for translation in parsed.translations {
        if !(1..=expected).contains(&translation.id) {
            return Err(LlmError::ResponseFormat(format!(
                "unknown response id {}; expected 1..={expected}",
                translation.id
            )));
        }
        if out[translation.id - 1].is_some() {
            return Err(LlmError::ResponseFormat(format!(
                "duplicate translation for id {}",
                translation.id
            )));
        }
        let text = translation.text.trim();
        if text.is_empty() {
            return Err(LlmError::ResponseFormat(format!(
                "empty translation for id {}",
                translation.id
            )));
        }
        out[translation.id - 1] = Some(text.to_string());
    }
    out.into_iter()
        .enumerate()
        .map(|(index, value)| {
            value.ok_or_else(|| {
                LlmError::ResponseFormat(format!("missing translation for id {}", index + 1))
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

    fn make_provider(server: &MockServer) -> OpenAiCompatibleProvider {
        OpenAiCompatibleProvider::new(
            &server.base_url(),
            "test-model",
            None,
            Duration::from_secs(5),
        )
    }

    fn ctx() -> TranslationContext {
        TranslationContext {
            source_lang: "ja".to_string(),
            target_lang: "en".to_string(),
            terminology_hints: vec![],
            engine: "mv_mz".to_string(),
            batch_size: DEFAULT_BATCH_SIZE,
            batch_delay_ms: 0,
            prompt_context_policy: PromptContextPolicy::EngineOwned,
            segment_contexts: vec![],
        }
    }

    fn mv_mz_prompt_context() -> SegmentPromptContext {
        SegmentPromptContext {
            segment_kind: "dialogue".to_string(),
            scene_id: Some("Map001.json:event:1:page:0".to_string()),
            speaker: Some("勇者".to_string()),
            branch_path: Some("if:4".to_string()),
            previous: vec![crate::llm::context::NeighborLine {
                segment_kind: "dialogue".to_string(),
                speaker: Some("仲間".to_string()),
                text: "待って！".to_string(),
            }],
            following: vec![crate::llm::context::NeighborLine {
                segment_kind: "dialogue".to_string(),
                speaker: Some("勇者".to_string()),
                text: "時間がない。".to_string(),
            }],
        }
    }

    #[tokio::test]
    async fn test_health_check_ok() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/v1/models");
            then.status(200).json_body(json!({ "data": [] }));
        });
        let provider = make_provider(&server);
        assert!(provider.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_health_check_fail() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/v1/models");
            then.status(503);
        });
        let provider = make_provider(&server);
        assert!(matches!(
            provider.health_check().await,
            Err(LlmError::Unavailable { .. })
        ));
    }

    #[tokio::test]
    async fn test_list_models_uses_bearer_auth_and_sorts() {
        let server = MockServer::start();
        let model_mock = server.mock(|when, then| {
            when.method(GET)
                .path("/v1/models")
                .header("authorization", "Bearer secret");
            then.status(200).json_body(json!({
                "data": [{ "id": "z-model" }, { "id": "a-model" }]
            }));
        });
        let provider = OpenAiCompatibleProvider::new(
            &server.base_url(),
            "test-model",
            Some("secret"),
            Duration::from_secs(5),
        );

        assert_eq!(
            provider.list_models().await.unwrap(),
            ["a-model", "z-model"]
        );
        model_mock.assert();
    }

    #[test]
    fn test_base_url_normalization_keeps_single_v1() {
        assert_eq!(
            normalize_openai_base_url("http://localhost:11434/"),
            "http://localhost:11434/v1"
        );
        assert_eq!(
            normalize_openai_base_url("https://router.huggingface.co/v1"),
            "https://router.huggingface.co/v1"
        );
    }

    #[tokio::test]
    async fn test_translate_basic() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1/chat/completions");
            then.status(200).json_body(json!({
                "choices": [{ "message": { "role": "assistant", "content": "[1] Hero" } }]
            }));
        });
        let provider = make_provider(&server);
        let result = provider
            .translate(vec!["主人公".to_string()], ctx())
            .await
            .expect("translate");
        assert_eq!(result, vec!["Hero"]);
    }

    #[test]
    fn ollama_preset_disables_reasoning_without_affecting_other_providers() {
        let ollama = OpenAiCompatibleProvider::new_for_preset(
            "ollama",
            "http://localhost:11434/v1",
            "gemma4:e4b",
            None,
            Duration::from_secs(5),
        );
        let lm_studio = OpenAiCompatibleProvider::new_for_preset(
            "lmstudio",
            "http://localhost:1234/v1",
            "local-model",
            None,
            Duration::from_secs(5),
        );

        assert_eq!(ollama.reasoning_effort, Some(ReasoningEffort::Disabled));
        assert_eq!(lm_studio.reasoning_effort, None);
    }

    #[test]
    fn chat_request_omits_optional_reasoning_for_generic_openai_apis() {
        let generic = OpenAiChatRequest {
            model: "model".to_string(),
            messages: vec![],
            stream: false,
            reasoning_effort: None,
            response_format: None,
        };
        let generic_json = serde_json::to_value(&generic).unwrap();
        let ollama = OpenAiChatRequest {
            model: "model".to_string(),
            messages: vec![],
            stream: false,
            reasoning_effort: Some(ReasoningEffort::Disabled),
            response_format: Some(json_translation_response_format()),
        };

        let ollama_json = serde_json::to_value(&ollama).unwrap();
        assert!(generic_json.get("reasoning_effort").is_none());
        assert!(generic_json.get("response_format").is_none());
        assert_eq!(ollama_json["reasoning_effort"], "none");
        assert_eq!(ollama_json["response_format"]["type"], "json_schema");
    }

    #[tokio::test]
    async fn ollama_translation_request_sends_reasoning_effort_none() {
        let server = MockServer::start();
        let request_mock = server.mock(|when, then| {
            when.method(POST)
                .path("/v1/chat/completions")
                .body_contains("\"reasoning_effort\":\"none\"")
                .body_contains("\"response_format\"");
            then.status(200).json_body(json!({
                "choices": [{ "message": { "role": "assistant", "content":
                    "{\"translations\":[{\"id\":1,\"text\":\"Héros\"}]}" } }]
            }));
        });
        let provider = OpenAiCompatibleProvider::new_for_preset(
            "ollama",
            &server.base_url(),
            "gemma4:e4b",
            None,
            Duration::from_secs(5),
        );

        let result = provider
            .translate(vec!["勇者".to_string()], ctx())
            .await
            .expect("translate");

        assert_eq!(result, vec!["Héros"]);
        request_mock.assert();
    }

    #[tokio::test]
    async fn successful_translation_records_openai_usage_metrics() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1/chat/completions");
            then.status(200).json_body(json!({
                "choices": [{ "message": { "role": "assistant", "content": "[1] Héros" } }],
                "usage": {
                    "prompt_tokens": 120,
                    "completion_tokens": 17,
                    "total_tokens": 137
                }
            }));
        });
        let provider = make_provider(&server);
        let mut context = ctx();
        context.terminology_hints = vec![TerminologyHint {
            source: "勇者".to_string(),
            target: "Hero".to_string(),
            semantic_type: "character".to_string(),
            part_of_speech: PartOfSpeech::Noun,
            enforcement: Enforcement::Required,
            accepted_targets: vec![],
        }];

        provider
            .translate(vec!["勇者".to_string()], context)
            .await
            .expect("translate");
        let metrics = provider.drain_metrics();

        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].task, ProviderTask::Translate);
        assert_eq!(metrics[0].model, "test-model");
        assert_eq!(metrics[0].input_units, 1);
        assert_eq!(metrics[0].terminology_hints, 1);
        assert!(metrics[0].prompt_chars > 0);
        assert_eq!(metrics[0].prompt_tokens, Some(120));
        assert_eq!(metrics[0].completion_tokens, Some(17));
        assert_eq!(metrics[0].total_tokens, Some(137));
        assert_eq!(metrics[0].attempts, 1);
        assert!(metrics[0].success);
        assert!(provider.drain_metrics().is_empty());
    }

    #[tokio::test]
    async fn failed_translation_records_attempts_without_inventing_usage() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1/chat/completions");
            then.status(503);
        });
        let provider = make_provider(&server);

        let result = provider.translate(vec!["勇者".to_string()], ctx()).await;
        let metrics = provider.drain_metrics();

        assert!(result.is_err());
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].attempts, MAX_RETRIES);
        assert!(!metrics[0].success);
        assert_eq!(metrics[0].prompt_tokens, None);
        assert_eq!(metrics[0].completion_tokens, None);
        assert_eq!(metrics[0].total_tokens, None);
    }

    #[tokio::test]
    async fn format_failure_still_records_provider_usage() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1/chat/completions");
            then.status(200).json_body(json!({
                "choices": [{ "message": { "role": "assistant", "content": "not numbered" } }],
                "usage": { "prompt_tokens": 42, "completion_tokens": 3, "total_tokens": 45 }
            }));
        });
        let provider = make_provider(&server);

        assert!(provider
            .translate(vec!["勇者".to_string()], ctx())
            .await
            .is_err());
        let metrics = provider.drain_metrics();

        assert_eq!(metrics[0].prompt_tokens, Some(42));
        assert_eq!(metrics[0].completion_tokens, Some(3));
        assert_eq!(metrics[0].total_tokens, Some(45));
        assert!(!metrics[0].success);
    }

    #[test]
    fn contextual_units_are_single_line_json_with_reference_metadata() {
        let rendered = render_translation_units(
            &["行こう\n今すぐ".to_string()],
            &[Some(mv_mz_prompt_context())],
        );
        let unit: serde_json::Value = serde_json::from_str(&rendered).unwrap();

        assert_eq!(rendered.lines().count(), 1);
        assert_eq!(unit["id"], 1);
        assert_eq!(unit["type"], "dialogue");
        assert_eq!(unit["speaker"], "勇者");
        assert_eq!(unit["previous"][0]["text"], "待って！");
        assert_eq!(unit["text"], "行こう⏎今すぐ");
        assert_eq!(unit["following"][0]["text"], "時間がない。");
    }

    #[test]
    fn context_free_units_use_the_same_json_protocol() {
        let rendered = render_translation_units(&["主人公".to_string()], &[None]);
        let unit: serde_json::Value = serde_json::from_str(&rendered).unwrap();

        assert_eq!(unit["id"], 1);
        assert_eq!(unit["type"], "unknown");
        assert_eq!(unit["text"], "主人公");
    }

    #[tokio::test]
    async fn openai_compatible_request_contains_mv_mz_context() {
        let server = MockServer::start();
        let request_mock = server.mock(|when, then| {
            when.method(POST)
                .path("/v1/chat/completions")
                .body_contains("dialogue")
                .body_contains("勇者")
                .body_contains("待って");
            then.status(200).json_body(json!({
                "choices": [{ "message": { "role": "assistant", "content": "[1] Let's go" } }]
            }));
        });
        let provider = make_provider(&server);
        let mut context = ctx();
        context.segment_contexts = vec![Some(mv_mz_prompt_context())];

        let result = provider
            .translate(vec!["行こう".to_string()], context)
            .await
            .unwrap();

        assert_eq!(result, vec!["Let's go"]);
        request_mock.assert();
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
    fn test_parse_numbered_response_rejects_free_form() {
        let raw = "Hello\nWorld";
        assert!(matches!(
            parse_numbered_response(raw, 2),
            Err(LlmError::ResponseFormat(_))
        ));
    }

    #[test]
    fn test_parse_think_block_stripped() {
        // qwen3 thinking mode: block is ignored, real answer is parsed
        let raw = "<think>\nraisonnement interne\n</think>\n[1] Hero";
        let result = parse_numbered_response(raw, 1).unwrap();
        assert_eq!(result, vec!["Hero"]);
    }

    #[test]
    fn test_parse_numbered_response_rejects_duplicate_unknown_and_empty_ids() {
        for raw in [
            "[1] Hero\n[1] Sword",
            "[1] Hero\n[3] Sword",
            "[1] Hero\n[2]   ",
        ] {
            assert!(matches!(
                parse_numbered_response(raw, 2),
                Err(LlmError::ResponseFormat(_))
            ));
        }
    }

    #[test]
    fn test_parse_structured_response_is_strict_and_ordered_by_id() {
        let raw = r#"{"translations":[{"id":2,"text":"Monde"},{"id":1,"text":"Bonjour"}]}"#;
        assert_eq!(
            parse_structured_response(raw, 2).unwrap(),
            vec!["Bonjour", "Monde"]
        );
        for invalid in [
            r#"{"translations":[{"id":1,"text":"A"},{"id":1,"text":"B"}]}"#,
            r#"{"translations":[{"id":3,"text":"A"}]}"#,
            r#"{"translations":[{"id":1,"text":""}]}"#,
        ] {
            assert!(parse_structured_response(invalid, 2).is_err());
        }
    }

    #[tokio::test]
    async fn test_system_prompt_contains_placeholder_instruction() {
        let server = MockServer::start();
        // The mock only matches if the request body contains "CRITICAL RULE".
        // If the system prompt omits it, the mock won't fire and the call fails.
        let m = server.mock(|when, then| {
            when.method(POST)
                .path("/v1/chat/completions")
                .body_contains("CRITICAL RULE");
            then.status(200).json_body(json!({
                "choices": [{ "message": { "role": "assistant", "content": "[1] Hero" } }]
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
            when.method(POST).path("/v1/chat/completions");
            then.status(200).json_body(json!({
                "choices": [{ "message": {
                    "role": "assistant",
                    "content": "[1] A rare injectable ampoule not yet widely available.⏎Fully restores HP and MP."
                } }]
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
