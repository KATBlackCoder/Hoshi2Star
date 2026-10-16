//! Tauri commands for exporting project data (game files, QA report, TM).

use std::{collections::HashMap, path::Path, path::PathBuf, sync::LazyLock};

use regex::Regex;
use serde::Serialize;

use std::io::Write as _;

use crate::{
    commands::coverage::enabled_plugin_sources,
    core::{originals, report, tm},
    domain::types::SourceFile,
    engines::{
        detector::guess_wolf_version_from_structure,
        filter::{classify_file_type, is_map_file, FileClass},
        mv_mz::{
            coverage::plugins::{context_is_fill_in, for_each_command, PLUGINS_FILE_NAME},
            injector, integrity, keke,
            locale::{self, LocaleUse},
            originals::{sha256_hex, SnapshotStore},
            plugins_js::{parse_plugin_key, PluginsJs},
        },
        vx_ace::injector as vx_injector,
        wolf::injector::{
            inject_all as wolf_inject_all, inject_all_to_memory as wolf_to_memory,
            injection_bucket, WolfTranslation,
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

/// Wolf RPG v3 speaker sprite index prefix: `@N\n` (leading only — see
/// `wolf::placeholders::RE_WOLF`). The engine's built-in name/face lookup
/// requires this to be the true first characters of the message, so the
/// font-size code must always be inserted after it, never before: prepending
/// `\f[N]` unconditionally used to push `@N\n` out of the leading position,
/// silently disabling the name/face box for every prefixed line.
static RE_WOLF_SPEAKER_PREFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^@\d+\n").expect("RE_WOLF_SPEAKER_PREFIX must compile"));

/// Split off a leading Wolf speaker prefix (`@N\n`), if any, so font-size
/// detection/insertion/stripping all operate on the text that follows it.
fn split_wolf_speaker_prefix(text: &str) -> (&str, &str) {
    match RE_WOLF_SPEAKER_PREFIX.find(text) {
        Some(m) => (m.as_str(), &text[m.end()..]),
        None => ("", text),
    }
}

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

/// Prepend the engine-appropriate font-size code to `text` (after any Wolf
/// speaker prefix, see [`RE_WOLF_SPEAKER_PREFIX`]), or replace the existing
/// prefix when `replace_existing` is true.
fn apply_font_prefix(text: &str, n: u32, replace_existing: bool, engine: &str) -> String {
    let (speaker_prefix, rest) = if engine == "wolf" {
        split_wolf_speaker_prefix(text)
    } else {
        ("", text)
    };
    let re = font_prefix_re(engine);
    let code = font_prefix_code(engine, n);
    if re.is_match(rest) {
        if replace_existing {
            format!("{speaker_prefix}{}", re.replace(rest, code.as_str()))
        } else {
            text.to_string()
        }
    } else {
        format!("{speaker_prefix}{code}{rest}")
    }
}

/// Apply the export font prefix to `(key, target, is_core)` rows. Only core
/// segments of prefix-eligible files are prefixed: optional text categories
/// (e.g. map display names) are drawn without escape codes.
fn with_font_prefix(
    rows: Vec<(String, String, bool)>,
    font_size: Option<u32>,
    eligible_file: bool,
    replace_existing: bool,
    engine: &str,
) -> Vec<(String, String)> {
    rows.into_iter()
        .map(|(key, text, is_core)| match font_size {
            Some(n) if eligible_file && is_core => {
                let prefixed = apply_font_prefix(&text, n, replace_existing, engine);
                (key, prefixed)
            }
            _ => (key, text),
        })
        .collect()
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

    // Fetch (file_type, target_text) of core segments so we can filter to
    // Map-only for mv_mz (optional categories never carry the prefix).
    let rows: Vec<(String, String)> = match source_file_id.as_deref() {
        Some(fid) => sqlx::query_as(
            "SELECT sf.file_type, s.target_text FROM segments s \
             JOIN source_files sf ON s.source_file_id = sf.id \
             WHERE s.source_file_id = ? AND s.target_text != '' AND s.text_category IS NULL",
        )
        .bind(fid)
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?,
        None => sqlx::query_as(
            "SELECT sf.file_type, s.target_text FROM segments s \
             JOIN source_files sf ON s.source_file_id = sf.id \
             WHERE sf.project_id = ? AND s.target_text != '' AND s.text_category IS NULL",
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
    // Skip past a leading Wolf speaker prefix first: the font code lives
    // after it, not at position 0, once apply_font_prefix has run.
    let existing_font_count = texts
        .iter()
        .filter(|t| {
            let rest = split_wolf_speaker_prefix(t).1;
            RE_FONT_PREFIX_WOLF.is_match(rest) || RE_FONT_PREFIX_MVMZ.is_match(rest)
        })
        .count() as i64;

    Ok(FontScanResult {
        existing_font_count,
        total_translated,
        engine,
    })
}

/// Strip any wolf (`\f[N]`) or mv_mz (`\FS[N]`) font-size prefix from `text`,
/// preserving a leading Wolf speaker prefix (`@N\n`) if present.
fn strip_font_prefix(text: &str) -> String {
    let (speaker_prefix, rest) = split_wolf_speaker_prefix(text);
    let s = RE_FONT_PREFIX_WOLF.replace(rest, "");
    let s = RE_FONT_PREFIX_MVMZ.replace(s.as_ref(), "");
    format!("{speaker_prefix}{s}")
}

/// Remove existing font-size prefixes from every core segment in the project
/// (optional categories are never prefixed, so their text is left alone).
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
         WHERE sf.project_id = ? AND s.target_text != '' AND s.text_category IS NULL",
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
/// `set_locale` (MV 1.6+ / MZ, off by default) also switches System.json
/// `locale` to the target language; `preflight_export` lists its consequences.
///
/// Returns the absolute path of the written zip file.
#[tauri::command]
pub async fn export_project(
    project_id: String,
    font_size: Option<u32>,
    replace_existing: bool,
    set_locale: Option<bool>,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let (game_path, engine, target_language): (String, String, String) =
        sqlx::query_as("SELECT game_path, engine, target_language FROM projects WHERE id = ?")
            .bind(&project_id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| e.to_string())?;

    let (qa_summary, _) = report::audit_project(&state.db, &project_id)
        .await
        .map_err(|error| error.to_string())?;
    if qa_summary.critical_count > 0 {
        return Err(format!(
            "Export bloqué : {} segment(s) contiennent encore une erreur QA critique. \
             Ouvrez le panneau QA, corrigez-les puis relancez l’export.",
            qa_summary.critical_count
        ));
    }

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

    // MV/MZ exports always start from the pristine originals, never from the
    // game folder (which may already hold a previous export, or a game update
    // the snapshot then follows).
    let mut snapshot = if engine == "mv_mz" {
        Some(originals::ensure_fresh_mv_mz_snapshot(&state.db, &state.data_dir, &project_id).await?)
    } else {
        None
    };
    let mut integrity_issues = Vec::new();

    // Optional language switch, planned and verified like any other edit.
    let locale_edit = match (&snapshot, set_locale) {
        (Some(store), Some(true)) => {
            let target = locale::target_locale(&target_language);
            match snapshot_locale(store, &files, &game_path)? {
                Some(current) if current != target => Some(target),
                Some(_) => None,
                None => {
                    return Err(
                        "This game has no language setting (System.json `locale`) to change."
                            .to_string(),
                    )
                }
            }
        }
        _ => None,
    };

    let has_wolf = files
        .iter()
        .any(|f| matches!(classify_file_type(&f.file_type), FileClass::Wolf));
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
        // Skill types renamed inside battle-command note tags (linked category).
        let renames = if engine == "mv_mz" {
            linked_skill_type_renames(&state.db, &project_id).await?
        } else {
            HashMap::new()
        };
        let mut has_label_edits = false;
        for file in &files {
            let rows: Vec<(String, String, bool, String)> = sqlx::query_as(
                "SELECT json_key, target_text, text_category IS NULL, source_text FROM segments \
                 WHERE source_file_id = ? AND target_text != ''",
            )
            .bind(&file.id)
            .fetch_all(&state.db)
            .await
            .map_err(|e| e.to_string())?;

            let note_file = engine == "mv_mz" && NOTE_FILE_TYPES.contains(&file.file_type.as_str());
            let locale_here = locale_edit
                .as_deref()
                .filter(|_| file.file_name == "System.json");
            if rows.is_empty() && !(note_file && !renames.is_empty()) && locale_here.is_none() {
                continue;
            }
            let (label_rows, rows): (Vec<_>, Vec<_>) = rows
                .into_iter()
                .partition(|(key, ..)| key.starts_with(keke::NOTETAG_KEY_PREFIX));
            let label_rows: Vec<keke::LabelRow> = label_rows
                .into_iter()
                .map(|(key, target, _, source)| (key, target, source))
                .collect();
            has_label_edits |= !label_rows.is_empty();

            // For mv_mz: apply prefix to core Map segments only (optional
            // categories such as map display names are drawn without escape
            // codes, so a prefix would show up literally).
            let should_prefix = engine != "mv_mz" || file.file_type == "map";
            let mut translations = with_font_prefix(
                rows.into_iter()
                    .map(|(key, target, is_core, _)| (key, target, is_core))
                    .collect(),
                font_size,
                should_prefix,
                replace_existing,
                &engine,
            );
            // Relative path inside the zip mirrors the on-disk layout.
            let rel = originals::game_relative_path(&game_path, &file.file_path, &file.file_name);

            // A project is single-engine, so the wolf branch above already
            // handled every wolf file; here it is VX Ace vs MV/MZ JSON.
            let bytes = match classify_file_type(&file.file_type) {
                FileClass::VxAce => {
                    let pairs: Vec<(&str, &str)> = translations
                        .iter()
                        .map(|(k, v)| (k.as_str(), v.as_str()))
                        .collect();
                    let raw = std::fs::read(&file.file_path)
                        .map_err(|e| format!("read {}: {e}", file.file_name))?;
                    vx_injector::inject_and_serialize(&raw, &pairs)
                        .map_err(|e| format!("inject {}: {e}", file.file_name))?
                }
                FileClass::PluginsJs => {
                    let raw = read_original_json(snapshot.as_ref(), &rel, file)?;
                    let rows = plugin_rows(&state.db, &file.id).await?;
                    let (output, planned) = build_plugins_js(&raw, &rows)?;
                    integrity_issues.extend(integrity::verify_plugins_edit(
                        &rel, &raw, &output, &planned,
                    ));
                    output.into_bytes()
                }
                _ => {
                    let raw = read_original_json(snapshot.as_ref(), &rel, file)?;
                    if note_file && (!label_rows.is_empty() || !renames.is_empty()) {
                        let original: serde_json::Value = serde_json::from_str(&raw)
                            .map_err(|e| format!("parse {}: {e}", file.file_name))?;
                        translations.extend(
                            keke::note_rewrites(&original, &label_rows, &renames)
                                .map_err(|e| format!("{}: {e}", file.file_name))?,
                        );
                    } else if !label_rows.is_empty() {
                        return Err(format!(
                            "{}: note-tag segments in a file whose notes the plugin does not read",
                            file.file_name
                        ));
                    }
                    if let Some(target) = locale_here {
                        translations.push(("/locale".to_string(), target.to_string()));
                    }
                    if translations.is_empty() {
                        continue;
                    }
                    let pairs: Vec<(&str, &str)> = translations
                        .iter()
                        .map(|(k, v)| (k.as_str(), v.as_str()))
                        .collect();
                    let bytes = injector::inject_to_bytes(&raw, &pairs)
                        .map_err(|e| format!("inject {}: {e}", file.file_name))?;
                    let planned: HashMap<String, String> = translations.iter().cloned().collect();
                    integrity_issues
                        .extend(integrity::verify_json_edit(&rel, &raw, &bytes, &planned));
                    bytes
                }
            };
            entries.push((rel, bytes));
        }

        if let Some(store) = snapshot.as_ref() {
            if !renames.is_empty() || has_label_edits {
                integrity_issues
                    .extend(keke_resolution_issues(store, &files, &game_path, &entries)?);
            }
        }
    }

    if !integrity_issues.is_empty() {
        return Err(format!(
            "Export blocked: the integrity check found {} problem(s); no file was written.\n{}",
            integrity_issues.len(),
            integrity::summarize(&integrity_issues)
        ));
    }

    let written: Vec<(String, String)> = entries
        .iter()
        .map(|(rel, bytes)| (rel.clone(), sha256_hex(bytes)))
        .collect();
    let zip_path = game_dir.join("hoshi2star.zip");
    write_zip(&zip_path, entries)?;
    // Users unzip the archive over the game: those files are ours, not an update.
    if let Some(store) = snapshot.as_mut() {
        originals::remember_written(store, &written);
    }
    Ok(zip_path.to_string_lossy().into_owned())
}

// ---------------------------------------------------------------------------
// Export preflight (optional language switch)
// ---------------------------------------------------------------------------

/// Switching the game's language setting to the project target language.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocaleOption {
    pub current: String,
    pub target: String,
    pub consequences: Vec<String>,
    /// Engine and plugin code lines that read the setting.
    pub uses: Vec<LocaleUse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPreflight {
    /// `None` when the game has no language setting (MV < 1.6, other engines)
    /// or it already matches the target language.
    pub locale: Option<LocaleOption>,
}

/// Options offered before an export, with their consequences for this game.
#[tauri::command]
pub async fn preflight_export(
    project_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<ExportPreflight, String> {
    let (game_path, engine, target_language): (String, String, String) =
        sqlx::query_as("SELECT game_path, engine, target_language FROM projects WHERE id = ?")
            .bind(&project_id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| e.to_string())?;
    if engine != "mv_mz" {
        return Ok(ExportPreflight { locale: None });
    }
    let store =
        originals::ensure_fresh_mv_mz_snapshot(&state.db, &state.data_dir, &project_id).await?;
    let files = sqlx::query_as::<_, SourceFile>(
        "SELECT id, project_id, file_name, file_path, file_type, translation_secs \
         FROM source_files WHERE project_id = ?",
    )
    .bind(&project_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;
    let target = locale::target_locale(&target_language);
    let locale =
        tokio::task::spawn_blocking(move || locale_option(&store, &files, &game_path, &target))
            .await
            .map_err(|e| e.to_string())??;
    Ok(ExportPreflight { locale })
}

/// System.json `locale` in the pristine snapshot (`None`: no such setting).
fn snapshot_locale(
    store: &SnapshotStore,
    files: &[SourceFile],
    game_path: &str,
) -> Result<Option<String>, String> {
    let Some(system) = files.iter().find(|file| file.file_name == "System.json") else {
        return Ok(None);
    };
    let rel = originals::game_relative_path(game_path, &system.file_path, &system.file_name);
    let bytes = store.read(&rel).map_err(|e| e.to_string())?;
    let json: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|e| format!("parse {rel}: {e}"))?;
    Ok(json
        .get("locale")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string))
}

fn locale_option(
    store: &SnapshotStore,
    files: &[SourceFile],
    game_path: &str,
    target: &str,
) -> Result<Option<LocaleOption>, String> {
    let Some(current) = snapshot_locale(store, files, game_path)? else {
        return Ok(None);
    };
    if current == target {
        return Ok(None);
    }

    // Code that reads the setting: the engine core (`js/rmmz_*.js` or
    // `js/rpg_*.js`) and every enabled plugin. `js/` sits next to `data/`.
    let mut sources = Vec::new();
    let js_dir = files
        .iter()
        .find(|file| file.file_name == "System.json")
        .and_then(|system| {
            Path::new(&system.file_path)
                .parent()?
                .parent()
                .map(|game| game.join("js"))
        });
    if let Some(js_dir) = &js_dir {
        let mut core: Vec<PathBuf> = std::fs::read_dir(js_dir)
            .map(|dir| {
                dir.filter_map(|entry| entry.ok().map(|e| e.path()))
                    .collect()
            })
            .unwrap_or_default();
        core.sort();
        for path in core {
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if (name.starts_with("rmmz_") || name.starts_with("rpg_")) && name.ends_with(".js") {
                if let Ok(text) = std::fs::read(&path) {
                    sources.push((
                        format!("js/{name}"),
                        String::from_utf8_lossy(&text).into_owned(),
                    ));
                }
            }
        }
        // Export never changes which plugins are on, so the game folder's
        // plugins.js lists the same enabled plugins as the original.
        if let Ok(plugins) = std::fs::read(js_dir.join(PLUGINS_FILE_NAME)) {
            let plugins = String::from_utf8_lossy(&plugins);
            for (name, text) in enabled_plugin_sources(js_dir, &plugins) {
                sources.push((format!("js/plugins/{name}.js"), text));
            }
        }
    }
    let uses = locale::locale_uses(&sources);

    // Name-input events decide whether the keyboard change is visible.
    let mut data_files = Vec::new();
    for file in files.iter().filter(|file| {
        is_map_file(&file.file_name, ".json")
            || file.file_name == "CommonEvents.json"
            || file.file_name == "Troops.json"
    }) {
        let rel = originals::game_relative_path(game_path, &file.file_path, &file.file_name);
        let bytes = store.read(&rel).map_err(|e| e.to_string())?;
        let json: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|e| format!("parse {rel}: {e}"))?;
        data_files.push((file.file_name.clone(), rel, json));
    }
    let mut name_inputs = 0;
    for_each_command(&data_files, &mut |_, _, _, _, command| {
        if command.get("code").and_then(serde_json::Value::as_i64) == Some(303) {
            name_inputs += 1;
        }
    });

    Ok(Some(LocaleOption {
        consequences: locale::consequences(&current, target, &uses, name_inputs),
        current,
        target: target.to_string(),
        uses,
    }))
}

/// MV/MZ database files (`source_files.file_type`) whose entries carry notes.
const NOTE_FILE_TYPES: &[&str] = &[
    "actors", "classes", "skills", "items", "weapons", "armors", "enemies", "states",
];

/// `source → target` of translated skill types that battle-command note tags
/// look up by name (the linked `system.skill_types` category).
async fn linked_skill_type_renames(
    db: &sqlx::SqlitePool,
    project_id: &str,
) -> Result<HashMap<String, String>, String> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT s.source_text, s.target_text FROM segments s \
         JOIN source_files sf ON sf.id = s.source_file_id \
         WHERE sf.project_id = ? AND s.text_category = 'system.skill_types' \
           AND s.target_text != '' AND s.context_json LIKE '%keke_skill_type%'",
    )
    .bind(project_id)
    .fetch_all(db)
    .await
    .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .filter(|(source, target)| source != target)
        .collect())
}

