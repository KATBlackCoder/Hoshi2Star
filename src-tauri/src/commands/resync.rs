//! Realign an MV/MZ project after a game update (lot R2b).
//!
//! The export refuses a segment whose original value is no longer its
//! recorded source (`IntegrityIssue::SourceChanged`). `resync_project` moves
//! each translation to its line in the new version of the game, adds the new
//! lines untranslated and keeps the translations of the lines that are gone
//! (`orphaned_segments` + translation memory), so the export can run again.
//!
//! The new version is read from the pristine snapshot, which already follows
//! a game update ([`originals::ensure_fresh_mv_mz_snapshot`]), with the same
//! extraction as `open_project` and the same candidates as enabling an
//! optional category. Pairing rules: [`resync::align`].

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::Serialize;
use serde_json::Value;
use sqlx::SqlitePool;
use tauri::Emitter;

use crate::commands::coverage::scan_project;
use crate::commands::project::{
    collect_json_file_entries, extract_mv_mz_file, insert_segments, insert_source_file,
    ExtractedFileSeg,
};
use crate::core::{manifest, originals, tm};
use crate::engines::detector::find_data_dir;
use crate::engines::filter::{classify_file_type, FileClass};
use crate::engines::mv_mz::coverage::CoverageReport;
use crate::engines::mv_mz::resync::{self, Alignment, Line};
use crate::state::AppState;

