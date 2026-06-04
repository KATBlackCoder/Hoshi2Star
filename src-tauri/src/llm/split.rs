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

pub(crate) const MAX_RETRIES: u32 = 3;

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
) -> Pin<Box<dyn Future<Output = Vec<(usize, String, bool)>> + Send + 'a>>
where
    P: LlmProvider,
{
    Box::pin(async move {
        let texts_for_llm: Vec<String> =
            indices.iter().map(|&i| tokenized[i].text.clone()).collect();

        let mut attempt = 0u32;
        let restored_texts: Vec<String> = loop {
            let llm_result = provider
                .translate(texts_for_llm.clone(), context.clone())
                .await;

            let llm_out = match llm_result {
                Ok(out) => out,
                Err(LlmError::ResponseFormat(_)) if attempt + 1 < MAX_RETRIES => {
                    attempt += 1;
                    continue;
                }
                Err(LlmError::ResponseFormat(_)) => {
                    if indices.len() > 1 {
                        let mid = indices.len() / 2;
                        let left = indices[..mid].to_vec();
                        let right = indices[mid..].to_vec();
                        let mut results =
                            llm_translate_with_split(left, tokenized, provider, context).await;
                        results.extend(
                            llm_translate_with_split(right, tokenized, provider, context).await,
                        );
                        return results;
                    } else {
                        log::warn!(
                            "[h2s] single-segment ResponseFormat after {} attempts — \
                             needs_review (local pos {})",
                            MAX_RETRIES,
                            indices[0]
                        );
                        return vec![(indices[0], String::new(), true)];
                    }
                }
                Err(e) => {
                    log::warn!("[h2s] non-recoverable LLM error in split batch: {e}");
                    return indices.iter().map(|&i| (i, String::new(), true)).collect();
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
            if attempt >= MAX_RETRIES {
                if indices.len() > 1 {
                    let mid = indices.len() / 2;
                    let left = indices[..mid].to_vec();
                    let right = indices[mid..].to_vec();
                    let mut results =
                        llm_translate_with_split(left, tokenized, provider, context).await;
                    results.extend(
                        llm_translate_with_split(right, tokenized, provider, context).await,
                    );
                    return results;
                } else {
                    log::warn!(
                        "[h2s] single-segment placeholder failure after {} attempts — \
                         needs_review (local pos {})",
                        MAX_RETRIES,
                        indices[0]
                    );
                    return vec![(indices[0], String::new(), true)];
                }
            }
        };

        indices
            .into_iter()
            .zip(restored_texts)
            .map(|(i, text)| (i, text, false))
            .collect()
    })
}
