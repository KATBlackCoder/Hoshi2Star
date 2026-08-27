//! Narrow Tauri API for the multilingual terminology library.
//!
//! Commands validate transport inputs and delegate all persistence and scan
//! behavior to the repository/service. SQL semantics stay out of this layer.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime, State};

use crate::core::terminology::repository;
use crate::core::terminology::scanner::{ScanEvent, ScanEventSink};
use crate::core::terminology::types::{
    CreateTermInput, PaginatedTerminology, TerminologyEntryView, TerminologyQuery,
    TerminologyStats, TerminologyTranslationView, UpdateTermInput, UpsertTranslationInput,
};
use crate::state::AppState;

pub const SCAN_PROGRESS_EVENT: &str = "h2s://terminology/scan-progress";
pub const SCAN_DONE_EVENT: &str = "h2s://terminology/scan-done";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StartTerminologyScanResponse {
    pub scan_id: String,
}

#[tauri::command]
pub async fn list_terminology(
    query: TerminologyQuery,
    state: State<'_, AppState>,
) -> Result<PaginatedTerminology, String> {
    repository::list(&state.db, &query)
        .await
        .map_err(stable_error)
}

#[tauri::command]
pub async fn get_terminology_stats(
    source_language: String,
    target_language: String,
    project_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<TerminologyStats, String> {
    if let Some(project_id) = project_id.as_deref() {
        ensure_project(&state, project_id).await?;
    }
    repository::stats(
        &state.db,
        &source_language,
        &target_language,
        project_id.as_deref(),
    )
    .await
    .map_err(stable_error)
}

#[tauri::command]
pub async fn create_terminology_entry(
    input: CreateTermInput,
    state: State<'_, AppState>,
) -> Result<TerminologyEntryView, String> {
    repository::create_entry(&state.db, &input)
        .await
        .map_err(stable_error)
}

#[tauri::command]
pub async fn update_terminology_entry(
    input: UpdateTermInput,
    state: State<'_, AppState>,
) -> Result<TerminologyEntryView, String> {
    repository::update_entry(&state.db, &input)
        .await
        .map_err(stable_error)
}

#[tauri::command]
pub async fn archive_terminology_entry(
    entry_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    repository::archive_entry(&state.db, &entry_id)
        .await
        .map_err(stable_error)
}

#[tauri::command]
pub async fn upsert_terminology_translation(
    input: UpsertTranslationInput,
    state: State<'_, AppState>,
) -> Result<TerminologyTranslationView, String> {
    if let Some(project_id) = input.project_id.as_deref() {
        ensure_project(&state, project_id).await?;
    }
    repository::upsert_translation(&state.db, &input)
        .await
        .map_err(stable_error)
}

#[tauri::command]
pub async fn start_terminology_scan(
    project_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<StartTerminologyScanResponse, String> {
    start_terminology_scan_with_app(project_id, app, &state).await
}

pub async fn start_terminology_scan_with_app<R: Runtime>(
    project_id: String,
    app: AppHandle<R>,
    state: &AppState,
) -> Result<StartTerminologyScanResponse, String> {
    let sink: ScanEventSink = std::sync::Arc::new(move |event| {
        let emitted = match event {
            ScanEvent::Progress(progress) => app.emit(SCAN_PROGRESS_EVENT, progress),
            ScanEvent::Done(done) => app.emit(SCAN_DONE_EVENT, done),
        };
        if let Err(error) = emitted {
            log::warn!("failed to emit terminology scan event: {error}");
        }
    });
    let scan_id = state
        .terminology
        .start_project_scan(state.db.clone(), project_id, Some(sink))
        .await
        .map_err(stable_error)?;
    Ok(StartTerminologyScanResponse { scan_id })
}

#[tauri::command]
pub async fn cancel_terminology_scan(
    scan_id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    if scan_id.trim().is_empty() {
        return Err("terminology: scan id must not be empty".to_string());
    }
    Ok(state.terminology.cancel_scan(&scan_id).await)
}

async fn ensure_project(state: &State<'_, AppState>, project_id: &str) -> Result<(), String> {
    if project_id.trim().is_empty() {
        return Err("terminology: project id must not be empty".to_string());
    }
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?)")
        .bind(project_id)
        .fetch_one(&state.db)
        .await
        .map_err(|error| stable_error(error.into()))?;
    if !exists {
        return Err(format!("terminology: project not found: {project_id}"));
    }
    Ok(())
}

fn stable_error(error: crate::core::terminology::TerminologyError) -> String {
    format!("terminology: {error}")
}
