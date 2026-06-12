//! Payload types for `h2s://llm/*` Tauri events.
//!
//! Kept separate from pipeline.rs so frontend-facing types can be found and
//! documented without reading the full orchestration logic.

use serde::Serialize;

/// Emitted as `h2s://llm/progress` after each inner batch.
#[derive(Debug, Clone, Serialize)]
pub struct ProgressPayload {
    pub done: usize,
    pub total: usize,
}

/// Emitted as `h2s://llm/placeholder-warning` when a segment falls back to
/// `needs_review` because placeholder restoration failed after all retries.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceholderWarningPayload {
    pub segment_id: String,
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
