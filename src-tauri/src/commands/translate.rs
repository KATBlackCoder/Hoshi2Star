//! Tauri commands for LLM translation and provider discovery.

use crate::{
    core::{glossary, manifest},
    domain::types::{ProviderConfig, SourceFile},
    llm::{
        pipeline,
        provider::{
            LlmProvider, OpenAiCompatibleProvider, PromptContextPolicy, TranslationContext,
        },
    },
    state::AppState,
};

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
        sqlx::query_as::<_, (String, String)>(
            "SELECT id, source_text FROM segments \
             WHERE source_file_id = ? AND (status = 'untranslated' OR target_text = '') \
             ORDER BY rowid",
        )
        .bind(fid)
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?
    } else {
        return Ok(());
    };

    if pairs.is_empty() {
        let _ = app.emit("h2s://llm/completed", serde_json::json!({ "count": 0 }));
        return Ok(());
    }

    let count = pairs.len();
    let _ = app.emit("h2s://llm/started", serde_json::json!({ "count": count }));

    let db = state.db.clone();
    let handle = app.clone();
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
            let msg = format!(
                "Fournisseur IA inaccessible ({}) : {e}",
                provider_config.url
            );
            let _ = handle.emit("h2s://llm/error", serde_json::json!({ "message": msg }));
            return;
        }

        // Resolve the project_id from the first segment so we can load glossary terms
        // and later update the manifest stats.
        let mut resolved_project_id: Option<String> = None;
        let mut project_engine = "mv_mz".to_string();
        let mut source_lang = "ja".to_string();
        let mut target_lang = "fr".to_string();
        let glossary_terms: Vec<(String, String)> = if let Some((first_id, _)) = pairs.first() {
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
            match row {
                Some((project_id, engine, project_source_lang, project_target_lang)) => {
                    resolved_project_id = Some(project_id.clone());
                    project_engine = engine;
                    source_lang = project_source_lang;
                    target_lang = project_target_lang;
                    let lang_pair = format!("{source_lang}-{target_lang}");
                    let sources: Vec<&str> = pairs.iter().map(|(_, src)| src.as_str()).collect();
                    glossary::relevant_terms(&db, &project_id, &lang_pair, &sources).await
                }
                None => vec![],
            }
        } else {
            vec![]
        };

        let context = TranslationContext {
            source_lang,
            target_lang,
            glossary_terms,
            engine: project_engine,
            batch_size: provider_config.effective_batch_size(),
            batch_delay_ms: provider_config.batch_delay_ms(),
            prompt_context_policy: PromptContextPolicy::EngineOwned,
            segment_contexts: vec![],
        };

        match pipeline::run(pairs, &provider, context, &db, &handle, None, None).await {
            Ok(_) => {
                // Update manifest stats once at end of batch (not per-segment)
                if let Some(ref pid) = resolved_project_id {
                    manifest::refresh_stats(&db, pid).await;
                }
                // Persist per-file translation duration when a whole file was translated
                if let Some(ref fid) = file_id {
                    let elapsed = translation_start.elapsed().as_secs() as i64;
                    let _ =
                        sqlx::query("UPDATE source_files SET translation_secs = ? WHERE id = ?")
                            .bind(elapsed)
                            .bind(fid)
                            .execute(&db)
                            .await;
                }
                let _ = handle.emit("h2s://llm/completed", serde_json::json!({ "count": count }));
            }
            Err(e) => {
                let _ = handle.emit(
                    "h2s://llm/error",
                    serde_json::json!({ "message": e.to_string() }),
                );
            }
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

    let total_untranslated: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM segments s \
         JOIN source_files sf ON s.source_file_id = sf.id \
         WHERE sf.project_id = ? AND (s.status = 'untranslated' OR s.target_text = '')",
    )
    .bind(&project_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    if total_untranslated == 0 {
        let _ = app.emit("h2s://llm/completed", serde_json::json!({ "count": 0 }));
        return Ok(());
    }

    let _ = app.emit(
        "h2s://llm/started",
        serde_json::json!({ "count": total_untranslated }),
    );

    let db = state.db.clone();
    let handle = app.clone();

    tokio::spawn(async move {
        let provider = OpenAiCompatibleProvider::new_for_preset(
            &provider_config.provider_id,
            &provider_config.url,
            &provider_config.model,
            provider_config.api_key.as_deref(),
            std::time::Duration::from_secs(180),
        );

        if let Err(e) = provider.health_check().await {
            let msg = format!(
                "Fournisseur IA inaccessible ({}) : {e}",
                provider_config.url
            );
            let _ = handle.emit("h2s://llm/error", serde_json::json!({ "message": msg }));
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
        let lang_pair = format!("{source_lang}-{target_lang}");

        let global_total = total_untranslated as usize;
        let mut done_offset: usize = 0;

        for file in &files {
            let pairs: Vec<(String, String)> = match sqlx::query_as::<_, (String, String)>(
                "SELECT id, source_text FROM segments \
                 WHERE source_file_id = ? AND (status = 'untranslated' OR target_text = '') \
                 ORDER BY rowid",
            )
            .bind(&file.id)
            .fetch_all(&db)
            .await
            {
                Ok(p) => p,
                Err(e) => {
                    let _ = handle.emit(
                        "h2s://llm/error",
                        serde_json::json!({ "message": e.to_string() }),
                    );
                    return;
                }
            };

            if pairs.is_empty() {
                continue;
            }

            // Load glossary terms filtered by batch content
            let sources: Vec<&str> = pairs.iter().map(|(_, src)| src.as_str()).collect();
            let glossary_terms =
                glossary::relevant_terms(&db, &project_id, &lang_pair, &sources).await;

            let context = TranslationContext {
                source_lang: source_lang.clone(),
                target_lang: target_lang.clone(),
                glossary_terms,
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
            )
            .await
            {
                Ok(_) => {
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
                Err(e) => {
                    let _ = handle.emit(
                        "h2s://llm/error",
                        serde_json::json!({ "message": e.to_string() }),
                    );
                    return;
                }
            }
        }

        // Update manifest stats once at the end
        manifest::refresh_stats(&db, &project_id).await;

        let _ = handle.emit(
            "h2s://llm/completed",
            serde_json::json!({ "count": total_untranslated }),
        );
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
