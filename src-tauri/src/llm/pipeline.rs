//! LLM translation pipeline — Passe 1 (translate) for the MVP.
//!
//! ## Flow (per batch)
//! 1. Deduplicate by hash (`batch::dedup_by_hash`)
//! 2. For each unique segment: check TM — if exact match, skip LLM
//! 3. Tokenize remaining segments (ADR-002: placeholders → ⟦ph_N⟧)
//! 4. Send tokenized batch to the provider via `split::llm_translate_with_split`
//! 5. Validate + restore placeholders; retry up to `MAX_RETRIES` times if invalid
//! 6. Retry semantic QA failures as isolated singletons with bounded context
//! 7. Persist the batch's `target_text`/`status` to the DB immediately
//!    (incremental persistence — a crash mid-run only loses the in-flight batch)
//! 8. Emit `h2s://llm/segments-updated` (the batch's persisted `target_text`/
//!    `status`) and `h2s://llm/progress` (if an `AppHandle` was provided), and
//!    call `on_progress(done, total)`. Per-segment outcomes are summarised once
//!    by the caller (`RunSummary`), never one event per segment.
//! 9. If a `CooldownState` was provided, possibly `.await` a rest period
//!    before starting the next batch (fine-grained automatic cooldown)
//!
//! A provider outage stops the run with `PipelineError::ProviderUnavailable`;
//! a cancellation flag stops it cleanly before the next batch. Failed segments
//! are never written (no source-as-target fallback).
//!
//! The pipeline is generic over `P: LlmProvider` so it can be tested with a
//! mock provider without needing dynamic dispatch.
//!
//! Recursive batch-split logic lives in `split.rs`.
//! Event payload types live in `progress.rs`.

use crate::core::{qa, terminology::resolver, tm};
use crate::llm::batch;
use crate::llm::context::{self, PromptContextClass};
use crate::llm::progress::{CoolingPayload, ProgressPayload, SegmentUpdatePayload};
use crate::llm::provider::{LlmError, LlmProvider, PromptContextPolicy, TranslationContext};
use crate::llm::semantic_retry::{self, SemanticRetryRequest};
use crate::llm::split::{llm_translate_with_split, PipelineBatchMetrics};
use crate::llm::tokenizer::Tokenizer;
use serde::Serialize;
use sqlx::SqlitePool;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::Emitter;
use thiserror::Error;

#[cfg(test)]
use crate::llm::provider::DEFAULT_BATCH_SIZE;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct TranslationResult {
    pub id: String,
    pub translated_text: String,
    /// `true` when the translation came from the TM (no LLM call was made).
    pub from_tm: bool,
    /// `true` when the translation has a critical QA error (status `needs_review`).
    pub needs_review: bool,
    /// `true` when no usable output was obtained (provider refused the request,
    /// unparseable response or broken placeholders after every retry). Nothing
    /// is written for a failed segment: it keeps its previous target and status.
    pub failed: bool,
    pub qa_score: u8,
    pub qa_errors: Vec<qa::QaError>,
}

#[derive(Debug)]
pub struct PipelineRunOutcome {
    pub results: Vec<TranslationResult>,
    pub metrics: PipelineBatchMetrics,
    /// The cancellation flag stopped the run before every segment was handled.
    pub cancelled: bool,
    /// Rows outside `results` updated by the canonical-name repair.
    pub repaired: usize,
}

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("LLM provider error: {0}")]
    Provider(String),
    /// The provider stopped answering; `processed` segments were already
    /// handled (and persisted) before the run stopped.
    #[error("LLM provider unavailable: {message}")]
    ProviderUnavailable { message: String, processed: usize },
    #[error("Database error: {0}")]
    Database(String),
}

impl From<LlmError> for PipelineError {
    fn from(e: LlmError) -> Self {
        PipelineError::Provider(e.to_string())
    }
}

/// Tracks the automatic "cooldown" rest period for "Translate All".
///
/// Checked once per batch (not just once per file) so large files don't run
/// past the configured threshold without ever pausing.
pub struct CooldownState {
    threshold: Duration,
    duration: Duration,
    last_rest: Instant,
}

impl CooldownState {
    /// `threshold_secs` is clamped to at least 1s (matches the previous
    /// `translate_all_segments` behavior where 0 meant "check every batch").
    pub fn new(threshold_secs: u64, duration_secs: u64) -> Self {
        Self {
            threshold: Duration::from_secs(threshold_secs.max(1)),
            duration: Duration::from_secs(duration_secs),
            last_rest: Instant::now(),
        }
    }

