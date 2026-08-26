//! Tauri commands for project lifecycle: open, browse, edit, export.
//!
//! All commands are `async`, return `Result<T, String>`, and receive the
//! database pool through `tauri::State<'_, AppState>`.

use std::path::{Path, PathBuf};

use sqlx::{QueryBuilder, Sqlite};

use crate::{
    core::{manifest, qa, tm},
    domain::types::*,
    engines::{
        detector::{detect_engine, guess_wolf_version_from_structure, Engine},
        filter,
        mv_mz::extractor,
        vx_ace::extractor as vx_extractor,
        wolf::extractor as wolf_extractor,
    },
    state::AppState,
};

const DEFAULT_SOURCE_LANG: &str = "ja";
const DEFAULT_TARGET_LANG: &str = "fr";

/// Normalize a compact BCP-47 language code at the Tauri boundary.
pub(crate) fn normalize_language(value: Option<String>, fallback: &str) -> Result<String, String> {
    let code = value.unwrap_or_else(|| fallback.to_string());
    let code = code.trim().to_ascii_lowercase();
    let valid = (2..=15).contains(&code.len())
        && code
            .split('-')
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_alphanumeric()));

    if valid {
        Ok(code)
    } else {
        Err(format!("invalid language code: {code}"))
    }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Open a game folder: detect engine, extract all segments, persist to DB.
