//! Tauri IPC commands for glossary management.
//!
//! The five commands here expose the two-level glossary (global + project-local)
//! to the TypeScript frontend via `invoke()`.
//!
//! `extract_glossary_terms` runs asynchronously: it returns `Ok(())` immediately
//! and emits `h2s://glossary/extraction-done` with the created terms when done.

use crate::{
    commands::project::ProviderConfig,
    core::glossary::{self, GlossaryTerm},
    llm::provider::{OllamaProvider, DEFAULT_OLLAMA_MODEL, DEFAULT_OLLAMA_URL},
    state::AppState,
};
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub async fn get_glossary(
    project_id: String,
    lang_pair: String,
    state: State<'_, AppState>,
) -> Result<Vec<GlossaryTerm>, String> {
    glossary::list_for_project(&state.db, &project_id, &lang_pair)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_glossary_term(
    source_text: String,
    target_text: String,
    lang_pair: String,
    domain: String,
    project_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<GlossaryTerm, String> {
    glossary::insert_term(
        &state.db,
        &source_text,
        &target_text,
        &lang_pair,
        &domain,
        project_id.as_deref(),
        false,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_glossary_term(
    id: String,
    source_text: String,
    target_text: String,
    domain: String,
    state: State<'_, AppState>,
) -> Result<GlossaryTerm, String> {
    glossary::update_term(&state.db, &id, &source_text, &target_text, &domain)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_glossary_term(id: String, state: State<'_, AppState>) -> Result<(), String> {
    glossary::delete_term(&state.db, &id)
        .await
        .map_err(|e| e.to_string())
}

/// Extract glossary terms from a project using the LLM.
///
/// Returns `Ok(())` immediately; the actual result arrives via the
/// `h2s://glossary/extraction-done` event once the background task completes.
#[tauri::command]
pub async fn extract_glossary_terms(
    project_id: String,
    lang_pair: String,
    provider_config: ProviderConfig,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    use std::time::Duration;

    let db = state.db.clone();

    tokio::spawn(async move {
        let url = if provider_config.url.is_empty() {
            DEFAULT_OLLAMA_URL
        } else {
            &provider_config.url
        };
        let model = if provider_config.model.is_empty() {
            DEFAULT_OLLAMA_MODEL
        } else {
            &provider_config.model
        };
        let provider = OllamaProvider::new(url, model, Duration::from_secs(120));

        let terms = glossary::extract_terms_from_project(&db, &provider, &project_id, &lang_pair)
            .await
            .unwrap_or_default();

        let _ = app.emit(
            "h2s://glossary/extraction-done",
            serde_json::json!({ "projectId": project_id, "terms": terms }),
        );
    });

    Ok(())
}
