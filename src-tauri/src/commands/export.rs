//! Tauri commands for exporting project data (game files, QA report, TM).

use std::{collections::HashMap, path::Path, path::PathBuf, sync::LazyLock};

use regex::Regex;
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

// ---------------------------------------------------------------------------
// Font-size helpers
// ---------------------------------------------------------------------------

static RE_FONT_PREFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\\f\[\d+\]").expect("RE_FONT_PREFIX must compile"));

/// Prepend `\f[n]` to `text`, or replace the existing `\f[*]` prefix if
/// `replace_existing` is true.  Returns the original string unchanged when
/// the prefix already exists and `replace_existing` is false.
fn apply_font_prefix(text: &str, n: u32, replace_existing: bool) -> String {
    if RE_FONT_PREFIX.is_match(text) {
        if replace_existing {
            RE_FONT_PREFIX
                .replace(text, format!("\\f[{n}]").as_str())
                .into_owned()
        } else {
            text.to_string()
        }
    } else {
        format!("\\f[{n}]{text}")
    }
}

/// Result returned by `scan_font_status`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FontScanResult {
    /// Segments whose `target_text` already starts with a `\f[N]` prefix.
    pub existing_font_count: i64,
    /// Total translated segments analysed.
    pub total_translated: i64,
}

/// Scan translated segments for existing `\f[N]` prefixes.
///
/// Pass either `source_file_id` (file-level scan) or `project_id`
/// (project-level scan).
#[tauri::command]
pub async fn scan_font_status(
    source_file_id: Option<String>,
    project_id: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<FontScanResult, String> {
    let texts: Vec<String> = match (source_file_id.as_deref(), project_id.as_deref()) {
        (Some(fid), _) => sqlx::query_scalar(
            "SELECT target_text FROM segments WHERE source_file_id = ? AND target_text != ''",
        )
        .bind(fid)
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?,
        (_, Some(pid)) => sqlx::query_scalar(
            "SELECT s.target_text FROM segments s \
             JOIN source_files sf ON s.source_file_id = sf.id \
             WHERE sf.project_id = ? AND s.target_text != ''",
        )
        .bind(pid)
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?,
        _ => return Err("source_file_id or project_id required".to_string()),
    };

    let total_translated = texts.len() as i64;
    let existing_font_count = texts.iter().filter(|t| RE_FONT_PREFIX.is_match(t)).count() as i64;

    Ok(FontScanResult {
        existing_font_count,
        total_translated,
    })
}

/// Apply `\f[n]` to every translated segment in a file or project, persisting
/// the change to the DB.  Called before injection when the user confirms a
/// font-size choice in the UI.
async fn persist_font_size(
    source_file_id: Option<&str>,
    project_id: Option<&str>,
    font_size: u32,
    replace_existing: bool,
    db: &sqlx::SqlitePool,
) -> Result<(), String> {
    let rows: Vec<(String, String)> = match (source_file_id, project_id) {
        (Some(fid), _) => sqlx::query_as(
            "SELECT id, target_text FROM segments WHERE source_file_id = ? AND target_text != ''",
        )
        .bind(fid)
        .fetch_all(db)
        .await
        .map_err(|e| e.to_string())?,
        (_, Some(pid)) => sqlx::query_as(
            "SELECT s.id, s.target_text FROM segments s \
             JOIN source_files sf ON s.source_file_id = sf.id \
             WHERE sf.project_id = ? AND s.target_text != ''",
        )
        .bind(pid)
        .fetch_all(db)
        .await
        .map_err(|e| e.to_string())?,
        _ => return Err("source_file_id or project_id required".to_string()),
    };

    for (id, text) in &rows {
        let new_text = apply_font_prefix(text, font_size, replace_existing);
        if new_text != *text {
            sqlx::query("UPDATE segments SET target_text = ? WHERE id = ?")
                .bind(&new_text)
                .bind(id)
                .execute(db)
                .await
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Re-inject all translated segments back into the game files.
///
/// Dispatches based on `file_type` prefix:
///  - `"wolf_*"` → batch call to `wolf_inject_all` (Option A: decrypted files)
///  - `"vx_*"`   → VX Ace Marshal round-trip
///  - otherwise  → MV/MZ JSON round-trip
#[tauri::command]
pub async fn export_project(
    project_id: String,
    font_size: Option<u32>,
    replace_existing: bool,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // Apply \f[N] prefix to all translated segments if the user requested it.
    if let Some(n) = font_size {
        persist_font_size(None, Some(&project_id), n, replace_existing, &state.db).await?;
    }

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

/// Inject translations for a single source file back into the game (debug/verify path).
///
/// Equivalent to `export_project` scoped to one file — writes in-place using the
/// same Option-A strategy.  The command enforces completeness server-side: it
/// returns `Err` if any segment in the file still has an empty `target_text`.
///
/// Returns the absolute path of the written file so the frontend can display it.
#[tauri::command]
pub async fn debug_inject_file(
    source_file_id: String,
    font_size: Option<u32>,
    replace_existing: bool,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    // Fetch file metadata.
    let file = sqlx::query_as::<_, SourceFile>(
        "SELECT sf.id, sf.project_id, sf.file_name, sf.file_path, sf.file_type, \
                sf.translation_secs, 0 as total_count, 0 as translated_count \
         FROM source_files sf WHERE sf.id = ?",
    )
    .bind(&source_file_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    // Fetch project info.
    let (game_path, _engine): (String, String) =
        sqlx::query_as("SELECT game_path, engine FROM projects WHERE id = ?")
            .bind(&file.project_id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| e.to_string())?;

    // Enforce completeness: refuse if any segment is untranslated.
    let untranslated: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM segments WHERE source_file_id = ? AND target_text = ''",
    )
    .bind(&source_file_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    if untranslated > 0 {
        return Err(format!("{untranslated} segment(s) not yet translated"));
    }

    // Apply \f[N] prefix if requested.
    if let Some(n) = font_size {
        persist_font_size(Some(&source_file_id), None, n, replace_existing, &state.db).await?;
    }

    // Fetch translations (key → target_text).
    let segs: Vec<(String, String)> = sqlx::query_as(
        "SELECT json_key, target_text FROM segments \
         WHERE source_file_id = ? AND target_text != ''",
    )
    .bind(&source_file_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    // Dispatch by engine.
    if file.file_type.starts_with("wolf_") {
        let mut translations_by_file: HashMap<String, Vec<WolfTranslation>> = HashMap::new();
        for (key, text) in segs {
            let parts: Vec<&str> = key.splitn(3, '/').collect();
            if parts.len() >= 2 {
                let file_key = format!("{}/{}", parts[0], parts[1]);
                translations_by_file
                    .entry(file_key)
                    .or_default()
                    .push(WolfTranslation { key, text });
            }
        }
        let game_dir = Path::new(&game_path);
        let version = crate::engines::detector::guess_wolf_version_from_structure(game_dir);
        let results = wolf_inject_all(game_dir, &translations_by_file, &version)
            .await
            .map_err(|e| e.to_string())?;
        let out_path = results
            .into_iter()
            .next()
            .map(|r| r.file_path)
            .unwrap_or_else(|| PathBuf::from(&file.file_path));
        Ok(out_path.to_string_lossy().to_string())
    } else if file.file_type.starts_with("vx_") {
        let pairs: Vec<(&str, &str)> = segs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let bytes =
            std::fs::read(&file.file_path).map_err(|e| format!("read {}: {e}", file.file_name))?;
        let out = vx_injector::inject_and_serialize(&bytes, &pairs)
            .map_err(|e| format!("inject {}: {e}", file.file_name))?;
        std::fs::write(&file.file_path, out)
            .map_err(|e| format!("write {}: {e}", file.file_name))?;
        Ok(file.file_path.clone())
    } else {
        let pairs: Vec<(&str, &str)> = segs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
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
        Ok(file.file_path.clone())
    }
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
