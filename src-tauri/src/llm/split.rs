//! Recursive batch-split helper for the LLM translation pipeline.
//!
//! Isolated from `pipeline.rs` so the split logic — the most complex part of
//! the translation path — can be read and debugged without scrolling through
//! the top-level orchestration.
//!
//! ## Algorithm
//! On `ResponseFormat` failure or placeholder-restore failure after `MAX_RETRIES`:
//! - If the batch contains more than one segment, split it in half and recurse
//!   on each half independently.
//! - If the batch is already a single segment (cannot split further), mark it
//!   `needs_review` and keep the source text as the provisional translation.

use std::future::Future;
use std::pin::Pin;

use crate::llm::provider::{LlmError, LlmProvider, TranslationContext};
use crate::llm::tokenizer::{Tokenized, Tokenizer};
use serde::Serialize;

pub(crate) const MAX_RETRIES: u32 = 3;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineBatchMetrics {
    pub response_format_retries: u32,
    pub placeholder_retries: u32,
    pub recursive_splits: u32,
    pub semantic_rejections: u32,
}

impl PipelineBatchMetrics {
    pub(crate) fn merge(&mut self, other: Self) {
        self.response_format_retries += other.response_format_retries;
        self.placeholder_retries += other.placeholder_retries;
        self.recursive_splits += other.recursive_splits;
        self.semantic_rejections += other.semantic_rejections;
    }
}

pub struct SplitOutcome {
    pub results: Vec<(usize, String, bool)>,
    pub metrics: PipelineBatchMetrics,
}

/// Translate segments at LOCAL positions `indices` within `tokenized`.
///
/// Returns `Vec<(local_idx, translated_text, needs_review)>`.
///
/// `indices` are LOCAL into `tokenized[]` — NOT into `unique_segs[]`.
/// The caller is responsible for mapping local indices back to global ones.
#[allow(clippy::type_complexity)]
pub(crate) fn llm_translate_with_split<'a, P>(
    indices: Vec<usize>,
    tokenized: &'a [Tokenized],
    provider: &'a P,
    context: &'a TranslationContext,
) -> Pin<Box<dyn Future<Output = SplitOutcome> + Send + 'a>>
where
    P: LlmProvider,
{
    Box::pin(async move {
        let texts_for_llm: Vec<String> =
            indices.iter().map(|&i| tokenized[i].text.clone()).collect();

        let mut attempt = 0u32;
        let mut metrics = PipelineBatchMetrics::default();
        let restored_texts: Vec<String> = loop {
            let mut request_context = context.clone();
            request_context.segment_contexts = indices
                .iter()
                .map(|&index| context.segment_contexts.get(index).cloned().unwrap_or(None))
                .collect();
            let llm_result = provider
                .translate(texts_for_llm.clone(), request_context)
                .await;

            let llm_out = match llm_result {
                Ok(out) => out,
                Err(LlmError::ResponseFormat(_)) if attempt + 1 < MAX_RETRIES => {
                    metrics.response_format_retries += 1;
                    attempt += 1;
                    continue;
                }
                Err(LlmError::ResponseFormat(_)) => {
                    metrics.response_format_retries += 1;
                    if indices.len() > 1 {
                        metrics.recursive_splits += 1;
                        let mid = indices.len() / 2;
                        let left = indices[..mid].to_vec();
                        let right = indices[mid..].to_vec();
                        let left_outcome =
                            llm_translate_with_split(left, tokenized, provider, context).await;
                        let right_outcome =
                            llm_translate_with_split(right, tokenized, provider, context).await;
                        metrics.merge(left_outcome.metrics);
                        metrics.merge(right_outcome.metrics);
                        let mut results = left_outcome.results;
                        results.extend(right_outcome.results);
                        return SplitOutcome { results, metrics };
                    } else {
                        log::warn!(
                            "[h2s] single-segment ResponseFormat after {} attempts — \
                             needs_review (local pos {})",
                            MAX_RETRIES,
                            indices[0]
                        );
                        return SplitOutcome {
                            results: vec![(indices[0], String::new(), true)],
                            metrics,
                        };
                    }
                }
                Err(e) => {
                    log::warn!("[h2s] non-recoverable LLM error in split batch: {e}");
                    return SplitOutcome {
                        results: indices.iter().map(|&i| (i, String::new(), true)).collect(),
                        metrics,
                    };
                }
            };

            let mut restore_ok = true;
            let mut restored = Vec::with_capacity(llm_out.len());
            for (resp, &local_idx) in llm_out.iter().zip(indices.iter()) {
                match Tokenizer::restore(resp, &tokenized[local_idx].map) {
                    Ok(r) => restored.push(r),
                    Err(_) => {
                        restore_ok = false;
                        break;
                    }
                }
            }

            if restore_ok {
                break restored;
            }

            attempt += 1;
            metrics.placeholder_retries += 1;
            if attempt >= MAX_RETRIES {
                if indices.len() > 1 {
                    metrics.recursive_splits += 1;
                    let mid = indices.len() / 2;
                    let left = indices[..mid].to_vec();
                    let right = indices[mid..].to_vec();
                    let left_outcome =
                        llm_translate_with_split(left, tokenized, provider, context).await;
                    let right_outcome =
                        llm_translate_with_split(right, tokenized, provider, context).await;
                    metrics.merge(left_outcome.metrics);
                    metrics.merge(right_outcome.metrics);
                    let mut results = left_outcome.results;
                    results.extend(right_outcome.results);
                    return SplitOutcome { results, metrics };
                } else {
                    log::warn!(
                        "[h2s] single-segment placeholder failure after {} attempts — \
                         needs_review (local pos {})",
                        MAX_RETRIES,
                        indices[0]
                    );
                    return SplitOutcome {
                        results: vec![(indices[0], String::new(), true)],
                        metrics,
                    };
                }
            }
        };

        SplitOutcome {
            results: indices
                .into_iter()
                .zip(restored_texts)
                .map(|(i, text)| (i, text, false))
                .collect(),
            metrics,
        }
    })
}
