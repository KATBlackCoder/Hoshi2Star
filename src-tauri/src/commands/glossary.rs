//! Tauri IPC commands for glossary management.
//!
//! The five commands here expose the two-level glossary (global + project-local)
//! to the TypeScript frontend via `invoke()`.
//!
//! `extract_glossary_terms` runs asynchronously: it returns `Ok(())` immediately
//! and emits `h2s://glossary/extraction-done` with the created terms when done.

use crate::{
    core::glossary::{self, GlossaryTerm},
    domain::types::ProviderConfig,
    engines::wolf::extractor::extract_wolf_speaker_names,
    llm::provider::LlmProvider,
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

/// Extract Wolf v2 speaker names from a project's segments and insert them
/// into the glossary as auto-generated character terms.
///
/// Reads source texts from the `segments` table (already stored by `open_project`),
/// applies `extract_wolf_speaker_names`, and inserts each unique canonical name with
/// `domain = "character"` and `auto_generated = true`. Idempotent: re-running skips
/// names already present in the glossary for this project.
///
/// Returns the list of newly inserted `GlossaryTerm`s.
#[tauri::command]
pub async fn extract_wolf_speakers(
    project_id: String,
    lang_pair: String,
    state: State<'_, AppState>,
) -> Result<Vec<GlossaryTerm>, String> {
    let source_texts: Vec<String> = sqlx::query_scalar(
        "SELECT s.source_text FROM segments s \
         JOIN source_files sf ON s.source_file_id = sf.id \
         WHERE sf.project_id = ?",
    )
    .bind(&project_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let speakers = extract_wolf_speaker_names(&source_texts);

    let mut inserted = Vec::new();
    for speaker in &speakers {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM glossary_terms \
             WHERE source_text = ? AND project_id = ? AND lang_pair = ?",
        )
        .bind(&speaker.canonical)
        .bind(&project_id)
        .bind(&lang_pair)
        .fetch_one(&state.db)
        .await
        .map_err(|e| e.to_string())?;

        if count == 0 {
            match glossary::insert_term(
                &state.db,
                &speaker.canonical,
                "",
                &lang_pair,
                "character",
                Some(&project_id),
                true,
            )
            .await
            {
                Ok(term) => inserted.push(term),
                Err(e) => eprintln!(
                    "[wolf-speakers] insert failed for '{}': {e}",
                    speaker.canonical
                ),
            }
        }
    }

    Ok(inserted)
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

        if let Err(e) = provider.health_check().await {
            let msg = format!("Ollama inaccessible ({url}) — vérifiez qu'il est démarré : {e}");
            eprintln!("[glossary] {msg}");
            let _ = app.emit(
                "h2s://glossary/extraction-done",
                serde_json::json!({ "projectId": project_id, "terms": [], "error": msg }),
            );
            return;
        }

        let payload =
            match glossary::extract_terms_from_project(&db, &provider, &project_id, &lang_pair)
                .await
            {
                Ok(terms) => {
                    serde_json::json!({ "projectId": project_id, "terms": terms, "error": null })
                }
                Err(e) => {
                    eprintln!("[glossary] extraction error: {e}");
                    serde_json::json!({ "projectId": project_id, "terms": [], "error": e })
                }
            };

        let _ = app.emit("h2s://glossary/extraction-done", payload);
    });

    Ok(())
}
