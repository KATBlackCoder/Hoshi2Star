//! Bounded recovery for provider outputs rejected by semantic QA.
//!
//! The first translation keeps the engine-owned context. Only a rejected
//! provider result reaches this module, where it is retried as a singleton
//! with progressively less reference metadata. Every candidate is checked
//! against the original semantic context before it can replace the first
//! output.

use crate::core::{qa, terminology::resolver::QaTerminologyRule};
use crate::llm::context::{QaNeighborContext, SegmentPromptContext};
use crate::llm::provider::{LlmProvider, TranslationContext};
use crate::llm::split::{llm_translate_with_split, PipelineBatchMetrics};
use crate::llm::tokenizer::Tokenized;
use sqlx::SqlitePool;

pub(crate) struct SemanticRetryRequest<'a> {
    pub source_text: &'a str,
    pub tokenized: &'a Tokenized,
    pub translation_context: &'a TranslationContext,
    pub segment_id: &'a str,
    pub terminology_rules: &'a [QaTerminologyRule],
    pub prompt_context: Option<&'a SegmentPromptContext>,
    pub qa_neighbors: &'a QaNeighborContext,
}

pub(crate) struct SemanticRetryOutcome {
    pub recovered_target: Option<String>,
    pub metrics: PipelineBatchMetrics,
}

pub(crate) async fn retry<P: LlmProvider>(
    request: SemanticRetryRequest<'_>,
    provider: &P,
    db: &SqlitePool,
) -> SemanticRetryOutcome {
    let segment_kind = request
        .prompt_context
        .map(|context| context.segment_kind.as_str())
        .unwrap_or("unknown");
    let qa_context = qa::QaSemanticContext {
        source_language: &request.translation_context.source_lang,
        target_language: &request.translation_context.target_lang,
        segment_kind,
        neighbor_sources: &request.qa_neighbors.sources,
        neighbor_targets: &request.qa_neighbors.targets,
    };
    let reduced_context = request.prompt_context.cloned().map(|mut context| {
        context.previous.clear();
        context.following.clear();
        context
    });

    let mut metrics = PipelineBatchMetrics::default();
    for prompt_context in [reduced_context, None] {
        metrics.semantic_retries += 1;
        let mut translation_context = request.translation_context.clone();
        translation_context.segment_contexts = vec![prompt_context];
        let segment_ids = vec![request.segment_id.to_string()];
        let outcome = llm_translate_with_split(
            vec![0],
            std::slice::from_ref(request.tokenized),
            provider,
            &translation_context,
            db,
            &segment_ids,
        )
        .await;
        metrics.merge(outcome.metrics);
        let Some((_, candidate, false)) = outcome.results.into_iter().next() else {
            continue;
        };
        let qa_result = qa::check_with_context(
            request.source_text,
            &candidate,
            request.terminology_rules,
            &request.translation_context.engine,
            &qa_context,
        );
        if !qa_result.has_critical_errors() {
            metrics.semantic_recoveries += 1;
            return SemanticRetryOutcome {
                recovered_target: Some(candidate),
                metrics,
            };
        }
    }

    SemanticRetryOutcome {
        recovered_target: None,
        metrics,
    }
}