/// Gone translations listed in the summary (all are kept either way).
const GONE_SHOWN: usize = 20;

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResyncFile {
    pub file: String,
    /// Translated or not, lines now at another key.
    pub moved: usize,
    /// New lines (untranslated), including rewritten ones.
    pub added: usize,
    /// Lines no longer in the game, including rewritten ones.
    pub gone: usize,
    /// Of `gone`, lines whose key is still extracted with another source.
    pub changed_sources: usize,
    /// Of `gone`, translated lines kept in `orphaned_segments`.
    pub orphaned: usize,
    /// A data file the project did not have yet.
    pub new_file: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GoneTranslation {
    pub file: String,
    pub key: String,
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResyncSummary {
    /// `false` for a preview: nothing was written.
    pub applied: bool,
    /// Files with at least one change.
    pub files: Vec<ResyncFile>,
    pub moved: usize,
    pub added: usize,
    pub gone: usize,
    pub changed_sources: usize,
    pub orphaned: usize,
    /// First translations that no longer have a line.
    pub gone_translations: Vec<GoneTranslation>,
}

/// Preview (`apply = false`) or apply the realignment of `project_id`.
/// Applying emits `h2s://project/resynced` so open views reload.
#[tauri::command]
pub async fn resync_project<R: tauri::Runtime>(
    project_id: String,
    apply: bool,
    app: tauri::AppHandle<R>,
    state: tauri::State<'_, AppState>,
) -> Result<ResyncSummary, String> {
    let summary = resync(&state, &project_id, apply).await?;
    if summary.applied {
        let _ = app.emit("h2s://project/resynced", summary.clone());
    }
    Ok(summary)
}

/// A recorded segment.
struct OldRow {
    id: String,
    line: Line,
    target: String,
    status: String,
    qa_score: Option<i64>,
    category: Option<String>,
}

/// A line of the new version, with what its row stores besides the text.
struct NewRow {
    line: Line,
    scene_id: Option<String>,
    sequence_index: Option<i64>,
    speaker: Option<String>,
    branch_path: Option<String>,
    context_json: Option<String>,
    category: Option<String>,
}

impl NewRow {
    fn core(segment: ExtractedFileSeg) -> Self {
        Self {
            line: Line::new(segment.key, segment.source_text, segment.segment_kind),
            scene_id: segment.scene_id,
            sequence_index: segment.sequence_index,
            speaker: segment.speaker,
            branch_path: segment.branch_path,
            context_json: segment.context_json,
            category: None,
        }
    }
}

/// One file of the project and how its rows map onto the new version.
struct FilePlan {
    file_id: String,
    file_name: String,
    rel: String,
    old: Vec<OldRow>,
    new: Vec<NewRow>,
    alignment: Alignment,
}

/// A data file the game gained since extraction.
struct NewFile {
    file_name: String,
    file_path: String,
    file_type: String,
    segments: Vec<ExtractedFileSeg>,
}

async fn resync(state: &AppState, project_id: &str, apply: bool) -> Result<ResyncSummary, String> {
    let (engine, game_path, source_language, target_language): (String, String, String, String) =
        sqlx::query_as(
            "SELECT engine, game_path, COALESCE(source_language, 'ja'), \
                    COALESCE(target_language, 'en') FROM projects WHERE id = ?",
        )
        .bind(project_id)
        .fetch_one(&state.db)
        .await
        .map_err(|error| error.to_string())?;
    if engine != "mv_mz" {
        return Err(
            "Realignment after a game update is only available for RPG Maker MV/MZ.".into(),
        );
    }
    // Refreshes the snapshot (it follows the game update) and lists the
    // candidates of every optional category, as enabling them would.
    let (report, mut store) = scan_project(
        &state.db,
        &state.data_dir,
        state.plugin_profiles.clone(),
        project_id,
    )
    .await?
    .ok_or_else(|| "This project has no MV/MZ snapshot.".to_string())?;

    let files: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT id, file_name, file_path, file_type FROM source_files WHERE project_id = ?",
    )
    .bind(project_id)
    .fetch_all(&state.db)
    .await
    .map_err(|error| error.to_string())?;
    let enabled: HashSet<String> = sqlx::query_scalar(
        "SELECT DISTINCT s.text_category FROM segments s \
         JOIN source_files sf ON sf.id = s.source_file_id \
         WHERE sf.project_id = ? AND s.text_category IS NOT NULL",
    )
    .bind(project_id)
    .fetch_all(&state.db)
    .await
    .map_err(|error| error.to_string())?
    .into_iter()
    .collect();

    let mut plans = Vec::new();
    for (file_id, file_name, file_path, file_type) in &files {
        let core = match classify_file_type(file_type) {
            FileClass::Json => true,
            FileClass::PluginsJs => false,
            _ => continue,
        };
        let rel = originals::game_relative_path(&game_path, file_path, file_name);
        let mut new = Vec::new();
        if core {
            // A file the snapshot does not hold is left as it is.
            let Ok(bytes) = store.read(&rel) else {
                continue;
            };
            let json: Value =
                serde_json::from_slice(&bytes).map_err(|error| format!("{rel}: {error}"))?;
            new.extend(
                extract_mv_mz_file(file_name, &json)
                    .into_iter()
                    .map(NewRow::core),
            );
        }
        new.extend(category_rows(&report, &enabled, file_name));
        let old = recorded_rows(&state.db, file_id).await?;
        let alignment = align_by_category(&old, &new);
        plans.push(FilePlan {
            file_id: file_id.clone(),
            file_name: file_name.clone(),
            rel,
            old,
            new,
            alignment,
        });
    }
    let new_files = new_data_files(&game_path, &files)?;

    let summary = summarize(&plans, &new_files, apply);
    if !apply {
        return Ok(summary);
    }

    write(&state.db, project_id, &plans, &new_files).await?;

    // The translations of lines that are gone stay suggested (best effort,
    // like every translation-memory insert).
    let lang_pair = format!("{source_language}-{target_language}");
    for plan in &plans {
        for &o in &plan.alignment.gone {
            let row = &plan.old[o];
            if !row.target.is_empty() {
                let _ = tm::insert(
                    &row.line.source,
                    &row.target,
                    &engine,
                    &lang_pair,
                    &state.db,
                )
                .await;
            }
        }
    }
    // The snapshot is the version the project now follows.
    for plan in &plans {
        store.clear_drift(&plan.rel);
    }
    store.save().map_err(|error| error.to_string())?;
    if !new_files.is_empty() {
        originals::ensure_mv_mz_snapshot(&state.db, &state.data_dir, project_id).await?;
    }
    manifest::refresh_stats(&state.db, project_id).await;
    Ok(summary)
}

async fn recorded_rows(db: &SqlitePool, file_id: &str) -> Result<Vec<OldRow>, String> {
    type Row = (
        String,
        String,
        String,
        String,
        String,
        String,
        Option<i64>,
        Option<String>,
    );
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT id, json_key, source_text, segment_kind, target_text, status, qa_score, \
                text_category FROM segments WHERE source_file_id = ? ORDER BY rowid",
    )
    .bind(file_id)
    .fetch_all(db)
    .await
    .map_err(|error| error.to_string())?;
    Ok(rows
        .into_iter()
        .map(
            |(id, key, source, kind, target, status, qa_score, category)| OldRow {
                id,
                line: Line::new(key, source, kind),
                target,
                status,
                qa_score,
                category,
            },
        )
        .collect())
}

