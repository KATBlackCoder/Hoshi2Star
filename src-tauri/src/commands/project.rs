//! Tauri commands for project lifecycle: open, browse, edit, export.
//!
//! All commands are `async`, return `Result<T, String>`, and receive the
//! database pool through `tauri::State<'_, AppState>`.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;
use std::path::Path;

use crate::{
    core::{glossary, qa, report, tm},
    engines::{
        detector::{detect_engine, find_data_dir, find_vx_ace_data_dir, Engine},
        mv_mz::{extractor, injector},
        vx_ace::{extractor as vx_extractor, injector as vx_injector},
    },
    llm::{
        pipeline,
        provider::{OllamaProvider, TranslationContext, DEFAULT_OLLAMA_MODEL, DEFAULT_OLLAMA_URL},
    },
    state::AppState,
};

// ---------------------------------------------------------------------------
// Domain types (serialised to/from TypeScript via Tauri IPC)
// ---------------------------------------------------------------------------

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
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            url: DEFAULT_OLLAMA_URL.to_string(),
            model: DEFAULT_OLLAMA_MODEL.to_string(),
            api_key: None,
        }
    }
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

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Open a game folder: detect engine, extract all segments, persist to DB.
///
/// Returns the newly created `Project` row (id, name, engine, game_path, timestamps).
#[tauri::command]
pub async fn open_project(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<Project, String> {
    let game_dir = Path::new(&path);

    // 1. Detect engine
    let engine = detect_engine(game_dir).map_err(|e| e.to_string())?;
    let engine_str = match engine {
        Engine::MvMz => "mv_mz",
        Engine::VxAce => "vx_ace",
    };

    // 2. Locate data directory (MV/MZ: data/ or www/data/ — VX Ace: Data/ or data/)
    let data_dir = match engine {
        Engine::VxAce => find_vx_ace_data_dir(game_dir)
            .ok_or_else(|| "Cannot find Data/ directory in VX Ace game folder".to_string())?,
        Engine::MvMz => find_data_dir(game_dir)
            .ok_or_else(|| "Cannot find data directory in game folder".to_string())?,
    };

    // 3. Read game title (MV/MZ: System.json gameTitle — VX Ace: System.rvdata2 game_title)
    let game_title = match engine {
        Engine::MvMz => read_game_title(&data_dir.join("System.json")),
        Engine::VxAce => read_vx_ace_game_title(&data_dir.join("System.rvdata2")),
    }
    .unwrap_or_else(|| {
        game_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string()
    });

    // 4. All inserts wrapped in a single transaction for performance
    let mut tx = state.db.begin().await.map_err(|e| e.to_string())?;

    let project_id = uuid::Uuid::new_v4().to_string();

    sqlx::query("INSERT INTO projects (id, name, engine, game_path) VALUES (?, ?, ?, ?)")
        .bind(&project_id)
        .bind(&game_title)
        .bind(engine_str)
        .bind(&path)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    // 5. Walk data directory: read, extract, insert source_files + segments
    match engine {
        Engine::MvMz => {
            let entries = collect_json_files(&data_dir).map_err(|e| e.to_string())?;
            for (file_name, file_path, file_type, json_value) in &entries {
                let file_id = uuid::Uuid::new_v4().to_string();
                sqlx::query(
                    "INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
                     VALUES (?, ?, ?, ?, ?)",
                )
                .bind(&file_id)
                .bind(&project_id)
                .bind(file_name)
                .bind(file_path)
                .bind(file_type)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;

                for seg in dispatch_extract(file_name, json_value) {
                    let seg_id = uuid::Uuid::new_v4().to_string();
                    sqlx::query(
                        "INSERT INTO segments (id, source_file_id, json_key, source_text) \
                         VALUES (?, ?, ?, ?)",
                    )
                    .bind(&seg_id)
                    .bind(&file_id)
                    .bind(&seg.key)
                    .bind(&seg.source)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?;
                }
            }
        }
        Engine::VxAce => {
            let entries = collect_rvdata2_files(&data_dir).map_err(|e| e.to_string())?;
            for (file_name, file_path, file_type, bytes) in &entries {
                let file_id = uuid::Uuid::new_v4().to_string();
                sqlx::query(
                    "INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
                     VALUES (?, ?, ?, ?, ?)",
                )
                .bind(&file_id)
                .bind(&project_id)
                .bind(file_name)
                .bind(file_path)
                .bind(file_type)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;

                for seg in vx_extractor::extract_from_bytes(file_name, bytes) {
                    let seg_id = uuid::Uuid::new_v4().to_string();
                    sqlx::query(
                        "INSERT INTO segments (id, source_file_id, json_key, source_text) \
                         VALUES (?, ?, ?, ?)",
                    )
                    .bind(&seg_id)
                    .bind(&file_id)
                    .bind(&seg.key)
                    .bind(&seg.source)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?;
                }
            }
        }
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    // 6. Fetch the newly created project row (includes DB-generated timestamps)
    let project = sqlx::query_as::<_, Project>(
        "SELECT id, name, engine, game_path, created_at, updated_at \
         FROM projects WHERE id = ?",
    )
    .bind(&project_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(project)
}

/// List all source files belonging to a project.
#[tauri::command]
pub async fn get_source_files(
    project_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SourceFile>, String> {
    sqlx::query_as::<_, SourceFile>(
        "SELECT id, project_id, file_name, file_path, file_type \
         FROM source_files WHERE project_id = ? ORDER BY file_name",
    )
    .bind(&project_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())
}

/// Return a paginated list of segments for a given source file.
///
/// `page` is 0-indexed; `page_size` should be between 1 and 500.
#[tauri::command]
pub async fn get_segments(
    project_id: String,
    file_id: String,
    page: i64,
    page_size: i64,
    state: tauri::State<'_, AppState>,
) -> Result<PaginatedSegments, String> {
    // Verify the file belongs to the given project (security check)
    let belongs: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM source_files WHERE id = ? AND project_id = ?")
            .bind(&file_id)
            .bind(&project_id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| e.to_string())?;

    if belongs == 0 {
        return Err("source file not found in project".to_string());
    }

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM segments WHERE source_file_id = ?")
        .bind(&file_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    let offset = page * page_size;
    let items = sqlx::query_as::<_, Segment>(
        "SELECT id, source_file_id, json_key, source_text, target_text, \
                status, qa_score, created_at, updated_at \
         FROM segments WHERE source_file_id = ? \
         ORDER BY rowid LIMIT ? OFFSET ?",
    )
    .bind(&file_id)
    .bind(page_size)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(PaginatedSegments {
        items,
        total,
        page,
        page_size,
    })
}

/// Save a manual translation for a segment.
///
/// 1. Runs QA checks (placeholders, line length, BOM) and stores the score.
/// 2. Inserts the (source, target) pair into the global TM (lang_pair: "ja-en").
/// 3. Sets `status = 'translated'` and updates `updated_at`.
///
/// Returns the updated `Segment` row (includes fresh `qa_score`).
#[tauri::command]
pub async fn update_segment(
    id: String,
    target_text: String,
    state: tauri::State<'_, AppState>,
) -> Result<Segment, String> {
    // Fetch source_text + engine for QA and TM
    let (source_text, engine): (String, String) = sqlx::query_as::<_, (String, String)>(
        "SELECT s.source_text, p.engine \
         FROM segments s \
         JOIN source_files sf ON s.source_file_id = sf.id \
         JOIN projects p ON sf.project_id = p.id \
         WHERE s.id = ?",
    )
    .bind(&id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    // QA check
    let qa_result = qa::check(&source_text, &target_text, &[]);
    let qa_score = qa_result.score as i64;

    // Update DB with new translation + QA score
    sqlx::query(
        "UPDATE segments \
         SET target_text = ?, status = 'translated', qa_score = ?, \
             updated_at = datetime('now') \
         WHERE id = ?",
    )
    .bind(&target_text)
    .bind(qa_score)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    // Insert into TM (best-effort — never fail the command if TM insert fails)
    let _ = tm::insert(&source_text, &target_text, &engine, "ja-en", &state.db).await;

    sqlx::query_as::<_, Segment>(
        "SELECT id, source_file_id, json_key, source_text, target_text, \
                status, qa_score, created_at, updated_at \
         FROM segments WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| e.to_string())
}

/// Re-inject all translated segments back into the game files.
///
/// Dispatches to the MV/MZ JSON injector or the VX Ace Marshal injector
/// based on the `file_type` prefix stored in `source_files`.
#[tauri::command]
pub async fn export_project(
    project_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let files = sqlx::query_as::<_, SourceFile>(
        "SELECT id, project_id, file_name, file_path, file_type \
         FROM source_files WHERE project_id = ?",
    )
    .bind(&project_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    for file in &files {
        // Only re-inject segments that have a translation
        let translations: Vec<(String, String)> = sqlx::query_as::<_, (String, String)>(
            "SELECT json_key, target_text FROM segments \
             WHERE source_file_id = ? AND target_text != ''",
        )
        .bind(&file.id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?;

        if translations.is_empty() {
            continue;
        }

        let pairs: Vec<(&str, &str)> = translations
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        if file.file_type.starts_with("vx_") {
            // VX Ace: binary Marshal round-trip
            let bytes = std::fs::read(&file.file_path)
                .map_err(|e| format!("read {}: {e}", file.file_name))?;
            let out = vx_injector::inject_and_serialize(&bytes, &pairs)
                .map_err(|e| format!("inject {}: {e}", file.file_name))?;
            std::fs::write(&file.file_path, out)
                .map_err(|e| format!("write {}: {e}", file.file_name))?;
        } else {
            // MV/MZ: JSON text round-trip
            let raw = std::fs::read_to_string(&file.file_path)
                .map_err(|e| format!("read {}: {e}", file.file_name))?;
            let mut json: serde_json::Value =
                serde_json::from_str(&raw).map_err(|e| format!("parse {}: {e}", file.file_name))?;
            injector::inject(&mut json, &pairs)
                .map_err(|e| format!("inject {}: {e}", file.file_name))?;
            let out = serde_json::to_string(&json)
                .map_err(|e| format!("serialise {}: {e}", file.file_name))?;
            std::fs::write(&file.file_path, out)
                .map_err(|e| format!("write {}: {e}", file.file_name))?;
        }
    }

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
    } else if let Some(fid) = file_id {
        sqlx::query_as::<_, (String, String)>(
            "SELECT id, source_text FROM segments \
             WHERE source_file_id = ? AND (status = 'untranslated' OR target_text = '') \
             ORDER BY rowid",
        )
        .bind(&fid)
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

    tokio::spawn(async move {
        let provider = OllamaProvider::new(
            &provider_config.url,
            &provider_config.model,
            std::time::Duration::from_secs(120),
        );

        // Resolve the project_id from the first segment so we can load glossary terms.
        let lang_pair = "ja-en";
        let glossary_terms: Vec<(String, String)> = if let Some((first_id, _)) = pairs.first() {
            let pid: Option<String> = sqlx::query_scalar(
                "SELECT sf.project_id FROM segments s \
                 JOIN source_files sf ON s.source_file_id = sf.id \
                 WHERE s.id = ? LIMIT 1",
            )
            .bind(first_id)
            .fetch_optional(&db)
            .await
            .ok()
            .flatten();
            match pid {
                Some(project_id) => {
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
        };

        match pipeline::run(pairs, &provider, context, &db, &handle).await {
            Ok(results) => {
                for r in results {
                    let status = if r.needs_review {
                        "needs_review"
                    } else {
                        "translated"
                    };
                    let _ = sqlx::query(
                        "UPDATE segments \
                         SET target_text = ?, status = ?, \
                             updated_at = datetime('now') \
                         WHERE id = ?",
                    )
                    .bind(&r.translated_text)
                    .bind(status)
                    .bind(&r.id)
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

/// Return TM fuzzy suggestions for a given source text.
/// Exact matches (score 1.0) sort to the top; fuzzy matches follow.
#[tauri::command]
pub async fn get_tm_suggestions(
    source_text: String,
    lang_pair: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<tm::TmSuggestion>, String> {
    tm::lookup_fuzzy(&source_text, &lang_pair, 0.80, 5, &state.db)
        .await
        .map_err(|e| e.to_string())
}

/// Run QA checks on a (source, target) pair and return the result.
///
/// Does not touch the database — useful for live checking in the UI
/// before the user saves.
#[tauri::command]
pub fn qa_check_segment(source_text: String, target_text: String) -> qa::QaResult {
    qa::check(&source_text, &target_text, &[])
}

/// Return a QA summary for all segments in a project.
#[tauri::command]
pub async fn get_qa_report(
    project_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<QaReport, String> {
    let total_segments: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM segments s \
         JOIN source_files sf ON s.source_file_id = sf.id \
         WHERE sf.project_id = ?",
    )
    .bind(&project_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let ok_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM segments s \
         JOIN source_files sf ON s.source_file_id = sf.id \
         WHERE sf.project_id = ? AND (qa_score = 100 OR qa_score IS NULL)",
    )
    .bind(&project_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    // Fetch all non-null qa_score to compute errors_by_type
    let qa_scores: Vec<i64> = sqlx::query_scalar(
        "SELECT qa_score FROM segments s \
         JOIN source_files sf ON s.source_file_id = sf.id \
         WHERE sf.project_id = ? AND qa_score IS NOT NULL AND qa_score < 100",
    )
    .bind(&project_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let error_count = qa_scores.len() as i64;

    // errors_by_type: approximate from score ranges
    // (exact type breakdown would require storing error types in DB — F3 improvement)
    let mut errors_by_type: HashMap<String, usize> = HashMap::new();
    for score in &qa_scores {
        if *score <= 75 {
            *errors_by_type.entry("placeholder".to_string()).or_insert(0) += 1;
        } else if *score <= 90 {
            *errors_by_type
                .entry("line_too_long".to_string())
                .or_insert(0) += 1;
        } else {
            *errors_by_type.entry("bom".to_string()).or_insert(0) += 1;
        }
    }

    Ok(QaReport {
        total_segments,
        ok_count,
        error_count,
        errors_by_type,
    })
}

/// Export a QA report for the project as a standalone HTML file.
///
/// QA errors are recalculated at export time — no DB storage required.
/// If all segments pass QA, the report is still generated with a pass message.
#[tauri::command]
pub async fn export_qa_report(
    project_id: String,
    output_path: String,
    lang: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let project_title: String = sqlx::query_scalar("SELECT name FROM projects WHERE id = ?")
        .bind(&project_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    let details = report::collect_qa_details(&state.db, &project_id)
        .await
        .map_err(|e| e.to_string())?;

    let html = report::generate_qa_html(&project_title, &details, &lang);

    tokio::fs::write(&output_path, html.as_bytes())
        .await
        .map_err(|e| e.to_string())
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

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Read `gameTitle` from a `System.json` path (MV/MZ).
fn read_game_title(system_json_path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(system_json_path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    v.get("gameTitle")?.as_str().map(|s| s.to_string())
}

/// Read `game_title` from a `System.rvdata2` path (VX Ace, snake_case field).
fn read_vx_ace_game_title(system_rvdata2_path: &Path) -> Option<String> {
    let bytes = std::fs::read(system_rvdata2_path).ok()?;
    let mv: marshal_rs::Value = marshal_rs::load_utf8(&bytes, None).ok()?;
    let json: serde_json::Value = mv.into();
    json.get("game_title")?.as_str().map(|s| s.to_string())
}

/// Collect all relevant `.rvdata2` files from a VX Ace data directory.
///
/// Returns `(file_name, absolute_file_path, file_type, raw_bytes)`.
/// Skips files with unrecognised names (`"unknown"` file type).
type RvData2Entry = (String, String, String, Vec<u8>);

fn collect_rvdata2_files(data_dir: &Path) -> Result<Vec<RvData2Entry>, std::io::Error> {
    let mut results = Vec::new();

    let entries = std::fs::read_dir(data_dir)?;
    for entry in entries.flatten() {
        let file_name = entry.file_name().to_string_lossy().to_string();
        if !file_name.ends_with(".rvdata2") {
            continue;
        }

        let file_type = classify_vx_ace_file(&file_name);
        if file_type == "unknown" {
            continue;
        }

        let file_path = entry.path().to_string_lossy().to_string();
        let bytes = match std::fs::read(entry.path()) {
            Ok(b) => b,
            Err(_) => continue, // skip unreadable files
        };

        results.push((file_name, file_path, file_type.to_string(), bytes));
    }

    // Deterministic order: sort by file name
    results.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(results)
}

/// Map a VX Ace filename to a file type identifier.
///
/// Prefixed with `"vx_"` to distinguish from MV/MZ types in `file_type` column.
/// Returns `"unknown"` for files that should be skipped.
fn classify_vx_ace_file(file_name: &str) -> &'static str {
    // Map files are Map001.rvdata2 … MapNNN.rvdata2 (but not MapInfos.rvdata2)
    if file_name.starts_with("Map")
        && file_name != "MapInfos.rvdata2"
        && file_name
            .trim_start_matches("Map")
            .trim_end_matches(".rvdata2")
            .parse::<u32>()
            .is_ok()
    {
        return "vx_map";
    }

    match file_name {
        "Actors.rvdata2" => "vx_actors",
        "Armors.rvdata2" => "vx_armors",
        "Classes.rvdata2" => "vx_classes",
        "CommonEvents.rvdata2" => "vx_common_events",
        "Enemies.rvdata2" => "vx_enemies",
        "Items.rvdata2" => "vx_items",
        "MapInfos.rvdata2" => "vx_map_infos",
        "Skills.rvdata2" => "vx_skills",
        "States.rvdata2" => "vx_states",
        "System.rvdata2" => "vx_system",
        "Troops.rvdata2" => "vx_troops",
        "Weapons.rvdata2" => "vx_weapons",
        _ => "unknown",
    }
}

/// Collect all relevant JSON files from the MV/MZ data directory.
///
/// Returns `(file_name, absolute_file_path, file_type, parsed_json)`.
fn collect_json_files(
    data_dir: &Path,
) -> Result<Vec<(String, String, String, serde_json::Value)>, std::io::Error> {
    let mut results = Vec::new();

    let entries = std::fs::read_dir(data_dir)?;
    for entry in entries.flatten() {
        let file_name = entry.file_name().to_string_lossy().to_string();
        if !file_name.ends_with(".json") {
            continue;
        }

        let file_type = classify_mv_mz_file(&file_name);
        if file_type == "unknown" {
            continue;
        }

        let file_path = entry.path().to_string_lossy().to_string();
        let content = match std::fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue, // skip unreadable files
        };
        let json: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => continue, // skip invalid JSON
        };

        results.push((file_name, file_path, file_type.to_string(), json));
    }

    // Deterministic order: sort by file name
    results.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(results)
}

/// Map an MV/MZ filename to a file type identifier.
///
/// Returns `"unknown"` for files that should be skipped.
fn classify_mv_mz_file(file_name: &str) -> &'static str {
    // Map files are Map001.json … MapNNN.json (but not MapInfos.json)
    if file_name.starts_with("Map")
        && file_name != "MapInfos.json"
        && file_name
            .trim_start_matches("Map")
            .trim_end_matches(".json")
            .parse::<u32>()
            .is_ok()
    {
        return "map";
    }

    match file_name {
        "Actors.json" => "actors",
        "Armors.json" => "armors",
        "Classes.json" => "classes",
        "CommonEvents.json" => "common_events",
        "Enemies.json" => "enemies",
        "Items.json" => "items",
        "MapInfos.json" => "map_infos",
        "Skills.json" => "skills",
        "States.json" => "states",
        "System.json" => "system",
        "Troops.json" => "troops",
        "Weapons.json" => "weapons",
        _ => "unknown",
    }
}

/// Dispatch extraction to the correct function based on file name.
fn dispatch_extract(file_name: &str, json: &serde_json::Value) -> Vec<extractor::ExtractedSegment> {
    if file_name.starts_with("Map")
        && file_name != "MapInfos.json"
        && file_name
            .trim_start_matches("Map")
            .trim_end_matches(".json")
            .parse::<u32>()
            .is_ok()
    {
        return extractor::extract_map(json);
    }

    match file_name {
        "Actors.json" => extractor::extract_actors(json),
        "Armors.json" => extractor::extract_armors(json),
        "Classes.json" => extractor::extract_classes(json),
        "CommonEvents.json" => extractor::extract_common_events(json),
        "Enemies.json" => extractor::extract_enemies(json),
        "Items.json" => extractor::extract_items(json),
        "MapInfos.json" => extractor::extract_map_infos(json),
        "Skills.json" => extractor::extract_skills(json),
        "States.json" => extractor::extract_states(json),
        "System.json" => extractor::extract_system(json),
        "Troops.json" => extractor::extract_troops(json),
        "Weapons.json" => extractor::extract_weapons(json),
        _ => vec![],
    }
}

/// Export the global TM for a given language pair to a TMX 1.4 file.
///
/// Writes the file to `output_path`. Compatible with OmegaT, Trados, memoQ.
#[tauri::command]
pub async fn export_tm(
    lang_pair: String,
    output_path: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let entries = sqlx::query_as::<_, tm::TmEntry>(
        "SELECT id, source_hash, source_text, target_text, engine, lang_pair, \
                confidence, created_at \
         FROM tm_entries WHERE lang_pair = ? ORDER BY created_at ASC",
    )
    .bind(&lang_pair)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let src_lang = lang_pair
        .split_once('-')
        .map(|(s, _)| s)
        .unwrap_or("und")
        .to_string();

    let tmx = tm::generate_tmx(&entries, &src_lang);

    tokio::fs::write(&output_path, tmx)
        .await
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_map_files() {
        assert_eq!(classify_mv_mz_file("Map001.json"), "map");
        assert_eq!(classify_mv_mz_file("Map999.json"), "map");
        assert_eq!(classify_mv_mz_file("MapInfos.json"), "map_infos");
        assert_eq!(classify_mv_mz_file("MapBoss.json"), "unknown"); // not a number
    }

    #[test]
    fn test_classify_data_files() {
        assert_eq!(classify_mv_mz_file("Actors.json"), "actors");
        assert_eq!(classify_mv_mz_file("System.json"), "system");
        assert_eq!(classify_mv_mz_file("Plugins.json"), "unknown");
        assert_eq!(classify_mv_mz_file("Unknown.json"), "unknown");
    }

    #[test]
    fn test_dispatch_actors() {
        let json = serde_json::json!([
            null,
            { "id": 1, "name": "主人公", "nickname": "", "profile": "" }
        ]);
        let segs = dispatch_extract("Actors.json", &json);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].source, "主人公");
    }

    #[test]
    fn test_dispatch_map() {
        let json = serde_json::json!({
            "events": [null, {
                "id": 1,
                "pages": [{
                    "list": [{ "code": 401, "parameters": ["セリフ"] }]
                }]
            }]
        });
        let segs = dispatch_extract("Map001.json", &json);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].source, "セリフ");
    }

    // --- VX Ace classify ---

    #[test]
    fn test_classify_vx_ace_map_files() {
        assert_eq!(classify_vx_ace_file("Map001.rvdata2"), "vx_map");
        assert_eq!(classify_vx_ace_file("Map999.rvdata2"), "vx_map");
        assert_eq!(classify_vx_ace_file("MapInfos.rvdata2"), "vx_map_infos");
        assert_eq!(classify_vx_ace_file("MapBoss.rvdata2"), "unknown");
    }

    #[test]
    fn test_classify_vx_ace_data_files() {
        assert_eq!(classify_vx_ace_file("Actors.rvdata2"), "vx_actors");
        assert_eq!(classify_vx_ace_file("Armors.rvdata2"), "vx_armors");
        assert_eq!(classify_vx_ace_file("Classes.rvdata2"), "vx_classes");
        assert_eq!(
            classify_vx_ace_file("CommonEvents.rvdata2"),
            "vx_common_events"
        );
        assert_eq!(classify_vx_ace_file("Enemies.rvdata2"), "vx_enemies");
        assert_eq!(classify_vx_ace_file("Items.rvdata2"), "vx_items");
        assert_eq!(classify_vx_ace_file("Skills.rvdata2"), "vx_skills");
        assert_eq!(classify_vx_ace_file("States.rvdata2"), "vx_states");
        assert_eq!(classify_vx_ace_file("System.rvdata2"), "vx_system");
        assert_eq!(classify_vx_ace_file("Troops.rvdata2"), "vx_troops");
        assert_eq!(classify_vx_ace_file("Weapons.rvdata2"), "vx_weapons");
        assert_eq!(classify_vx_ace_file("Scripts.rvdata2"), "unknown");
    }
}
