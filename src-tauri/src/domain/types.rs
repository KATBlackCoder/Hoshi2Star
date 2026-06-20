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
    pub created_at: String,
    pub updated_at: String,
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
    #[sqlx(default)]
    pub translated_count: i64,
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

/// LLM provider configuration passed from the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    /// Base URL of the Ollama instance (e.g. "http://localhost:11434").
    pub url: String,
    /// Model to use (e.g. "qwen3:4b").
    pub model: String,
    /// Optional API key (for cloud providers like OpenAI / DeepSeek).
    pub api_key: Option<String>,
    /// Number of segments sent to the provider per LLM call.
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
}

fn default_batch_size() -> usize {
    DEFAULT_BATCH_SIZE
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            url: DEFAULT_OLLAMA_URL.to_string(),
            model: DEFAULT_OLLAMA_MODEL.to_string(),
            api_key: None,
            batch_size: DEFAULT_BATCH_SIZE,
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

/// Project-level statistics: used by the frontend to gate the "Export All" flow.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStats {
    pub file_count: i64,
    pub total_segments: i64,
    pub untranslated_count: i64,
}

/// Summary of QA errors for a whole project.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QaReport {
    pub total_segments: i64,
    pub ok_count: i64,
    pub error_count: i64,
    pub errors_by_type: HashMap<String, usize>,
}
