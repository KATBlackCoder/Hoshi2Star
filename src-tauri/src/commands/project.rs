//! Tauri commands for project lifecycle: open, browse, edit, export.
//!
//! All commands are `async`, return `Result<T, String>`, and receive the
//! database pool through `tauri::State<'_, AppState>`.

use std::path::Path;

use crate::{
    core::{manifest, qa, tm},
    domain::types::*,
    engines::{
        detector::{
            detect_engine, find_data_dir, find_vx_ace_data_dir, guess_wolf_version_from_structure,
            Engine,
        },
        mv_mz::extractor,
        vx_ace::extractor as vx_extractor,
        wolf::extractor as wolf_extractor,
    },
    state::AppState,
};

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
                    "SELECT id, name, engine, game_path, created_at, updated_at \
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

    let game_dir = Path::new(&path);

    // 1. Detect engine
    let engine = detect_engine(game_dir).map_err(|e| e.to_string())?;
    let engine_str = match engine {
        Engine::MvMz => "mv_mz",
        Engine::VxAce => "vx_ace",
        Engine::Wolf => "wolf",
    };

    // 2. Locate data directory (MV/MZ: data/ or www/data/ — VX Ace: Data/ or data/)
    //    Wolf uses game_dir directly (extract_all_wolf does its own data/ walk).
    let data_dir = match engine {
        Engine::VxAce => find_vx_ace_data_dir(game_dir)
            .ok_or_else(|| "Cannot find Data/ directory in VX Ace game folder".to_string())?,
        Engine::MvMz => find_data_dir(game_dir)
            .ok_or_else(|| "Cannot find data directory in game folder".to_string())?,
        Engine::Wolf => game_dir.to_path_buf(),
    };

    // 3. Read game title (MV/MZ: System.json — VX Ace: System.rvdata2 — Wolf: Game.ini)
    let game_title = match engine {
        Engine::MvMz => read_game_title(&data_dir.join("System.json")),
        Engine::VxAce => read_vx_ace_game_title(&data_dir.join("System.rvdata2")),
        Engine::Wolf => read_wolf_game_title(game_dir),
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

    let project_id = preserved_project_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    sqlx::query("INSERT INTO projects (id, name, engine, game_path) VALUES (?, ?, ?, ?)")
        .bind(&project_id)
        .bind(&game_title)
        .bind(engine_str)
        .bind(&path)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    // 5. Walk data directory: read, extract, insert source_files + segments
    let mut file_count: u32 = 0;
    let mut segment_count: u32 = 0;

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
                file_count += 1;

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
                    segment_count += 1;
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
                file_count += 1;

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
                    segment_count += 1;
                }
            }
        }
        Engine::Wolf => {
            let wolf_version = guess_wolf_version_from_structure(game_dir);
            let entries = wolf_extractor::extract_all_wolf(game_dir, &wolf_version)
                .map_err(|e| e.to_string())?;
            for (file_name, file_type, segments) in &entries {
                let file_id = uuid::Uuid::new_v4().to_string();
                let sub_dir = if file_type == "wolf_map" {
                    "MapData"
                } else {
                    "BasicData"
                };
                let file_path = game_dir
                    .join("Data")
                    .join(sub_dir)
                    .join(file_name)
                    .to_string_lossy()
                    .to_string();
                sqlx::query(
                    "INSERT INTO source_files \
                     (id, project_id, file_name, file_path, file_type) \
                     VALUES (?, ?, ?, ?, ?)",
                )
                .bind(&file_id)
                .bind(&project_id)
                .bind(file_name)
                .bind(&file_path)
                .bind(file_type)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
                file_count += 1;

                for seg in segments {
                    let seg_id = uuid::Uuid::new_v4().to_string();
                    sqlx::query(
                        "INSERT INTO segments \
                         (id, source_file_id, json_key, source_text) \
                         VALUES (?, ?, ?, ?)",
                    )
                    .bind(&seg_id)
                    .bind(&file_id)
                    .bind(&seg.key)
                    .bind(&seg.source_text)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?;
                    segment_count += 1;
                }
            }
        }
    }

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
        "SELECT id, name, engine, game_path, created_at, updated_at \
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
    sqlx::query_as::<_, SourceFile>(
        "SELECT sf.id, sf.project_id, sf.file_name, sf.file_path, sf.file_type, \
                sf.translation_secs, \
                COUNT(s.id) as total_count, \
                SUM(CASE WHEN s.target_text != '' THEN 1 ELSE 0 END) as translated_count \
         FROM source_files sf \
         LEFT JOIN segments s ON s.source_file_id = sf.id \
         WHERE sf.project_id = ? \
         GROUP BY sf.id \
         ORDER BY sf.file_name",
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

    // QA check — use the project's engine for correct placeholder patterns.
    let qa_result = qa::check(&source_text, &target_text, &[], &engine);
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

    // Update manifest stats (best-effort — indicative only, never blocks the command)
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
         FROM segments s
           JOIN source_files sf ON s.source_file_id = sf.id
           JOIN projects p ON sf.project_id = p.id
         WHERE s.id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.db)
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
    let (file_count, total_segments, untranslated_count, translated_count, needs_review_count) =
        sqlx::query_as::<_, (i64, i64, i64, i64, i64)>(
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
                   WHERE sf.project_id = ?1 AND s.status = 'needs_review')",
        )
        .bind(&project_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(ProjectStats {
        file_count,
        total_segments,
        untranslated_count,
        translated_count,
        needs_review_count,
    })
}

