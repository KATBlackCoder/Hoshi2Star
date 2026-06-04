//! LLM translation pipeline — Passe 1 (translate) for the MVP.
//!
//! ## Flow (per batch)
//! 1. Deduplicate by hash (`batch::dedup_by_hash`)
//! 2. For each unique segment: check TM — if exact match, skip LLM
//! 3. Tokenize remaining segments (ADR-002: placeholders → ⟦ph_N⟧)
//! 4. Send tokenized batch to the provider via `split::llm_translate_with_split`
//! 5. Validate + restore placeholders; retry up to `MAX_RETRIES` times if invalid
//! 6. Spread translations back to all original positions
//! 7. Call `on_progress(done, total)` after each batch
//!
//! The pipeline is generic over `P: LlmProvider` so it can be tested with a
//! mock provider without needing dynamic dispatch.
//!
//! Recursive batch-split logic lives in `split.rs`.
//! Event payload types live in `progress.rs`.

use crate::core::tm;
use crate::llm::batch;
use crate::llm::progress::{PlaceholderWarningPayload, ProgressPayload};
use crate::llm::provider::{LlmError, LlmProvider, TranslationContext};
use crate::llm::split::llm_translate_with_split;
use crate::llm::tokenizer::{Engine as TokEngine, Tokenizer};
use serde::Serialize;
use sqlx::SqlitePool;
use tauri::Emitter;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const DEFAULT_BATCH_SIZE: usize = 20;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct TranslationResult {
    pub id: String,
    pub translated_text: String,
    /// `true` when the translation came from the TM (no LLM call was made).
    pub from_tm: bool,
    /// `true` when placeholder validation failed after all retries — source text
    /// was kept as a temporary translation with status `needs_review`.
    pub needs_review: bool,
}

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("LLM provider error: {0}")]
    Provider(String),
    #[error("Database error: {0}")]
    Database(String),
}

impl From<LlmError> for PipelineError {
    fn from(e: LlmError) -> Self {
        PipelineError::Provider(e.to_string())
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run the translation pipeline on a batch of segments.
///
/// `segments` is a list of `(id, source_text)` pairs.
/// `on_progress` is called after each inner batch with `(done, total)`.
///
/// The public `run` function wraps this with a real Tauri `AppHandle` for
/// event emission.  Tests call `run_inner` directly with a closure.
pub async fn run_inner<P, F>(
    segments: Vec<(String, String)>,
    provider: &P,
    context: TranslationContext,
    db: &SqlitePool,
    mut on_progress: F,
) -> Result<Vec<TranslationResult>, PipelineError>
where
    P: LlmProvider,
    F: FnMut(usize, usize),
{
    let total = segments.len();
    let lang_pair = format!("{}-{}", context.source_lang, context.target_lang);

    // Index: id → source_text  (for fast lookup by id)
    let seg_map: std::collections::HashMap<String, String> = segments.iter().cloned().collect();

    let all_ids: Vec<String> = segments.iter().map(|(id, _)| id.clone()).collect();

    // Results in the same order as the input
    let mut results: Vec<Option<TranslationResult>> = vec![None; total];
    // Map id → original index
    let id_to_idx: std::collections::HashMap<String, usize> = all_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.clone(), i))
        .collect();

    let batches = batch::group_segments(all_ids, DEFAULT_BATCH_SIZE);
    let mut done = 0usize;

    for batch_ids in &batches {
        let batch_segs: Vec<(String, String)> = batch_ids
            .iter()
            .map(|id| (id.clone(), seg_map[id.as_str()].clone()))
            .collect();

        let batch_results = translate_batch(batch_segs, provider, &context, &lang_pair, db).await?;

        for tr in batch_results {
            if let Some(&orig_idx) = id_to_idx.get(&tr.id) {
                results[orig_idx] = Some(tr);
            }
        }

        done += batch_ids.len();
        on_progress(done, total);
    }

    Ok(results.into_iter().flatten().collect())
}

// ---------------------------------------------------------------------------
// Entry point used by Tauri commands (emits events via AppHandle)
// ---------------------------------------------------------------------------

/// Tauri-aware wrapper: emits `h2s://llm/progress` after each batch,
/// and `h2s://llm/placeholder-warning` for each segment that fell back to needs_review.
pub async fn run<P>(
    segments: Vec<(String, String)>,
    provider: &P,
    context: TranslationContext,
    db: &SqlitePool,
    app_handle: &tauri::AppHandle,
) -> Result<Vec<TranslationResult>, PipelineError>
where
    P: LlmProvider,
{
    let handle = app_handle.clone();
    let results = run_inner(segments, provider, context, db, move |done, total| {
        let _ = handle.emit("h2s://llm/progress", ProgressPayload { done, total });
    })
    .await?;

    for r in results.iter().filter(|r| r.needs_review) {
        let _ = app_handle.emit(
            "h2s://llm/placeholder-warning",
            PlaceholderWarningPayload {
                segment_id: r.id.clone(),
            },
        );
    }

    Ok(results)
}

// ---------------------------------------------------------------------------
// Batch translation (TM → LLM with adaptive split)
// ---------------------------------------------------------------------------

