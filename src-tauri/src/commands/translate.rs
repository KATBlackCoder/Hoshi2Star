//! Tauri commands for LLM translation and provider discovery.

use serde::Deserialize;

use crate::{
    core::{glossary, manifest},
    domain::types::{ProviderConfig, SourceFile},
    llm::{
        pipeline,
        provider::{LlmProvider, OllamaProvider, TranslationContext},
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
        let provider = OllamaProvider::new(
            &provider_config.url,
            &provider_config.model,
            std::time::Duration::from_secs(120),
        );

        if let Err(e) = provider.health_check().await {
            let msg = format!(
                "Ollama inaccessible ({}) — vérifiez qu'il est démarré : {e}",
                provider_config.url
            );
            let _ = handle.emit("h2s://llm/error", serde_json::json!({ "message": msg }));
            return;
        }

        // Resolve the project_id from the first segment so we can load glossary terms
        // and later update the manifest stats.
        let lang_pair = "ja-en";
        let mut resolved_project_id: Option<String> = None;
        let mut project_engine = "mv_mz".to_string();
        let glossary_terms: Vec<(String, String)> = if let Some((first_id, _)) = pairs.first() {
            let row = sqlx::query_as::<_, (String, String)>(
                "SELECT sf.project_id, p.engine FROM segments s \
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
                Some((project_id, engine)) => {
                    resolved_project_id = Some(project_id.clone());
                    project_engine = engine;
                    let all_terms = glossary::list_for_project(&db, &project_id, lang_pair)
                        .await
                        .unwrap_or_default();

                    // Keep only terms whose source appears in at least one segment
                    let mut relevant: Vec<(String, String)> = all_terms
                        .iter()
                        .filter(|t| pairs.iter().any(|(_, src)| src.contains(&t.source_text)))
                        .take(20)
                        .map(|t| (t.source_text.clone(), t.target_text.clone()))
                        .collect();

                    // Fallback: 10 shortest terms (short proper names = less prompt noise)
                    if relevant.is_empty() {
                        let mut by_len = all_terms;
                        by_len.sort_by_key(|t| t.source_text.len());
                        relevant = by_len
                            .into_iter()
                            .take(10)
                            .map(|t| (t.source_text, t.target_text))
                            .collect();
                    }

                    relevant
                }
                None => vec![],
            }
        } else {
            vec![]
        };

        let context = TranslationContext {
            source_lang: "ja".to_string(),
            target_lang: "en".to_string(),
            glossary_terms,
            engine: project_engine,
            batch_size: provider_config.batch_size,
        };

        match pipeline::run(pairs, &provider, context, &db, &handle, None, None).await {
            Ok(_) => {
                // Update manifest stats once at end of batch (not per-segment)
                if let Some(ref pid) = resolved_project_id {
                    let stats_row = sqlx::query_as::<_, (String, i64, i64, i64, i64)>(
                        "SELECT p.game_path,
                            (SELECT COUNT(*) FROM source_files sf2 WHERE sf2.project_id = p.id),
                            (SELECT COUNT(*) FROM segments s2
                               JOIN source_files sf2 ON s2.source_file_id = sf2.id
                               WHERE sf2.project_id = p.id),
                            (SELECT COUNT(*) FROM segments s2
                               JOIN source_files sf2 ON s2.source_file_id = sf2.id
                               WHERE sf2.project_id = p.id AND s2.status = 'translated'),
                            (SELECT COUNT(*) FROM glossary_terms g
                               WHERE g.project_id = p.id OR g.project_id IS NULL)
                         FROM projects p WHERE p.id = ?",
                    )
                    .bind(pid)
                    .fetch_optional(&db)
                    .await;
                    if let Ok(Some((game_path, files, segs, translated, glossary))) = stats_row {
                        let _ = manifest::update_stats(
                            &game_path,
                            manifest::ManifestStats {
                                file_count: files as u32,
                                segment_count: segs as u32,
                                translated_count: translated as u32,
                                glossary_term_count: glossary as u32,
                            },
                        );
                    }
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
        let provider = OllamaProvider::new(
            &provider_config.url,
            &provider_config.model,
            std::time::Duration::from_secs(120),
        );

        if let Err(e) = provider.health_check().await {
            let msg = format!(
                "Ollama inaccessible ({}) — vérifiez qu'il est démarré : {e}",
                provider_config.url
            );
            let _ = handle.emit("h2s://llm/error", serde_json::json!({ "message": msg }));
            return;
        }

        let lang_pair = "ja-en";
        let mut cooldown =
            pipeline::CooldownState::new(cooldown_threshold_secs, cooldown_duration_secs);

        let project_engine: String = sqlx::query_scalar("SELECT engine FROM projects WHERE id = ?")
            .bind(&project_id)
            .fetch_optional(&db)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "mv_mz".to_string());

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
            let all_terms = glossary::list_for_project(&db, &project_id, lang_pair)
                .await
                .unwrap_or_default();
            let mut glossary_terms: Vec<(String, String)> = all_terms
                .iter()
                .filter(|t| pairs.iter().any(|(_, src)| src.contains(&t.source_text)))
                .take(20)
                .map(|t| (t.source_text.clone(), t.target_text.clone()))
                .collect();
            if glossary_terms.is_empty() {
                let mut by_len = all_terms;
                by_len.sort_by_key(|t| t.source_text.len());
                glossary_terms = by_len
                    .into_iter()
                    .take(10)
                    .map(|t| (t.source_text, t.target_text))
                    .collect();
            }

            let context = TranslationContext {
                source_lang: "ja".to_string(),
                target_lang: "en".to_string(),
                glossary_terms,
                engine: project_engine.clone(),
                batch_size: provider_config.batch_size,
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
        let stats_row = sqlx::query_as::<_, (String, i64, i64, i64, i64)>(
            "SELECT p.game_path,
                (SELECT COUNT(*) FROM source_files sf2 WHERE sf2.project_id = p.id),
                (SELECT COUNT(*) FROM segments s2
                   JOIN source_files sf2 ON s2.source_file_id = sf2.id
                   WHERE sf2.project_id = p.id),
                (SELECT COUNT(*) FROM segments s2
                   JOIN source_files sf2 ON s2.source_file_id = sf2.id
                   WHERE sf2.project_id = p.id AND s2.status = 'translated'),
                (SELECT COUNT(*) FROM glossary_terms g
                   WHERE g.project_id = p.id OR g.project_id IS NULL)
             FROM projects p WHERE p.id = ?",
        )
        .bind(&project_id)
        .fetch_optional(&db)
        .await;
        if let Ok(Some((game_path, files_c, segs, translated, glossary))) = stats_row {
            let _ = manifest::update_stats(
                &game_path,
                manifest::ManifestStats {
                    file_count: files_c as u32,
                    segment_count: segs as u32,
                    translated_count: translated as u32,
                    glossary_term_count: glossary as u32,
                },
            );
        }

        let _ = handle.emit(
            "h2s://llm/completed",
            serde_json::json!({ "count": total_untranslated }),
        );
    });

    Ok(())
}

/// Fetch the list of available models from an Ollama instance.
///
/// Calls `GET {url}/api/tags` with a 5-second timeout and returns the model
/// names. Returns an error string if the server is unreachable or the response
/// cannot be parsed.
#[tauri::command]
pub async fn get_ollama_models(url: String) -> Result<Vec<String>, String> {
    #[derive(Deserialize)]
    struct OllamaModel {
        name: String,
    }
    #[derive(Deserialize)]
    struct OllamaTagsResponse {
        models: Vec<OllamaModel>,
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let endpoint = format!("{}/api/tags", url.trim_end_matches('/'));
    let resp = client
        .get(&endpoint)
        .send()
        .await
        .map_err(|_| "Impossible de contacter Ollama — vérifiez l'URL".to_string())?;

    let body: OllamaTagsResponse = resp
        .json()
        .await
        .map_err(|e| format!("Réponse inattendue d'Ollama : {e}"))?;

    Ok(body.models.into_iter().map(|m| m.name).collect())
}