/// List all projects in the DB, ordered by most recently updated.
#[tauri::command]
pub async fn list_projects(state: tauri::State<'_, AppState>) -> Result<Vec<Project>, String> {
    sqlx::query_as::<_, Project>(
        "SELECT id, name, engine, game_path, created_at, updated_at \
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
    use crate::engines::wolf::extractor::WolfSegmentKind;
    use std::collections::HashMap;

    #[derive(serde::Serialize)]
    struct DebugSegment {
        key: String,
        source_text: String,
        kind: String,
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

    let mut dump_files: Vec<DebugFileEntry> = Vec::new();
    let engine_label;

    match engine {
        Engine::Wolf => {
            engine_label = "wolf";
            let wolf_version = guess_wolf_version_from_structure(game_dir);
            let entries = wolf_extractor::extract_all_wolf(game_dir, &wolf_version)
                .map_err(|e| e.to_string())?;

            for (file_name, file_type, segs) in entries {
                let segments: Vec<DebugSegment> = segs
                    .into_iter()
                    .map(|s| {
                        let kind = match &s.kind {
                            WolfSegmentKind::MapMessage { .. } => "map_message",
                            WolfSegmentKind::DatabaseField { .. } => "database_field",
                            WolfSegmentKind::CommonEventMessage { .. } => "common_event_message",
                        }
                        .to_string();
                        DebugSegment {
                            key: s.key,
                            source_text: s.source_text,
                            kind,
                        }
                    })
                    .collect();
                let segment_count = segments.len();
                dump_files.push(DebugFileEntry {
                    file_name,
                    file_type,
                    segment_count,
                    segments,
                });
            }
        }

        Engine::MvMz => {
            engine_label = "mv_mz";
            let data_dir = find_data_dir(game_dir)
                .ok_or_else(|| "Cannot find data directory in game folder".to_string())?;

            for (file_name, _file_path, file_type, json_value) in
                collect_json_files(&data_dir).map_err(|e| e.to_string())?
            {
                let raw_segs = dispatch_extract(&file_name, &json_value);
                let segments: Vec<DebugSegment> = raw_segs
                    .into_iter()
                    .map(|s| DebugSegment {
                        key: s.key,
                        source_text: s.source,
                        kind: format!("{:?}", s.kind),
                    })
                    .collect();
                let segment_count = segments.len();
                dump_files.push(DebugFileEntry {
                    file_name,
                    file_type,
                    segment_count,
                    segments,
                });
            }
        }

        Engine::VxAce => {
            engine_label = "vx_ace";
            let data_dir = find_vx_ace_data_dir(game_dir)
                .ok_or_else(|| "Cannot find Data/ directory in VX Ace game folder".to_string())?;

            for (file_name, _file_path, file_type, bytes) in
                collect_rvdata2_files(&data_dir).map_err(|e| e.to_string())?
            {
                let raw_segs = vx_extractor::extract_from_bytes(&file_name, &bytes);
                let segments: Vec<DebugSegment> = raw_segs
                    .into_iter()
                    .map(|s| DebugSegment {
                        key: s.key,
                        source_text: s.source,
                        kind: format!("{:?}", s.kind),
                    })
                    .collect();
                let segment_count = segments.len();
                dump_files.push(DebugFileEntry {
                    file_name,
                    file_type,
                    segment_count,
                    segments,
                });
            }
        }
    }

    let total_segments: usize = dump_files.iter().map(|f| f.segment_count).sum();
    let mut by_kind: HashMap<String, usize> = HashMap::new();
    for f in &dump_files {
        for s in &f.segments {
            *by_kind.entry(s.kind.clone()).or_insert(0) += 1;
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

/// Read `GameTitle` from `Game.ini` in a Wolf RPG game directory.
///
/// `Game.ini` uses Shift-JIS encoding for v2 games. We read as bytes and
/// attempt Shift-JIS decoding; falls back to UTF-8 then lossy on failure.
/// Returns `None` if the file is absent or the key is not found.
fn read_wolf_game_title(game_dir: &Path) -> Option<String> {
    let ini_path = game_dir.join("Game.ini");
    let bytes = std::fs::read(&ini_path).ok()?;

    // Try Shift-JIS first (most Wolf v2 games), then UTF-8, then lossy.
    let content = {
        use encoding_rs::SHIFT_JIS;
        let (decoded, _, had_errors) = SHIFT_JIS.decode(&bytes);
        if !had_errors {
            decoded.into_owned()
        } else {
            String::from_utf8(bytes.clone())
                .unwrap_or_else(|_| String::from_utf8_lossy(&bytes).into_owned())
        }
    };

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("GameTitle=") {
            let title = rest.trim().to_string();
            if !title.is_empty() {
                return Some(title);
            }
        }
    }
    None
}

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