async fn translate_batch<P>(
    segments: Vec<(String, String)>,
    provider: &P,
    context: &TranslationContext,
    lang_pair: &str,
    db: &SqlitePool,
) -> Result<Vec<TranslationResult>, PipelineError>
where
    P: LlmProvider,
{
    let batch_len = segments.len();
    let (unique_segs, idx_map) = batch::dedup_by_hash(segments.clone());

    // Translation table: (translated_text, from_tm, needs_review)
    let mut translations: Vec<Option<(String, bool, bool)>> = vec![None; batch_len];

    // Track which unique segments still need LLM
    let mut to_translate: Vec<usize> = Vec::new(); // indices into `unique_segs`

    for (unique_idx, seg) in unique_segs.iter().enumerate() {
        let tm_hit = tm::lookup_exact(&seg.hash, lang_pair, db)
            .await
            .map_err(|e| PipelineError::Database(e.to_string()))?;

        if let Some(entry) = tm_hit {
            for &orig_idx in idx_map.get(&seg.hash).into_iter().flatten() {
                translations[orig_idx] = Some((entry.target_text.clone(), true, false));
            }
        } else {
            to_translate.push(unique_idx);
        }
    }

    // Translate remaining segments via LLM with adaptive split on failure
    if !to_translate.is_empty() {
        let tokenized: Vec<_> = to_translate
            .iter()
            .map(|&i| Tokenizer::tokenize(&unique_segs[i].text, TokEngine::MvMz))
            .collect();

        let local_indices: Vec<usize> = (0..to_translate.len()).collect();
        let llm_results =
            llm_translate_with_split(local_indices, &tokenized, provider, context).await;

        for (local_idx, text, needs_review) in llm_results {
            let global_unique_idx = to_translate[local_idx]; // LOCAL → GLOBAL
            let final_text = if needs_review {
                log::warn!(
                    "[h2s] segment '{}' marked needs_review after adaptive split",
                    unique_segs[global_unique_idx].id
                );
                unique_segs[global_unique_idx].text.clone()
            } else {
                text
            };
            for &orig_idx in idx_map
                .get(&unique_segs[global_unique_idx].hash)
                .into_iter()
                .flatten()
            {
                translations[orig_idx] = Some((final_text.clone(), false, needs_review));
            }
        }
    }

    // Build final Vec<TranslationResult> (same order as input `segments`)
    Ok(segments
        .iter()
        .enumerate()
        .map(|(i, (id, _))| {
            let (translated_text, from_tm, needs_review) =
                translations[i].take().unwrap_or_default();
            TranslationResult {
                id: id.clone(),
                translated_text,
                from_tm,
                needs_review,
            }
        })
        .collect())
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
    }

    impl MockProvider {
        fn new(responses: Vec<Result<Vec<String>, LlmError>>) -> Self {
            Self {
                responses: std::sync::Mutex::new(responses.into()),
                call_count: std::sync::atomic::AtomicU32::new(0),
            }
        }

        fn calls(&self) -> u32 {
            self.call_count.load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    impl LlmProvider for MockProvider {
        async fn translate(
            &self,
            _segments: Vec<String>,
            _context: TranslationContext,
        ) -> Result<Vec<String>, LlmError> {
            self.call_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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
            glossary_terms: vec![],
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
            |_, _| {},
        )
        .await
        .unwrap();

        assert_eq!(provider.calls(), 1);
        assert_eq!(results[0].translated_text, "Hero");
        assert!(!results[0].from_tm);
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
            |_, _| {},
        )
        .await
        .unwrap();

        assert_eq!(provider.calls(), 2, "should retry once on ResponseFormat");
        assert_eq!(results[0].translated_text, "Hero");
        assert!(!results[0].from_tm);
    }

    #[tokio::test]
    async fn test_placeholder_failure_falls_back_to_needs_review() {
        let (db, _f) = test_db().await;

        // MockProvider always returns a response without ⟦ph_0⟧ — fails restore every time.
        // After MAX_RETRIES (3) the segment must have needs_review=true and keep source_text.
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
            |_, _| {},
        )
        .await
        .unwrap();

        assert_eq!(provider.calls(), 3, "should exhaust all retries");
        assert!(results[0].needs_review, "segment must be needs_review");
        assert_eq!(
            results[0].translated_text, r"\V[12] pièces",
            "source_text kept as fallback"
        );
    }

    #[tokio::test]
    async fn test_response_format_exhausted_falls_back_to_needs_review() {
        let (db, _f) = test_db().await;

        // Provider always returns ResponseFormat — should exhaust MAX_RETRIES (3)
        // and fall back to needs_review instead of crashing the whole batch.
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
            |_, _| {},
        )
        .await
        .unwrap();

        assert_eq!(provider.calls(), 3, "should exhaust all retries");
        assert!(results[0].needs_review, "segment must be needs_review");
        assert_eq!(
            results[0].translated_text, "ポーション",
            "source_text kept as fallback"
        );
    }

    #[tokio::test]
    async fn test_response_format_triggers_split() {
        let (db, _f) = test_db().await;
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
            |_, _| {},
        )
        .await
        .unwrap();

        assert_eq!(provider.calls(), 5);
        assert_eq!(results[0].translated_text, "A");
        assert!(!results[0].needs_review);
        assert_eq!(results[1].translated_text, "B");
        assert!(!results[1].needs_review);
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
            rf(), // [s4] → needs_review (len==1)
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
        assert_eq!(results[3].translated_text, "四"); // source_text kept as fallback
        assert!(results[3].needs_review);
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
            |done, total| progress.push((done, total)),
        )
        .await
        .unwrap();

        assert!(!progress.is_empty());
        assert_eq!(progress.last().unwrap(), &(3, 3));
    }
}
