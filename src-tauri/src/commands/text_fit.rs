//! Text fitting commands (RPG Maker MZ): settings, scan, preview, per-text
//! choices and the game font. The export applies the same fitting through
//! [`environment`] and [`fit_file`]. Wolf RPG messages (plan WR) share the
//! settings, the dialog and the scan, through [`super::wolf_text_fit`].

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;
use sqlx::SqlitePool;

use crate::commands::coverage::enabled_plugin_sources;
use crate::commands::wolf_text_fit::{self, WolfFitInfo};
use crate::core::originals;
use crate::core::text_fit::{self, TextFitSettings};
use crate::domain::types::SourceFile;
use crate::engines::filter::is_map_file;
use crate::engines::mv_mz::coverage::plugins::PLUGINS_FILE_NAME;
use crate::engines::mv_mz::layout::fit::{FitContext, FitOutcome, FitSettings};
use crate::engines::mv_mz::layout::font::{FontCache, Metrics};
use crate::engines::mv_mz::layout::geometry::{
    plugin_layout, Geometry, LayoutWarning, PluginLayout,
};
use crate::engines::mv_mz::layout::plan::{
    fit_descriptions, fit_messages, FitItem, FitOverride, StoredOverride, Target,
};
use crate::engines::mv_mz::layout::text::Inserts;
use crate::engines::mv_mz::originals::SnapshotStore;
use crate::engines::mv_mz::profiles::ProfileSet;
use crate::state::AppState;

/// The game font as used for layout.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FontInfo {
    /// `fonts/<file>` named in System.json.
    pub file: Option<String>,
    /// Widths come from the font file (otherwise they are estimated).
    pub loaded: bool,
    pub error: Option<String>,
}

/// Everything needed to fit an MZ project's texts.
pub struct FitEnvironment {
    pub geometry: Geometry,
    pub metrics: Metrics,
    pub font: FontInfo,
    pub font_path: Option<PathBuf>,
    pub plugins: PluginLayout,
    pub inserts: Inserts,
    pub no_space_join: bool,
    pub store: SnapshotStore,
    pub files: Vec<SourceFile>,
    pub game_path: String,
}

impl FitEnvironment {
    pub fn context(&self) -> FitContext<'_> {
        FitContext {
            metrics: &self.metrics,
            geometry: &self.geometry,
            inserts: &self.inserts,
            no_space_join: self.no_space_join,
            plain_description_first_line: self.plugins.plain_description_first_line,
        }
    }

    pub fn read_json(&self, file: &SourceFile) -> Result<Value, String> {
        let rel = originals::game_relative_path(&self.game_path, &file.file_path, &file.file_name);
        let bytes = self.store.read(&rel).map_err(|e| e.to_string())?;
        serde_json::from_slice(&bytes).map_err(|e| format!("{rel}: {e}"))
    }
}

/// An MZ project ready for fitting, or why it is not.
pub enum Support {
    Ready(Box<FitEnvironment>),
    Unsupported(String),
}

fn is_event_file(name: &str) -> bool {
    is_map_file(name, ".json") || name == "CommonEvents.json" || name == "Troops.json"
}

fn is_description_file(name: &str) -> bool {
    matches!(
        name,
        "Items.json" | "Skills.json" | "Weapons.json" | "Armors.json" | "Actors.json"
    )
}

