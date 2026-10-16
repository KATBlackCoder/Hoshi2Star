//! Optional text coverage commands (RPG Maker MV/MZ).
//!
//! `scan_text_coverage` reports every optional category with its safety state
//! and consequences; `set_text_categories` applies the user's decisions. The
//! scan always runs on the pristine snapshot, and `set_text_categories`
//! re-scans server-side instead of trusting what the UI displayed. Nothing is
//! ever enabled without the user: `get_text_coverage_status` only tells the UI
//! whether to offer the review.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqlitePool;

use crate::core::{manifest, originals};
use crate::engines::filter::{classify_file_type, FileClass};
use crate::engines::mv_mz::coverage::plugins::PLUGINS_FILE_NAME;
use crate::engines::mv_mz::coverage::{
    self, CategoryGroup, CategoryState, Constraint, CoreSource, CoreWarning, CoverageInput,
    CoverageReport, NotCovered, Reference, TextCategory,
};
use crate::engines::mv_mz::originals::{Provenance, SnapshotStore};
use crate::engines::mv_mz::plugins_js::PluginsJs;
use crate::engines::mv_mz::profiles::ProfileSet;
use crate::llm::context::{mv_mz_context_class, PromptContextClass};
use crate::state::AppState;

const SAMPLES_SHOWN: usize = 5;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryView {
    pub key: String,
    pub title: String,
    pub group: CategoryGroup,
    pub state: CategoryState,
    /// Current decision (segments exist exactly when `true`).
    pub enabled: bool,
    /// Whether the user ever decided.
    pub decided: bool,
    pub default_enabled: bool,
    pub visible_in: String,
    pub consequences: Vec<String>,
    pub constraints: Vec<Constraint>,
    pub segment_count: usize,
    pub translated_count: i64,
    pub samples: Vec<String>,
    pub references: Vec<Reference>,
    pub reference_count: usize,
    pub blocked_reason: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriftedFileView {
    pub file: String,
    pub pointers: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverageView {
    /// `false` for engines without optional categories (Wolf, VX Ace).
    pub supported: bool,
    pub has_decisions: bool,
    /// The game folder held a previous Hoshi2Star export; originals were
    /// reconstructed from the project database.
    pub reconstructed: bool,
    /// Files whose extracted text changed since extraction (game updated).
    pub drifted_files: Vec<DriftedFileView>,
    pub plugins_unreadable: Option<String>,
    pub categories: Vec<CategoryView>,
    pub core_warnings: Vec<CoreWarning>,
    /// Plugin text that no verified profile covers (report only).
    pub not_covered: Vec<NotCovered>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryChange {
    pub category_key: String,
    pub enabled: bool,
    /// Required to enable a `linked` category: the user confirmed its consequences.
    #[serde(default)]
    pub accept_consequences: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverageStatus {
    /// `false` for engines without optional categories (Wolf, VX Ace).
    pub supported: bool,
    pub has_decisions: bool,
}

/// Whether optional categories apply to a project and were ever decided,
/// without scanning (a scan may have to snapshot the whole game first).
#[tauri::command]
pub async fn get_text_coverage_status(
    project_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<CoverageStatus, String> {
    let engine: String = sqlx::query_scalar("SELECT engine FROM projects WHERE id = ?")
        .bind(&project_id)
        .fetch_one(&state.db)
        .await
        .map_err(|error| error.to_string())?;
    let decisions: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM project_text_categories WHERE project_id = ?")
            .bind(&project_id)
            .fetch_one(&state.db)
            .await
            .map_err(|error| error.to_string())?;
    Ok(CoverageStatus {
        supported: engine == "mv_mz",
        has_decisions: decisions > 0,
    })
}

/// Report every optional text category of an MV/MZ project.
#[tauri::command]
pub async fn scan_text_coverage(
    project_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<CoverageView, String> {
    let Some((report, store)) = scan_project(
        &state.db,
        &state.data_dir,
        state.plugin_profiles.clone(),
        &project_id,
    )
    .await?
    else {
        return Ok(unsupported_view());
    };
    build_view(&state.db, &project_id, report, &store).await
}

/// Enable/disable categories. Blocked categories can never be enabled; linked
/// ones only with `accept_consequences`.
#[tauri::command]
pub async fn set_text_categories(
    project_id: String,
    changes: Vec<CategoryChange>,
    state: tauri::State<'_, AppState>,
) -> Result<CoverageView, String> {
    let Some((report, store)) = scan_project(
        &state.db,
        &state.data_dir,
        state.plugin_profiles.clone(),
        &project_id,
    )
    .await?
    else {
        return Err("This engine has no optional text categories.".to_string());
    };
    apply_changes(&state.db, &project_id, &report, &changes).await?;
    manifest::refresh_stats(&state.db, &project_id).await;
    build_view(&state.db, &project_id, report, &store).await
}

fn unsupported_view() -> CoverageView {
    CoverageView {
        supported: false,
        has_decisions: false,
        reconstructed: false,
        drifted_files: Vec::new(),
        plugins_unreadable: None,
        categories: Vec::new(),
        core_warnings: Vec::new(),
        not_covered: Vec::new(),
    }
}

/// Load the pristine inputs and run the scan. `None` for non-MV/MZ projects.
pub async fn scan_project(
    db: &SqlitePool,
    data_dir: &Path,
    profiles: std::sync::Arc<ProfileSet>,
    project_id: &str,
) -> Result<Option<(CoverageReport, SnapshotStore)>, String> {
    let (game_path, engine): (String, String) =
        sqlx::query_as("SELECT game_path, engine FROM projects WHERE id = ?")
            .bind(project_id)
            .fetch_one(db)
            .await
            .map_err(|error| error.to_string())?;
    if engine != "mv_mz" {
        return Ok(None);
    }

    let mut store = originals::ensure_fresh_mv_mz_snapshot(db, data_dir, project_id).await?;
    let files: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT file_name, file_path, file_type FROM source_files WHERE project_id = ?",
    )
    .bind(project_id)
    .fetch_all(db)
    .await
    .map_err(|error| error.to_string())?;
    let core_rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT s.segment_kind, s.source_text FROM segments s \
         JOIN source_files sf ON sf.id = s.source_file_id \
         WHERE sf.project_id = ? AND s.text_category IS NULL",
    )
    .bind(project_id)
    .fetch_all(db)
    .await
    .map_err(|error| error.to_string())?;

    let worker = tokio::task::spawn_blocking(move || -> Result<_, String> {
        let mut data_files = Vec::new();
        let mut data_dir_on_disk: Option<PathBuf> = None;
        for (file_name, file_path, file_type) in files {
            if !matches!(classify_file_type(&file_type), FileClass::Json)
                || !file_name.ends_with(".json")
            {
                continue;
            }
            let rel = originals::game_relative_path(&game_path, &file_path, &file_name);
            let bytes = store.read(&rel).map_err(|error| error.to_string())?;
            let json: Value =
                serde_json::from_slice(&bytes).map_err(|error| format!("{rel}: {error}"))?;
            if file_name == "System.json" {
                data_dir_on_disk = Path::new(&file_path).parent().map(Path::to_path_buf);
            }
            data_files.push((file_name, rel, json));
        }

        // `js/` sits next to `data/` (MZ: <game>/js, MV: <game>/www/js).
        let js_dir = data_dir_on_disk.and_then(|dir| dir.parent().map(|p| p.join("js")));
        let mut plugins_text = None;
        let mut plugins_rel = "js/plugins.js".to_string();
        if let Some(js_dir) = &js_dir {
            let disk = js_dir.join(PLUGINS_FILE_NAME);
            if disk.is_file() {
                let rel = originals::game_relative_path(
                    &game_path,
                    &disk.to_string_lossy(),
                    "js/plugins.js",
                );
                plugins_rel = rel.clone();
                if originals::capture_verbatim(&mut store, &rel, &disk)? {
                    store.save().map_err(|error| error.to_string())?;
                }
                let bytes = store.read(&rel).map_err(|error| error.to_string())?;
                plugins_text = Some(String::from_utf8_lossy(&bytes).into_owned());
            }
        }

        let plugin_sources = match (&js_dir, &plugins_text) {
            (Some(js_dir), Some(text)) => enabled_plugin_sources(js_dir, text),
            _ => Vec::new(),
        };

        let core_sources: Vec<CoreSource> = core_rows
            .into_iter()
            .filter(|(kind, _)| mv_mz_context_class(kind) == PromptContextClass::Canonical)
            .map(|(segment_kind, source)| CoreSource {
                segment_kind,
                source,
            })
            .collect();

        let report = coverage::scan(&CoverageInput {
            data_files: &data_files,
            plugins_js: plugins_text.as_deref(),
            plugin_sources: &plugin_sources,
            core_sources: &core_sources,
            plugins_rel: &plugins_rel,
            profiles: &profiles,
        });
        Ok((report, store))
    });
    let (report, store) = worker.await.map_err(|error| error.to_string())??;
    Ok(Some((report, store)))
}

/// `(plugin name, source)` of every enabled plugin of `plugins_text` whose
/// file exists under `js_dir/plugins` (names may contain `/` subfolders).
pub(crate) fn enabled_plugin_sources(js_dir: &Path, plugins_text: &str) -> Vec<(String, String)> {
    let Ok(plugins) = PluginsJs::parse(plugins_text) else {
        return Vec::new();
    };
    let mut sources = Vec::new();
    for entry in plugins.entries().iter().filter(|entry| entry.status) {
        let Some(path) = plugin_script_path(js_dir, &entry.name) else {
            log::warn!("ignoring plugin with an unsafe name: {:?}", entry.name);
            continue;
        };
        if let Ok(source) = std::fs::read(&path) {
            sources.push((
                entry.name.clone(),
                String::from_utf8_lossy(&source).into_owned(),
            ));
        }
    }
    sources
}

/// `js/plugins/<name>.js`, like the engine's `PluginManager.loadScript`
/// (subfolders allowed). `None` for a name that could leave `js/plugins`:
/// `plugins.js` comes from the game folder and is not trusted.
fn plugin_script_path(js_dir: &Path, name: &str) -> Option<PathBuf> {
    let mut path = js_dir.join("plugins");
    for part in name.split('/') {
        let safe =
            !part.is_empty() && part != "." && part != ".." && !part.contains(['\\', ':', '\0']);
        if !safe {
            return None;
        }
        path.push(part);
    }
    let mut file = path.into_os_string();
    file.push(".js");
    Some(PathBuf::from(file))
}

async fn build_view(
    db: &SqlitePool,
    project_id: &str,
    report: CoverageReport,
    store: &SnapshotStore,
) -> Result<CoverageView, String> {
    let decisions: Vec<(String, i64)> = sqlx::query_as(
        "SELECT category_key, enabled FROM project_text_categories WHERE project_id = ?",
    )
    .bind(project_id)
    .fetch_all(db)
    .await
    .map_err(|error| error.to_string())?;
    let decisions: HashMap<String, bool> = decisions
        .into_iter()
        .map(|(key, enabled)| (key, enabled != 0))
        .collect();
    let counts: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT s.text_category, COUNT(*), SUM(CASE WHEN s.target_text != '' THEN 1 ELSE 0 END) \
         FROM segments s JOIN source_files sf ON sf.id = s.source_file_id \
         WHERE sf.project_id = ? AND s.text_category IS NOT NULL GROUP BY s.text_category",
    )
    .bind(project_id)
    .fetch_all(db)
    .await
    .map_err(|error| error.to_string())?;
    let translated: HashMap<String, i64> = counts
        .into_iter()
        .map(|(key, _, translated)| (key, translated))
        .collect();

    let categories = report
        .categories
        .into_iter()
        .map(|category| {
            let decided = decisions.get(&category.key).copied();
            CategoryView {
                enabled: decided.unwrap_or(false),
                decided: decided.is_some(),
                translated_count: translated.get(&category.key).copied().unwrap_or(0),
                segment_count: category.candidates.len(),
                samples: category
                    .candidates
                    .iter()
                    .take(SAMPLES_SHOWN)
                    .map(|candidate| candidate.source.clone())
                    .collect(),
                key: category.key,
                title: category.title,
                group: category.group,
                state: category.state,
                default_enabled: category.default_enabled,
                visible_in: category.visible_in,
                consequences: category.consequences,
                constraints: category.constraints,
                references: category.references,
                reference_count: category.reference_count,
                blocked_reason: category.blocked_reason,
            }
        })
        .collect();

    let mut reconstructed = false;
    let mut drifted_files = Vec::new();
    for (file, entry) in store.entries() {
        reconstructed |= entry.provenance == Provenance::Reconstructed;
        if !entry.drift.is_empty() {
            drifted_files.push(DriftedFileView {
                file: file.to_string(),
                pointers: entry.drift.len(),
            });
        }
    }

    Ok(CoverageView {
        supported: true,
        has_decisions: !decisions.is_empty(),
        reconstructed,
        drifted_files,
        plugins_unreadable: report.plugins_unreadable,
        categories,
        core_warnings: report.core_warnings,
        not_covered: report.not_covered,
    })
}

async fn apply_changes(
    db: &SqlitePool,
    project_id: &str,
    report: &CoverageReport,
    changes: &[CategoryChange],
) -> Result<(), String> {
    // Validate everything before touching the database.
    for change in changes.iter().filter(|change| change.enabled) {
        let category = find_category(report, &change.category_key)?;
        match category.state {
            CategoryState::Blocked => {
                return Err(format!(
                    "\"{}\" cannot be enabled: {}",
                    category.title,
                    category
                        .blocked_reason
                        .as_deref()
                        .unwrap_or("it is blocked")
                ))
            }
            CategoryState::Linked if !change.accept_consequences => {
                return Err(format!(
                    "\"{}\" changes other game data; its consequences must be accepted first.",
                    category.title
                ))
            }
            _ => {}
        }
    }

    let mut file_ids: HashMap<String, String> = sqlx::query_as::<_, (String, String)>(
        "SELECT file_name, id FROM source_files WHERE project_id = ?",
    )
    .bind(project_id)
    .fetch_all(db)
    .await
    .map_err(|error| error.to_string())?
    .into_iter()
    .collect();
    let game_path: String = sqlx::query_scalar("SELECT game_path FROM projects WHERE id = ?")
        .bind(project_id)
        .fetch_one(db)
        .await
        .map_err(|error| error.to_string())?;

    let mut tx = db.begin().await.map_err(|error| error.to_string())?;
    // js/plugins.js is not part of core extraction: register it on first use.
    let needs_plugins_row = changes
        .iter()
        .filter(|change| change.enabled)
        .any(|change| {
            find_category(report, &change.category_key).is_ok_and(|category| {
                category
                    .candidates
                    .iter()
                    .any(|candidate| candidate.file_name == PLUGINS_FILE_NAME)
            })
        });
    if needs_plugins_row && !file_ids.contains_key(PLUGINS_FILE_NAME) {
        let rel = report
            .categories
            .iter()
            .flat_map(|category| &category.candidates)
            .find(|candidate| candidate.file_name == PLUGINS_FILE_NAME)
            .map(|candidate| candidate.file_rel.clone())
            .unwrap_or_else(|| "js/plugins.js".to_string());
        let mut absolute = PathBuf::from(&game_path);
        for part in rel.split('/') {
            absolute.push(part);
        }
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
             VALUES (?, ?, ?, ?, 'plugins_js')",
        )
        .bind(&id)
        .bind(project_id)
        .bind(PLUGINS_FILE_NAME)
        .bind(absolute.to_string_lossy().to_string())
        .execute(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
        file_ids.insert(PLUGINS_FILE_NAME.to_string(), id);
    }
    for change in changes {
        if change.enabled {
            let category = find_category(report, &change.category_key)?;
            enable_category(&mut tx, project_id, category, &file_ids).await?;
        } else {
            disable_category(&mut tx, project_id, &change.category_key).await?;
        }
        let accepted = change.enabled && change.accept_consequences;
        sqlx::query(
            "INSERT INTO project_text_categories (project_id, category_key, enabled, accepted_at) \
             VALUES (?, ?, ?, CASE WHEN ? THEN datetime('now') END) \
             ON CONFLICT(project_id, category_key) DO UPDATE SET \
               enabled = excluded.enabled, \
               accepted_at = COALESCE(excluded.accepted_at, project_text_categories.accepted_at), \
               updated_at = datetime('now')",
        )
        .bind(project_id)
        .bind(&change.category_key)
        .bind(i64::from(change.enabled))
        .bind(accepted)
        .execute(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
    }
    tx.commit().await.map_err(|error| error.to_string())
}

fn find_category<'a>(report: &'a CoverageReport, key: &str) -> Result<&'a TextCategory, String> {
    report
        .categories
        .iter()
        .find(|category| category.key == key)
        .ok_or_else(|| format!("unknown text category: {key}"))
}

async fn enable_category(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    project_id: &str,
    category: &TextCategory,
    file_ids: &HashMap<String, String>,
) -> Result<(), String> {
    for candidate in &category.candidates {
        let file_id = file_ids
            .get(&candidate.file_name)
            .ok_or_else(|| format!("{} is not part of this project", candidate.file_name))?;
        let exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM segments WHERE source_file_id = ? AND json_key = ?",
        )
        .bind(file_id)
        .bind(&candidate.key)
        .fetch_one(&mut **tx)
        .await
        .map_err(|error| error.to_string())?;
        if exists > 0 {
            continue;
        }
        let parked: Option<(String, String, Option<i64>)> = sqlx::query_as(
            "SELECT target_text, status, qa_score FROM parked_segments \
             WHERE project_id = ? AND file_name = ? AND json_key = ? AND source_text = ?",
        )
        .bind(project_id)
        .bind(&candidate.file_name)
        .bind(&candidate.key)
        .bind(&candidate.source)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| error.to_string())?;
        let (target, status, qa_score) =
            parked.unwrap_or_else(|| (String::new(), "untranslated".to_string(), None));
        sqlx::query(
            "INSERT INTO segments \
             (id, source_file_id, json_key, source_text, target_text, status, qa_score, \
              segment_kind, context_json, text_category) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(file_id)
        .bind(&candidate.key)
        .bind(&candidate.source)
        .bind(&target)
        .bind(&status)
        .bind(qa_score)
        .bind(candidate.kind.as_str())
        .bind(&candidate.context_json)
        .bind(&category.key)
        .execute(&mut **tx)
        .await
        .map_err(|error| error.to_string())?;
    }
    sqlx::query("DELETE FROM parked_segments WHERE project_id = ? AND category_key = ?")
        .bind(project_id)
        .bind(&category.key)
        .execute(&mut **tx)
        .await
        .map_err(|error| error.to_string())?;
    Ok(())
}

async fn disable_category(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    project_id: &str,
    category_key: &str,
) -> Result<(), String> {
    sqlx::query(
        "INSERT OR REPLACE INTO parked_segments \
         (project_id, category_key, file_name, json_key, source_text, target_text, status, qa_score) \
         SELECT sf.project_id, s.text_category, sf.file_name, s.json_key, s.source_text, \
                s.target_text, s.status, s.qa_score \
         FROM segments s JOIN source_files sf ON sf.id = s.source_file_id \
         WHERE sf.project_id = ? AND s.text_category = ? AND s.target_text != ''",
    )
    .bind(project_id)
    .bind(category_key)
    .execute(&mut **tx)
    .await
    .map_err(|error| error.to_string())?;
    sqlx::query(
        "DELETE FROM segments WHERE text_category = ? \
         AND source_file_id IN (SELECT id FROM source_files WHERE project_id = ?)",
    )
    .bind(category_key)
    .bind(project_id)
    .execute(&mut **tx)
    .await
    .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_script_paths_stay_inside_js_plugins() {
        let js = Path::new("game").join("js");
        assert_eq!(
            plugin_script_path(&js, "nuun/NUUN_EnemyBook"),
            Some(js.join("plugins").join("nuun").join("NUUN_EnemyBook.js"))
        );
        // A dot in the name is part of it, not an extension to replace.
        assert_eq!(
            plugin_script_path(&js, "Foo.v2"),
            Some(js.join("plugins").join("Foo.v2.js"))
        );
        for bad in [
            "../../secret",
            "a/../../b",
            "/abs",
            "C:/x",
            r"..\..\x",
            "a//b",
            ".",
            "",
        ] {
            assert_eq!(plugin_script_path(&js, bad), None, "{bad}");
        }
    }
}
