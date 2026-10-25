//! Payload types for `h2s://llm/*` Tauri events.
//!
//! Kept separate from pipeline.rs so frontend-facing types can be found and
//! documented without reading the full orchestration logic.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::llm::pipeline::TranslationResult;

/// Emitted as `h2s://llm/progress` after each inner batch.
#[derive(Debug, Clone, Serialize)]
pub struct ProgressPayload {
    pub done: usize,
    pub total: usize,
}

/// Tally of one translation run, emitted once as `h2s://llm/completed` so the
/// frontend shows a single summary instead of one notice per segment.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSummary {
    /// Segments handled by the run (translated, flagged or failed).
    pub count: usize,
    pub translated: usize,
    pub needs_review: usize,
    /// No usable output: nothing was written, the segment is unchanged.
    pub failed: usize,
    /// Critical QA error kinds among `needs_review` results.
    pub qa_errors_by_type: BTreeMap<String, usize>,
    /// The user stopped the run before every segment was handled.
    pub cancelled: bool,
    /// Other rows of the same canonical name (earlier source fallbacks)
    /// updated with this run's translation — not part of `count`.
    pub repaired: usize,
}

impl RunSummary {
    pub fn add(&mut self, results: &[TranslationResult]) {
        for result in results {
            self.count += 1;
            if result.failed {
                self.failed += 1;
            } else if result.needs_review {
                self.needs_review += 1;
                for error in result.qa_errors.iter().filter(|error| error.is_critical()) {
                    *self
                        .qa_errors_by_type
                        .entry(error.type_key().to_string())
                        .or_default() += 1;
                }
            } else {
                self.translated += 1;
            }
        }
    }
}

/// Emitted once as `h2s://llm/error` when a run stops on an error.
/// `processed`/`total` are set when the provider stopped answering mid-run.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmErrorPayload {
    pub message: String,
    pub provider_unavailable: bool,
    pub processed: usize,
    pub total: usize,
}

/// One segment's `target_text`/`status` as just persisted to the DB.
///
/// Emitted as part of `h2s://llm/segments-updated` (one event per batch, with
/// a `Vec<SegmentUpdatePayload>`) so the frontend can patch its in-memory
/// segment list without re-querying the DB.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentUpdatePayload {
    pub id: String,
    pub target_text: String,
    pub status: String,
}

/// Emitted as `h2s://llm/cooling` once per second while the pipeline is
/// pausing between batches (automatic cooldown).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoolingPayload {
    pub remaining_secs: u64,
}