/// Load what fitting needs: pristine data, geometry, font, plugins, inserts.
pub async fn environment(
    db: &SqlitePool,
    data_dir: &Path,
    profiles: &ProfileSet,
    fonts: &FontCache,
    project_id: &str,
) -> Result<Support, String> {
    let (game_path, engine, target_language): (String, String, String) = sqlx::query_as(
        "SELECT game_path, engine, COALESCE(target_language, 'en') FROM projects WHERE id = ?",
    )
    .bind(project_id)
    .fetch_one(db)
    .await
    .map_err(|e| e.to_string())?;
    if engine != "mv_mz" {
        return Ok(Support::Unsupported(
            "Text fitting is available for RPG Maker MZ games.".into(),
        ));
    }
    let store = originals::ensure_mv_mz_snapshot(db, data_dir, project_id).await?;
    let files = sqlx::query_as::<_, SourceFile>(
        "SELECT id, project_id, file_name, file_path, file_type, translation_secs \
         FROM source_files WHERE project_id = ?",
    )
    .bind(project_id)
    .fetch_all(db)
    .await
    .map_err(|e| e.to_string())?;
    let Some(system_file) = files.iter().find(|f| f.file_name == "System.json") else {
        return Ok(Support::Unsupported(
            "System.json was not extracted.".into(),
        ));
    };
    let js_dir = Path::new(&system_file.file_path)
        .parent()
        .and_then(Path::parent)
        .map(|game| game.join("js"));
    let Some(js_dir) = js_dir.filter(|dir| dir.join("rmmz_core.js").is_file()) else {
        return Ok(Support::Unsupported(
            "Text fitting is available for RPG Maker MZ games (this looks like MV).".into(),
        ));
    };
    let system_rel =
        originals::game_relative_path(&game_path, &system_file.file_path, &system_file.file_name);
    let system: Value =
        serde_json::from_slice(&store.read(&system_rel).map_err(|e| e.to_string())?)
            .map_err(|e| format!("{system_rel}: {e}"))?;
    let geometry = Geometry::from_system(&system);

    let font_path = geometry
        .font_file
        .as_ref()
        .and_then(|name| js_dir.parent().map(|game| game.join("fonts").join(name)));
    let (metrics, font) = match &font_path {
        Some(path) => match fonts.load(path) {
            Ok(font) => (
                Metrics::Font(font),
                FontInfo {
                    file: geometry.font_file.clone(),
                    loaded: true,
                    error: None,
                },
            ),
            Err(error) => (
                Metrics::Estimated,
                FontInfo {
                    file: geometry.font_file.clone(),
                    loaded: false,
                    error: Some(error.to_string()),
                },
            ),
        },
        None => (
            Metrics::Estimated,
            FontInfo {
                file: None,
                loaded: false,
                error: Some("System.json names no main font file.".into()),
            },
        ),
    };

    let plugins = match std::fs::read(js_dir.join(PLUGINS_FILE_NAME)) {
        Ok(bytes) => plugin_layout(
            &enabled_plugin_sources(&js_dir, &String::from_utf8_lossy(&bytes)),
            profiles,
        ),
        Err(_) => PluginLayout::default(),
    };

    // Inserted text: translated actor names and currency unit.
    let named: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT sf.file_name, s.json_key, s.target_text FROM segments s \
         JOIN source_files sf ON sf.id = s.source_file_id \
         WHERE sf.project_id = ? AND s.target_text != '' \
           AND ((sf.file_name = 'Actors.json' AND s.json_key LIKE '/%/name') \
             OR (sf.file_name = 'System.json' AND s.json_key = '/currencyUnit'))",
    )
    .bind(project_id)
    .fetch_all(db)
    .await
    .map_err(|e| e.to_string())?;
    let translated: HashMap<(String, String), String> =
        named.into_iter().map(|(f, k, t)| ((f, k), t)).collect();
    let mut inserts = Inserts {
        currency: translated
            .get(&("System.json".to_string(), "/currencyUnit".to_string()))
            .cloned()
            .or_else(|| system["currencyUnit"].as_str().map(str::to_string))
            .unwrap_or_default(),
        plugin_codes: plugins.inserted_codes.clone(),
        ..Inserts::default()
    };
    if let Some(actors_file) = files.iter().find(|f| f.file_name == "Actors.json") {
        let rel = originals::game_relative_path(
            &game_path,
            &actors_file.file_path,
            &actors_file.file_name,
        );
        if let Ok(bytes) = store.read(&rel) {
            let actors: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
            for (id, actor) in actors.as_array().into_iter().flatten().enumerate() {
                let Some(name) = actor.get("name").and_then(Value::as_str) else {
                    continue;
                };
                let key = ("Actors.json".to_string(), format!("/{id}/name"));
                let name = translated.get(&key).map_or(name, String::as_str);
                inserts.actor_names.insert(id as u32, name.to_string());
            }
        }
    }

    Ok(Support::Ready(Box::new(FitEnvironment {
        geometry,
        metrics,
        font,
        font_path,
        plugins,
        inserts,
        no_space_join: ["zh", "ja", "ko"]
            .iter()
            .any(|code| target_language.starts_with(code)),
        store,
        files,
        game_path,
    })))
}

