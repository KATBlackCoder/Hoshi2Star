//! Tauri commands for exporting project data (game files, QA report, TM).

use std::{collections::HashMap, path::Path, path::PathBuf, sync::LazyLock};

use regex::Regex;
use serde::Serialize;

use std::io::Write as _;

use crate::{
    core::{report, tm},
    domain::types::SourceFile,
    engines::{
        detector::guess_wolf_version_from_structure,
        mv_mz::injector,
        vx_ace::injector as vx_injector,
        wolf::injector::{
            inject_all as wolf_inject_all, inject_all_to_memory as wolf_to_memory, WolfTranslation,
        },
    },
    state::AppState,
};

// ---------------------------------------------------------------------------
// Font-size helpers
// ---------------------------------------------------------------------------

/// Wolf RPG font-size prefix: `\f[N]`
static RE_FONT_PREFIX_WOLF: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\\f\[\d+\]").expect("RE_FONT_PREFIX_WOLF must compile"));

/// RPG Maker MZ font-size prefix: `\FS[N]`
static RE_FONT_PREFIX_MVMZ: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\\FS\[\d+\]").expect("RE_FONT_PREFIX_MVMZ must compile"));

fn font_prefix_code(engine: &str, n: u32) -> String {
    if engine == "wolf" {
        format!("\\f[{n}]")
    } else {
        format!("\\FS[{n}]")
    }
}

fn font_prefix_re(engine: &str) -> &'static Regex {
    if engine == "wolf" {
        &RE_FONT_PREFIX_WOLF
    } else {
        &RE_FONT_PREFIX_MVMZ
    }
}

/// Prepend the engine-appropriate font-size code to `text`, or replace the
/// existing prefix when `replace_existing` is true.
fn apply_font_prefix(text: &str, n: u32, replace_existing: bool, engine: &str) -> String {
    let re = font_prefix_re(engine);
    let code = font_prefix_code(engine, n);
    if re.is_match(text) {
        if replace_existing {
            re.replace(text, code.as_str()).into_owned()
        } else {
            text.to_string()
        }
    } else {
        format!("{code}{text}")
    }
}

/// Result returned by `scan_font_status`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FontScanResult {
    /// Segments whose `target_text` already starts with the engine font prefix.
    pub existing_font_count: i64,
    /// Total translated segments analysed.
    pub total_translated: i64,
    /// Engine of the project (`"wolf"`, `"mv_mz"`, etc.).
    pub engine: String,
}