///
/// If a `.hoshi2star.json` manifest exists and the project is already in the DB,
/// returns the existing project immediately (`was_restored: true`) without
/// re-extracting. Otherwise performs a full extraction (`was_restored: false`).
#[tauri::command]
pub async fn open_project(
    path: String,
    source_lang: Option<String>,
    target_lang: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<OpenProjectResult, String> {
    // 0. Smart restore: check manifest before doing any engine detection
    let mut preserved_project_id: Option<String> = None;
    match manifest::read_manifest(&path) {
        Ok(Some(mf)) => {
            let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects WHERE id = ?")
                .bind(&mf.project_id)
                .fetch_one(&state.db)
                .await
                .unwrap_or(0);
            if count > 0 {
                // Project exists in DB — update last_opened_at (via update_stats, which
                // refreshes the timestamp using the private now_iso8601())
                if let Err(e) = manifest::update_stats(&path, mf.stats.clone()) {
                    log::warn!("manifest update failed for {path}: {e}");
                }
                let project = sqlx::query_as::<_, Project>(
                    "SELECT id, name, engine, game_path, \
                            COALESCE(source_language, 'ja') AS source_lang, \
                            COALESCE(target_language, 'en') AS target_lang, \
                            created_at, updated_at \
                     FROM projects WHERE id = ?",
                )
                .bind(&mf.project_id)
                .fetch_one(&state.db)
                .await
                .map_err(|e| e.to_string())?;
                return Ok(OpenProjectResult {
                    project,
                    was_restored: true,
                });
            } else {
                // Manifest exists but project was deleted from DB — reuse the same ID
                preserved_project_id = Some(mf.project_id.clone());
            }
        }
        Ok(None) => {}
        Err(e) => {
            log::warn!("manifest read error for {path}: {e}");
        }
    }

    // Historical projects keep their DB pair through the restore path above.
    // Only a newly extracted project consumes the current defaults/settings.
    let source_lang = normalize_language(source_lang, DEFAULT_SOURCE_LANG)?;
    let target_lang = normalize_language(target_lang, DEFAULT_TARGET_LANG)?;

    // 1–3. Detection and metadata reads use synchronous filesystem/JSON APIs;
    // keep them off Tauri's async runtime threads.
    let setup_path = path.clone();
    let (engine, data_dir, game_title) = tokio::task::spawn_blocking(move || {
        let game_dir = PathBuf::from(&setup_path);
        let engine = detect_engine(&game_dir).map_err(|e| e.to_string())?;
        let data_dir = engine.data_dir(&game_dir)?;
        let game_title = engine.game_title(&game_dir, &data_dir).unwrap_or_else(|| {
            game_dir
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Unknown")
                .to_string()
        });
        Ok::<_, String>((engine, data_dir, game_title))
    })
    .await
    .map_err(|error| format!("project detection worker failed: {error}"))??;
    let engine_str = engine.db_str();

    // 4. All inserts wrapped in a single transaction for performance
    let mut tx = state.db.begin().await.map_err(|e| e.to_string())?;

    let project_id = preserved_project_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    sqlx::query(
        "INSERT INTO projects \
         (id, name, engine, game_path, source_language, target_language) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&project_id)
    .bind(&game_title)
    .bind(engine_str)
    .bind(&path)
    .bind(&source_lang)
    .bind(&target_lang)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    // 5. Walk data directory: extract (shared with debug_dump_segments), then
    //    insert source_files + segments inside the single transaction.
    let mut file_count: u32 = 0;
    let mut segment_count: u32 = 0;

    // The blocking producer parses at most a small bounded number of files
    // ahead of SQLite. MV/MZ JSON values are dropped immediately after each
    // file is converted to segments instead of retaining the whole game tree.
    let (file_sender, mut file_receiver) = tokio::sync::mpsc::channel(2);
    let producer_engine = engine.clone();
    let producer_game_dir = PathBuf::from(&path);
    let producer_data_dir = data_dir.clone();
    let extraction_worker = tokio::task::spawn_blocking(move || {
        visit_extracted_files(
            &producer_engine,
            &producer_game_dir,
            &producer_data_dir,
            |file| {
                file_sender
                    .blocking_send(file)
                    .map_err(|_| "extraction consumer closed".to_string())
            },
        )
    });

    while let Some(file) = file_receiver.recv().await {
        let file_id = uuid::Uuid::new_v4().to_string();
        insert_source_file(
            &mut tx,
            &file_id,
            &project_id,
            &file.file_name,
            &file.file_path,
            &file.file_type,
        )
        .await
        .map_err(|e| e.to_string())?;
        file_count += 1;

        insert_segments(&mut tx, &file_id, &file.segments)
            .await
            .map_err(|e| e.to_string())?;
        segment_count = segment_count.saturating_add(file.segments.len() as u32);
    }

    extraction_worker
        .await
        .map_err(|error| format!("extraction worker failed: {error}"))??;

    tx.commit().await.map_err(|e| e.to_string())?;

    // Write manifest (best-effort — never fail open_project if this errors)
    let manifest_data = manifest::ManifestData::new(
        project_id.clone(),
        game_title.clone(),
        engine_str.to_string(),
        path.clone(),
        manifest::ManifestStats {
            file_count,
            segment_count,
            translated_count: 0,
            glossary_term_count: 0,
        },
    );
    if let Err(e) = manifest::write_manifest(&path, &manifest_data) {
        log::warn!("manifest write failed for {path}: {e}");
    }

    // 6. Fetch the newly created project row (includes DB-generated timestamps)
    let project = sqlx::query_as::<_, Project>(
        "SELECT id, name, engine, game_path, \
                COALESCE(source_language, 'ja') AS source_lang, \
                COALESCE(target_language, 'en') AS target_lang, \
                created_at, updated_at \
         FROM projects WHERE id = ?",
    )
    .bind(&project_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(OpenProjectResult {
        project,
        was_restored: false,
    })
}

/// List all source files belonging to a project.
#[tauri::command]
pub async fn get_source_files(
    project_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SourceFile>, String> {
    fetch_source_files(&state.db, &project_id)
        .await
        .map_err(|e| e.to_string())
}

/// Per-file counters: `translated_count` counts `status = 'translated'`
/// strictly (a `needs_review` segment has a target but is NOT done), and
/// `needs_review_count` exposes the remaining review work per file.
async fn fetch_source_files(
    pool: &sqlx::SqlitePool,
    project_id: &str,
) -> Result<Vec<SourceFile>, sqlx::Error> {
    sqlx::query_as::<_, SourceFile>(
        "SELECT sf.id, sf.project_id, sf.file_name, sf.file_path, sf.file_type, \
                sf.translation_secs, \
                COUNT(s.id) as total_count, \
                SUM(CASE WHEN s.status = 'translated' THEN 1 ELSE 0 END) as translated_count, \
                SUM(CASE WHEN s.status = 'needs_review' THEN 1 ELSE 0 END) as needs_review_count \
         FROM source_files sf \
         LEFT JOIN segments s ON s.source_file_id = sf.id \
         WHERE sf.project_id = ? \
         GROUP BY sf.id \
         ORDER BY sf.file_name",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
}

/// Return a paginated list of segments for a given source file.
///
/// `page` is 0-indexed. The UI loads a file's segments in successive pages
/// (2000 per call) until `total` is reached — see `SegmentGrid::loadSegments`.
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
                segment_kind, scene_id, sequence_index, speaker, branch_path, \
                context_json, status, qa_score, created_at, updated_at \
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
/// 1. An empty/whitespace target clears the translation and restores
///    `status = 'untranslated'` without polluting the TM.
/// 2. Otherwise runs QA checks and stores the score.
/// 3. Inserts the (source, target) pair into the global TM for the project language pair.
///
/// Returns the updated `Segment` row (includes fresh `qa_score`).
#[tauri::command]
pub async fn update_segment(
    id: String,
    target_text: String,
    state: tauri::State<'_, AppState>,
) -> Result<Segment, String> {
    // Fetch source_text + project context for QA, TM and manifest refresh.
    let (source_text, engine, project_id, source_lang, target_lang): (
        String,
        String,
        String,
        String,
        String,
    ) = sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT s.source_text, p.engine, p.id, \
                COALESCE(p.source_language, 'ja'), \
                COALESCE(p.target_language, 'en') \
             FROM segments s \
             JOIN source_files sf ON s.source_file_id = sf.id \
             JOIN projects p ON sf.project_id = p.id \
             WHERE s.id = ?",
    )
    .bind(&id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let is_empty = target_text.trim().is_empty();
    let (stored_target, status, qa_score) = if is_empty {
        ("", "untranslated", None)
    } else {
        // QA check — use the project's engine for correct placeholder patterns.
        let qa_result = qa::check(&source_text, &target_text, &[], &engine);
        (
            target_text.as_str(),
            "translated",
            Some(qa_result.score as i64),
        )
    };

    // Update DB with new translation + QA score
    sqlx::query(
        "UPDATE segments \
         SET target_text = ?, status = ?, qa_score = ?, \
             updated_at = datetime('now') \
         WHERE id = ?",
    )
    .bind(stored_target)
    .bind(status)
    .bind(qa_score)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    // Insert into TM (best-effort — never fail the command if TM insert fails).
    if !is_empty {
        let lang_pair = format!("{source_lang}-{target_lang}");
        let _ = tm::insert(&source_text, &target_text, &engine, &lang_pair, &state.db).await;
    }

    // Update manifest stats (best-effort — indicative only, never blocks the command)
    manifest::refresh_stats(&state.db, &project_id).await;

    sqlx::query_as::<_, Segment>(
        "SELECT id, source_file_id, json_key, source_text, target_text, \
                segment_kind, scene_id, sequence_index, speaker, branch_path, \
                context_json, status, qa_score, created_at, updated_at \
         FROM segments WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| e.to_string())
}

/// Return lightweight project statistics used to gate the "Export All" action.
///
/// `untranslated_count` counts only `status = 'untranslated'` segments.
/// Segments with `status = 'needs_review'` carry a fallback target_text and are
/// considered exportable.
#[tauri::command]
pub async fn get_project_stats(
    project_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<ProjectStats, String> {
    fetch_project_stats(&state.db, &project_id)
        .await
        .map_err(|e| e.to_string())
}

/// Project-wide counters. All four statuses are counted so that
/// `untranslated + translated + needs_review + reviewed == total_segments`.
async fn fetch_project_stats(
    pool: &sqlx::SqlitePool,
    project_id: &str,
) -> Result<ProjectStats, sqlx::Error> {
    let (
        file_count,
        total_segments,
        untranslated_count,
        translated_count,
        needs_review_count,
        reviewed_count,
    ) = sqlx::query_as::<_, (i64, i64, i64, i64, i64, i64)>(
        "SELECT \
                (SELECT COUNT(*) FROM source_files WHERE project_id = ?1), \
                (SELECT COUNT(*) FROM segments s \
                   JOIN source_files sf ON s.source_file_id = sf.id \
                   WHERE sf.project_id = ?1), \
                (SELECT COUNT(*) FROM segments s \
                   JOIN source_files sf ON s.source_file_id = sf.id \
                   WHERE sf.project_id = ?1 AND s.status = 'untranslated'), \
                (SELECT COUNT(*) FROM segments s \
                   JOIN source_files sf ON s.source_file_id = sf.id \
                   WHERE sf.project_id = ?1 AND s.status = 'translated'), \
                (SELECT COUNT(*) FROM segments s \
                   JOIN source_files sf ON s.source_file_id = sf.id \
                   WHERE sf.project_id = ?1 AND s.status = 'needs_review'), \
                (SELECT COUNT(*) FROM segments s \
                   JOIN source_files sf ON s.source_file_id = sf.id \
                   WHERE sf.project_id = ?1 AND s.status = 'reviewed')",
    )
    .bind(project_id)
    .fetch_one(pool)
    .await?;

    Ok(ProjectStats {
        file_count,
        total_segments,
        untranslated_count,
        translated_count,
        needs_review_count,
        reviewed_count,
    })
}

/// List all projects in the DB, ordered by most recently updated.
#[tauri::command]
pub async fn list_projects(state: tauri::State<'_, AppState>) -> Result<Vec<Project>, String> {
    sqlx::query_as::<_, Project>(
        "SELECT id, name, engine, game_path, \
                COALESCE(source_language, 'ja') AS source_lang, \
                COALESCE(target_language, 'en') AS target_lang, \
                created_at, updated_at \
         FROM projects ORDER BY updated_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())
}

/// Delete a project and all its data (cascades to source_files + segments).
/// Also removes the `.hoshi2star.json` manifest from the game folder (best-effort).
#[tauri::command]
pub async fn delete_project(
    project_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // Fetch game_path before deleting so we can remove the manifest
    let game_path: Option<String> =
        sqlx::query_scalar("SELECT game_path FROM projects WHERE id = ?")
            .bind(&project_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| e.to_string())?;

    sqlx::query("DELETE FROM projects WHERE id = ?")
        .bind(&project_id)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(path) = game_path {
        let manifest_path = std::path::Path::new(&path).join(".hoshi2star.json");
        let _ = std::fs::remove_file(manifest_path);
    }

    Ok(())
}

/// Dump all translatable segments from any supported game to a JSON file.
///
/// Writes `hoshi2star_debug_extract.json` inside the game directory and returns
/// its absolute path. Works for Wolf RPG, RPG Maker MV/MZ, and VX Ace.
/// Intended for Claude-assisted analysis: open the JSON in a Claude conversation
/// to identify which texts need translation vs which can be skipped.
#[tauri::command]
pub async fn debug_dump_segments(game_path: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || debug_dump_segments_blocking(game_path))
        .await
        .map_err(|error| format!("debug extraction worker failed: {error}"))?
}

fn debug_dump_segments_blocking(game_path: String) -> Result<String, String> {
    use std::collections::HashMap;

    #[derive(serde::Serialize)]
    struct DebugSegment {
        key: String,
        source_text: String,
        #[serde(rename = "kind")]
        segment_kind: String,
        scene_id: Option<String>,
        sequence_index: Option<i64>,
        speaker: Option<String>,
        branch_path: Option<String>,
        context_json: Option<String>,
    }

    #[derive(serde::Serialize)]
    struct DebugFileEntry {
        file_name: String,
        file_type: String,
        segment_count: usize,
        segments: Vec<DebugSegment>,
    }

    #[derive(serde::Serialize)]
    struct DebugDump {
        engine: String,
        game_path: String,
        total_files: usize,
        total_segments: usize,
        by_kind: HashMap<String, usize>,
        files: Vec<DebugFileEntry>,
    }

    let game_dir = Path::new(&game_path);
    let engine = detect_engine(game_dir).map_err(|e| e.to_string())?;
    let engine_label = engine.db_str();
    let data_dir = engine.data_dir(game_dir)?;

    // Shared extraction (identical to open_project's, minus persistence).
    let dump_files: Vec<DebugFileEntry> = extract_project(&engine, game_dir, &data_dir)?
        .into_iter()
        .map(|file| {
            let segments: Vec<DebugSegment> = file
                .segments
                .into_iter()
                .map(|s| DebugSegment {
                    key: s.key,
                    source_text: s.source_text,
                    segment_kind: s.segment_kind,
                    scene_id: s.scene_id,
                    sequence_index: s.sequence_index,
                    speaker: s.speaker,
                    branch_path: s.branch_path,
                    context_json: s.context_json,
                })
                .collect();
            DebugFileEntry {
                file_name: file.file_name,
                file_type: file.file_type,
                segment_count: segments.len(),
                segments,
            }
        })
        .collect();

    let total_segments: usize = dump_files.iter().map(|f| f.segment_count).sum();
    let mut by_kind: HashMap<String, usize> = HashMap::new();
    for f in &dump_files {
        for s in &f.segments {
            *by_kind.entry(s.segment_kind.clone()).or_insert(0) += 1;
        }
    }

    let dump = DebugDump {
        engine: engine_label.to_string(),
        game_path: game_path.clone(),
        total_files: dump_files.len(),
        total_segments,
        by_kind,
        files: dump_files,
    };

    let json = serde_json::to_string_pretty(&dump).map_err(|e| e.to_string())?;
    let output_path = game_dir.join("hoshi2star_debug_extract.json");
    std::fs::write(&output_path, &json).map_err(|e| e.to_string())?;

    Ok(output_path.to_string_lossy().to_string())
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

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
    if filter::is_map_file(file_name, ".rvdata2") {
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

/// Collect sorted MV/MZ file metadata without reading or parsing file bodies.
/// Each JSON document is loaded later, immediately before extraction, so only
/// one parsed tree needs to be resident at a time.
type JsonFileEntry = (String, PathBuf, String);

fn collect_json_file_entries(data_dir: &Path) -> Result<Vec<JsonFileEntry>, std::io::Error> {
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

        results.push((file_name, entry.path(), file_type.to_string()));
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
    if filter::is_map_file(file_name, ".json") {
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
    if filter::is_map_file(file_name, ".json") {
        return extractor::extract_map_with_source(file_name, json);
    }

    match file_name {
        "Actors.json" => extractor::extract_actors(json),
        "Armors.json" => extractor::extract_armors(json),
        "Classes.json" => extractor::extract_classes(json),
        "CommonEvents.json" => extractor::extract_common_events_with_source(file_name, json),
        "Enemies.json" => extractor::extract_enemies(json),
        "Items.json" => extractor::extract_items(json),
        "MapInfos.json" => extractor::extract_map_infos(json),
        "Skills.json" => extractor::extract_skills(json),
        "States.json" => extractor::extract_states(json),
        "System.json" => extractor::extract_system(json),
        "Troops.json" => extractor::extract_troops_with_source(file_name, json),
        "Weapons.json" => extractor::extract_weapons(json),
        _ => vec![],
    }
}

// ---------------------------------------------------------------------------
// Shared extraction (open_project + debug_dump_segments)
// ---------------------------------------------------------------------------

/// One extracted segment normalized for persistence.
///
/// MV/MZ currently owns the only implemented context adapter. Every other
/// engine remains `unknown`/`None` until it implements its own logic.
pub(crate) struct ExtractedFileSeg {
    pub(crate) key: String,
    pub(crate) source_text: String,
    pub(crate) segment_kind: String,
    pub(crate) scene_id: Option<String>,
    pub(crate) sequence_index: Option<i64>,
    pub(crate) speaker: Option<String>,
    pub(crate) branch_path: Option<String>,
    pub(crate) context_json: Option<String>,
}

impl ExtractedFileSeg {
    fn without_context(key: String, source_text: String) -> Self {
        Self {
            key,
            source_text,
            segment_kind: "unknown".to_string(),
            scene_id: None,
            sequence_index: None,
            speaker: None,
            branch_path: None,
            context_json: None,
        }
    }
}

/// One extracted source file, normalized across engines.
pub(crate) struct ExtractedFile {
    pub(crate) file_name: String,
    pub(crate) file_path: String,
    pub(crate) file_type: String,
    pub(crate) segments: Vec<ExtractedFileSeg>,
}

/// Extract every source file + segment for `engine` from `game_dir`, normalized
/// to a common shape consumed by both `open_project` (persistence) and
/// `debug_dump_segments` (JSON dump).
///
/// Per-file order is preserved (MV/VX sorted by name inside the `collect_*`
/// helpers; Wolf in extractor order) and per-segment order is extractor order —
/// `get_segments`' `ORDER BY rowid` relies on this insertion order.
fn extract_project(
    engine: &Engine,
    game_dir: &Path,
    data_dir: &Path,
) -> Result<Vec<ExtractedFile>, String> {
    let mut files = Vec::new();
    visit_extracted_files(engine, game_dir, data_dir, |file| {
        files.push(file);
        Ok(())
    })?;
    Ok(files)
}

/// Visit extracted files in deterministic order. The callback makes the same
/// extractor usable by the bounded open-project producer and by diagnostics
/// that intentionally collect a complete dump.
pub(crate) fn visit_extracted_files<F>(
    engine: &Engine,
    game_dir: &Path,
    data_dir: &Path,
    mut visit: F,
) -> Result<(), String>
where
    F: FnMut(ExtractedFile) -> Result<(), String>,
{
    match engine {
        Engine::MvMz => {
            for (file_name, source_path, file_type) in
                collect_json_file_entries(data_dir).map_err(|e| e.to_string())?
            {
                let content = match std::fs::read_to_string(&source_path) {
                    Ok(content) => content,
                    Err(error) => {
                        log::warn!(
                            "skipping unreadable JSON {}: {error}",
                            source_path.display()
                        );
                        continue;
                    }
                };
                let json_value: serde_json::Value = match serde_json::from_str(&content) {
                    Ok(value) => value,
                    Err(error) => {
                        log::warn!("skipping invalid JSON {}: {error}", source_path.display());
                        continue;
                    }
                };
                let segments = dispatch_extract(&file_name, &json_value)
                    .into_iter()
                    .map(|s| ExtractedFileSeg {
                        key: s.key,
                        source_text: s.source,
                        segment_kind: s.kind.as_str().to_string(),
                        scene_id: s.context.scene_id,
                        sequence_index: s.context.sequence_index,
                        speaker: s.context.speaker,
                        branch_path: s.context.branch_path,
                        context_json: s.context.context_json,
                    })
                    .collect();
                visit(ExtractedFile {
                    file_name,
                    file_path: source_path.to_string_lossy().to_string(),
                    file_type,
                    segments,
                })?;
            }
        }
        Engine::VxAce => {
            for (file_name, file_path, file_type, bytes) in
                collect_rvdata2_files(data_dir).map_err(|e| e.to_string())?
            {
                let segments = vx_extractor::extract_from_bytes(&file_name, &bytes)
                    .into_iter()
                    .map(|s| ExtractedFileSeg::without_context(s.key, s.source))
                    .collect();
                visit(ExtractedFile {
                    file_name,
                    file_path,
                    file_type,
                    segments,
                })?;
            }
        }
        Engine::Wolf => {
            let wolf_version = guess_wolf_version_from_structure(game_dir);
            let entries = wolf_extractor::extract_all_wolf(game_dir, &wolf_version)
                .map_err(|e| e.to_string())?;
            for (file_name, file_type, segs) in entries {
                // Wolf file_path is constructed (not carried): map files live in
                // Data/MapData, everything else in Data/BasicData.
                let sub_dir = if file_type == "wolf_map" {
                    "MapData"
                } else {
                    "BasicData"
                };
                let file_path = game_dir
                    .join("Data")
                    .join(sub_dir)
                    .join(&file_name)
                    .to_string_lossy()
                    .to_string();
                let segments = segs
                    .into_iter()
                    .map(|s| ExtractedFileSeg::without_context(s.key, s.source_text))
                    .collect();
                visit(ExtractedFile {
                    file_name,
                    file_path,
                    file_type,
                    segments,
                })?;
            }
        }
    }
    Ok(())
}

/// Insert one `source_files` row inside the open-project transaction.
pub(crate) async fn insert_source_file(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    file_id: &str,
    project_id: &str,
    file_name: &str,
    file_path: &str,
    file_type: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(file_id)
    .bind(project_id)
    .bind(file_name)
    .bind(file_path)
    .bind(file_type)
    .execute(&mut **tx)
    .await
    .map(|_| ())
}

/// Insert segment rows in chunks that stay below SQLite's default 999 bind
/// parameter limit (90 rows × 10 values = 900 binds).
pub(crate) async fn insert_segments(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    file_id: &str,
    segments: &[ExtractedFileSeg],
) -> Result<(), sqlx::Error> {
    for chunk in segments.chunks(90) {
        let mut query = QueryBuilder::<Sqlite>::new(
            "INSERT INTO segments \
             (id, source_file_id, json_key, source_text, segment_kind, scene_id, \
              sequence_index, speaker, branch_path, context_json) ",
        );
        query.push_values(chunk, |mut row, segment| {
            row.push_bind(uuid::Uuid::new_v4().to_string())
                .push_bind(file_id)
                .push_bind(&segment.key)
                .push_bind(&segment.source_text)
                .push_bind(&segment.segment_kind)
                .push_bind(&segment.scene_id)
                .push_bind(segment.sequence_index)
                .push_bind(&segment.speaker)
                .push_bind(&segment.branch_path)
                .push_bind(&segment.context_json);
        });
        query.build().execute(&mut **tx).await?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_language_accepts_bcp47_like_codes() {
        assert_eq!(
            normalize_language(Some(" PT-BR ".to_string()), "ja").unwrap(),
            "pt-br"
        );
        assert_eq!(normalize_language(None, "fr").unwrap(), "fr");
    }

    #[test]
    fn test_normalize_language_rejects_invalid_codes() {
        for code in ["", "f", "fr_FR", "-fr", "fr-", "fr--ca", "français"] {
            assert!(
                normalize_language(Some(code.to_string()), "ja").is_err(),
                "{code} should be rejected"
            );
        }
    }

    // --- Phase 5 (audit 2026-07-01): counter semantics, one segment per status ---

    /// Seed one project / one file / four segments, one per status.
    /// Every segment has a NON-empty target so the strict counters cannot be
    /// satisfied by the old `target_text != ''` definition.
    async fn seed_one_per_status(pool: &sqlx::SqlitePool) {
        sqlx::query(
            "INSERT INTO projects (id, name, engine, game_path) VALUES ('p1','T','mv_mz','/tmp')",
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
             VALUES ('f1','p1','Map001.json','/tmp/Map001.json','map')",
        )
        .execute(pool)
        .await
        .unwrap();
        for (id, status) in [
            ("s1", "untranslated"),
            ("s2", "translated"),
            ("s3", "needs_review"),
            ("s4", "reviewed"),
        ] {
            sqlx::query(
                "INSERT INTO segments (id, source_file_id, json_key, source_text, target_text, status) \
                 VALUES (?, 'f1', ?, 'ソース', 'draft', ?)",
            )
            .bind(id)
            .bind(format!("/{id}"))
            .bind(status)
            .execute(pool)
            .await
            .unwrap();
        }
    }

    #[tokio::test]
    async fn test_counter_semantics_one_segment_per_status() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let pool = crate::db::pool::init(tmp.path().to_str().unwrap())
            .await
            .unwrap();
        seed_one_per_status(&pool).await;

        // Per-file: translated_count is STRICT (status = 'translated'), not
        // "any non-empty target"; needs_review work is exposed separately.
        let files = fetch_source_files(&pool, "p1").await.unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].total_count, 4);
        assert_eq!(
            files[0].translated_count, 1,
            "translated_count must count status='translated' only"
        );
        assert_eq!(files[0].needs_review_count, 1);

        // Project-wide: all four statuses are counted and sum to the total.
        let stats = fetch_project_stats(&pool, "p1").await.unwrap();
        assert_eq!(stats.total_segments, 4);
        assert_eq!(stats.untranslated_count, 1);
        assert_eq!(stats.translated_count, 1);
        assert_eq!(stats.needs_review_count, 1);
        assert_eq!(stats.reviewed_count, 1);
        assert_eq!(
            stats.untranslated_count
                + stats.translated_count
                + stats.needs_review_count
                + stats.reviewed_count,
            stats.total_segments,
            "the four statuses must sum to the total"
        );
    }

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
        assert_eq!(segs[0].kind.as_str(), "dialogue");
        assert_eq!(
            segs[0].context.scene_id.as_deref(),
            Some("Map001.json:event:1:page:0")
        );
        assert_eq!(segs[0].context.sequence_index, Some(0));
    }

    #[test]
    fn test_non_mv_mz_normalization_is_explicitly_context_free() {
        let segment = ExtractedFileSeg::without_context("/key".into(), "text".into());

        assert_eq!(segment.segment_kind, "unknown");
        assert_eq!(segment.scene_id, None);
        assert_eq!(segment.sequence_index, None);
        assert_eq!(segment.speaker, None);
        assert_eq!(segment.branch_path, None);
        assert_eq!(segment.context_json, None);
    }

    #[tokio::test]
    async fn test_insert_segments_persists_complete_context_contract() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let pool = crate::db::pool::init(tmp.path().to_str().unwrap())
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO projects (id, name, engine, game_path) \
             VALUES ('p1','T','mv_mz','/tmp')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO source_files \
             (id, project_id, file_name, file_path, file_type) \
             VALUES ('f1','p1','Map001.json','/tmp/Map001.json','map')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let segments = vec![ExtractedFileSeg {
            key: "/events/1/pages/0/list/1/parameters/0".into(),
            source_text: "出発します！".into(),
            segment_kind: "dialogue".into(),
            scene_id: Some("Map001.json:event:1:page:0".into()),
            sequence_index: Some(1),
            speaker: Some("勇者".into()),
            branch_path: Some("if:0".into()),
            context_json: Some(r#"{"schemaVersion":1}"#.into()),
        }];
        let mut tx = pool.begin().await.unwrap();
        insert_segments(&mut tx, "f1", &segments).await.unwrap();
        tx.commit().await.unwrap();

        let row: (String, String, i64, String, String, String) = sqlx::query_as(
            "SELECT segment_kind, scene_id, sequence_index, speaker, branch_path, context_json \
             FROM segments WHERE source_file_id = 'f1'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(row.0, "dialogue");
        assert_eq!(row.1, "Map001.json:event:1:page:0");
        assert_eq!(row.2, 1);
        assert_eq!(row.3, "勇者");
        assert_eq!(row.4, "if:0");
        assert_eq!(row.5, r#"{"schemaVersion":1}"#);
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