    /// If enough time has elapsed since the last rest, sleep for `duration`,
    /// emitting `h2s://llm/cooling { remainingSecs }` once per second so the
    /// frontend can display a countdown. No-op if `duration` is zero or the
    /// threshold hasn't elapsed yet. A set `cancel` flag ends the rest early.
    async fn maybe_rest(&mut self, handle: &tauri::AppHandle, cancel: Option<&AtomicBool>) {
        if self.duration.is_zero() || self.last_rest.elapsed() < self.threshold {
            return;
        }

        let mut remaining = self.duration.as_secs();
        while remaining > 0 && !cancel.is_some_and(|flag| flag.load(Ordering::SeqCst)) {
            let _ = handle.emit(
                "h2s://llm/cooling",
                CoolingPayload {
                    remaining_secs: remaining,
                },
            );
            tokio::time::sleep(Duration::from_secs(1)).await;
            remaining -= 1;
        }
        let _ = handle.emit("h2s://llm/cooling", CoolingPayload { remaining_secs: 0 });

        self.last_rest = Instant::now();
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// The DB `status` value for a translation result.
fn result_status(r: &TranslationResult) -> &'static str {
    if r.needs_review {
        "needs_review"
    } else {
        "translated"
    }
}

/// Persist a batch's `target_text`/`status` immediately so a crash mid-run
/// only loses the in-flight batch. The complete batch is committed in one
/// transaction, avoiding one SQLite autocommit per translated segment.
///
/// Returns how many other rows were repaired: earlier source fallbacks of
/// the same canonical name that now receive this batch's translation.
async fn persist_batch_results(
    db: &SqlitePool,
    results: &[TranslationResult],
) -> Result<usize, PipelineError> {
    let mut tx = db
        .begin()
        .await
        .map_err(|error| PipelineError::Database(error.to_string()))?;
    for r in results.iter().filter(|result| !result.failed) {
        let status = result_status(r);
        sqlx::query(
            "UPDATE segments \
             SET target_text = ?, status = ?, qa_score = ?, \
                 updated_at = datetime('now') \
             WHERE id = ?",
        )
        .bind(&r.translated_text)
        .bind(status)
        .bind(i64::from(r.qa_score))
        .bind(&r.id)
        .execute(&mut *tx)
        .await
        .map_err(|error| PipelineError::Database(error.to_string()))?;
    }
    // If an earlier occurrence of a canonical MV/MZ name fell back to the
    // Japanese source after a provider-format failure, a later validated
    // translation repairs that fallback. Reviewed/manual text and genuine
    // translated variants are never overwritten.
    let mut repaired = 0_u64;
    for r in results
        .iter()
        .filter(|result| !result.needs_review && !result.failed)
    {
        repaired += sqlx::query(
            "UPDATE segments SET target_text = ?, status = 'translated', qa_score = ?, \
                    updated_at = datetime('now') \
             WHERE status = 'needs_review' AND target_text = source_text \
               AND segment_kind IN (\
                    'speaker', 'actor_name', 'class_name', 'item_name', 'skill_name', \
                    'enemy_name', 'state_name', 'map_name', 'common_event_name', \
                    'system_term', 'game_title'\
               ) \
               AND source_text = (SELECT source_text FROM segments WHERE id = ?) \
               AND segment_kind = (SELECT segment_kind FROM segments WHERE id = ?) \
               AND source_file_id IN (\
                    SELECT candidate_file.id FROM source_files candidate_file \
                    WHERE candidate_file.project_id = (\
                        SELECT current_file.project_id FROM segments current \
                        JOIN source_files current_file ON current.source_file_id = current_file.id \
                        JOIN projects project ON project.id = current_file.project_id \
                        WHERE current.id = ? AND project.engine = 'mv_mz'\
                    )\
               )",
        )
        .bind(&r.translated_text)
        .bind(i64::from(r.qa_score))
        .bind(&r.id)
        .bind(&r.id)
        .bind(&r.id)
        .execute(&mut *tx)
        .await
        .map_err(|error| PipelineError::Database(error.to_string()))?
        .rows_affected();
    }
    tx.commit()
        .await
        .map_err(|error| PipelineError::Database(error.to_string()))?;
    Ok(usize::try_from(repaired).unwrap_or(usize::MAX))
}

/// Run the translation pipeline on a batch of segments.
///
/// `segments` is a list of `(id, source_text)` pairs. After each inner batch:
/// - `target_text`/`status` are persisted to the DB immediately
/// - if `app_handle` is `Some`, `h2s://llm/segments-updated` and
///   `h2s://llm/progress` are emitted
/// - `on_progress(done, total)` is called (used by tests)
/// - if `cooldown` is `Some` (and `app_handle` is `Some`), the automatic
///   cooldown may `.await` a rest period before the next batch
///
/// `global_progress`, when `Some((done_offset, global_total))`, rewrites the
/// `done`/`total` reported to `on_progress` and emitted in `ProgressPayload`
/// as `(done_offset + done, global_total)` — used by "Translate All" so the
/// progress bar reflects the whole project, not just the current file.
///
/// The public `run` function wraps this with a real Tauri `AppHandle` for
/// event emission. Tests call `run_inner` directly with `None, None, None`
/// and a closure.
#[allow(clippy::too_many_arguments)]
pub async fn run_inner<P, F>(
    segments: Vec<(String, String)>,
    provider: &P,
    context: TranslationContext,
    db: &SqlitePool,
    app_handle: Option<&tauri::AppHandle>,
    cooldown: Option<&mut CooldownState>,
    global_progress: Option<(usize, usize)>,
    on_progress: F,
) -> Result<Vec<TranslationResult>, PipelineError>
where
    P: LlmProvider,
    F: FnMut(usize, usize),
{
    Ok(run_inner_with_metrics(
        segments,
        provider,
        context,
        db,
        app_handle,
        cooldown,
        global_progress,
        on_progress,
    )
    .await?
    .results)
}

/// Variant of [`run_inner`] used by diagnostics and real-project pilots that
/// need aggregate retry counters without depending on Tauri event listeners.
#[allow(clippy::too_many_arguments)]
pub async fn run_inner_with_metrics<P, F>(
    segments: Vec<(String, String)>,
    provider: &P,
    context: TranslationContext,
    db: &SqlitePool,
    app_handle: Option<&tauri::AppHandle>,
    cooldown: Option<&mut CooldownState>,
    global_progress: Option<(usize, usize)>,
    on_progress: F,
) -> Result<PipelineRunOutcome, PipelineError>
where
    P: LlmProvider,
    F: FnMut(usize, usize),
{
    run_controlled(
        segments,
        provider,
        context,
        db,
        app_handle,
        cooldown,
        global_progress,
        None,
        on_progress,
    )
    .await
}

/// Core loop. `cancel`, when set, stops the run before the next batch; the
/// batches already handled stay persisted and `cancelled` is reported.
#[allow(clippy::too_many_arguments)]
async fn run_controlled<P, F>(
    segments: Vec<(String, String)>,
    provider: &P,
    context: TranslationContext,
    db: &SqlitePool,
    app_handle: Option<&tauri::AppHandle>,
    mut cooldown: Option<&mut CooldownState>,
    global_progress: Option<(usize, usize)>,
    cancel: Option<&AtomicBool>,
    mut on_progress: F,
) -> Result<PipelineRunOutcome, PipelineError>
where
    P: LlmProvider,
    F: FnMut(usize, usize),
{
    let total = segments.len();
    let mut cancelled = false;
    let mut repaired = 0_usize;
    let lang_pair = format!("{}-{}", context.source_lang, context.target_lang);

    let batch_size = context.batch_size.clamp(1, 100);
    let mut results = Vec::with_capacity(total);
    let mut run_metrics = PipelineBatchMetrics::default();
    let mut done = 0usize;

    for batch_segments in segments.chunks(batch_size) {
        if cancel.is_some_and(|flag| flag.load(Ordering::SeqCst)) {
            cancelled = true;
            break;
        }
        let batch_outcome =
            translate_batch(batch_segments, provider, &context, &lang_pair, db).await;

        // A real Tauri session consumes metrics per batch for live reporting.
        // Tests and the upcoming isolated pilot keep them available for an
        // explicit `drain_metrics()` after `run_inner` returns.
        if let Some(handle) = app_handle {
            let metrics = provider.drain_metrics();
            if !metrics.is_empty() {
                let _ = handle.emit("h2s://llm/metrics", metrics);
            }
        }

        let (batch_results, pipeline_metrics) = batch_outcome.map_err(|error| match error {
            PipelineError::ProviderUnavailable { message, .. } => {
                PipelineError::ProviderUnavailable {
                    message,
                    processed: global_progress.map_or(0, |(offset, _)| offset) + done,
                }
            }
            other => other,
        })?;
        run_metrics.merge(pipeline_metrics.clone());

        if let Some(handle) = app_handle {
            let _ = handle.emit("h2s://llm/pipeline-metrics", pipeline_metrics);
        }

        repaired += persist_batch_results(db, &batch_results).await?;

        if let Some(handle) = app_handle {
            let updates: Vec<SegmentUpdatePayload> = batch_results
                .iter()
                .filter(|r| !r.failed)
                .map(|r| SegmentUpdatePayload {
                    id: r.id.clone(),
                    target_text: r.translated_text.clone(),
                    status: result_status(r).to_string(),
                })
                .collect();
            let _ = handle.emit("h2s://llm/segments-updated", updates);
        }

        done += batch_segments.len();
        let (emit_done, emit_total) = match global_progress {
            Some((offset, global_total)) => (offset + done, global_total),
            None => (done, total),
        };
        on_progress(emit_done, emit_total);

        if let Some(handle) = app_handle {
            let _ = handle.emit(
                "h2s://llm/progress",
                ProgressPayload {
                    done: emit_done,
                    total: emit_total,
                },
            );
        }

        if let (Some(cd), Some(handle)) = (cooldown.as_deref_mut(), app_handle) {
            cd.maybe_rest(handle, cancel).await;
        }

        results.extend(batch_results);

        if done < total && context.batch_delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(context.batch_delay_ms)).await;
        }
    }

    Ok(PipelineRunOutcome {
        results,
        metrics: run_metrics,
        cancelled,
        repaired,
    })
}

// ---------------------------------------------------------------------------
// Entry point used by Tauri commands (emits events via AppHandle)
// ---------------------------------------------------------------------------

/// Tauri-aware wrapper around `run_inner`. Persists each batch to the DB,
/// emits `h2s://llm/segments-updated` and `h2s://llm/progress`, stops before
/// the next batch once `cancel` is set, and — if `cooldown` is provided —
/// applies the automatic cooldown between batches.
#[allow(clippy::too_many_arguments)]
pub async fn run<P>(
    segments: Vec<(String, String)>,
    provider: &P,
    context: TranslationContext,
    db: &SqlitePool,
    app_handle: &tauri::AppHandle,
    cooldown: Option<&mut CooldownState>,
    global_progress: Option<(usize, usize)>,
    cancel: &AtomicBool,
) -> Result<PipelineRunOutcome, PipelineError>
where
    P: LlmProvider,
{
    run_controlled(
        segments,
        provider,
        context,
        db,
        Some(app_handle),
        cooldown,
        global_progress,
        Some(cancel),
        |_, _| {},
    )
    .await
}

// ---------------------------------------------------------------------------
// Batch translation (TM → LLM with adaptive split)
// ---------------------------------------------------------------------------

