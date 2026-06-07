//! Tauri commands for exporting project data (game files, QA report, TM).

use std::{collections::HashMap, path::Path};

use serde::Serialize;

use crate::{
    core::{report, tm},
    domain::types::SourceFile,
    engines::{
        detector::guess_wolf_version_from_structure,
        mv_mz::injector,
        vx_ace::injector as vx_injector,
        wolf::injector::{inject_all as wolf_inject_all, WolfTranslation},
    },
    state::AppState,
};

/// Re-inject all translated segments back into the game files.
///
/// Dispatches based on `file_type` prefix:
///  - `"wolf_*"` → batch call to `wolf_inject_all` (Option A: decrypted files)
///  - `"vx_*"`   → VX Ace Marshal round-trip
///  - otherwise  → MV/MZ JSON round-trip
#[tauri::command]
pub async fn export_project(
    project_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let files = sqlx::query_as::<_, SourceFile>(
        "SELECT id, project_id, file_name, file_path, file_type, translation_secs \
         FROM source_files WHERE project_id = ?",
    )
    .bind(&project_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    // Detect Wolf files early — they are handled in a single batch call.
    let has_wolf = files.iter().any(|f| f.file_type.starts_with("wolf_"));
    if has_wolf {
        return export_project_wolf(&project_id, &files, &state.db).await;
    }

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

/// Wolf RPG export: collect all translated segments, group by file key,
/// and call `inject_all` once (Option A — writes decrypted files to Data/).
async fn export_project_wolf(
    project_id: &str,
    files: &[SourceFile],
    db: &sqlx::SqlitePool,
) -> Result<(), String> {
    // Fetch game_path for game_dir and wolf version detection.
    let game_path: String = sqlx::query_scalar("SELECT game_path FROM projects WHERE id = ?")
        .bind(project_id)
        .fetch_one(db)
        .await
        .map_err(|e| e.to_string())?;
    let game_dir = Path::new(&game_path);
    let version = guess_wolf_version_from_structure(game_dir);

    // Build HashMap<file_key, Vec<WolfTranslation>> from all Wolf source files.
    let mut translations_by_file: HashMap<String, Vec<WolfTranslation>> = HashMap::new();

    for file in files {
        if !file.file_type.starts_with("wolf_") {
            continue;
        }
        let segs: Vec<(String, String)> = sqlx::query_as::<_, (String, String)>(
            "SELECT json_key, target_text FROM segments \
             WHERE source_file_id = ? AND target_text != ''",
        )
        .bind(&file.id)
        .fetch_all(db)
        .await
        .map_err(|e| e.to_string())?;

        for (key, text) in segs {
            // Derive the file_key from the first two path components of json_key.
            // e.g. "MapData/Map001/events/..." → "MapData/Map001"
            //      "Database/Actors/0/..."    → "Database/Actors"
            let parts: Vec<&str> = key.splitn(3, '/').collect();
            if parts.len() >= 2 {
                let file_key = format!("{}/{}", parts[0], parts[1]);
                translations_by_file
                    .entry(file_key)
                    .or_default()
                    .push(WolfTranslation { key, text });
            }
        }
    }

    if translations_by_file.is_empty() {
        return Ok(());
    }

    wolf_inject_all(game_dir, &translations_by_file, &version)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
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

/// Export all segments for a project as a flat JSON file for debugging.
///
/// Writes `hoshi2star_debug.json` directly into the project's `game_path` folder.
/// Only segments with a non-empty `target_text` are included.
/// Returns the absolute path of the written file.
#[tauri::command]
pub async fn export_debug_json(
    project_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    #[derive(Serialize)]
    struct DebugSegment {
        file: String,
        key: String,
        source: String,
        target: String,
        status: String,
    }

    let game_path: String = sqlx::query_scalar("SELECT game_path FROM projects WHERE id = ?")
        .bind(&project_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    let rows: Vec<(String, String, String, String, String)> = sqlx::query_as(
        "SELECT sf.file_name, s.json_key, s.source_text, s.target_text, s.status \
         FROM segments s \
         JOIN source_files sf ON s.source_file_id = sf.id \
         WHERE sf.project_id = ? AND s.target_text != '' \
         ORDER BY sf.file_name, s.rowid",
    )
    .bind(&project_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let segments: Vec<DebugSegment> = rows
        .into_iter()
        .map(|(file, key, source, target, status)| DebugSegment {
            file,
            key,
            source,
            target,
            status,
        })
        .collect();

    let output_path = std::path::Path::new(&game_path)
        .join("hoshi2star_debug.json")
        .to_string_lossy()
        .to_string();

    let json = serde_json::to_string_pretty(&segments).map_err(|e| e.to_string())?;
    tokio::fs::write(&output_path, json.as_bytes())
        .await
        .map_err(|e| e.to_string())?;

    Ok(output_path)
}
