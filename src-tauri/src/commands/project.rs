//! Tauri commands for project lifecycle: open, browse, edit, export.
//!
//! All commands are `async`, return `Result<T, String>`, and receive the
//! database pool through `tauri::State<'_, AppState>`.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::path::Path;

use crate::{
    engines::{
        detector::{detect_engine, find_data_dir, Engine},
        mv_mz::{extractor, injector},
    },
    state::AppState,
};

// ---------------------------------------------------------------------------
// Domain types (serialised to/from TypeScript via Tauri IPC)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub engine: String,
    pub game_path: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct SourceFile {
    pub id: String,
    pub project_id: String,
    pub file_name: String,
    pub file_path: String,
    pub file_type: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
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
pub struct PaginatedSegments {
    pub items: Vec<Segment>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
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

    // 1. Detect engine (currently only MV/MZ)
    let engine = detect_engine(game_dir).map_err(|e| e.to_string())?;
    let engine_str = match engine {
        Engine::MvMz => "mv_mz",
    };

    // 2. Locate data directory
    let data_dir = find_data_dir(game_dir)
        .ok_or_else(|| "Cannot find data directory in game folder".to_string())?;

    // 3. Read game title from System.json (fallback: folder name)
    let game_title = read_game_title(&data_dir.join("System.json")).unwrap_or_else(|| {
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

        let segments = dispatch_extract(file_name, json_value);

        for seg in segments {
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
/// Sets `status = 'translated'` and updates `updated_at`.
/// Returns the updated `Segment` row.
#[tauri::command]
pub async fn update_segment(
    id: String,
    target_text: String,
    state: tauri::State<'_, AppState>,
) -> Result<Segment, String> {
    sqlx::query(
        "UPDATE segments \
         SET target_text = ?, status = 'translated', updated_at = datetime('now') \
         WHERE id = ?",
    )
    .bind(&target_text)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(|e| e.to_string())?;

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
/// Reads `target_text` from DB for each source file, calls the MV/MZ injector,
/// and writes the result back to the original file path.
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

        let raw = std::fs::read_to_string(&file.file_path)
            .map_err(|e| format!("read {}: {e}", file.file_name))?;
        let mut json: serde_json::Value =
            serde_json::from_str(&raw).map_err(|e| format!("parse {}: {e}", file.file_name))?;

        let pairs: Vec<(&str, &str)> = translations
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        injector::inject(&mut json, &pairs)
            .map_err(|e| format!("inject {}: {e}", file.file_name))?;

        let out = serde_json::to_string(&json)
            .map_err(|e| format!("serialise {}: {e}", file.file_name))?;
        std::fs::write(&file.file_path, out)
            .map_err(|e| format!("write {}: {e}", file.file_name))?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Read `gameTitle` from a `System.json` path.
fn read_game_title(system_json_path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(system_json_path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    v.get("gameTitle")?.as_str().map(|s| s.to_string())
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
}