async fn translate_batch<P>(
    segments: &[(String, String)],
    provider: &P,
    context: &TranslationContext,
    lang_pair: &str,
    db: &SqlitePool,
) -> Result<(Vec<TranslationResult>, PipelineBatchMetrics), PipelineError>
where
    P: LlmProvider,
{
    let batch_len = segments.len();
    let segment_ids: Vec<String> = segments.iter().map(|(id, _)| id.clone()).collect();
    let qa_rules = resolver::resolve_qa_rules_for_segments(db, &segment_ids, &context.target_lang)
        .await
        .map_err(|error| PipelineError::Database(error.to_string()))?;
    let context_bundles = context::build_context_bundles(db, &context.engine, &segment_ids)
        .await
        .map_err(|error| PipelineError::Database(error.to_string()))?;
    let prompt_contexts = context_bundles
        .iter()
        .map(|bundle| match context.prompt_context_policy {
            PromptContextPolicy::Disabled => None,
            PromptContextPolicy::EngineOwned => bundle.prompt.clone(),
        })
        .collect::<Vec<_>>();
    // Placeholder syntax comes from persisted segment facts, never from the
    // prompt policy: `%1` in a System term must be protected either way.
    let placeholder_syntaxes = context_bundles
        .iter()
        .map(|bundle| bundle.tokenizer(&context.engine))
        .collect::<Vec<_>>();
    let (unique_segs, idx_map) =
        batch::dedup_with_context(segments, &prompt_contexts, &placeholder_syntaxes);

    // Translation table: (translated_text, from_tm); `None` text = failed.
    let mut translations: Vec<Option<(Option<String>, bool)>> = vec![None; batch_len];
    let mut pipeline_metrics = PipelineBatchMetrics::default();

    // Track which unique segments still need LLM
    let mut to_translate: Vec<usize> = Vec::new(); // indices into `unique_segs`

    let hashes: Vec<String> = unique_segs
        .iter()
        .map(|segment| segment.hash.clone())
        .collect();
    let tm_hits = tm::lookup_exact_many(&hashes, lang_pair, db)
        .await
        .map_err(|error| PipelineError::Database(error.to_string()))?;

    for (unique_idx, seg) in unique_segs.iter().enumerate() {
        let original_index = seg.original_index;
        let is_canonical = context.engine == "mv_mz"
            && prompt_contexts
                .get(original_index)
                .and_then(Option::as_ref)
                .map(|prompt_context| {
                    context::mv_mz_context_class(&prompt_context.segment_kind)
                        == PromptContextClass::Canonical
                })
                .unwrap_or(false);
        if is_canonical {
            let reused: Option<String> = sqlx::query_scalar(
                "SELECT candidate.target_text FROM segments current \
                 JOIN source_files current_file ON current.source_file_id = current_file.id \
                 JOIN source_files candidate_file \
                   ON candidate_file.project_id = current_file.project_id \
                 JOIN segments candidate ON candidate.source_file_id = candidate_file.id \
                 WHERE current.id = ? AND candidate.id != current.id \
                   AND candidate.segment_kind = current.segment_kind \
                   AND candidate.source_text = current.source_text \
                   AND candidate.status IN ('translated', 'reviewed') \
                   AND TRIM(candidate.target_text) != '' \
                 ORDER BY candidate.updated_at DESC, candidate.rowid LIMIT 1",
            )
            .bind(&seg.id)
            .fetch_optional(db)
            .await
            .map_err(|error| PipelineError::Database(error.to_string()))?;
            if let Some(target) = reused {
                for &orig_idx in idx_map.get(&seg.dedup_key).into_iter().flatten() {
                    translations[orig_idx] = Some((Some(target.clone()), true));
                }
                continue;
            }
        }
        // The current TM schema is source-hash-only. It cannot prove that an
        // MV/MZ dialogue/name came from the same semantic kind or scene, so do
        // not let a historical ambiguous entry bypass engine-owned context.
        let allows_unscoped_tm = context.engine != "mv_mz"
            || prompt_contexts
                .get(original_index)
                .and_then(Option::as_ref)
                .map(|prompt_context| {
                    context::mv_mz_context_class(&prompt_context.segment_kind)
                        == PromptContextClass::Isolated
                })
                .unwrap_or(true);
        if allows_unscoped_tm {
            if let Some(entry) = tm_hits.get(&seg.hash) {
                for &orig_idx in idx_map.get(&seg.dedup_key).into_iter().flatten() {
                    translations[orig_idx] = Some((Some(entry.target_text.clone()), true));
                }
                continue;
            }
        }
        to_translate.push(unique_idx);
    }

    // Translate remaining segments via LLM with adaptive split on failure
    if !to_translate.is_empty() {
        let tokenized: Vec<_> = to_translate
            .iter()
            .map(|&i| {
                let segment = &unique_segs[i];
                Tokenizer::tokenize(&segment.text, placeholder_syntaxes[segment.original_index])
            })
            .collect();

        let mut request_context = context.clone();
        request_context.segment_contexts = to_translate
            .iter()
            .map(|&unique_idx| prompt_contexts[unique_segs[unique_idx].original_index].clone())
            .collect();
        let request_segment_ids = to_translate
            .iter()
            .map(|&unique_idx| unique_segs[unique_idx].id.clone())
            .collect::<Vec<_>>();
        // Never mix names/terms with dialogue in one prompt. Small contextual
        // groups reduce neighbour contamination and peak context size on local
        // models, while isolated database strings retain the configured limit.
        let mut grouped = std::collections::HashMap::<PromptContextClass, Vec<usize>>::new();
        for local_idx in 0..to_translate.len() {
            let class = request_context
                .segment_contexts
                .get(local_idx)
                .and_then(Option::as_ref)
                .map(|prompt_context| {
                    if context.engine == "mv_mz" {
                        context::mv_mz_context_class(&prompt_context.segment_kind)
                    } else {
                        PromptContextClass::Isolated
                    }
                })
                .unwrap_or(PromptContextClass::Isolated);
            grouped.entry(class).or_default().push(local_idx);
        }

        let mut llm_results = Vec::with_capacity(to_translate.len());
        for class in [
            PromptContextClass::Canonical,
            PromptContextClass::Isolated,
            PromptContextClass::Branch,
            PromptContextClass::Dialogue,
        ] {
            let Some(indices) = grouped.remove(&class) else {
                continue;
            };
            let limit = match class {
                PromptContextClass::Dialogue => 8,
                PromptContextClass::Branch => 10,
                PromptContextClass::Canonical | PromptContextClass::Isolated => {
                    context.batch_size.clamp(1, 100)
                }
            };
            for chunk in indices.chunks(limit) {
                let outcome = llm_translate_with_split(
                    chunk.to_vec(),
                    &tokenized,
                    provider,
                    &request_context,
                    db,
                    &request_segment_ids,
                )
                .await
                .map_err(|error| PipelineError::ProviderUnavailable {
                    message: error.to_string(),
                    processed: 0,
                })?;
                pipeline_metrics.merge(outcome.metrics);
                llm_results.extend(outcome.results);
            }
        }

        for (local_idx, text) in llm_results {
            let global_unique_idx = to_translate[local_idx]; // LOCAL → GLOBAL
            if text.is_none() {
                log::warn!(
                    "[h2s] segment '{}' failed after adaptive split — left unchanged",
                    unique_segs[global_unique_idx].id
                );
            }
            for &orig_idx in idx_map
                .get(&unique_segs[global_unique_idx].dedup_key)
                .into_iter()
                .flatten()
            {
                translations[orig_idx] = Some((text.clone(), false));
            }
        }
    }

    // Build final Vec<TranslationResult> (same order as input `segments`)
    let mut results = Vec::with_capacity(segments.len());
    for (index, (id, source_text)) in segments.iter().enumerate() {
        let (translated, from_tm) = translations[index].take().unwrap_or((None, false));
        let Some(mut raw_target) = translated.filter(|text| !text.trim().is_empty()) else {
            results.push(TranslationResult {
                id: id.clone(),
                translated_text: String::new(),
                from_tm,
                needs_review: false,
                failed: true,
                qa_score: 0,
                qa_errors: Vec::new(),
            });
            continue;
        };
        let prompt_context = prompt_contexts.get(index).and_then(Option::as_ref);
        let bundle = &context_bundles[index];
        let qa_neighbors = &bundle.qa_neighbors;
        let segment_kind = bundle
            .segment_kind
            .as_deref()
            .or_else(|| prompt_context.map(|value| value.segment_kind.as_str()))
            .unwrap_or("unknown");
        let context_json = bundle.context_json.as_deref();
        let json_key = bundle.json_key.as_deref();
        let qa_context = qa::QaSemanticContext {
            source_language: &context.source_lang,
            target_language: &context.target_lang,
            segment_kind,
            context_json,
            json_key,
            neighbor_sources: &qa_neighbors.sources,
            neighbor_targets: &qa_neighbors.targets,
        };
        let segment_rules = qa_rules.get(id).map(Vec::as_slice).unwrap_or(&[]);
        let mut qa_result = qa::check_with_context(
            source_text,
            &raw_target,
            segment_rules,
            &context.engine,
            &qa_context,
        );
        if qa_result.has_critical_errors() {
            pipeline_metrics.semantic_rejections += 1;
            if !from_tm {
                let tokenized = Tokenizer::tokenize(source_text, placeholder_syntaxes[index]);
                let retry = semantic_retry::retry(
                    SemanticRetryRequest {
                        source_text,
                        tokenized: &tokenized,
                        translation_context: context,
                        segment_id: id,
                        terminology_rules: segment_rules,
                        prompt_context,
                        segment_kind,
                        context_json,
                        json_key,
                        qa_neighbors,
                    },
                    provider,
                    db,
                )
                .await;
                pipeline_metrics.merge(retry.metrics);
                if let Some(recovered_target) = retry.recovered_target {
                    raw_target = recovered_target;
                    qa_result = qa::check_with_context(
                        source_text,
                        &raw_target,
                        segment_rules,
                        &context.engine,
                        &qa_context,
                    );
                }
            }
        }
        let needs_review = qa_result.has_critical_errors();
        results.push(TranslationResult {
            id: id.clone(),
            translated_text: raw_target,
            from_tm,
            needs_review,
            failed: false,
            qa_score: qa_result.score,
            qa_errors: qa_result.errors,
        });
    }
    Ok((results, pipeline_metrics))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::pool::init;
    use tempfile::NamedTempFile;

    // ── Mock provider ────────────────────────────────────────────────────────

    struct MockProvider {
        /// Responses returned in sequence.  Each call pops the front.
        responses: std::sync::Mutex<std::collections::VecDeque<Result<Vec<String>, LlmError>>>,
        call_count: std::sync::atomic::AtomicU32,
        received_contexts: std::sync::Mutex<Vec<TranslationContext>>,
    }

    impl MockProvider {
        fn new(responses: Vec<Result<Vec<String>, LlmError>>) -> Self {
            Self {
                responses: std::sync::Mutex::new(responses.into()),
                call_count: std::sync::atomic::AtomicU32::new(0),
                received_contexts: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn calls(&self) -> u32 {
            self.call_count.load(std::sync::atomic::Ordering::SeqCst)
        }

        fn last_context(&self) -> TranslationContext {
            self.received_contexts
                .lock()
                .unwrap()
                .last()
                .unwrap()
                .clone()
        }

        fn contexts(&self) -> Vec<TranslationContext> {
            self.received_contexts.lock().unwrap().clone()
        }
    }

    impl LlmProvider for MockProvider {
        async fn translate(
            &self,
            _segments: Vec<String>,
            context: TranslationContext,
        ) -> Result<Vec<String>, LlmError> {
            self.call_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.received_contexts.lock().unwrap().push(context);
            let mut q = self.responses.lock().unwrap();
            match q.pop_front() {
                Some(result) => result,
                None => panic!("MockProvider: no more responses queued"),
            }
        }

        async fn health_check(&self) -> Result<(), LlmError> {
            Ok(())
        }

        async fn chat(&self, _system: &str, _user: &str) -> Result<String, LlmError> {
            Ok(String::new())
        }
    }

    // ── Helpers ──────────────────────────────────────────────────────────────

    async fn test_db() -> (sqlx::SqlitePool, NamedTempFile) {
        let file = NamedTempFile::new().expect("tempfile");
        let path = file.path().to_str().expect("utf-8").to_string();
        let pool = init(&path).await.expect("pool");
        (pool, file)
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

    async fn insert_dialogue_scene(db: &SqlitePool) {
        sqlx::query(
            "INSERT INTO projects (id, name, engine, game_path) \
             VALUES ('p1', 'Test', 'mv_mz', '/tmp')",
        )
        .execute(db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
             VALUES ('f1', 'p1', 'Map001.json', '/tmp/Map001.json', 'map')",
        )
        .execute(db)
        .await
        .unwrap();
        for (id, sequence, text, speaker) in [
            ("before", 0, "待って", Some("仲間")),
            ("center", 1, "行こう", Some("勇者")),
            ("after", 2, "はい", Some("仲間")),
        ] {
            sqlx::query(
                "INSERT INTO segments \
                 (id, source_file_id, json_key, source_text, segment_kind, scene_id, \
                  sequence_index, speaker) \
                 VALUES (?, 'f1', ?, ?, 'dialogue', 'Map001.json:event:1:page:0', ?, ?)",
            )
            .bind(id)
            .bind(format!("/{id}"))
            .bind(text)
            .bind(sequence)
            .bind(speaker)
            .execute(db)
            .await
            .unwrap();
        }
    }

    // ── Tests ────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_tm_hit_skips_llm() {
        let (db, _f) = test_db().await;

        // Pre-populate TM
        tm::insert("主人公", "Hero", "mv_mz", "ja-en", &db)
            .await
            .unwrap();

        // Provider should NOT be called
        let provider = MockProvider::new(vec![]);

        let mut progress_calls = vec![];
        let results = run_inner(
            vec![("seg1".to_string(), "主人公".to_string())],
            &provider,
            ctx(),
            &db,
            None,
            None,
            None,
            |d, t| progress_calls.push((d, t)),
        )
        .await
        .unwrap();

        assert_eq!(provider.calls(), 0, "LLM must not be called on TM hit");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].translated_text, "Hero");
        assert!(results[0].from_tm);
    }

    #[tokio::test]
    async fn test_pipeline_calls_llm_on_cache_miss() {
        let (db, _f) = test_db().await;
        // MockProvider contract: return final translations (no numbering)
        let provider = MockProvider::new(vec![Ok(vec!["Hero".to_string()])]);

        let results = run_inner(
            vec![("seg1".to_string(), "主人公".to_string())],
            &provider,
            ctx(),
            &db,
            None,
            None,
            None,
            |_, _| {},
        )
        .await
        .unwrap();

        assert_eq!(provider.calls(), 1);
        assert_eq!(results[0].translated_text, "Hero");
        assert!(!results[0].from_tm);
    }

    #[tokio::test]
    async fn mv_mz_context_reaches_provider_with_bounded_neighbors() {
        let (db, _f) = test_db().await;
        insert_dialogue_scene(&db).await;
        let provider = MockProvider::new(vec![Ok(vec!["Let's go".to_string()])]);

        run_inner(
            vec![("center".to_string(), "行こう".to_string())],
            &provider,
            ctx(),
            &db,
            None,
            None,
            None,
            |_, _| {},
        )
        .await
        .unwrap();
        let received = provider.last_context();
        let prompt_context = received.segment_contexts[0].as_ref().unwrap();

        assert_eq!(prompt_context.segment_kind, "dialogue");
        assert_eq!(prompt_context.speaker.as_deref(), Some("勇者"));
        assert_eq!(prompt_context.previous[0].text, "待って");
        assert_eq!(prompt_context.following[0].text, "はい");
    }

    #[tokio::test]
    async fn semantic_failure_retries_singleton_without_neighbors() {
        let (db, _f) = test_db().await;
        insert_dialogue_scene(&db).await;
        let contaminated = "Let's go. This output wrongly continues with a very long neighbouring conversation that does not belong to the source segment at all.";
        let provider = MockProvider::new(vec![
            Ok(vec![contaminated.to_string()]),
            Ok(vec!["Let's go.".to_string()]),
        ]);

        let (results, metrics) = translate_batch(
            &[("center".to_string(), "行こう".to_string())],
            &provider,
            &ctx(),
            "ja-en",
            &db,
        )
        .await
        .unwrap();

        assert_eq!(provider.calls(), 2);
        assert_eq!(results[0].translated_text, "Let's go.");
        assert!(!results[0].needs_review);
        assert_eq!(metrics.semantic_rejections, 1);
        assert_eq!(metrics.semantic_retries, 1);
        assert_eq!(metrics.semantic_recoveries, 1);
        let contexts = provider.contexts();
        let retry = contexts[1].segment_contexts[0].as_ref().unwrap();
        assert!(retry.previous.is_empty());
        assert!(retry.following.is_empty());
        assert_eq!(retry.speaker.as_deref(), Some("勇者"));
    }

    #[tokio::test]
    async fn exhausted_semantic_retries_keep_first_output_for_review() {
        let (db, _f) = test_db().await;
        insert_dialogue_scene(&db).await;
        let first = "Let's go. This output wrongly continues with a very long neighbouring conversation that does not belong to the source segment at all.";
        let second = "Another excessively expanded answer that keeps inventing unrelated dialogue and remains far too long for this tiny source.";
        let third = "A final excessively expanded answer that still invents unrelated dialogue and must never be silently accepted.";
        let provider = MockProvider::new(vec![
            Ok(vec![first.to_string()]),
            Ok(vec![second.to_string()]),
            Ok(vec![third.to_string()]),
        ]);

        let (results, metrics) = translate_batch(
            &[("center".to_string(), "行こう".to_string())],
            &provider,
            &ctx(),
            "ja-en",
            &db,
        )
        .await
        .unwrap();

        assert_eq!(provider.calls(), 3);
        assert_eq!(results[0].translated_text, first);
        assert!(results[0].needs_review);
        assert_eq!(metrics.semantic_rejections, 1);
        assert_eq!(metrics.semantic_retries, 2);
        assert_eq!(metrics.semantic_recoveries, 0);
        assert_eq!(provider.contexts()[2].segment_contexts, vec![None]);
    }

    #[tokio::test]
    async fn translated_neighbor_overlap_triggers_isolated_retry() {
        let (db, _f) = test_db().await;
        insert_dialogue_scene(&db).await;
        sqlx::query(
            "UPDATE segments SET target_text = 'We should go and continue the mission.', \
                    status = 'translated' WHERE id = 'after'",
        )
        .execute(&db)
        .await
        .unwrap();
        let provider = MockProvider::new(vec![
            Ok(vec!["Let's go and continue the mission.".to_string()]),
            Ok(vec!["Let's go.".to_string()]),
        ]);

        let results = run_inner(
            vec![("center".to_string(), "行こう".to_string())],
            &provider,
            ctx(),
            &db,
            None,
            None,
            None,
            |_, _| {},
        )
        .await
        .unwrap();

        assert_eq!(provider.calls(), 2);
        assert_eq!(results[0].translated_text, "Let's go.");
        assert!(!results[0].needs_review);
    }

    #[tokio::test]
    async fn canonical_speaker_translation_is_reused_across_pipeline_runs() {
        let (db, _f) = test_db().await;
        sqlx::query(
            "INSERT INTO projects (id, name, engine, game_path) \
             VALUES ('p1', 'Test', 'mv_mz', '/tmp')",
        )
        .execute(&db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
             VALUES ('f1', 'p1', 'Map001.json', '/tmp/Map001.json', 'map')",
        )
        .execute(&db)
        .await
        .unwrap();
        for id in ["speaker-1", "speaker-2"] {
            sqlx::query(
                "INSERT INTO segments \
                 (id, source_file_id, json_key, source_text, segment_kind, scene_id, \
                  sequence_index, speaker) \
                 VALUES (?, 'f1', ?, '見知らぬ男性', 'speaker', \
                         'Map001.json:event:1:page:0', 0, '見知らぬ男性')",
            )
            .bind(id)
            .bind(format!("/{id}"))
            .execute(&db)
            .await
            .unwrap();
        }

        let first_provider = MockProvider::new(vec![Ok(vec!["Homme inconnu".to_string()])]);
        let first = run_inner(
            vec![("speaker-1".into(), "見知らぬ男性".into())],
            &first_provider,
            ctx(),
            &db,
            None,
            None,
            None,
            |_, _| {},
        )
        .await
        .unwrap();
        assert_eq!(first[0].qa_score, 100);

        let second_provider = MockProvider::new(vec![]);
        let second = run_inner(
            vec![("speaker-2".into(), "見知らぬ男性".into())],
            &second_provider,
            ctx(),
            &db,
            None,
            None,
            None,
            |_, _| {},
        )
        .await
        .unwrap();

        assert_eq!(second_provider.calls(), 0);
        assert_eq!(second[0].translated_text, "Homme inconnu");
        assert!(second[0].from_tm);
        let persisted: (String, i64) =
            sqlx::query_as("SELECT status, qa_score FROM segments WHERE id = 'speaker-2'")
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!(persisted, ("translated".to_string(), 100));
    }

    #[tokio::test]
    async fn later_canonical_success_repairs_an_earlier_source_fallback() {
        let (db, _f) = test_db().await;
        sqlx::query(
            "INSERT INTO projects (id, name, engine, game_path) \
             VALUES ('p1', 'Test', 'mv_mz', '/tmp')",
        )
        .execute(&db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
             VALUES ('f1', 'p1', 'Map001.json', '/tmp/Map001.json', 'map')",
        )
        .execute(&db)
        .await
        .unwrap();
        for id in ["speaker-failed", "speaker-success"] {
            sqlx::query(
                "INSERT INTO segments \
                 (id, source_file_id, json_key, source_text, segment_kind, scene_id, \
                  sequence_index, speaker) \
                 VALUES (?, 'f1', ?, '見知らぬ男性', 'speaker', \
                         'Map001.json:event:1:page:0', 0, '見知らぬ男性')",
            )
            .bind(id)
            .bind(format!("/{id}"))
            .execute(&db)
            .await
            .unwrap();
        }

        // State left by versions before R1, which copied the source into the
        // target on failure. The pipeline no longer writes it, but old
        // projects still contain such rows.
        sqlx::query(
            "UPDATE segments SET target_text = source_text, status = 'needs_review', \
                    qa_score = 0 WHERE id = 'speaker-failed'",
        )
        .execute(&db)
        .await
        .unwrap();

        let success_provider = MockProvider::new(vec![Ok(vec!["Homme inconnu".into()])]);
        let outcome = run_inner_with_metrics(
            vec![("speaker-success".into(), "見知らぬ男性".into())],
            &success_provider,
            ctx(),
            &db,
            None,
            None,
            None,
            |_, _| {},
        )
        .await
        .unwrap();
        assert_eq!(outcome.results.len(), 1);
        assert_eq!(outcome.repaired, 1, "the repaired fallback is reported");

        let repaired: (String, String, i64) = sqlx::query_as(
            "SELECT target_text, status, qa_score FROM segments WHERE id = 'speaker-failed'",
        )
        .fetch_one(&db)
        .await
        .unwrap();
        assert_eq!(
            repaired,
            ("Homme inconnu".to_string(), "translated".to_string(), 100)
        );
    }

    #[tokio::test]
    async fn dialogue_groups_are_capped_at_eight_units() {
        let (db, _f) = test_db().await;
        sqlx::query(
            "INSERT INTO projects (id, name, engine, game_path) \
             VALUES ('p1', 'Test', 'mv_mz', '/tmp')",
        )
        .execute(&db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
             VALUES ('f1', 'p1', 'Map001.json', '/tmp/Map001.json', 'map')",
        )
        .execute(&db)
        .await
        .unwrap();
        let mut segments = Vec::new();
        for index in 0..9 {
            let id = format!("d{index}");
            let source = format!("台詞{index}");
            sqlx::query(
                "INSERT INTO segments \
                 (id, source_file_id, json_key, source_text, segment_kind, scene_id, \
                  sequence_index) VALUES (?, 'f1', ?, ?, 'dialogue', ?, ?)",
            )
            .bind(&id)
            .bind(format!("/{id}"))
            .bind(&source)
            .bind(format!("scene-{index}"))
            .bind(i64::from(index))
            .execute(&db)
            .await
            .unwrap();
            segments.push((id, source));
        }
        let provider = MockProvider::new(vec![
            Ok((0..8).map(|index| format!("Réplique {index}")).collect()),
            Ok(vec!["Réplique 8".to_string()]),
        ]);

        let results = run_inner(segments, &provider, ctx(), &db, None, None, None, |_, _| {})
            .await
            .unwrap();

        assert_eq!(provider.calls(), 2);
        assert_eq!(provider.contexts()[0].segment_contexts.len(), 8);
        assert_eq!(provider.contexts()[1].segment_contexts.len(), 1);
        assert!(results.iter().all(|result| !result.needs_review));
    }

    #[tokio::test]
    async fn canonical_and_dialogue_requests_receive_only_their_own_terminology() {
        let (db, _f) = test_db().await;
        sqlx::query(
            "INSERT INTO projects (id, name, engine, game_path, source_language, target_language) \
             VALUES ('term-p', 'Terms', 'mv_mz', '/tmp', 'ja', 'en')",
        )
        .execute(&db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
             VALUES ('term-f', 'term-p', 'Map001.json', '/tmp/Map001.json', 'map')",
        )
        .execute(&db)
        .await
        .unwrap();
        for (id, source, kind, sequence) in [
            ("term-name", "勇者", "actor_name", 0_i64),
            ("term-dialogue", "剣を取れ", "dialogue", 1_i64),
        ] {
            sqlx::query(
                "INSERT INTO segments \
                 (id, source_file_id, json_key, source_text, segment_kind, scene_id, \
                  sequence_index, speaker) \
                 VALUES (?, 'term-f', ?, ?, ?, 'Map001.json:event:1:page:0', ?, '王')",
            )
            .bind(id)
            .bind(format!("/{id}"))
            .bind(source)
            .bind(kind)
            .bind(sequence)
            .execute(&db)
            .await
            .unwrap();
        }
        for (entry_id, source, semantic_type, segment_id, target) in [
            ("term-e-hero", "勇者", "character", "term-name", "Hero"),
            ("term-e-sword", "剣", "item", "term-dialogue", "Sword"),
        ] {
            sqlx::query(
                "INSERT INTO terminology_entries (\
                    id, source_language, canonical_text, normalized_text, part_of_speech, \
                    semantic_type, sense_key, status, origin, confidence\
                 ) VALUES (?, 'ja', ?, ?, 'noun', ?, '', 'active', 'manual', 1.0)",
            )
            .bind(entry_id)
            .bind(source)
            .bind(source)
            .bind(semantic_type)
            .execute(&db)
            .await
            .unwrap();
            sqlx::query(
                "INSERT INTO terminology_occurrences \
                 (entry_id, project_id, segment_id, surface_text, engine_kind, occurrence_count) \
                 VALUES (?, 'term-p', ?, ?, ?, 1)",
            )
            .bind(entry_id)
            .bind(segment_id)
            .bind(source)
            .bind(if segment_id == "term-name" {
                "actor_name"
            } else {
                "dialogue"
            })
            .execute(&db)
            .await
            .unwrap();
            sqlx::query(
                "INSERT INTO terminology_translations (\
                    id, entry_id, target_language, target_text, enforcement, confidence\
                 ) VALUES (?, ?, 'en', ?, 'required', 1.0)",
            )
            .bind(format!("translation-{entry_id}"))
            .bind(entry_id)
            .bind(target)
            .execute(&db)
            .await
            .unwrap();
        }

        let provider = MockProvider::new(vec![
            Ok(vec!["Hero".to_string()]),
            Ok(vec!["Take the Sword".to_string()]),
        ]);
        run_inner(
            vec![
                ("term-name".to_string(), "勇者".to_string()),
                ("term-dialogue".to_string(), "剣を取れ".to_string()),
            ],
            &provider,
            ctx(),
            &db,
            None,
            None,
            None,
            |_, _| {},
        )
        .await
        .unwrap();

        let contexts = provider.contexts();
        assert_eq!(contexts.len(), 2);
        assert_eq!(contexts[0].terminology_hints.len(), 1);
        assert_eq!(contexts[0].terminology_hints[0].source, "勇者");
        assert_eq!(contexts[1].terminology_hints.len(), 1);
        assert_eq!(contexts[1].terminology_hints[0].source, "剣");
    }

    #[tokio::test]
    async fn disabled_context_policy_keeps_mv_mz_tokenization_without_prompt_context() {
        let (db, _f) = test_db().await;
        let provider = MockProvider::new(vec![Ok(vec!["Hero ⟦ph_0⟧".to_string()])]);
        let mut context = ctx();
        context.prompt_context_policy = PromptContextPolicy::Disabled;

        let results = run_inner(
            vec![("missing".to_string(), r"勇者\V[1]".to_string())],
            &provider,
            context,
            &db,
            None,
            None,
            None,
            |_, _| {},
        )
        .await
        .unwrap();

        assert_eq!(results[0].translated_text, r"Hero \V[1]");
        assert_eq!(provider.last_context().segment_contexts, vec![None]);
    }

    #[tokio::test]
    async fn system_term_format_specifiers_are_protected_under_every_context_policy() {
        for policy in [
            PromptContextPolicy::EngineOwned,
            PromptContextPolicy::Disabled,
        ] {
            let (db, _f) = test_db().await;
            sqlx::query(
                "INSERT INTO projects (id, name, engine, game_path) \
                 VALUES ('p1', 'Test', 'mv_mz', '/tmp')",
            )
            .execute(&db)
            .await
            .unwrap();
            sqlx::query(
                "INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
                 VALUES ('fs', 'p1', 'System.json', '/tmp/System.json', 'system')",
            )
            .execute(&db)
            .await
            .unwrap();
            sqlx::query(
                "INSERT INTO segments (id, source_file_id, json_key, source_text, segment_kind) \
                 VALUES ('term', 'fs', '/terms/messages/actorDamage', ?, 'system_term')",
            )
            .bind("%1は%2のダメージを受けた！")
            .execute(&db)
            .await
            .unwrap();
            // The provider can only echo `%n` back if they were sent as tokens.
            let provider =
                MockProvider::new(vec![Ok(vec!["⟦ph_0⟧ took ⟦ph_1⟧ damage!".to_string()])]);
            let mut context = ctx();
            context.prompt_context_policy = policy;

            let results = run_inner(
                vec![("term".to_string(), "%1は%2のダメージを受けた！".to_string())],
                &provider,
                context,
                &db,
                None,
                None,
                None,
                |_, _| {},
            )
            .await
            .unwrap();

            assert_eq!(
                results[0].translated_text, "%1 took %2 damage!",
                "{policy:?}"
            );
            assert!(!results[0].needs_review, "{policy:?}");
        }
    }

    #[tokio::test]
    async fn test_wolf_engine_preserves_e_code() {
        let (db, _f) = test_db().await;
        // \E (Wolf "instant text" code) only exists in RE_WOLF, not RE_MVMZ.
        // With engine = "wolf", it must be tokenized so the LLM is told to
        // preserve it, then restored in the final translation.
        let provider = MockProvider::new(vec![Ok(vec!["⟦ph_0⟧Two months later...".to_string()])]);

        let mut wolf_ctx = ctx();
        wolf_ctx.engine = "wolf".to_string();

        let results = run_inner(
            vec![("seg1".to_string(), "\\E二ヶ月後……".to_string())],
            &provider,
            wolf_ctx,
            &db,
            None,
            None,
            None,
            |_, _| {},
        )
        .await
        .unwrap();

        assert_eq!(results[0].translated_text, "\\ETwo months later...");
        assert!(!results[0].needs_review);
        assert_eq!(provider.last_context().segment_contexts, vec![None]);
    }

    #[tokio::test]
    async fn wolf_message_line_breaks_are_not_imposed_on_the_llm() {
        // Plan WR-E6: the soft breaks of a Wolf message are sent joined; a
        // response without them is accepted and broken again into the
        // source's three lines (no « …the work ⏎. »).
        let (db, _f) = test_db().await;
        sqlx::query(
            "INSERT INTO projects (id, name, engine, game_path) \
             VALUES ('w-p', 'Wolf', 'wolf', '/tmp')",
        )
        .execute(&db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
             VALUES ('w-f', 'w-p', 'Map001.mps', '/tmp/Map001.mps', 'map')",
        )
        .execute(&db)
        .await
        .unwrap();
        let source = "@4\n短い内容となりますが、\n作品全体の雰囲気等をつかむ一助に\nお役立ていただければ幸いです。";
        sqlx::query(
            "INSERT INTO segments (id, source_file_id, json_key, source_text) \
             VALUES ('msg', 'w-f', 'MapData/Map001/events/0/pages/0/12', ?)",
        )
        .bind(source)
        .execute(&db)
        .await
        .unwrap();
        let english = "This is a short piece, but we hope it will be of some help in grasping \
                       the overall atmosphere of the work.";
        // The face prefix `@4⏎` is not sent (plan FP): no token at all.
        let provider = MockProvider::new(vec![Ok(vec![english.to_string()])]);
        let mut wolf_ctx = ctx();
        wolf_ctx.engine = "wolf".to_string();

        let results = run_inner(
            vec![("msg".to_string(), source.to_string())],
            &provider,
            wolf_ctx,
            &db,
            None,
            None,
            None,
            |_, _| {},
        )
        .await
        .unwrap();

        assert_eq!(provider.calls(), 1, "accepted without a retry");
        let target = &results[0].translated_text;
        let body = target.strip_prefix("@4\n").expect("face prefix kept");
        let lines: Vec<&str> = body.split('\n').collect();
        assert_eq!(lines.len(), 3, "{target}");
        assert!(lines
            .iter()
            .all(|line| line.chars().any(char::is_alphanumeric)));
        assert_eq!(lines.join(" "), english);
        assert!(!results[0].needs_review, "{:?}", results[0].qa_errors);
    }

    /// One Wolf common-event message (`CommonEvent.dat`, トロイメライ) in a
    /// fresh project, for the face prefix runs of plan FP.
    async fn insert_wolf_message(db: &SqlitePool, source: &str) {
        sqlx::query(
            "INSERT INTO projects (id, name, engine, game_path) \
             VALUES ('fp-p', 'Wolf', 'wolf', '/tmp')",
        )
        .execute(db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
             VALUES ('fp-f', 'fp-p', 'CommonEvent.dat', '/tmp/CommonEvent.dat', 'common_event')",
        )
        .execute(db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO segments (id, source_file_id, json_key, source_text) \
             VALUES ('fp-msg', 'fp-f', 'CommonEvents/Intro/3/7', ?)",
        )
        .bind(source)
        .execute(db)
        .await
        .unwrap();
    }

    async fn persisted_target(db: &SqlitePool) -> (String, String) {
        sqlx::query_as("SELECT target_text, status FROM segments WHERE id = 'fp-msg'")
            .fetch_one(db)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn wolf_face_prefix_is_kept_in_front_of_the_persisted_translation() {
        let (db, _f) = test_db().await;
        let source = "@1\n「\\cself[8]」を失った。";
        insert_wolf_message(&db, source).await;
        // The LLM only sees the body, so its answer has no prefix.
        let provider = MockProvider::new(vec![Ok(vec!["Lost 「⟦ph_0⟧」.".to_string()])]);
        let mut wolf_ctx = ctx();
        wolf_ctx.engine = "wolf".to_string();

        let results = run_inner(
            vec![("fp-msg".to_string(), source.to_string())],
            &provider,
            wolf_ctx,
            &db,
            None,
            None,
            None,
            |_, _| {},
        )
        .await
        .unwrap();

        assert_eq!(provider.calls(), 1);
        assert_eq!(results[0].translated_text, "@1\nLost 「\\cself[8]」.");
        assert!(!results[0].needs_review, "{:?}", results[0].qa_errors);
        assert!(!results[0]
            .qa_errors
            .iter()
            .any(|error| matches!(error, qa::QaError::MisplacedFacePrefix { .. })));
        let (target, status) = persisted_target(&db).await;
        assert!(target.starts_with("@1\n"), "{target}");
        assert_eq!(target, "@1\nLost 「\\cself[8]」.");
        assert_eq!(status, "translated");
    }

    #[tokio::test]
    async fn wolf_empty_answer_is_not_saved_as_a_bare_face_prefix() {
        // Before plan FP, an empty answer lost the prefix token and failed;
        // it must still fail now that the prefix is put back after restore.
        let (db, _f) = test_db().await;
        let source = "@1\nテキスト";
        insert_wolf_message(&db, source).await;
        let provider = MockProvider::new(vec![Ok(vec![String::new()])]);
        let mut wolf_ctx = ctx();
        wolf_ctx.engine = "wolf".to_string();

        let results = run_inner(
            vec![("fp-msg".to_string(), source.to_string())],
            &provider,
            wolf_ctx,
            &db,
            None,
            None,
            None,
            |_, _| {},
        )
        .await
        .unwrap();

        assert!(results[0].failed);
        let target: Option<String> =
            sqlx::query_scalar("SELECT target_text FROM segments WHERE id = 'fp-msg'")
                .fetch_one(&db)
                .await
                .unwrap();
        assert!(target.unwrap_or_default().is_empty(), "nothing written");
    }

    #[tokio::test]
    async fn wolf_face_prefix_written_inside_the_answer_needs_review() {
        // Old behaviour of the 2026-10-30 run: `@1⏎` moved inside the quotes.
        // The prefix is still put in front, and the stray copy is flagged.
        let (db, _f) = test_db().await;
        let source = "@1\n「\\cself[8]」を失った。";
        insert_wolf_message(&db, source).await;
        let moved = "Lost 「⟦ph_0⟧@1\n」.".to_string();
        // First answer, then the two semantic retries.
        let provider = MockProvider::new(vec![
            Ok(vec![moved.clone()]),
            Ok(vec![moved.clone()]),
            Ok(vec![moved]),
        ]);
        let mut wolf_ctx = ctx();
        wolf_ctx.engine = "wolf".to_string();

        let results = run_inner(
            vec![("fp-msg".to_string(), source.to_string())],
            &provider,
            wolf_ctx,
            &db,
            None,
            None,
            None,
            |_, _| {},
        )
        .await
        .unwrap();

        assert_eq!(provider.calls(), 3, "first answer and two semantic retries");
        assert!(results[0].needs_review);
        assert!(results[0]
            .qa_errors
            .contains(&qa::QaError::MisplacedFacePrefix {
                expected: "@1".to_string(),
                found: "@1".to_string(),
            }));
        let (target, status) = persisted_target(&db).await;
        assert!(target.starts_with("@1\n"), "{target}");
        assert_eq!(status, "needs_review");
    }

    #[tokio::test]
    async fn test_invalid_placeholder_triggers_retry() {
        let (db, _f) = test_db().await;

        // First response drops the token → Tokenizer::restore fails → retry
        // Second response preserves the token → restore succeeds → \V[12] coins
        let provider = MockProvider::new(vec![
            Ok(vec!["lost token".to_string()]),   // ⟦ph_0⟧ absent → retry
            Ok(vec!["⟦ph_0⟧ coins".to_string()]), // ⟦ph_0⟧ present → \V[12] coins
        ]);

        let results = run_inner(
            vec![("s1".to_string(), r"\V[12] pièces".to_string())],
            &provider,
            ctx(),
            &db,
            None,
            None,
            None,
            |_, _| {},
        )
        .await
        .unwrap();

        assert_eq!(provider.calls(), 2, "should retry once");
        assert_eq!(results[0].translated_text, r"\V[12] coins");
    }

    #[tokio::test]
    async fn test_response_format_error_triggers_retry() {
        let (db, _f) = test_db().await;

        // Premier appel : ResponseFormat (ex: qwen3 retourne vide après strip)
        // Deuxième appel : réponse correcte
        let provider = MockProvider::new(vec![
            Err(LlmError::ResponseFormat(
                "expected 1 lines, got 0".to_string(),
            )),
            Ok(vec!["Hero".to_string()]),
        ]);

        let results = run_inner(
            vec![("s1".to_string(), "主人公".to_string())],
            &provider,
            ctx(),
            &db,
            None,
            None,
            None,
            |_, _| {},
        )
        .await
        .unwrap();

        assert_eq!(provider.calls(), 2, "should retry once on ResponseFormat");
        assert_eq!(results[0].translated_text, "Hero");
        assert!(!results[0].from_tm);
    }

    #[tokio::test]
    async fn test_placeholder_failure_leaves_segment_unchanged() {
        let (db, _f) = test_db().await;

        // MockProvider always returns a response without ⟦ph_0⟧ — fails restore every time.
        // After MAX_RETRIES (3) the segment is reported failed and nothing is written.
        let provider = MockProvider::new(vec![
            Ok(vec!["lost token".to_string()]),
            Ok(vec!["lost token".to_string()]),
            Ok(vec!["lost token".to_string()]),
        ]);

        let results = run_inner(
            vec![("s1".to_string(), r"\V[12] pièces".to_string())],
            &provider,
            ctx(),
            &db,
            None,
            None,
            None,
            |_, _| {},
        )
        .await
        .unwrap();

        assert_eq!(provider.calls(), 3, "should exhaust all retries");
        assert!(results[0].failed, "segment must be reported failed");
        assert!(!results[0].needs_review);
        assert_eq!(
            results[0].translated_text, "",
            "source is never used as target"
        );
    }

    #[tokio::test]
    async fn test_response_format_exhausted_leaves_segment_unchanged() {
        let (db, _f) = test_db().await;

        // Provider always returns ResponseFormat — should exhaust MAX_RETRIES (3)
        // and report the segment failed instead of crashing the whole batch.
        let provider = MockProvider::new(vec![
            Err(LlmError::ResponseFormat(
                "missing translation for line 2".to_string(),
            )),
            Err(LlmError::ResponseFormat(
                "missing translation for line 2".to_string(),
            )),
            Err(LlmError::ResponseFormat(
                "missing translation for line 2".to_string(),
            )),
        ]);

        let results = run_inner(
            vec![("s1".to_string(), "ポーション".to_string())],
            &provider,
            ctx(),
            &db,
            None,
            None,
            None,
            |_, _| {},
        )
        .await
        .unwrap();

        assert_eq!(provider.calls(), 3, "should exhaust all retries");
        assert!(results[0].failed, "segment must be reported failed");
        assert!(!results[0].needs_review);
        assert_eq!(
            results[0].translated_text, "",
            "source is never used as target"
        );
    }

    #[tokio::test]
    async fn test_response_format_triggers_split() {
        let (db, _f) = test_db().await;
        sqlx::query(
            "INSERT INTO projects (id, name, engine, game_path) \
             VALUES ('split-p', 'Split', 'mv_mz', '/tmp')",
        )
        .execute(&db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
             VALUES ('split-f', 'split-p', 'Map001.json', '/tmp/Map001.json', 'map')",
        )
        .execute(&db)
        .await
        .unwrap();
        for (id, sequence, text, speaker) in [("s1", 0, "一", "勇者"), ("s2", 1, "二", "魔王")]
        {
            sqlx::query(
                "INSERT INTO segments \
                 (id, source_file_id, json_key, source_text, segment_kind, scene_id, \
                  sequence_index, speaker) \
                 VALUES (?, 'split-f', ?, ?, 'dialogue', \
                         'Map001.json:event:1:page:0', ?, ?)",
            )
            .bind(id)
            .bind(format!("/{id}"))
            .bind(text)
            .bind(sequence)
            .bind(speaker)
            .execute(&db)
            .await
            .unwrap();
        }
        let rf = || Err(LlmError::ResponseFormat("bad format".to_string()));
        let provider = MockProvider::new(vec![
            rf(),
            rf(),
            rf(),                      // batch [s1,s2] — 3 retries → split
            Ok(vec!["A".to_string()]), // [s1] alone → ok
            Ok(vec!["B".to_string()]), // [s2] alone → ok
        ]);

        let results = run_inner(
            vec![
                ("s1".to_string(), "一".to_string()),
                ("s2".to_string(), "二".to_string()),
            ],
            &provider,
            ctx(),
            &db,
            None,
            None,
            None,
            |_, _| {},
        )
        .await
        .unwrap();

        assert_eq!(provider.calls(), 5);
        assert_eq!(results[0].translated_text, "A");
        assert!(!results[0].needs_review);
        assert_eq!(results[1].translated_text, "B");
        assert!(!results[1].needs_review);
        let received_contexts = provider.contexts();
        assert!(received_contexts[..3]
            .iter()
            .all(|context| context.segment_contexts.len() == 2));
        assert_eq!(
            received_contexts[3].segment_contexts[0]
                .as_ref()
                .unwrap()
                .speaker
                .as_deref(),
            Some("勇者")
        );
        assert_eq!(
            received_contexts[4].segment_contexts[0]
                .as_ref()
                .unwrap()
                .speaker
                .as_deref(),
            Some("魔王")
        );
    }

    #[tokio::test]
    async fn test_split_partial_success() {
        let (db, _f) = test_db().await;
        let rf = || Err(LlmError::ResponseFormat("bad format".to_string()));
        let provider = MockProvider::new(vec![
            rf(),
            rf(),
            rf(),                                       // [s1,s2,s3,s4] → split
            Ok(vec!["A".to_string(), "B".to_string()]), // [s1,s2] → ok
            rf(),
            rf(),
            rf(),                      // [s3,s4] → split
            Ok(vec!["C".to_string()]), // [s3] → ok
            rf(),
            rf(),
            rf(), // [s4] → failed (len==1)
        ]);

        let results = run_inner(
            vec![
                ("s1".to_string(), "一".to_string()),
                ("s2".to_string(), "二".to_string()),
                ("s3".to_string(), "三".to_string()),
                ("s4".to_string(), "四".to_string()),
            ],
            &provider,
            ctx(),
            &db,
            None,
            None,
            None,
            |_, _| {},
        )
        .await
        .unwrap();

        assert_eq!(provider.calls(), 11);
        assert_eq!(results[0].translated_text, "A");
        assert!(!results[0].needs_review);
        assert_eq!(results[1].translated_text, "B");
        assert!(!results[1].needs_review);
        assert_eq!(results[2].translated_text, "C");
        assert!(!results[2].needs_review);
        assert!(results[3].failed, "s4 failed, nothing written");
        assert_eq!(results[3].translated_text, "");
        assert!(!results[2].failed);
    }

    #[tokio::test]
    async fn test_progress_events_emitted() {
        let (db, _f) = test_db().await;

        // 3 segments → 1 batch (< DEFAULT_BATCH_SIZE) → 1 progress call
        let provider = MockProvider::new(vec![Ok(vec![
            "A".to_string(),
            "B".to_string(),
            "C".to_string(),
        ])]);

        let mut progress = vec![];
        run_inner(
            vec![
                ("s1".to_string(), "一".to_string()),
                ("s2".to_string(), "二".to_string()),
                ("s3".to_string(), "三".to_string()),
            ],
            &provider,
            ctx(),
            &db,
            None,
            None,
            None,
            |done, total| progress.push((done, total)),
        )
        .await
        .unwrap();

        assert!(!progress.is_empty());
        assert_eq!(progress.last().unwrap(), &(3, 3));
    }

    /// "Translate All" runs `run_inner` once per file with `global_progress`
    /// carrying the running offset and the project-wide total. The reported
    /// `done` must accumulate across files and never drop back down when a
    /// new file starts.
    #[tokio::test]
    async fn test_global_progress_offset_accumulates_across_files() {
        let (db, _f) = test_db().await;
        let global_total = 10usize;

        // File 1: 3 segments, offset 0 of 10
        let provider1 = MockProvider::new(vec![Ok(vec!["A".into(), "B".into(), "C".into()])]);
        let mut progress1 = vec![];
        run_inner(
            vec![
                ("s1".to_string(), "一".to_string()),
                ("s2".to_string(), "二".to_string()),
                ("s3".to_string(), "三".to_string()),
            ],
            &provider1,
            ctx(),
            &db,
            None,
            None,
            Some((0, global_total)),
            |done, total| progress1.push((done, total)),
        )
        .await
        .unwrap();

        assert_eq!(progress1.last().unwrap(), &(3, global_total));

        // File 2: 7 segments, offset 3 of 10 — must continue from 3, not reset
        let provider2 = MockProvider::new(vec![Ok(vec![
            "1".into(),
            "2".into(),
            "3".into(),
            "4".into(),
            "5".into(),
            "6".into(),
            "7".into(),
        ])]);
        let mut progress2 = vec![];
        run_inner(
            vec![
                ("s4".to_string(), "四".to_string()),
                ("s5".to_string(), "五".to_string()),
                ("s6".to_string(), "六".to_string()),
                ("s7".to_string(), "七".to_string()),
                ("s8".to_string(), "八".to_string()),
                ("s9".to_string(), "九".to_string()),
                ("s10".to_string(), "十".to_string()),
            ],
            &provider2,
            ctx(),
            &db,
            None,
            None,
            Some((3, global_total)),
            |done, total| progress2.push((done, total)),
        )
        .await
        .unwrap();

        assert_eq!(progress2.last().unwrap(), &(10, global_total));
    }

    // ── R1: provider outage and technical failures write nothing ────────────

    async fn insert_plain_segments(db: &SqlitePool, rows: &[(&str, &str, &str, &str)]) {
        sqlx::query(
            "INSERT INTO projects (id, name, engine, game_path) \
             VALUES ('r1-p', 'R1', 'mv_mz', '/tmp')",
        )
        .execute(db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
             VALUES ('r1-f', 'r1-p', 'Items.json', '/tmp/Items.json', 'database')",
        )
        .execute(db)
        .await
        .unwrap();
        for (id, source, target, status) in rows {
            sqlx::query(
                "INSERT INTO segments (id, source_file_id, json_key, source_text, target_text, status) \
                 VALUES (?, 'r1-f', ?, ?, ?, ?)",
            )
            .bind(id)
            .bind(format!("/{id}"))
            .bind(source)
            .bind(target)
            .bind(status)
            .execute(db)
            .await
            .unwrap();
        }
    }

    async fn segment_state(db: &SqlitePool, id: &str) -> (String, String) {
        sqlx::query_as("SELECT target_text, status FROM segments WHERE id = ?")
            .bind(id)
            .fetch_one(db)
            .await
            .unwrap()
    }

    fn unavailable() -> Result<Vec<String>, LlmError> {
        Err(LlmError::Unavailable {
            message: "connection refused".to_string(),
        })
    }

    #[tokio::test]
    async fn provider_outage_stops_the_run_and_writes_nothing() {
        let (db, _f) = test_db().await;
        insert_plain_segments(&db, &[("a", "剣", "", "untranslated")]).await;
        let provider = MockProvider::new(vec![unavailable()]);

        let result = run_inner(
            vec![("a".to_string(), "剣".to_string())],
            &provider,
            ctx(),
            &db,
            None,
            None,
            None,
            |_, _| {},
        )
        .await;

        assert!(
            matches!(
                result,
                Err(PipelineError::ProviderUnavailable { processed: 0, .. })
            ),
            "{result:?}"
        );
        assert_eq!(provider.calls(), 1, "no split or retry on an outage");
        assert_eq!(
            segment_state(&db, "a").await,
            (String::new(), "untranslated".to_string())
        );
    }

    #[tokio::test]
    async fn outage_after_first_batch_keeps_it_and_leaves_the_rest_untranslated() {
        let (db, _f) = test_db().await;
        insert_plain_segments(
            &db,
            &[
                ("a", "剣", "", "untranslated"),
                ("b", "盾", "", "untranslated"),
            ],
        )
        .await;
        let provider = MockProvider::new(vec![Ok(vec!["Sword".to_string()]), unavailable()]);
        let mut context = ctx();
        context.batch_size = 1;

        let result = run_inner(
            vec![
                ("a".to_string(), "剣".to_string()),
                ("b".to_string(), "盾".to_string()),
            ],
            &provider,
            context,
            &db,
            None,
            None,
            None,
            |_, _| {},
        )
        .await;

        assert!(
            matches!(
                result,
                Err(PipelineError::ProviderUnavailable { processed: 1, .. })
            ),
            "{result:?}"
        );
        assert_eq!(
            segment_state(&db, "a").await,
            ("Sword".to_string(), "translated".to_string())
        );
        assert_eq!(
            segment_state(&db, "b").await,
            (String::new(), "untranslated".to_string())
        );
    }

    #[tokio::test]
    async fn failed_retranslation_never_overwrites_a_reviewed_target() {
        let (db, _f) = test_db().await;
        insert_plain_segments(&db, &[("a", "剣", "Blade", "reviewed")]).await;

        for response in [
            Err(LlmError::TranslationFailed {
                attempts: 1,
                reason: "HTTP 400".to_string(),
            }),
            unavailable(),
        ] {
            let provider = MockProvider::new(vec![response]);
            let _ = run_inner(
                vec![("a".to_string(), "剣".to_string())],
                &provider,
                ctx(),
                &db,
                None,
                None,
                None,
                |_, _| {},
            )
            .await;
            assert_eq!(
                segment_state(&db, "a").await,
                ("Blade".to_string(), "reviewed".to_string())
            );
        }
    }

    #[tokio::test]
    async fn refused_batch_is_reported_failed_and_the_run_continues() {
        let (db, _f) = test_db().await;
        insert_plain_segments(
            &db,
            &[
                ("a", "剣", "", "untranslated"),
                ("b", "盾", "", "untranslated"),
            ],
        )
        .await;
        let provider = MockProvider::new(vec![
            Err(LlmError::TranslationFailed {
                attempts: 1,
                reason: "HTTP 413".to_string(),
            }),
            Ok(vec!["Shield".to_string()]),
        ]);
        let mut context = ctx();
        context.batch_size = 1;

        let results = run_inner(
            vec![
                ("a".to_string(), "剣".to_string()),
                ("b".to_string(), "盾".to_string()),
            ],
            &provider,
            context,
            &db,
            None,
            None,
            None,
            |_, _| {},
        )
        .await
        .unwrap();

        assert!(results[0].failed);
        assert!(!results[1].failed);
        assert_eq!(
            segment_state(&db, "a").await,
            (String::new(), "untranslated".to_string())
        );
        assert_eq!(
            segment_state(&db, "b").await,
            ("Shield".to_string(), "translated".to_string())
        );
    }

    #[tokio::test]
    async fn cancellation_stops_before_the_next_batch() {
        let (db, _f) = test_db().await;
        insert_plain_segments(
            &db,
            &[
                ("a", "剣", "", "untranslated"),
                ("b", "盾", "", "untranslated"),
            ],
        )
        .await;
        let provider = MockProvider::new(vec![Ok(vec!["Sword".to_string()])]);
        let mut context = ctx();
        context.batch_size = 1;
        let cancel = AtomicBool::new(false);

        let outcome = run_controlled(
            vec![
                ("a".to_string(), "剣".to_string()),
                ("b".to_string(), "盾".to_string()),
            ],
            &provider,
            context,
            &db,
            None,
            None,
            None,
            Some(&cancel),
            // The user presses "Stop" while the first batch is running.
            |_, _| cancel.store(true, Ordering::SeqCst),
        )
        .await
        .unwrap();

        assert!(outcome.cancelled);
        assert_eq!(outcome.results.len(), 1);
        assert_eq!(provider.calls(), 1);
        assert_eq!(
            segment_state(&db, "a").await,
            ("Sword".to_string(), "translated".to_string())
        );
        assert_eq!(
            segment_state(&db, "b").await,
            (String::new(), "untranslated".to_string())
        );
    }
}