/// Candidates of the enabled optional categories found in `file_name`.
fn category_rows<'a>(
    report: &'a CoverageReport,
    enabled: &'a HashSet<String>,
    file_name: &'a str,
) -> impl Iterator<Item = NewRow> + 'a {
    report
        .categories
        .iter()
        .filter(|category| enabled.contains(&category.key))
        .flat_map(move |category| {
            category
                .candidates
                .iter()
                .filter(move |candidate| candidate.file_name == file_name)
                .map(move |candidate| NewRow {
                    line: Line::new(
                        candidate.key.clone(),
                        candidate.source.clone(),
                        candidate.kind.as_str(),
                    ),
                    scene_id: None,
                    sequence_index: None,
                    speaker: None,
                    branch_path: None,
                    context_json: Some(candidate.context_json.clone()),
                    category: Some(category.key.clone()),
                })
        })
}

/// [`resync::align`] run separately for core rows and for each category, so
/// a translation never changes category.
fn align_by_category(old: &[OldRow], new: &[NewRow]) -> Alignment {
    let mut groups: HashMap<Option<&str>, (Vec<usize>, Vec<usize>)> = HashMap::new();
    for (index, row) in old.iter().enumerate() {
        groups
            .entry(row.category.as_deref())
            .or_default()
            .0
            .push(index);
    }
    for (index, row) in new.iter().enumerate() {
        groups
            .entry(row.category.as_deref())
            .or_default()
            .1
            .push(index);
    }
    let mut alignment = Alignment::default();
    for (old_ids, new_ids) in groups.into_values() {
        let old_lines: Vec<Line> = old_ids.iter().map(|&o| old[o].line.clone()).collect();
        let new_lines: Vec<Line> = new_ids.iter().map(|&n| new[n].line.clone()).collect();
        let part = resync::align(&old_lines, &new_lines);
        alignment
            .paired
            .extend(part.paired.iter().map(|&(o, n)| (old_ids[o], new_ids[n])));
        alignment
            .added
            .extend(part.added.iter().map(|&n| new_ids[n]));
        alignment.gone.extend(part.gone.iter().map(|&o| old_ids[o]));
    }
    alignment.paired.sort_unstable();
    alignment.added.sort_unstable();
    alignment.gone.sort_unstable();
    alignment
}

/// Data files in the game folder that the project does not have.
fn new_data_files(
    game_path: &str,
    files: &[(String, String, String, String)],
) -> Result<Vec<NewFile>, String> {
    let Some(data_dir) = find_data_dir(Path::new(game_path)) else {
        return Ok(Vec::new());
    };
    let known: HashSet<&str> = files.iter().map(|(_, name, ..)| name.as_str()).collect();
    let mut found = Vec::new();
    for (file_name, path, file_type) in
        collect_json_file_entries(&data_dir).map_err(|error| error.to_string())?
    {
        if known.contains(file_name.as_str()) {
            continue;
        }
        // Never exported, so the disk is the original.
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(json) = serde_json::from_str::<Value>(&text) else {
            log::warn!("realignment skips invalid JSON {}", path.display());
            continue;
        };
        found.push(NewFile {
            segments: extract_mv_mz_file(&file_name, &json),
            file_name,
            file_path: path.to_string_lossy().into_owned(),
            file_type,
        });
    }
    Ok(found)
}

