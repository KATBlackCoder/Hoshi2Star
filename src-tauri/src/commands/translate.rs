//! Tauri commands for LLM translation and provider discovery.

use std::sync::atomic::Ordering;

use crate::{
    core::manifest,
    domain::types::{ProviderConfig, SourceFile},
    llm::{
        pipeline::{self, PipelineError},
        progress::{LlmErrorPayload, RunSummary},
        provider::{
            LlmProvider, OpenAiCompatibleProvider, PromptContextPolicy, TranslationContext,
        },
    },
    state::AppState,
};

/// Segments a "translate the rest" run picks up: never translated, emptied,
/// or left by versions before R1 with the source copied as a `needs_review`
/// target (provider failures used to be stored that way).
pub(crate) const RESUMABLE_SEGMENT: &str = "(status = 'untranslated' OR target_text = '' \
     OR (status = 'needs_review' AND target_text = source_text))";

/// Emit the single error notice of a run.
fn emit_run_error(handle: &tauri::AppHandle, error: &PipelineError, total: usize) {
    use tauri::Emitter;
    let payload = match error {
        PipelineError::ProviderUnavailable { message, processed } => LlmErrorPayload {
            message: message.clone(),
            provider_unavailable: true,
            processed: *processed,
            total,
        },
        other => LlmErrorPayload {
            message: other.to_string(),
            provider_unavailable: false,
            processed: 0,
            total,
        },
    };
    let _ = handle.emit("h2s://llm/error", payload);
}

/// Emit the provider-unreachable notice raised by the start-up health check.
fn emit_health_error(
    handle: &tauri::AppHandle,
    url: &str,
    error: &crate::llm::provider::LlmError,
    total: usize,
) {
    use tauri::Emitter;
    let _ = handle.emit(
        "h2s://llm/error",
        LlmErrorPayload {
            message: format!("{url} : {error}"),
            provider_unavailable: true,
            processed: 0,
            total,
        },
    );
}

/// Ask the running translation to stop before its next batch.
#[tauri::command]
pub async fn cancel_translation(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.translation_cancel.store(true, Ordering::SeqCst);
    Ok(())
}

/// Launch a batch LLM translation in a background task (non-blocking).
///
/// If `ids` is non-empty, translates exactly those segments.
/// If `ids` is empty and `file_id` is provided, translates all untranslated
/// segments in that file (status = 'untranslated').
///
/// Spawns a `tokio::spawn` task and emits `h2s://llm/started` immediately,
/// then `h2s://llm/progress` per batch.
#[tauri::command]
pub async fn translate_segments(
    ids: Vec<String>,
    file_id: Option<String>,
    provider_config: ProviderConfig,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use tauri::Emitter;

    // Fetch (id, source_text) pairs — either from explicit ids or from file
    let pairs: Vec<(String, String)> = if !ids.is_empty() {
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let query = format!("SELECT id, source_text FROM segments WHERE id IN ({placeholders})");
        let mut q = sqlx::query_as::<_, (String, String)>(&query);
        for id in &ids {
            q = q.bind(id);
        }
        q.fetch_all(&state.db).await.map_err(|e| e.to_string())?
    } else if let Some(ref fid) = file_id {
        sqlx::query_as::<_, (String, String)>(&format!(
            "SELECT id, source_text FROM segments \
             WHERE source_file_id = ? AND {RESUMABLE_SEGMENT} ORDER BY rowid"
        ))
        .bind(fid)
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?
    } else {
        return Ok(());
    };

    if pairs.is_empty() {
        let _ = app.emit("h2s://llm/completed", RunSummary::default());
        return Ok(());
    }

    let count = pairs.len();
    let _ = app.emit("h2s://llm/started", serde_json::json!({ "count": count }));

    let db = state.db.clone();
    let handle = app.clone();
    let cancel = state.translation_cancel.clone();
    cancel.store(false, Ordering::SeqCst);
    let translation_start = std::time::Instant::now();

    tokio::spawn(async move {
        let provider = OpenAiCompatibleProvider::new_for_preset(
            &provider_config.provider_id,
            &provider_config.url,
            &provider_config.model,
            provider_config.api_key.as_deref(),
            std::time::Duration::from_secs(180),
        );

        if let Err(e) = provider.health_check().await {
            emit_health_error(&handle, &provider_config.url, &e, count);
            return;
        }

        // Resolve project metadata once; terminology is resolved later for
        // each exact provider request from segment occurrences.
        let mut resolved_project_id: Option<String> = None;
        let mut project_engine = "mv_mz".to_string();
        let mut source_lang = "ja".to_string();
        let mut target_lang = "fr".to_string();
        if let Some((first_id, _)) = pairs.first() {
            let row = sqlx::query_as::<_, (String, String, String, String)>(
                "SELECT sf.project_id, p.engine, \
                        COALESCE(p.source_language, 'ja'), \
                        COALESCE(p.target_language, 'en') FROM segments s \
                 JOIN source_files sf ON s.source_file_id = sf.id \
                 JOIN projects p ON p.id = sf.project_id \
                 WHERE s.id = ? LIMIT 1",
            )
            .bind(first_id)
            .fetch_optional(&db)
            .await
            .ok()
            .flatten();
            if let Some((project_id, engine, project_source_lang, project_target_lang)) = row {
                resolved_project_id = Some(project_id);
                project_engine = engine;
                source_lang = project_source_lang;
                target_lang = project_target_lang;
            }
        }

        let context = TranslationContext {
            source_lang,
            target_lang,
            terminology_hints: vec![],
            engine: project_engine,
            batch_size: provider_config.effective_batch_size(),
            batch_delay_ms: provider_config.batch_delay_ms(),
            prompt_context_policy: PromptContextPolicy::EngineOwned,
            segment_contexts: vec![],
        };

        let outcome =
            pipeline::run(pairs, &provider, context, &db, &handle, None, None, &cancel).await;
        if let Some(ref pid) = resolved_project_id {
            manifest::refresh_stats(&db, pid).await;
        }
        match outcome {
            Ok(outcome) => {
                // Persist per-file translation duration when a whole file was translated
                if let (Some(ref fid), false) = (&file_id, outcome.cancelled) {
                    let elapsed = translation_start.elapsed().as_secs() as i64;
                    let _ =
                        sqlx::query("UPDATE source_files SET translation_secs = ? WHERE id = ?")
                            .bind(elapsed)
                            .bind(fid)
                            .execute(&db)
                            .await;
                }
                let mut summary = RunSummary::default();
                summary.add(&outcome.results);
                summary.cancelled = outcome.cancelled;
                let _ = handle.emit("h2s://llm/completed", summary);
            }
            Err(error) => emit_run_error(&handle, &error, count),
        }
    });

    Ok(())
}