/// Differential check of Keke command tags over every note file: original
/// files (snapshot) against the archive entries (or the original, when a file
/// is not exported), with the original vs exported skill-type lists.
fn keke_resolution_issues(
    store: &SnapshotStore,
    files: &[SourceFile],
    game_path: &str,
    entries: &[(String, Vec<u8>)],
) -> Result<Vec<integrity::IntegrityIssue>, String> {
    let parse = |bytes: &[u8], label: &str| -> Result<serde_json::Value, String> {
        serde_json::from_slice(bytes).map_err(|e| format!("parse {label}: {e}"))
    };
    let output_of = |rel: &str| {
        entries
            .iter()
            .find(|(name, _)| name == rel)
            .map(|(_, b)| b.clone())
    };
    let skill_types = |json: &serde_json::Value| -> Vec<String> {
        json.get("skillTypes")
            .and_then(serde_json::Value::as_array)
            .map(|list| {
                list.iter()
                    .map(|v| v.as_str().unwrap_or_default().to_string())
                    .collect()
            })
            .unwrap_or_default()
    };
    let Some(system) = files.iter().find(|file| file.file_name == "System.json") else {
        return Ok(Vec::new());
    };
    let system_rel = originals::game_relative_path(game_path, &system.file_path, &system.file_name);
    let original_system = parse(
        &store.read(&system_rel).map_err(|e| e.to_string())?,
        &system_rel,
    )?;
    let output_system = match output_of(&system_rel) {
        Some(bytes) => parse(&bytes, &system_rel)?,
        None => original_system.clone(),
    };
    let mut note_files = Vec::new();
    for file in files
        .iter()
        .filter(|f| NOTE_FILE_TYPES.contains(&f.file_type.as_str()))
    {
        let rel = originals::game_relative_path(game_path, &file.file_path, &file.file_name);
        let original = parse(&store.read(&rel).map_err(|e| e.to_string())?, &rel)?;
        let output = match output_of(&rel) {
            Some(bytes) => parse(&bytes, &rel)?,
            None => original.clone(),
        };
        note_files.push((rel, original, output));
    }
    Ok(integrity::verify_keke_resolution(
        &skill_types(&original_system),
        &skill_types(&output_system),
        &note_files,
    ))
}