fn summarize(plans: &[FilePlan], new_files: &[NewFile], applied: bool) -> ResyncSummary {
    let mut summary = ResyncSummary {
        applied,
        ..ResyncSummary::default()
    };
    let old_lines = |plan: &FilePlan| {
        plan.old
            .iter()
            .map(|row| row.line.clone())
            .collect::<Vec<_>>()
    };
    let new_lines = |plan: &FilePlan| {
        plan.new
            .iter()
            .map(|row| row.line.clone())
            .collect::<Vec<_>>()
    };
    for plan in plans {
        let (old, new) = (old_lines(plan), new_lines(plan));
        if plan.alignment.is_unchanged(&old, &new) {
            continue;
        }
        let orphaned: Vec<&OldRow> = plan
            .alignment
            .gone
            .iter()
            .map(|&o| &plan.old[o])
            .filter(|row| !row.target.is_empty())
            .collect();
        let room = GONE_SHOWN.saturating_sub(summary.gone_translations.len());
        for row in orphaned.iter().take(room) {
            summary.gone_translations.push(GoneTranslation {
                file: plan.file_name.clone(),
                key: row.line.key.clone(),
                source: row.line.source.clone(),
                target: row.target.clone(),
            });
        }
        summary.files.push(ResyncFile {
            file: plan.file_name.clone(),
            moved: plan.alignment.moved(&old, &new).count(),
            added: plan.alignment.added.len(),
            gone: plan.alignment.gone.len(),
            changed_sources: plan.alignment.changed_sources(&old, &new),
            orphaned: orphaned.len(),
            new_file: false,
        });
    }
    for file in new_files {
        summary.files.push(ResyncFile {
            file: file.file_name.clone(),
            added: file.segments.len(),
            new_file: true,
            ..ResyncFile::default()
        });
    }
    for file in &summary.files {
        summary.moved += file.moved;
        summary.added += file.added;
        summary.gone += file.gone;
        summary.changed_sources += file.changed_sources;
        summary.orphaned += file.orphaned;
    }
    summary
}