/// Scan translated segments for existing font-size prefixes.
///
/// Pass either `source_file_id` (file-level scan) or `project_id`
/// (project-level scan). The result includes `engine` so the frontend can
/// pick the correct code label (`\f[N]` vs `\FS[N]`).
#[tauri::command]
pub async fn scan_font_status(
    source_file_id: Option<String>,
    project_id: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<FontScanResult, String> {
    // Resolve project id and engine from DB.
    let (resolved_pid, engine): (String, String) =
        match (source_file_id.as_deref(), project_id.as_deref()) {
            (Some(fid), _) => sqlx::query_as(
                "SELECT p.id, p.engine FROM projects p \
             JOIN source_files sf ON sf.project_id = p.id \
             WHERE sf.id = ?",
            )
            .bind(fid)
            .fetch_one(&state.db)
            .await
            .map_err(|e| e.to_string())?,
            (_, Some(pid)) => sqlx::query_as("SELECT id, engine FROM projects WHERE id = ?")
                .bind(pid)
                .fetch_one(&state.db)
                .await
                .map_err(|e| e.to_string())?,
            _ => return Err("source_file_id or project_id required".to_string()),
        };

    // Fetch (file_type, target_text) so we can filter to Map-only for mv_mz.
    let rows: Vec<(String, String)> = match source_file_id.as_deref() {
        Some(fid) => sqlx::query_as(
            "SELECT sf.file_type, s.target_text FROM segments s \
             JOIN source_files sf ON s.source_file_id = sf.id \
             WHERE s.source_file_id = ? AND s.target_text != ''",
        )
        .bind(fid)
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?,
        None => sqlx::query_as(
            "SELECT sf.file_type, s.target_text FROM segments s \
             JOIN source_files sf ON s.source_file_id = sf.id \
             WHERE sf.project_id = ? AND s.target_text != ''",
        )
        .bind(&resolved_pid)
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?,
    };

    // For mv_mz: prefix applies to Map files only.
    let texts: Vec<&str> = rows
        .iter()
        .filter(|(ft, _)| engine != "mv_mz" || ft == "map")
        .map(|(_, t)| t.as_str())
        .collect();

    let total_translated = texts.len() as i64;
    // Count any font prefix regardless of type — catches cross-engine leftovers.
    let existing_font_count = texts
        .iter()
        .filter(|t| RE_FONT_PREFIX_WOLF.is_match(t) || RE_FONT_PREFIX_MVMZ.is_match(t))
        .count() as i64;

    Ok(FontScanResult {
        existing_font_count,
        total_translated,
        engine,
    })
}

/// Strip any wolf (`\f[N]`) or mv_mz (`\FS[N]`) font-size prefix from `text`.
fn strip_font_prefix(text: &str) -> String {
    let s = RE_FONT_PREFIX_WOLF.replace(text, "");
    RE_FONT_PREFIX_MVMZ.replace(s.as_ref(), "").into_owned()
}

/// Remove existing font-size prefixes from every segment in the project.
///
/// Called when the user skips the font-size dialog, to undo any prefix that
/// was written by a previous (now-fixed) export.  No-op if no prefix exists.
#[tauri::command]
pub async fn strip_font_prefixes(
    project_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT s.id, s.target_text FROM segments s \
         JOIN source_files sf ON s.source_file_id = sf.id \
         WHERE sf.project_id = ? AND s.target_text != ''",
    )
    .bind(&project_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    for (id, text) in &rows {
        let clean = strip_font_prefix(text);
        if clean != *text {
            sqlx::query("UPDATE segments SET target_text = ? WHERE id = ?")
                .bind(&clean)
                .bind(id)
                .execute(&state.db)
                .await
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Re-inject all translated segments and write `hoshi2star.zip` in the game root.
///
/// Each engine contributes `(relative_zip_path, bytes)` entries:
///  - `"wolf_*"` → `inject_all_to_memory` (no disk I/O on the Wolf side)
///  - `"vx_*"`   → `inject_and_serialize` (Marshal round-trip)
///  - otherwise  → `inject_to_bytes` (JSON round-trip)
///
/// Returns the absolute path of the written zip file.
#[tauri::command]
pub async fn export_project(
    project_id: String,
    font_size: Option<u32>,
    replace_existing: bool,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let (game_path, engine): (String, String) =
        sqlx::query_as("SELECT game_path, engine FROM projects WHERE id = ?")
            .bind(&project_id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| e.to_string())?;

    let game_dir = Path::new(&game_path);

    let files = sqlx::query_as::<_, SourceFile>(
        "SELECT id, project_id, file_name, file_path, file_type, translation_secs \
         FROM source_files WHERE project_id = ?",
    )
    .bind(&project_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();

    let has_wolf = files.iter().any(|f| f.file_type.starts_with("wolf_"));
    if has_wolf {
        let wolf_entries = collect_wolf_zip_entries(
            &project_id,
            &files,
            game_dir,
            font_size,
            replace_existing,
            &engine,
            &state.db,
        )
        .await?;
        entries.extend(wolf_entries);
    } else {
        for file in &files {
            let mut translations: Vec<(String, String)> = sqlx::query_as::<_, (String, String)>(
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

            // For mv_mz: apply prefix to Map files only.
            let should_prefix = engine != "mv_mz" || file.file_type == "map";
            if let Some(n) = font_size {
                if should_prefix {
                    for (_, text) in &mut translations {
                        *text = apply_font_prefix(text, n, replace_existing, &engine);
                    }
                }
            }

            let pairs: Vec<(&str, &str)> = translations
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();

            let bytes = if file.file_type.starts_with("vx_") {
                let raw = std::fs::read(&file.file_path)
                    .map_err(|e| format!("read {}: {e}", file.file_name))?;
                vx_injector::inject_and_serialize(&raw, &pairs)
                    .map_err(|e| format!("inject {}: {e}", file.file_name))?
            } else {
                let raw = std::fs::read_to_string(&file.file_path)
                    .map_err(|e| format!("read {}: {e}", file.file_name))?;
                injector::inject_to_bytes(&raw, &pairs)
                    .map_err(|e| format!("inject {}: {e}", file.file_name))?
            };

            // Relative path inside the zip mirrors the on-disk layout.
            let rel = PathBuf::from(&file.file_path)
                .strip_prefix(game_dir)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| file.file_name.clone());
            entries.push((rel, bytes));
        }
    }

    let zip_path = game_dir.join("hoshi2star.zip");
    write_zip(&zip_path, entries)?;
    Ok(zip_path.to_string_lossy().into_owned())
}

/// Collect Wolf zip entries in memory without writing to disk.
async fn collect_wolf_zip_entries(
    _project_id: &str,
    files: &[SourceFile],
    game_dir: &Path,
    font_size: Option<u32>,
    replace_existing: bool,
    engine: &str,
    db: &sqlx::SqlitePool,
) -> Result<Vec<(String, Vec<u8>)>, String> {
    let version = guess_wolf_version_from_structure(game_dir);
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

        for (key, mut text) in segs {
            if let Some(n) = font_size {
                text = apply_font_prefix(&text, n, replace_existing, engine);
            }
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
        return Ok(vec![]);
    }

    wolf_to_memory(game_dir, &translations_by_file, &version)
        .await
        .map_err(|e| e.to_string())
}

/// Write a zip archive at `path` containing the given `(relative_path, bytes)` entries.
fn write_zip(path: &Path, entries: Vec<(String, Vec<u8>)>) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (rel_path, bytes) in entries {
        zip.start_file(&rel_path, options)
            .map_err(|e| e.to_string())?;
        zip.write_all(&bytes).map_err(|e| e.to_string())?;
    }
    zip.finish().map_err(|e| e.to_string())?;
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
    let (game_path, engine): (String, String) =
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

    // Fetch translations (key → target_text).
    let mut segs: Vec<(String, String)> = sqlx::query_as(
        "SELECT json_key, target_text FROM segments \
         WHERE source_file_id = ? AND target_text != ''",
    )
    .bind(&source_file_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    // Apply font-size prefix in-memory (never written to DB).
    // For mv_mz: Map files only.
    let should_prefix = engine != "mv_mz" || file.file_type == "map";
    if let Some(n) = font_size {
        if should_prefix {
            for (_, text) in &mut segs {
                *text = apply_font_prefix(text, n, replace_existing, &engine);
            }
        }
    }

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