/// Translations of every segment of a project, by file then pointer.
pub async fn project_targets(
    db: &SqlitePool,
    project_id: &str,
) -> Result<HashMap<String, HashMap<String, Target>>, String> {
    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT sf.file_name, s.json_key, s.id, s.target_text FROM segments s \
         JOIN source_files sf ON sf.id = s.source_file_id WHERE sf.project_id = ?",
    )
    .bind(project_id)
    .fetch_all(db)
    .await
    .map_err(|e| e.to_string())?;
    let mut out: HashMap<String, HashMap<String, Target>> = HashMap::new();
    for (file, key, segment_id, text) in rows {
        out.entry(file)
            .or_default()
            .insert(key, Target { segment_id, text });
    }
    Ok(out)
}

/// Fit one file: messages of event files (if `dialogue`), descriptions of
/// database files (if `descriptions`).
pub fn fit_file(
    env: &FitEnvironment,
    file: &SourceFile,
    targets: &HashMap<String, Target>,
    fit: &FitSettings,
    overrides: &HashMap<String, StoredOverride>,
    dialogue: bool,
    descriptions: bool,
) -> Result<Vec<FitItem>, String> {
    let name = file.file_name.as_str();
    if dialogue && is_event_file(name) {
        let json = env.read_json(file)?;
        return Ok(fit_messages(
            name,
            &json,
            targets,
            fit,
            overrides,
            &env.context(),
        ));
    }
    if descriptions && is_description_file(name) {
        let json = env.read_json(file)?;
        return Ok(fit_descriptions(
            name,
            &json,
            targets,
            fit,
            overrides,
            &env.context(),
        ));
    }
    Ok(Vec::new())
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextFitView {
    pub supported: bool,
    pub reason: Option<String>,
    pub settings: TextFitSettings,
    pub geometry: Option<Geometry>,
    pub font: Option<FontInfo>,
    pub warnings: Vec<LayoutWarning>,
    pub notes: Vec<String>,
    /// Wolf RPG: the measured message window (`None` for MZ).
    pub wolf: Option<WolfFitInfo>,
}

async fn project_engine(db: &SqlitePool, project_id: &str) -> Result<String, String> {
    sqlx::query_scalar("SELECT engine FROM projects WHERE id = ?")
        .bind(project_id)
        .fetch_one(db)
        .await
        .map_err(|e| e.to_string())
}

async fn support(state: &AppState, project_id: &str) -> Result<Support, String> {
    environment(
        &state.db,
        &state.data_dir,
        &state.plugin_profiles,
        &state.game_fonts,
        project_id,
    )
    .await
}

/// Settings, detected box/font and plugin warnings.
#[tauri::command]
pub async fn get_text_fit(
    project_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<TextFitView, String> {
    if project_engine(&state.db, &project_id).await? == "wolf" {
        let settings = text_fit::load_settings_for_engine(&state.db, &project_id, "wolf")
            .await
            .map_err(|e| e.to_string())?;
        let env =
            wolf_text_fit::wolf_environment(&state.db, &state.game_fonts, &project_id, &settings)
                .await?;
        return Ok(TextFitView {
            supported: true,
            reason: None,
            settings,
            geometry: None,
            font: Some(env.font.clone()),
            warnings: Vec::new(),
            notes: wolf_text_fit::notes(&env),
            wolf: Some(env.info),
        });
    }
    let settings = text_fit::load_settings(&state.db, &project_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(match support(&state, &project_id).await? {
        Support::Ready(env) => TextFitView {
            supported: true,
            reason: None,
            settings,
            geometry: Some(env.geometry.clone()),
            font: Some(env.font.clone()),
            warnings: env.plugins.warnings.clone(),
            notes: env.plugins.notes.clone(),
            wolf: None,
        },
        Support::Unsupported(reason) => TextFitView {
            supported: false,
            reason: Some(reason),
            settings,
            geometry: None,
            font: None,
            warnings: Vec::new(),
            notes: Vec::new(),
            wolf: None,
        },
    })
}

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FitCounts {
    pub total: usize,
    pub fits: usize,
    pub rewrapped: usize,
    pub new_page: usize,
    pub shrunk: usize,
    pub too_long: usize,
    pub skipped: usize,
}

impl FitCounts {
    pub(crate) fn add(&mut self, outcome: &FitOutcome) {
        self.total += 1;
        match outcome {
            FitOutcome::Fits => self.fits += 1,
            FitOutcome::Rewrapped => self.rewrapped += 1,
            FitOutcome::NewPage { .. } => self.new_page += 1,
            FitOutcome::Shrunk { .. } => self.shrunk += 1,
            FitOutcome::TooLong { .. } => self.too_long += 1,
            FitOutcome::Skipped { .. } => self.skipped += 1,
        }
    }
}

/// A text worth reviewing (new page, shrunk, too long, skipped).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FitRow {
    pub file_id: String,
    pub file_name: String,
    pub pointer: String,
    pub description: bool,
    /// First translated line (to open it in the editor).
    pub segment_id: Option<String>,
    pub outcome: FitOutcome,
    pub sample: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingChar {
    pub character: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FitScan {
    pub dialogue: FitCounts,
    pub descriptions: FitCounts,
    pub rows: Vec<FitRow>,
    /// Characters the game font lacks (drawn with a fallback font).
    pub missing_chars: Vec<MissingChar>,
}

const MAX_ROWS: usize = 500;

/// Fit every message and description with `settings` (the saved settings
/// when `None`), whether or not fitting is enabled, and summarise.
#[tauri::command]
pub async fn scan_text_fit(
    project_id: String,
    settings: Option<TextFitSettings>,
    state: tauri::State<'_, AppState>,
) -> Result<FitScan, String> {
    if project_engine(&state.db, &project_id).await? == "wolf" {
        let settings = match settings {
            Some(settings) => {
                settings.validate()?;
                settings
            }
            None => text_fit::load_settings_for_engine(&state.db, &project_id, "wolf")
                .await
                .map_err(|e| e.to_string())?,
        };
        let env =
            wolf_text_fit::wolf_environment(&state.db, &state.game_fonts, &project_id, &settings)
                .await?;
        return wolf_text_fit::scan(&state.db, &project_id, env, &settings).await;
    }
    let settings = match settings {
        Some(settings) => {
            settings.validate()?;
            settings
        }
        None => text_fit::load_settings(&state.db, &project_id)
            .await
            .map_err(|e| e.to_string())?,
    };
    let Support::Ready(env) = support(&state, &project_id).await? else {
        return Err("Text fitting is not available for this project.".into());
    };
    let targets = project_targets(&state.db, &project_id).await?;
    let overrides = text_fit::load_overrides(&state.db, &project_id)
        .await
        .map_err(|e| e.to_string())?;
    tokio::task::spawn_blocking(move || scan(&env, &targets, &overrides, &settings))
        .await
        .map_err(|e| e.to_string())?
}

fn scan(
    env: &FitEnvironment,
    targets: &HashMap<String, HashMap<String, Target>>,
    overrides: &HashMap<String, HashMap<String, StoredOverride>>,
    settings: &TextFitSettings,
) -> Result<FitScan, String> {
    let empty_targets = HashMap::new();
    let empty_overrides = HashMap::new();
    let mut dialogue = FitCounts::default();
    let mut descriptions = FitCounts::default();
    let mut rows = Vec::new();
    let mut missing: BTreeMap<char, usize> = BTreeMap::new();
    let mut files: Vec<&SourceFile> = env.files.iter().collect();
    files.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    for file in files {
        let items = fit_file(
            env,
            file,
            targets.get(&file.file_name).unwrap_or(&empty_targets),
            &settings.fit_settings(),
            overrides.get(&file.file_name).unwrap_or(&empty_overrides),
            true,
            true,
        )?;
        for item in items {
            let counts = if item.description {
                &mut descriptions
            } else {
                &mut dialogue
            };
            counts.add(&item.fitted.outcome);
            for (text, id) in item.texts.iter().zip(&item.segment_ids) {
                if id.is_some() {
                    for c in env.metrics.missing_chars(text) {
                        *missing.entry(c).or_default() += 1;
                    }
                }
            }
            let notable = !matches!(
                item.fitted.outcome,
                FitOutcome::Fits | FitOutcome::Rewrapped
            );
            if notable && rows.len() < MAX_ROWS {
                let first = item
                    .segment_ids
                    .iter()
                    .position(Option::is_some)
                    .unwrap_or(0);
                rows.push(FitRow {
                    file_id: file.id.clone(),
                    file_name: file.file_name.clone(),
                    pointer: item.pointer.clone(),
                    description: item.description,
                    segment_id: item.segment_ids.get(first).cloned().flatten(),
                    outcome: item.fitted.outcome.clone(),
                    sample: item
                        .texts
                        .get(first)
                        .map_or_else(String::new, |t| t.chars().take(80).collect()),
                });
            }
        }
    }
    let mut missing_chars: Vec<MissingChar> = missing
        .into_iter()
        .map(|(c, count)| MissingChar {
            character: c.to_string(),
            count,
        })
        .collect();
    missing_chars.sort_by_key(|m| std::cmp::Reverse(m.count));
    Ok(FitScan {
        dialogue,
        descriptions,
        rows,
        missing_chars,
    })
}

/// Save the project's settings.
#[tauri::command]
pub async fn set_text_fit(
    project_id: String,
    settings: TextFitSettings,
    state: tauri::State<'_, AppState>,
) -> Result<TextFitSettings, String> {
    settings.validate()?;
    if project_engine(&state.db, &project_id).await? == "wolf" {
        settings.validate_wolf()?;
        // Never on where export cannot fit (see below): the window must be known.
        if settings.enabled_dialogue {
            let env = wolf_text_fit::wolf_environment(
                &state.db,
                &state.game_fonts,
                &project_id,
                &settings,
            )
            .await?;
            if env.info.budget.is_none() {
                return Err("The message window is unknown: set the line width and the \
                            lines per window."
                    .into());
            }
        }
        text_fit::save_settings(&state.db, &project_id, &settings)
            .await
            .map_err(|e| e.to_string())?;
        return Ok(settings);
    }
    // Never on where export cannot fit: QA would hide "line too long" for
    // text that is then exported unfitted.
    if settings.enabled() {
        if let Support::Unsupported(reason) = support(&state, &project_id).await? {
            return Err(reason);
        }
    }
    text_fit::save_settings(&state.db, &project_id, &settings)
        .await
        .map_err(|e| e.to_string())?;
    Ok(settings)
}

/// How one message or description will look in game.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextFitPreview {
    pub file_name: String,
    pub item: FitItem,
    /// Fitting is on for this kind of text (otherwise this is what it would do).
    pub enabled: bool,
    pub inner_width: f32,
    pub inner_height: f32,
    pub text_x: f32,
    pub limit: f32,
    pub font_size: f32,
}

/// Fit the message or description that contains `segment_id` (with an
/// unsaved `draft_target` for that segment, if given).
async fn preview_item(
    state: &AppState,
    project_id: &str,
    segment_id: &str,
    draft_target: Option<String>,
) -> Result<Option<TextFitPreview>, String> {
    let Some((file_id, file_name, key)): Option<(String, String, String)> = sqlx::query_as(
        "SELECT sf.id, sf.file_name, s.json_key FROM segments s \
         JOIN source_files sf ON sf.id = s.source_file_id \
         WHERE s.id = ? AND sf.project_id = ?",
    )
    .bind(segment_id)
    .bind(project_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| e.to_string())?
    else {
        return Ok(None);
    };
    if !is_event_file(&file_name) && !is_description_file(&file_name) {
        return Ok(None);
    }
    let Support::Ready(env) = support(state, project_id).await? else {
        return Ok(None);
    };
    let settings = text_fit::load_settings(&state.db, project_id)
        .await
        .map_err(|e| e.to_string())?;
    let mut targets = project_targets(&state.db, project_id)
        .await?
        .remove(&file_name)
        .unwrap_or_default();
    if let (Some(draft), Some(target)) = (draft_target, targets.get_mut(&key)) {
        target.text = draft;
    }
    let overrides = text_fit::load_overrides(&state.db, project_id)
        .await
        .map_err(|e| e.to_string())?
        .remove(&file_name)
        .unwrap_or_default();
    tokio::task::spawn_blocking(move || -> Result<Option<TextFitPreview>, String> {
        let Some(file) = env.files.iter().find(|f| f.id == file_id) else {
            return Ok(None);
        };
        let items = fit_file(
            &env,
            file,
            &targets,
            &settings.fit_settings(),
            &overrides,
            true,
            true,
        )?;
        let Some(item) = items.into_iter().find(|item| item.lines.contains(&key)) else {
            return Ok(None);
        };
        let enabled = if item.description {
            settings.enabled_descriptions
        } else {
            settings.enabled_dialogue
        };
        let (text_x, limit, inner_width, inner_height) = if item.description {
            let help = &env.geometry.help;
            (
                help.text_x,
                help.text_x + help.text_width,
                env.geometry.message.inner_width,
                help.inner_height,
            )
        } else {
            let message = &env.geometry.message;
            let x = message.text_x(item.face);
            (
                x,
                x + message.text_width(item.face),
                message.inner_width,
                message.inner_height,
            )
        };
        Ok(Some(TextFitPreview {
            file_name: file.file_name.clone(),
            item,
            enabled,
            inner_width,
            inner_height,
            text_x,
            limit,
            font_size: env.geometry.font_size,
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Preview the in-game layout of the text containing `segment_id`.
#[tauri::command]
pub async fn preview_text_fit(
    project_id: String,
    segment_id: String,
    draft_target: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<Option<TextFitPreview>, String> {
    preview_item(&state, &project_id, &segment_id, draft_target).await
}

/// Store (or clear, with `None`) the choice for the text containing
/// `segment_id`; returns the updated preview.
#[tauri::command]
pub async fn set_text_fit_override(
    project_id: String,
    segment_id: String,
    choice: Option<FitOverride>,
    state: tauri::State<'_, AppState>,
) -> Result<Option<TextFitPreview>, String> {
    let Some(preview) = preview_item(&state, &project_id, &segment_id, None).await? else {
        return Err("This text is not fitted.".into());
    };
    text_fit::save_override(
        &state.db,
        &project_id,
        &preview.file_name,
        &preview.item.pointer,
        &preview.item.source_hash,
        choice.as_ref(),
    )
    .await
    .map_err(|e| e.to_string())?;
    preview_item(&state, &project_id, &segment_id, None).await
}

/// The game's main font file (for drawing the preview with it).
#[tauri::command]
pub async fn get_game_font(
    project_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<tauri::ipc::Response, String> {
    let Support::Ready(env) = support(&state, &project_id).await? else {
        return Err("Text fitting is not available for this project.".into());
    };
    let path = env
        .font_path
        .filter(|_| env.font.loaded)
        .ok_or_else(|| "The game font could not be read.".to_string())?;
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    Ok(tauri::ipc::Response::new(bytes))
}