/// Apply every plan in one transaction.
async fn write(
    db: &SqlitePool,
    project_id: &str,
    plans: &[FilePlan],
    new_files: &[NewFile],
) -> Result<(), String> {
    let error = |error: sqlx::Error| error.to_string();
    let mut tx = db.begin().await.map_err(error)?;
    let mut next_rowid: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(rowid), 0) + 1 FROM segments")
            .fetch_one(&mut *tx)
            .await
            .map_err(error)?;

    for plan in plans {
        let old_lines: Vec<Line> = plan.old.iter().map(|row| row.line.clone()).collect();
        let new_lines: Vec<Line> = plan.new.iter().map(|row| row.line.clone()).collect();
        if plan.alignment.is_unchanged(&old_lines, &new_lines) {
            continue;
        }

        // Moved keys go through a unique temporary value first: a key may be
        // taken over by another row of the same file (unique category index).
        for (o, _) in plan.alignment.moved(&old_lines, &new_lines) {
            let id = &plan.old[o].id;
            sqlx::query("UPDATE segments SET json_key = ? WHERE id = ?")
                .bind(format!("\u{1}resync/{id}"))
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(error)?;
        }
        for &o in &plan.alignment.gone {
            let row = &plan.old[o];
            if !row.target.is_empty() {
                sqlx::query(
                    "INSERT OR REPLACE INTO orphaned_segments \
                     (id, project_id, file_name, json_key, source_text, target_text, status, \
                      qa_score, segment_kind, text_category) \
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&row.id)
                .bind(project_id)
                .bind(&plan.file_name)
                .bind(&row.line.key)
                .bind(&row.line.source)
                .bind(&row.target)
                .bind(&row.status)
                .bind(row.qa_score)
                .bind(&row.line.kind)
                .bind(&row.category)
                .execute(&mut *tx)
                .await
                .map_err(error)?;
            }
            sqlx::query("DELETE FROM segments WHERE id = ?")
                .bind(&row.id)
                .execute(&mut *tx)
                .await
                .map_err(error)?;
        }

        // Rows in the new order: paired ones keep their id (translation,
        // status, terminology links), new ones get one.
        let paired: HashMap<usize, usize> =
            plan.alignment.paired.iter().map(|&(o, n)| (n, o)).collect();
        for (n, new) in plan.new.iter().enumerate() {
            let id = match paired.get(&n) {
                Some(&o) => {
                    let id = plan.old[o].id.clone();
                    sqlx::query(
                        "UPDATE segments SET json_key = ?, segment_kind = ?, scene_id = ?, \
                         sequence_index = ?, speaker = ?, branch_path = ?, context_json = ?, \
                         updated_at = datetime('now') WHERE id = ?",
                    )
                    .bind(&new.line.key)
                    .bind(&new.line.kind)
                    .bind(&new.scene_id)
                    .bind(new.sequence_index)
                    .bind(&new.speaker)
                    .bind(&new.branch_path)
                    .bind(&new.context_json)
                    .bind(&id)
                    .execute(&mut *tx)
                    .await
                    .map_err(error)?;
                    id
                }
                None => {
                    let id = uuid::Uuid::new_v4().to_string();
                    sqlx::query(
                        "INSERT INTO segments \
                         (id, source_file_id, json_key, source_text, segment_kind, scene_id, \
                          sequence_index, speaker, branch_path, context_json, text_category) \
                         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    )
                    .bind(&id)
                    .bind(&plan.file_id)
                    .bind(&new.line.key)
                    .bind(&new.line.source)
                    .bind(&new.line.kind)
                    .bind(&new.scene_id)
                    .bind(new.sequence_index)
                    .bind(&new.speaker)
                    .bind(&new.branch_path)
                    .bind(&new.context_json)
                    .bind(&new.category)
                    .execute(&mut *tx)
                    .await
                    .map_err(error)?;
                    id
                }
            };
            // `get_segments` lists a file by rowid: follow the new order.
            sqlx::query("UPDATE segments SET rowid = ? WHERE id = ?")
                .bind(next_rowid)
                .bind(&id)
                .execute(&mut *tx)
                .await
                .map_err(error)?;
            next_rowid += 1;
        }
    }

    for file in new_files {
        let file_id = uuid::Uuid::new_v4().to_string();
        insert_source_file(
            &mut tx,
            &file_id,
            project_id,
            &file.file_name,
            &file.file_path,
            &file.file_type,
        )
        .await
        .map_err(error)?;
        insert_segments(&mut tx, &file_id, &file.segments)
            .await
            .map_err(error)?;
    }
    tx.commit().await.map_err(error)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn old(id: &str, key: &str, source: &str, category: Option<&str>) -> OldRow {
        OldRow {
            id: id.into(),
            line: Line::new(key, source, "dialogue"),
            target: format!("EN {source}"),
            status: "translated".into(),
            qa_score: None,
            category: category.map(Into::into),
        }
    }

    fn new(key: &str, source: &str, category: Option<&str>) -> NewRow {
        NewRow {
            line: Line::new(key, source, "dialogue"),
            scene_id: None,
            sequence_index: None,
            speaker: None,
            branch_path: None,
            context_json: None,
            category: category.map(Into::into),
        }
    }

    #[test]
    fn a_translation_never_changes_category() {
        let rows = [
            old("a", "/events/1/pages/0/list/0/parameters/0", "看板", None),
            old(
                "b",
                "/events/1/pages/0/list/1/parameters/3/text",
                "看板",
                Some("plugin"),
            ),
        ];
        let lines = [
            new("/events/1/pages/0/list/1/parameters/0", "看板", None),
            new(
                "/events/1/pages/0/list/2/parameters/3/text",
                "看板",
                Some("plugin"),
            ),
        ];
        let alignment = align_by_category(&rows, &lines);
        assert_eq!(alignment.paired, vec![(0, 0), (1, 1)]);
    }
}
