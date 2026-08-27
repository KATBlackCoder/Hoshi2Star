//! Domain types serialised to/from TypeScript via Tauri IPC.
//!
//! Extracted from commands/project.rs so that other modules (sync, export, F5)
//! can depend on these types without creating a dependency on commands/.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::llm::provider::{DEFAULT_BATCH_SIZE, DEFAULT_OLLAMA_MODEL, DEFAULT_OLLAMA_URL};

#[derive(Debug, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub engine: String,
    pub game_path: String,
    pub source_lang: String,
    pub target_lang: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Project {
    /// BCP-47-like pair used to scope translation memory and terminology.
    pub fn lang_pair(&self) -> String {
        format!("{}-{}", self.source_lang, self.target_lang)
    }
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct SourceFile {
    pub id: String,
    pub project_id: String,
    pub file_name: String,
    pub file_path: String,
    pub file_type: String,
    pub translation_secs: Option<i64>,
    /// Segments with `status = 'translated'` (strict — a `needs_review`
    /// segment has a target but is not counted here).
    #[sqlx(default)]
    pub translated_count: i64,
    /// Segments with `status = 'needs_review'` (remaining review work).
    #[sqlx(default)]
    pub needs_review_count: i64,
    #[sqlx(default)]
    pub total_count: i64,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    pub id: String,
    pub source_file_id: String,
    pub json_key: String,
    pub source_text: String,
    pub segment_kind: String,
    pub scene_id: Option<String>,
    pub sequence_index: Option<i64>,
    pub speaker: Option<String>,
    pub branch_path: Option<String>,
    pub context_json: Option<String>,
    pub target_text: String,
    pub status: String,
    pub qa_score: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedSegments {
    pub items: Vec<Segment>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

/// One project-wide search hit: a [`Segment`] row plus its file's name so the
/// results view can group hits by file without a second lookup.
#[derive(Debug, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct SegmentSearchHit {
    pub id: String,
    pub source_file_id: String,
    pub json_key: String,
    pub source_text: String,
    pub segment_kind: String,
    pub scene_id: Option<String>,
    pub sequence_index: Option<i64>,
    pub speaker: Option<String>,
    pub branch_path: Option<String>,
    pub context_json: Option<String>,
    pub target_text: String,
    pub status: String,
    pub qa_score: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
    pub file_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentSearchResult {
    pub items: Vec<SegmentSearchHit>,
    /// Real match count — `items` is capped at the requested limit.
    pub total: i64,
}

/// Shared throughput policy for local and cloud-compatible providers.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceProfile {
    Eco,
    #[default]
    Balanced,
    Fast,
}

/// LLM provider configuration passed from the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    /// Stable UI preset identifier (`ollama`, `lmstudio`, `huggingface`, `custom`).
    #[serde(default = "default_provider_id")]
    pub provider_id: String,
    /// OpenAI-compatible `/v1` base URL.
    pub url: String,
    /// Model to use (e.g. "qwen3:4b").
    pub model: String,
    /// Optional API key (for cloud providers like OpenAI / DeepSeek).
    pub api_key: Option<String>,
    /// Number of segments sent to the provider per LLM call.
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    #[serde(default)]
    pub resource_profile: ResourceProfile,
}

impl ProviderConfig {
    /// Bound peak prompt size independently of potentially stale UI settings.
    pub fn effective_batch_size(&self) -> usize {
        let requested = self.batch_size.clamp(1, 100);
        match self.resource_profile {
            ResourceProfile::Eco => requested.min(8),
            ResourceProfile::Balanced => requested.min(20),
            ResourceProfile::Fast => requested,
        }
    }

    /// A small sequential pause reduces sustained local CPU/GPU pressure. The
    /// pipeline remains single-request-at-a-time for every profile.
    pub fn batch_delay_ms(&self) -> u64 {
        match self.resource_profile {
            ResourceProfile::Eco => 500,
            ResourceProfile::Balanced => 100,
            ResourceProfile::Fast => 0,
        }
    }
}

fn default_batch_size() -> usize {
    DEFAULT_BATCH_SIZE
}

fn default_provider_id() -> String {
    "ollama".to_string()
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            provider_id: default_provider_id(),
            url: DEFAULT_OLLAMA_URL.to_string(),
            model: DEFAULT_OLLAMA_MODEL.to_string(),
            api_key: None,
            batch_size: DEFAULT_BATCH_SIZE,
            resource_profile: ResourceProfile::Balanced,
        }
    }
}

/// Wrapper returned by `open_project`.
///
/// `was_restored: true` means the project already existed in DB and was loaded
/// from the manifest — no re-extraction was performed.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenProjectResult {
    pub project: Project,
    pub was_restored: bool,
}

/// Project-level statistics: used by the frontend for the "Export All" gate and progress display.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStats {
    pub file_count: i64,
    pub total_segments: i64,
    pub untranslated_count: i64,
    pub translated_count: i64,
    pub needs_review_count: i64,
    /// Segments with `status = 'reviewed'` — with the other three counters,
    /// the four statuses sum to `total_segments`.
    pub reviewed_count: i64,
}

/// Summary of QA errors for a whole project.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QaTerminologyIssue {
    pub entry_id: String,
    pub source_term: String,
    pub expected_targets: Vec<String>,
    pub severity: String,
    pub occurrences: usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QaReport {
    pub total_segments: i64,
    pub ok_count: i64,
    pub error_count: i64,
    pub critical_count: i64,
    pub errors_by_type: HashMap<String, usize>,
    pub terminology_issues: Vec<QaTerminologyIssue>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_profiles_bound_batch_size_and_delay() {
        let mut config = ProviderConfig {
            batch_size: 80,
            ..ProviderConfig::default()
        };

        config.resource_profile = ResourceProfile::Eco;
        assert_eq!(config.effective_batch_size(), 8);
        assert_eq!(config.batch_delay_ms(), 500);

        config.resource_profile = ResourceProfile::Balanced;
        assert_eq!(config.effective_batch_size(), 20);
        assert_eq!(config.batch_delay_ms(), 100);

        config.resource_profile = ResourceProfile::Fast;
        assert_eq!(config.effective_batch_size(), 80);
        assert_eq!(config.batch_delay_ms(), 0);
    }

    #[test]
    fn missing_resource_profile_deserializes_as_balanced() {
        let config: ProviderConfig = serde_json::from_value(serde_json::json!({
            "providerId": "ollama",
            "url": "http://localhost:11434/v1",
            "model": "test",
            "apiKey": null,
            "batchSize": 20
        }))
        .unwrap();

        assert_eq!(config.resource_profile, ResourceProfile::Balanced);
    }
}