/// Translate all untranslated segments across all files in a project, sequentially.
///
/// After each file, if `cooldown_threshold_secs` have elapsed since the last
/// cooldown (or since start), the task sleeps for `cooldown_duration_secs` and
/// emits one `h2s://llm/cooling { remainingSecs }` event per second so the
/// frontend can display a countdown.
#[tauri::command]
pub async fn translate_all_segments(
    project_id: String,
    provider_config: ProviderConfig,
    cooldown_threshold_secs: u64,
    cooldown_duration_secs: u64,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use tauri::Emitter;

    let files = sqlx::query_as::<_, SourceFile>(
        "SELECT id, project_id, file_name, file_path, file_type, translation_secs \
         FROM source_files WHERE project_id = ? ORDER BY file_name",
    )
    .bind(&project_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let total_untranslated: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM segments \
         WHERE source_file_id IN (SELECT id FROM source_files WHERE project_id = ?) \
           AND {RESUMABLE_SEGMENT}"
    ))
    .bind(&project_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    if total_untranslated == 0 {
        let _ = app.emit("h2s://llm/completed", RunSummary::default());
        return Ok(());
    }

    let _ = app.emit(
        "h2s://llm/started",
        serde_json::json!({ "count": total_untranslated }),
    );

    let db = state.db.clone();
    let handle = app.clone();
    let cancel = state.translation_cancel.clone();
    cancel.store(false, Ordering::SeqCst);

    tokio::spawn(async move {
        let provider = OpenAiCompatibleProvider::new_for_preset(
            &provider_config.provider_id,
            &provider_config.url,
            &provider_config.model,
            provider_config.api_key.as_deref(),
            std::time::Duration::from_secs(180),
        );

        if let Err(e) = provider.health_check().await {
            emit_health_error(
                &handle,
                &provider_config.url,
                &e,
                total_untranslated as usize,
            );
            return;
        }

        let mut cooldown =
            pipeline::CooldownState::new(cooldown_threshold_secs, cooldown_duration_secs);

        let project_context: Option<(String, String, String)> = sqlx::query_as(
            "SELECT engine, COALESCE(source_language, 'ja'), \
                        COALESCE(target_language, 'en') FROM projects WHERE id = ?",
        )
        .bind(&project_id)
        .fetch_optional(&db)
        .await
        .ok()
        .flatten();
        let (project_engine, source_lang, target_lang) = project_context
            .unwrap_or_else(|| ("mv_mz".to_string(), "ja".to_string(), "fr".to_string()));

        let global_total = total_untranslated as usize;
        let mut done_offset: usize = 0;
        let mut summary = RunSummary::default();

        for file in &files {
            if cancel.load(Ordering::SeqCst) {
                summary.cancelled = true;
                break;
            }
            let pairs: Vec<(String, String)> =
                match sqlx::query_as::<_, (String, String)>(&format!(
                    "SELECT id, source_text FROM segments \
                     WHERE source_file_id = ? AND {RESUMABLE_SEGMENT} ORDER BY rowid"
                ))
                .bind(&file.id)
                .fetch_all(&db)
                .await
                {
                    Ok(p) => p,
                    Err(e) => {
                        let error = PipelineError::Database(e.to_string());
                        emit_run_error(&handle, &error, global_total);
                        return;
                    }
                };

            if pairs.is_empty() {
                continue;
            }

            let context = TranslationContext {
                source_lang: source_lang.clone(),
                target_lang: target_lang.clone(),
                terminology_hints: vec![],
                engine: project_engine.clone(),
                batch_size: provider_config.effective_batch_size(),
                batch_delay_ms: provider_config.batch_delay_ms(),
                prompt_context_policy: PromptContextPolicy::EngineOwned,
                segment_contexts: vec![],
            };

            let translation_start = std::time::Instant::now();
            let pair_count = pairs.len();

            match pipeline::run(
                pairs,
                &provider,
                context,
                &db,
                &handle,
                Some(&mut cooldown),
                Some((done_offset, global_total)),
                &cancel,
            )
            .await
            {
                Ok(outcome) => {
                    summary.add(&outcome.results);
                    if outcome.cancelled {
                        summary.cancelled = true;
                        break;
                    }
                    done_offset += pair_count;
                    // Update per-file translation duration
                    let elapsed = translation_start.elapsed().as_secs() as i64;
                    let _ =
                        sqlx::query("UPDATE source_files SET translation_secs = ? WHERE id = ?")
                            .bind(elapsed)
                            .bind(&file.id)
                            .execute(&db)
                            .await;
                }
                Err(error) => {
                    manifest::refresh_stats(&db, &project_id).await;
                    emit_run_error(&handle, &error, global_total);
                    return;
                }
            }
        }

        // Update manifest stats once at the end
        manifest::refresh_stats(&db, &project_id).await;

        let _ = handle.emit("h2s://llm/completed", summary);
    });

    Ok(())
}