/// `(json_key, target, source, context_json)` of every translated plugins.js segment.
async fn plugin_rows(
    db: &sqlx::SqlitePool,
    file_id: &str,
) -> Result<Vec<(String, String, String, Option<String>)>, String> {
    sqlx::query_as(
        "SELECT json_key, target_text, source_text, context_json FROM segments \
         WHERE source_file_id = ? AND target_text != ''",
    )
    .bind(file_id)
    .fetch_all(db)
    .await
    .map_err(|e| e.to_string())
}

/// Apply translated plugin parameters to the original plugins.js text.
/// Returns the new text and the planned edits for the integrity gate.
fn build_plugins_js(
    original: &str,
    rows: &[(String, String, String, Option<String>)],
) -> Result<(String, Vec<integrity::PlannedPluginEdit>), String> {
    let mut planned = Vec::with_capacity(rows.len());
    for (key, target, source, context) in rows {
        let (plugin, occurrence, path) = parse_plugin_key(key)
            .ok_or_else(|| format!("invalid plugins.js segment key: {key}"))?;
        let fill_in = context_is_fill_in(context.as_deref());
        planned.push(integrity::PlannedPluginEdit {
            plugin,
            occurrence,
            path,
            value: target.clone(),
            expected_original: if fill_in {
                String::new()
            } else {
                source.clone()
            },
        });
    }
    let edits: Vec<(String, u32, Vec<String>, String)> = planned
        .iter()
        .map(|edit| {
            (
                edit.plugin.clone(),
                edit.occurrence,
                edit.path.clone(),
                edit.value.clone(),
            )
        })
        .collect();
    let output = PluginsJs::parse(original)
        .and_then(|plugins| plugins.apply_edits(&edits))
        .map_err(|e| format!("plugins.js: {e}"))?;
    Ok((output, planned))
}

