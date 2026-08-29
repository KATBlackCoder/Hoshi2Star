//! Narrow Tauri API for the multilingual terminology library.
//!
//! Commands validate transport inputs and delegate all persistence and scan
//! behavior to the repository/service. SQL semantics stay out of this layer.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Runtime, State};

use crate::core::terminology::normalize::normalize_term;
use crate::core::terminology::repository;
use crate::core::terminology::resolver::{self, QaTerminologyRule};
use crate::core::terminology::scanner::{ScanEvent, ScanEventSink};
use crate::core::terminology::translator::{self, TranslationProgressSink};
use crate::core::terminology::types::{
    CreateTermInput, GlobalizeTranslationsInput, GlobalizeTranslationsSummary,
    PaginatedTerminology, TerminologyEntryView, TerminologyQuery, TerminologyStats,
    TerminologyTranslationView, TranslationScope, UpdateTermInput, UpsertTranslationInput,
};
use crate::domain::types::ProviderConfig;
use crate::engines::wolf::extractor::extract_wolf_speaker_names;
use crate::llm::provider::{LlmProvider, OpenAiCompatibleProvider};
use crate::state::AppState;

pub const SCAN_PROGRESS_EVENT: &str = "h2s://terminology/scan-progress";
pub const SCAN_DONE_EVENT: &str = "h2s://terminology/scan-done";
pub const TRANSLATE_PROGRESS_EVENT: &str = "h2s://terminology/translate-progress";
pub const TRANSLATE_DONE_EVENT: &str = "h2s://terminology/translate-done";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StartTerminologyScanResponse {
    pub scan_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslateTerminologyCommandInput {
    pub entry_ids: Vec<String>,
    pub target_language: String,
    pub project_id: Option<String>,
    pub scope: TranslationScope,
    pub provider_config: ProviderConfig,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StartTerminologyTranslationResponse {
    pub job_id: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WolfSpeakerTerminology {
    pub entry_id: String,
    pub canonical_text: String,
    pub occurrence_count: usize,
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
pub async fn get_segment_terminology(
    segment_id: String,
    target_language: String,
    state: State<'_, AppState>,
) -> Result<Vec<QaTerminologyRule>, String> {
    if segment_id.trim().is_empty() {
        return Err("terminology: segment id must not be empty".to_string());
    }
    resolver::resolve_qa_rules_for_segments(
        &state.db,
        std::slice::from_ref(&segment_id),
        &target_language,
    )
    .await
    .map(|mut rules| rules.remove(&segment_id).unwrap_or_default())
    .map_err(stable_error)
}

/// Compatibility IPC for the existing Wolf workflow. Speaker names now feed
/// the shared terminology library and its per-segment occurrence index.
#[tauri::command]
pub async fn extract_wolf_speakers(
    project_id: String,
    lang_pair: String,
    state: State<'_, AppState>,
) -> Result<Vec<WolfSpeakerTerminology>, String> {
    let source_language = lang_pair
        .split_once('-')
        .map(|(source, _)| source.trim())
        .filter(|source| !source.is_empty())
        .ok_or_else(|| format!("terminology: invalid language pair: {lang_pair:?}"))?;
    extract_wolf_speakers_into_terminology(&state.db, &project_id, source_language)
        .await
        .map_err(stable_error)
}

async fn extract_wolf_speakers_into_terminology(
    pool: &sqlx::SqlitePool,
    project_id: &str,
    source_language: &str,
) -> crate::core::terminology::Result<Vec<WolfSpeakerTerminology>> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?)")
        .bind(project_id)
        .fetch_one(pool)
        .await?;
    if !exists {
        return Err(crate::core::terminology::TerminologyError::NotFound(
            project_id.to_string(),
        ));
    }
    let segments: Vec<(String, String)> = sqlx::query_as(
        "SELECT segment.id, segment.source_text FROM segments segment \
         JOIN source_files file ON file.id = segment.source_file_id \
         WHERE file.project_id = ? ORDER BY segment.rowid",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;
    let source_texts = segments
        .iter()
        .map(|(_, source)| source.clone())
        .collect::<Vec<_>>();
    let speakers = extract_wolf_speaker_names(&source_texts);
    let segment_speakers = segments
        .iter()
        .filter_map(|(segment_id, source_text)| {
            extract_wolf_speaker_names(std::slice::from_ref(source_text))
                .into_iter()
                .next()
                .map(|speaker| (segment_id, speaker.canonical))
        })
        .collect::<Vec<_>>();
    let mut transaction = pool.begin().await?;
    let mut result = Vec::with_capacity(speakers.len());
    for speaker in speakers {
        let normalized = normalize_term(&speaker.canonical, source_language)?;
        let entry_id: Option<String> = sqlx::query_scalar(
            "SELECT id FROM terminology_entries \
             WHERE source_language = ? AND normalized_text = ? \
               AND semantic_type = 'speaker' AND sense_key = '' LIMIT 1",
        )
        .bind(source_language)
        .bind(&normalized)
        .fetch_optional(&mut *transaction)
        .await?;
        let entry_id = match entry_id {
            Some(entry_id) => entry_id,
            None => {
                let entry_id = uuid::Uuid::new_v4().to_string();
                sqlx::query(
                    "INSERT INTO terminology_entries (\
                        id, source_language, canonical_text, normalized_text, part_of_speech, \
                        semantic_type, origin, confidence\
                     ) VALUES (?, ?, ?, ?, 'proper_noun', 'speaker', 'engine', 1.0)",
                )
                .bind(&entry_id)
                .bind(source_language)
                .bind(&speaker.canonical)
                .bind(&normalized)
                .execute(&mut *transaction)
                .await?;
                entry_id
            }
        };
        for (segment_id, canonical) in &segment_speakers {
            if canonical == &speaker.canonical {
                sqlx::query(
                    "INSERT INTO terminology_occurrences (\
                        entry_id, project_id, segment_id, surface_text, engine_kind, occurrence_count\
                     ) VALUES (?, ?, ?, ?, 'speaker', 1) \
                     ON CONFLICT(entry_id, segment_id, surface_text) DO UPDATE SET \
                        project_id = excluded.project_id, engine_kind = excluded.engine_kind, \
                        occurrence_count = excluded.occurrence_count",
                )
                .bind(&entry_id)
                .bind(project_id)
                .bind(segment_id)
                .bind(&speaker.canonical)
                .execute(&mut *transaction)
                .await?;
            }
        }
        result.push(WolfSpeakerTerminology {
            entry_id,
            canonical_text: speaker.canonical,
            occurrence_count: speaker.occurrences,
        });
    }
    transaction.commit().await?;
    Ok(result)
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
pub async fn delete_terminology_entries(
    entry_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<u64, String> {
    repository::delete_entries(&state.db, &entry_ids)
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
pub async fn globalize_terminology_translations(
    input: GlobalizeTranslationsInput,
    state: State<'_, AppState>,
) -> Result<GlobalizeTranslationsSummary, String> {
    ensure_project(&state, &input.project_id).await?;
    repository::globalize_translations(
        &state.db,
        &input.entry_ids,
        &input.target_language,
        &input.project_id,
    )
    .await
    .map_err(stable_error)
}

#[tauri::command]
pub async fn globalize_filtered_terminology_translations(
    query: TerminologyQuery,
    state: State<'_, AppState>,
) -> Result<GlobalizeTranslationsSummary, String> {
    if let Some(project_id) = query.project_id.as_deref() {
        ensure_project(&state, project_id).await?;
    }
    repository::globalize_filtered_translations(&state.db, &query)
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

#[tauri::command]
pub async fn translate_terminology_entries(
    input: TranslateTerminologyCommandInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<StartTerminologyTranslationResponse, String> {
    if input.entry_ids.is_empty() || input.entry_ids.len() > 500 {
        return Err("terminology: select between 1 and 500 terminology entries".into());
    }
    if input.provider_config.url.trim().is_empty() || input.provider_config.model.trim().is_empty()
    {
        return Err("terminology: provider URL and model must not be empty".into());
    }
    if let Some(project_id) = input.project_id.as_deref() {
        ensure_project(&state, project_id).await?;
    }
    if matches!(
        input.scope,
        TranslationScope::Project | TranslationScope::Both
    ) && input.project_id.is_none()
    {
        return Err("terminology: project scope requires a project".into());
    }
    let job_id = uuid::Uuid::new_v4().to_string();
    let response = StartTerminologyTranslationResponse {
        job_id: job_id.clone(),
    };
    let pool = state.db.clone();
    let service = state.terminology.clone();
    tokio::spawn(async move {
        let _permit = match service.translation_permit().await {
            Ok(permit) => permit,
            Err(error) => {
                let _ = app.emit(
                    TRANSLATE_DONE_EVENT,
                    serde_json::json!({
                        "jobId": job_id, "summary": null, "error": stable_error(error)
                    }),
                );
                return;
            }
        };
        let provider = OpenAiCompatibleProvider::new_for_preset(
            &input.provider_config.provider_id,
            &input.provider_config.url,
            &input.provider_config.model,
            input.provider_config.api_key.as_deref(),
            std::time::Duration::from_secs(180),
        );
        if let Err(error) = provider.health_check().await {
            let _ = app.emit(
                TRANSLATE_DONE_EVENT,
                serde_json::json!({
                    "jobId": job_id, "summary": null, "error": error.to_string()
                }),
            );
            return;
        }
        let progress_app = app.clone();
        let progress_job_id = job_id.clone();
        let progress: TranslationProgressSink = std::sync::Arc::new(move |progress| {
            let _ = progress_app.emit(
                TRANSLATE_PROGRESS_EVENT,
                serde_json::json!({
                    "jobId": progress_job_id,
                    "processed": progress.processed,
                    "total": progress.total
                }),
            );
        });
        let (translation_project_id, also_global) = match input.scope {
            TranslationScope::Project => (input.project_id.as_deref(), false),
            TranslationScope::Global => (None, false),
            TranslationScope::Both => (input.project_id.as_deref(), true),
        };
        let result = translator::translate_selected(
            &pool,
            &provider,
            &input.entry_ids,
            &input.target_language,
            translation_project_id,
            also_global,
            input.provider_config.resource_profile,
            &input.provider_config.provider_id,
            &input.provider_config.model,
            Some(progress),
        )
        .await;
        let metrics = provider.drain_metrics();
        if !metrics.is_empty() {
            let _ = app.emit("h2s://llm/metrics", metrics);
        }
        match result {
            Ok(summary) => {
                let _ = app.emit(
                    TRANSLATE_DONE_EVENT,
                    serde_json::json!({
                        "jobId": job_id, "summary": summary, "error": null
                    }),
                );
            }
            Err(error) => {
                let _ = app.emit(
                    TRANSLATE_DONE_EVENT,
                    serde_json::json!({
                        "jobId": job_id, "summary": null, "error": stable_error(error)
                    }),
                );
            }
        }
    });
    Ok(response)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn wolf_speaker_compatibility_command_is_idempotent_and_indexes_occurrences() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let pool = crate::db::pool::init(file.path().to_str().unwrap())
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO projects (id, name, engine, game_path) \
             VALUES ('wolf-project', 'Wolf game', 'wolf', '/tmp/wolf')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
             VALUES ('wolf-file', 'wolf-project', 'CommonEvent.dat', '/tmp/CommonEvent.dat', 'wolf_dat')",
        )
        .execute(&pool)
        .await
        .unwrap();
        for (id, source) in [
            ("wolf-segment-1", "\\E勇者\nこんにちは"),
            ("wolf-segment-2", "\\E勇者\nさようなら"),
            ("wolf-segment-3", "地の文"),
        ] {
            sqlx::query(
                "INSERT INTO segments (\
                    id, source_file_id, json_key, source_text, target_text, status\
                 ) VALUES (?, 'wolf-file', ?, ?, '', 'untranslated')",
            )
            .bind(id)
            .bind(format!("/{id}"))
            .bind(source)
            .execute(&pool)
            .await
            .unwrap();
        }

        let first = extract_wolf_speakers_into_terminology(&pool, "wolf-project", "ja")
            .await
            .unwrap();
        let second = extract_wolf_speakers_into_terminology(&pool, "wolf-project", "ja")
            .await
            .unwrap();

        assert_eq!(first.len(), 1);
        assert_eq!(first[0].canonical_text, "勇者");
        assert_eq!(first[0].occurrence_count, 2);
        assert_eq!(first, second);
        let entry: (String, String, String) =
            sqlx::query_as("SELECT part_of_speech, semantic_type, origin FROM terminology_entries")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            entry,
            ("proper_noun".into(), "speaker".into(), "engine".into())
        );
        let occurrences: (i64, i64) =
            sqlx::query_as("SELECT COUNT(*), SUM(occurrence_count) FROM terminology_occurrences")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(occurrences, (2, 2));
    }
}