/// Fetch the models exposed by an OpenAI-compatible provider.
#[tauri::command]
pub async fn get_provider_models(provider_config: ProviderConfig) -> Result<Vec<String>, String> {
    let provider = OpenAiCompatibleProvider::new_for_preset(
        &provider_config.provider_id,
        &provider_config.url,
        &provider_config.model,
        provider_config.api_key.as_deref(),
        std::time::Duration::from_secs(10),
    );
    provider
        .list_models()
        .await
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::RESUMABLE_SEGMENT;

    #[tokio::test]
    async fn resume_picks_untranslated_and_legacy_source_fallbacks_only() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = crate::db::pool::init(file.path().to_str().unwrap())
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO projects (id, name, engine, game_path) VALUES ('p', 'P', 'mv_mz', '/tmp')",
        )
        .execute(&db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
             VALUES ('f', 'p', 'Map001.json', '/tmp/Map001.json', 'map')",
        )
        .execute(&db)
        .await
        .unwrap();
        for (id, target, status) in [
            ("untranslated", "", "untranslated"),
            ("legacy-fallback", "剣", "needs_review"),
            ("real-review", "Swrod", "needs_review"),
            ("translated", "Sword", "translated"),
            ("reviewed", "Blade", "reviewed"),
        ] {
            sqlx::query(
                "INSERT INTO segments (id, source_file_id, json_key, source_text, target_text, status) \
                 VALUES (?, 'f', ?, '剣', ?, ?)",
            )
            .bind(id)
            .bind(format!("/{id}"))
            .bind(target)
            .bind(status)
            .execute(&db)
            .await
            .unwrap();
        }

        let picked: Vec<String> = sqlx::query_scalar(&format!(
            "SELECT id FROM segments WHERE {RESUMABLE_SEGMENT} ORDER BY id"
        ))
        .fetch_all(&db)
        .await
        .unwrap();

        assert_eq!(picked, vec!["legacy-fallback", "untranslated"]);
    }
}