/// Text of an original MV/MZ JSON file: the verified snapshot when one exists
/// (always, for MV/MZ projects), the game folder otherwise.
fn read_original_json(
    snapshot: Option<&SnapshotStore>,
    rel: &str,
    file: &SourceFile,
) -> Result<String, String> {
    let bytes = match snapshot {
        Some(store) => store.read(rel).map_err(|e| e.to_string())?,
        None => {
            std::fs::read(&file.file_path).map_err(|e| format!("read {}: {e}", file.file_name))?
        }
    };
    String::from_utf8(bytes).map_err(|e| format!("read {}: {e}", file.file_name))
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
        if !matches!(classify_file_type(&file.file_type), FileClass::Wolf) {
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
            if let Some(file_key) = injection_bucket(&key) {
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

    let (total_checked, details) = report::collect_qa_details(&state.db, &project_id)
        .await
        .map_err(|e| e.to_string())?;

    let html = report::generate_qa_html(&project_title, &details, total_checked, &lang);

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

    // Linked edits span several files (System.json + every note referencing
    // a skill type): writing one file alone would break battle commands.
    if engine == "mv_mz"
        && !linked_skill_type_renames(&state.db, &file.project_id)
            .await?
            .is_empty()
    {
        return Err(
            "This project translates skill types that battle-command note tags look up by name; \
             those edits span several files. Use Export instead of single-file injection."
                .to_string(),
        );
    }

    // Fetch translations (key → target_text, core?, source).
    let rows: Vec<(String, String, bool, String)> = sqlx::query_as(
        "SELECT json_key, target_text, text_category IS NULL, source_text FROM segments \
         WHERE source_file_id = ? AND target_text != ''",
    )
    .bind(&source_file_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;
    let (label_rows, rows): (Vec<_>, Vec<_>) = rows
        .into_iter()
        .partition(|(key, ..)| key.starts_with(keke::NOTETAG_KEY_PREFIX));
    let label_rows: Vec<keke::LabelRow> = label_rows
        .into_iter()
        .map(|(key, target, _, source)| (key, target, source))
        .collect();

    // Apply font-size prefix in-memory (never written to DB).
    // For mv_mz: core Map segments only.
    let should_prefix = engine != "mv_mz" || file.file_type == "map";
    let mut segs = with_font_prefix(
        rows.into_iter()
            .map(|(key, target, is_core, _)| (key, target, is_core))
            .collect(),
        font_size,
        should_prefix,
        replace_existing,
        &engine,
    );

    // Dispatch by engine class.
    match classify_file_type(&file.file_type) {
        FileClass::Wolf => {
            let mut translations_by_file: HashMap<String, Vec<WolfTranslation>> = HashMap::new();
            for (key, text) in segs {
                if let Some(file_key) = injection_bucket(&key) {
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
        }
        FileClass::VxAce => {
            let pairs: Vec<(&str, &str)> =
                segs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
            let bytes = std::fs::read(&file.file_path)
                .map_err(|e| format!("read {}: {e}", file.file_name))?;
            let out = vx_injector::inject_and_serialize(&bytes, &pairs)
                .map_err(|e| format!("inject {}: {e}", file.file_name))?;
            std::fs::write(&file.file_path, out)
                .map_err(|e| format!("write {}: {e}", file.file_name))?;
            Ok(file.file_path.clone())
        }
        FileClass::Json => {
            let rel = originals::game_relative_path(&game_path, &file.file_path, &file.file_name);
            let mut snapshot = if engine == "mv_mz" {
                Some(
                    originals::ensure_fresh_mv_mz_snapshot(
                        &state.db,
                        &state.data_dir,
                        &file.project_id,
                    )
                    .await?,
                )
            } else {
                None
            };
            let raw = read_original_json(snapshot.as_ref(), &rel, &file)?;
            if !label_rows.is_empty() {
                let original: serde_json::Value = serde_json::from_str(&raw)
                    .map_err(|e| format!("parse {}: {e}", file.file_name))?;
                segs.extend(
                    keke::note_rewrites(&original, &label_rows, &HashMap::new())
                        .map_err(|e| format!("{}: {e}", file.file_name))?,
                );
            }
            let pairs: Vec<(&str, &str)> =
                segs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
            let out = injector::inject_to_bytes(&raw, &pairs)
                .map_err(|e| format!("inject {}: {e}", file.file_name))?;
            let planned: HashMap<String, String> = segs.iter().cloned().collect();
            let issues = integrity::verify_json_edit(&rel, &raw, &out, &planned);
            if !issues.is_empty() {
                return Err(integrity::summarize(&issues));
            }
            let written = [(rel, sha256_hex(&out))];
            std::fs::write(&file.file_path, out)
                .map_err(|e| format!("write {}: {e}", file.file_name))?;
            if let Some(store) = snapshot.as_mut() {
                originals::remember_written(store, &written);
            }
            Ok(file.file_path.clone())
        }
        FileClass::PluginsJs => {
            let rel = originals::game_relative_path(&game_path, &file.file_path, &file.file_name);
            let mut snapshot = originals::ensure_fresh_mv_mz_snapshot(
                &state.db,
                &state.data_dir,
                &file.project_id,
            )
            .await?;
            let raw = read_original_json(Some(&snapshot), &rel, &file)?;
            let rows = plugin_rows(&state.db, &file.id).await?;
            let (output, planned) = build_plugins_js(&raw, &rows)?;
            let issues = integrity::verify_plugins_edit(&rel, &raw, &output, &planned);
            if !issues.is_empty() {
                return Err(integrity::summarize(&issues));
            }
            let written = [(rel, sha256_hex(output.as_bytes()))];
            std::fs::write(&file.file_path, output)
                .map_err(|e| format!("write {}: {e}", file.file_name))?;
            originals::remember_written(&mut snapshot, &written);
            Ok(file.file_path.clone())
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugins_js_build_plans_fill_ins_against_an_empty_original() {
        let original = "var $plugins =\n[\n{\"name\":\"A\",\"status\":true,\"description\":\"\",\"parameters\":{\"Cmd\":\"図鑑\",\"Row\":\"\"}}\n];\n";
        let key = |param: &str| {
            crate::engines::mv_mz::plugins_js::plugin_key("A", 1, &[param.to_string()])
        };
        let rows = vec![
            (key("Cmd"), "Bestiary".to_string(), "図鑑".to_string(), None),
            (
                key("Row"),
                "Kills".to_string(),
                "倒した数".to_string(),
                Some(r#"{"fillIn":true}"#.to_string()),
            ),
        ];
        let (output, planned) = build_plugins_js(original, &rows).unwrap();
        assert!(output.contains(r#""Cmd":"Bestiary""#));
        assert!(output.contains(r#""Row":"Kills""#));
        assert_eq!(planned[1].expected_original, "");
        assert!(
            integrity::verify_plugins_edit("js/plugins.js", original, &output, &planned).is_empty()
        );

        let bad = vec![(
            "/not/a/plugin/key".to_string(),
            "x".to_string(),
            "y".to_string(),
            None,
        )];
        assert!(build_plugins_js(original, &bad).is_err());
    }

    /// Regression for a real-world silent break: a Wolf v3 message starting
    /// with the `@N\n` speaker prefix (name/face box lookup) must keep that
    /// prefix as the true first characters — the font-size code goes after
    /// it, not before, or the engine never recognizes the speaker marker and
    /// displays "@2" as literal text with no name/face box.
    #[test]
    fn test_apply_font_prefix_wolf_after_speaker_prefix() {
        let text = "@2\n\"Haa... I couldn't figure anything out today either...\"";
        let result = apply_font_prefix(text, 23, false, "wolf");
        assert_eq!(
            result,
            "@2\n\\f[23]\"Haa... I couldn't figure anything out today either...\""
        );
    }

    #[test]
    fn test_apply_font_prefix_wolf_no_speaker_prefix() {
        // Unchanged behavior when there's no leading @N\n to preserve.
        let result = apply_font_prefix("Hello there.", 18, false, "wolf");
        assert_eq!(result, "\\f[18]Hello there.");
    }

    #[test]
    fn test_apply_font_prefix_mvmz_unaffected() {
        // "@2\n..." is not a speaker prefix outside Wolf — must not be split.
        let result = apply_font_prefix("@2\nHello", 18, false, "mv_mz");
        assert_eq!(result, "\\FS[18]@2\nHello");
    }

    #[test]
    fn test_apply_font_prefix_wolf_replace_existing_after_speaker_prefix() {
        let text = "@2\n\\f[18]Hello there.";
        let result = apply_font_prefix(text, 23, true, "wolf");
        assert_eq!(result, "@2\n\\f[23]Hello there.");
    }

    #[test]
    fn test_apply_font_prefix_wolf_keep_existing_after_speaker_prefix() {
        // replace_existing = false and a font code is already present: no-op.
        let text = "@2\n\\f[18]Hello there.";
        let result = apply_font_prefix(text, 23, false, "wolf");
        assert_eq!(result, text);
    }

    #[test]
    fn test_strip_font_prefix_preserves_wolf_speaker_prefix() {
        let text = "@2\n\\f[23]Hello there.";
        assert_eq!(strip_font_prefix(text), "@2\nHello there.");
    }

    #[test]
    fn test_strip_font_prefix_no_speaker_prefix() {
        assert_eq!(strip_font_prefix("\\f[23]Hello"), "Hello");
        assert_eq!(strip_font_prefix("\\FS[23]Hello"), "Hello");
    }

    #[test]
    fn test_split_wolf_speaker_prefix() {
        assert_eq!(split_wolf_speaker_prefix("@2\nHello"), ("@2\n", "Hello"));
        assert_eq!(split_wolf_speaker_prefix("@10\nHello"), ("@10\n", "Hello"));
        assert_eq!(split_wolf_speaker_prefix("Hello"), ("", "Hello"));
    }
}
