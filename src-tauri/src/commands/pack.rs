//! Tauri commands for the `.h2s` exchange pack — share a project's
//! translation state so another Hoshi2Star user can resume it on their own
//! copy of the game.
//!
//! Three commands:
//! - [`export_h2s_pack`]     — write the pack (segments + glossary ± TM)
//! - [`preview_h2s_import`]  — mandatory dry-run: validate + classify, zero writes
//! - [`apply_h2s_import`]    — auto-backup, then apply in ONE SQLite transaction
//!
//! Segments are matched across machines by `(file_name, json_key)` with an
//! ordinal fallback for duplicate keys (nth occurrence ↔ nth occurrence, both
//! sides ordered by extraction order) — never by segment UUIDs, which are
//! local to each database.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{Emitter, Manager};

use crate::core::h2s_pack::{
    self, H2sPack, H2sPackError, PackFileStat, PackGlossaryTerm, PackManifest, PackSegment,
    PackTmEntry, FORMAT_VERSION,
};
use crate::core::{glossary, manifest, tm};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// One local segment, loaded for matching. Ordered by extraction order
/// (`rowid`) inside its `(file_name, json_key)` bucket.
struct LocalSeg {
    id: String,
    source_text: String,
    target_text: String,
    status: String,
}

/// Classification of one pack segment against the local project (spec: the
/// dry-run classes shown to the user before anything is written).
#[derive(Clone, Copy, PartialEq)]
enum SegClass {
    /// Local target is empty — filling it is safe under every policy.
    Applicable,
    /// Local target and status already equal the pack's — nothing to do.
    Identical,
    /// Local target differs and is not empty — gated by the chosen policy.
    Conflict,
    /// Key found but the Japanese source differs (game updated) — never
    /// applied silently; opt-in, forced to `needs_review`.
    SourceChanged,
    /// `(file_name, json_key)` not found locally (or occurrences exhausted).
    Orphan,
}

/// Build the full pack of a project's current state. Used by the export
/// command AND by the automatic pre-import backup (same format = the backup
/// can be re-imported with the "overwrite all" policy to undo an import).
async fn build_project_pack(
    db: &sqlx::SqlitePool,
    project_id: &str,
    lang_pair: &str,
    include_tm: bool,
) -> Result<H2sPack, String> {
    let (game_title, engine): (String, String) =
        sqlx::query_as("SELECT name, engine FROM projects WHERE id = ?")
            .bind(project_id)
            .fetch_one(db)
            .await
            .map_err(|e| e.to_string())?;

    let file_stats: Vec<(String, String, i64, i64)> = sqlx::query_as(
        "SELECT sf.file_name, sf.file_type, COUNT(s.id), \
                COALESCE(SUM(CASE WHEN s.target_text != '' THEN 1 ELSE 0 END), 0) \
         FROM source_files sf LEFT JOIN segments s ON s.source_file_id = sf.id \
         WHERE sf.project_id = ? GROUP BY sf.id ORDER BY sf.file_name ASC",
    )
    .bind(project_id)
    .fetch_all(db)
    .await
    .map_err(|e| e.to_string())?;

    // Deterministic order (file, then extraction order) so duplicate keys
    // keep a stable ordinal on both machines.
    let segments: Vec<(String, String, String, String, String, Option<i64>)> = sqlx::query_as(
        "SELECT sf.file_name, s.json_key, s.source_text, s.target_text, s.status, s.qa_score \
         FROM segments s JOIN source_files sf ON s.source_file_id = sf.id \
         WHERE sf.project_id = ? AND s.target_text != '' \
         ORDER BY sf.file_name ASC, s.rowid ASC",
    )
    .bind(project_id)
    .fetch_all(db)
    .await
    .map_err(|e| e.to_string())?;

    let glossary_terms = glossary::list_for_project(db, project_id, lang_pair)
        .await
        .map_err(|e| e.to_string())?;

    let tm_entries: Vec<(String, String, String, f64)> = if include_tm {
        sqlx::query_as(
            "SELECT source_text, target_text, engine, confidence \
             FROM tm_entries WHERE lang_pair = ? AND engine = ? ORDER BY created_at ASC",
        )
        .bind(lang_pair)
        .bind(&engine)
        .fetch_all(db)
        .await
        .map_err(|e| e.to_string())?
    } else {
        vec![]
    };

    Ok(H2sPack {
        manifest: PackManifest {
            format_version: FORMAT_VERSION,
            app_version: manifest::VERSION.to_string(),
            engine,
            game_title,
            lang_pair: lang_pair.to_string(),
            created_at: crate::utils::time::now_iso8601(),
            sender_project_id: project_id.to_string(),
            files: file_stats
                .into_iter()
                .map(|(file_name, file_type, total, translated)| PackFileStat {
                    file_name,
                    file_type,
                    segment_count: total as u32,
                    translated_count: translated as u32,
                })
                .collect(),
        },
        segments: segments
            .into_iter()
            .map(
                |(file_name, json_key, source_text, target_text, status, qa_score)| PackSegment {
                    file_name,
                    json_key,
                    source_text,
                    target_text,
                    status,
                    qa_score,
                },
            )
            .collect(),
        glossary: glossary_terms
            .into_iter()
            .map(|t| PackGlossaryTerm {
                source_text: t.source_text,
                target_text: t.target_text,
                domain: t.domain,
            })
            .collect(),
        tm: tm_entries
            .into_iter()
            .map(
                |(source_text, target_text, engine, confidence)| PackTmEntry {
                    source_text,
                    target_text,
                    engine,
                    confidence,
                },
            )
            .collect(),
    })
}

/// Load every local segment of the project, bucketed by `(file_name, json_key)`
/// in extraction order — the matching index for classification.
async fn load_local_index(
    db: &sqlx::SqlitePool,
    project_id: &str,
) -> Result<HashMap<(String, String), Vec<LocalSeg>>, String> {
    let rows: Vec<(String, String, String, String, String, String)> = sqlx::query_as(
        "SELECT sf.file_name, s.json_key, s.id, s.source_text, s.target_text, s.status \
         FROM segments s JOIN source_files sf ON s.source_file_id = sf.id \
         WHERE sf.project_id = ? ORDER BY sf.file_name ASC, s.rowid ASC",
    )
    .bind(project_id)
    .fetch_all(db)
    .await
    .map_err(|e| e.to_string())?;

    let mut index: HashMap<(String, String), Vec<LocalSeg>> = HashMap::new();
    for (file_name, json_key, id, source_text, target_text, status) in rows {
        index
            .entry((file_name, json_key))
            .or_default()
            .push(LocalSeg {
                id,
                source_text,
                target_text,
                status,
            });
    }
    Ok(index)
}

/// Classify every pack segment against the local index (ordinal matching for
/// duplicate keys: pack order is deterministic, cursors consume local
/// occurrences in the same order).
fn classify<'a>(
    pack: &'a H2sPack,
    index: &'a HashMap<(String, String), Vec<LocalSeg>>,
) -> Vec<(&'a PackSegment, SegClass, Option<&'a LocalSeg>)> {
    let mut cursors: HashMap<(&str, &str), usize> = HashMap::new();
    pack.segments
        .iter()
        .map(|seg| {
            let key = (seg.file_name.as_str(), seg.json_key.as_str());
            let bucket = index.get(&(seg.file_name.clone(), seg.json_key.clone()));
            let cursor = cursors.entry(key).or_insert(0);
            let local = bucket.and_then(|b| b.get(*cursor));
            let Some(local) = local else {
                return (seg, SegClass::Orphan, None);
            };
            *cursor += 1;
            let class = if local.source_text != seg.source_text {
                SegClass::SourceChanged
            } else if local.target_text == seg.target_text && local.status == seg.status {
                SegClass::Identical
            } else if local.target_text.is_empty() {
                SegClass::Applicable
            } else {
                SegClass::Conflict
            };
            (seg, class, Some(local))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackExportSummary {
    pub segment_count: u32,
    pub file_count: u32,
    pub glossary_count: u32,
    pub tm_count: u32,
}

/// Export the project's translation state as a `.h2s` pack at `output_path`.
#[tauri::command]
pub async fn export_h2s_pack(
    project_id: String,
    output_path: String,
    include_tm: bool,
    lang_pair: String,
    state: tauri::State<'_, AppState>,
) -> Result<PackExportSummary, String> {
    let pack = build_project_pack(&state.db, &project_id, &lang_pair, include_tm).await?;
    h2s_pack::write_pack(Path::new(&output_path), &pack).map_err(|e| e.to_string())?;
    Ok(PackExportSummary {
        segment_count: pack.segments.len() as u32,
        file_count: pack.manifest.files.len() as u32,
        glossary_count: pack.glossary.len() as u32,
        tm_count: pack.tm.len() as u32,
    })
}

// ---------------------------------------------------------------------------
// Preview (dry-run)
// ---------------------------------------------------------------------------

/// Hard reason why a pack cannot be imported — typed so the frontend can show
/// a proper localized message for each misuse case.
#[derive(Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ImportBlocker {
    /// Not a ZIP or no manifest.json (e.g. the player-facing export ZIP).
    NotAPack,
    /// Pack written by a newer format — the receiver must update the app.
    UnsupportedVersion { version: u32 },
    EngineMismatch {
        pack_engine: String,
        project_engine: String,
    },
    LangPairMismatch {
        pack_lang_pair: String,
        project_lang_pair: String,
    },
    /// Structurally invalid pack (truncated transfer, bad JSON, oversized…).
    InvalidPack { detail: String },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    /// `Some` = import impossible, counts are zero. `None` = ready to apply.
    pub blocker: Option<ImportBlocker>,
    pub pack_game_title: String,
    pub pack_app_version: String,
    pub pack_created_at: String,
    /// Different game titles are a warning, never a blocker (renamed folders).
    pub title_mismatch: bool,
    pub applicable: u32,
    pub identical: u32,
    pub conflicts: u32,
    pub source_changed: u32,
    pub orphans: u32,
    pub glossary_count: u32,
    pub tm_count: u32,
}

impl ImportPreview {
    fn blocked(blocker: ImportBlocker) -> Self {
        Self {
            blocker: Some(blocker),
            pack_game_title: String::new(),
            pack_app_version: String::new(),
            pack_created_at: String::new(),
            title_mismatch: false,
            applicable: 0,
            identical: 0,
            conflicts: 0,
            source_changed: 0,
            orphans: 0,
            glossary_count: 0,
            tm_count: 0,
        }
    }
}

/// Map a pack read error to a user-facing blocker; real I/O failures stay
/// hard errors (`Err`) — they are environment problems, not pack problems.
fn blocker_from_error(err: H2sPackError) -> Result<ImportBlocker, String> {
    match err {
        H2sPackError::NotAPack => Ok(ImportBlocker::NotAPack),
        H2sPackError::UnsupportedVersion(v) => Ok(ImportBlocker::UnsupportedVersion { version: v }),
        H2sPackError::Io(e) => Err(e.to_string()),
        other => Ok(ImportBlocker::InvalidPack {
            detail: other.to_string(),
        }),
    }
}

/// Validate `pack` against the target project. Returns the project's game
/// title alongside so `apply` can reuse the check.
async fn validate_pack_against_project(
    db: &sqlx::SqlitePool,
    project_id: &str,
    lang_pair: &str,
    pack: &H2sPack,
) -> Result<(Option<ImportBlocker>, String), String> {
    let (project_title, project_engine): (String, String) =
        sqlx::query_as("SELECT name, engine FROM projects WHERE id = ?")
            .bind(project_id)
            .fetch_one(db)
            .await
            .map_err(|e| e.to_string())?;

    let blocker = if pack.manifest.engine != project_engine {
        Some(ImportBlocker::EngineMismatch {
            pack_engine: pack.manifest.engine.clone(),
            project_engine,
        })
    } else if pack.manifest.lang_pair != lang_pair {
        Some(ImportBlocker::LangPairMismatch {
            pack_lang_pair: pack.manifest.lang_pair.clone(),
            project_lang_pair: lang_pair.to_string(),
        })
    } else {
        None
    };
    Ok((blocker, project_title))
}

/// Dry-run: validate the pack and classify every segment. Writes NOTHING.
#[tauri::command]
pub async fn preview_h2s_import(
    project_id: String,
    pack_path: String,
    lang_pair: String,
    state: tauri::State<'_, AppState>,
) -> Result<ImportPreview, String> {
    let pack = match h2s_pack::read_pack(Path::new(&pack_path)) {
        Ok(p) => p,
        Err(e) => return Ok(ImportPreview::blocked(blocker_from_error(e)?)),
    };

    let (blocker, project_title) =
        validate_pack_against_project(&state.db, &project_id, &lang_pair, &pack).await?;
    if let Some(blocker) = blocker {
        return Ok(ImportPreview::blocked(blocker));
    }

    let index = load_local_index(&state.db, &project_id).await?;
    let classified = classify(&pack, &index);

    let count = |c: SegClass| classified.iter().filter(|(_, cl, _)| *cl == c).count() as u32;

    Ok(ImportPreview {
        blocker: None,
        title_mismatch: pack.manifest.game_title != project_title,
        pack_game_title: pack.manifest.game_title.clone(),
        pack_app_version: pack.manifest.app_version.clone(),
        pack_created_at: pack.manifest.created_at.clone(),
        applicable: count(SegClass::Applicable),
        identical: count(SegClass::Identical),
        conflicts: count(SegClass::Conflict),
        source_changed: count(SegClass::SourceChanged),
        orphans: count(SegClass::Orphan),
        glossary_count: pack.glossary.len() as u32,
        tm_count: pack.tm.len() as u32,
    })
}

// ---------------------------------------------------------------------------
// Apply
// ---------------------------------------------------------------------------

/// Collision policy chosen by the user in the import wizard.
enum Policy {
    /// Only fill segments whose local target is empty (default, safest).
    Fill,
    /// Overwrite everything except locally `reviewed` segments.
    OverwriteExceptReviewed,
    /// Overwrite everything (strong confirmation required in the UI).
    OverwriteAll,
}

impl Policy {
    fn parse(s: &str) -> Result<Self, String> {
        match s {
            "fill" => Ok(Self::Fill),
            "overwrite_except_reviewed" => Ok(Self::OverwriteExceptReviewed),
            "overwrite_all" => Ok(Self::OverwriteAll),
            other => Err(format!("unknown import policy: {other:?}")),
        }
    }

    /// May this policy overwrite the given non-empty local segment?
    fn allows_overwrite(&self, local: &LocalSeg) -> bool {
        match self {
            Self::Fill => false,
            Self::OverwriteExceptReviewed => local.status != "reviewed",
            Self::OverwriteAll => true,
        }
    }
}

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub applied: u32,
    pub applied_source_changed: u32,
    pub skipped_conflicts: u32,
    pub skipped_source_changed: u32,
    pub identical: u32,
    pub orphans: u32,
    pub glossary_added: u32,
    pub glossary_conflicts: u32,
    pub tm_added: u32,
    pub backup_path: String,
}

/// Apply a pack to the project: automatic backup first, then every write in
/// ONE SQLite transaction (all or nothing).
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn apply_h2s_import(
    project_id: String,
    pack_path: String,
    policy: String,
    apply_source_changed: bool,
    import_glossary: bool,
    import_tm: bool,
    lang_pair: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<ImportReport, String> {
    let policy = Policy::parse(&policy)?;

    // Defensive re-validation — the UI always previews first, but apply must
    // hold on its own (spec: no write path without the same checks).
    let pack = h2s_pack::read_pack(Path::new(&pack_path)).map_err(|e| e.to_string())?;
    let (blocker, _) =
        validate_pack_against_project(&state.db, &project_id, &lang_pair, &pack).await?;
    if blocker.is_some() {
        return Err("pack incompatible with this project (engine or language pair)".to_string());
    }

    // Automatic backup BEFORE any write: the current state, as a normal .h2s
    // pack — undo = re-import it with the "overwrite all" policy.
    let backup_path = write_backup(&app, &state.db, &project_id, &lang_pair).await?;

    let index = load_local_index(&state.db, &project_id).await?;
    let classified = classify(&pack, &index);

    let mut report = ImportReport {
        backup_path: backup_path.to_string_lossy().into_owned(),
        ..Default::default()
    };

    let mut tx = state.db.begin().await.map_err(|e| e.to_string())?;

    apply_segments(
        &mut tx,
        &classified,
        &policy,
        apply_source_changed,
        &mut report,
    )
    .await?;

    if import_glossary {
        let (added, conflicts) =
            import_glossary_terms(&mut tx, &project_id, &lang_pair, &pack.glossary).await?;
        report.glossary_added = added;
        report.glossary_conflicts = conflicts;
    }

    if import_tm {
        report.tm_added = import_tm_entries(&mut tx, &lang_pair, &pack.tm).await?;
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    manifest::refresh_stats(&state.db, &project_id).await;
    let _ = app.emit("h2s://project/import-done", report.clone());

    Ok(report)
}

/// Apply the classified segments under the chosen policy. The single write
/// path of the import — shared between the command and the tests.
async fn apply_segments(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    classified: &[(&PackSegment, SegClass, Option<&LocalSeg>)],
    policy: &Policy,
    apply_source_changed: bool,
    report: &mut ImportReport,
) -> Result<(), String> {
    for (seg, class, local) in classified {
        match class {
            SegClass::Identical => report.identical += 1,
            SegClass::Orphan => report.orphans += 1,
            SegClass::Applicable => {
                let local = local.expect("applicable implies a local match");
                write_segment(tx, &local.id, &seg.target_text, &seg.status, seg.qa_score).await?;
                report.applied += 1;
            }
            SegClass::Conflict => {
                let local = local.expect("conflict implies a local match");
                if policy.allows_overwrite(local) {
                    write_segment(tx, &local.id, &seg.target_text, &seg.status, seg.qa_score)
                        .await?;
                    report.applied += 1;
                } else {
                    report.skipped_conflicts += 1;
                }
            }
            SegClass::SourceChanged => {
                let local = local.expect("source-changed implies a local match");
                let allowed = local.target_text.is_empty() || policy.allows_overwrite(local);
                if apply_source_changed && allowed {
                    // Forced to needs_review with no QA score: the source
                    // differs, so the translation must be re-checked.
                    write_segment(tx, &local.id, &seg.target_text, "needs_review", None).await?;
                    report.applied_source_changed += 1;
                } else {
                    report.skipped_source_changed += 1;
                }
            }
        }
    }
    Ok(())
}

async fn write_segment(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    id: &str,
    target_text: &str,
    status: &str,
    qa_score: Option<i64>,
) -> Result<(), String> {
    sqlx::query(
        "UPDATE segments SET target_text = ?, status = ?, qa_score = ?, \
         updated_at = datetime('now') WHERE id = ?",
    )
    .bind(target_text)
    .bind(status)
    .bind(qa_score)
    .bind(id)
    .execute(&mut **tx)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Import glossary terms as PROJECT terms (never global — the receiver's
/// global glossary is theirs). Existing identical pairs are skipped; same
/// source with a different target keeps the local term and is reported.
async fn import_glossary_terms(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    project_id: &str,
    lang_pair: &str,
    terms: &[PackGlossaryTerm],
) -> Result<(u32, u32), String> {
    let existing: Vec<(String, String)> = sqlx::query_as(
        "SELECT source_text, target_text FROM glossary_terms \
         WHERE lang_pair = ? AND (project_id = ? OR project_id IS NULL)",
    )
    .bind(lang_pair)
    .bind(project_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| e.to_string())?;

    let mut pairs: HashSet<(String, String)> = existing.iter().cloned().collect();
    let mut sources: HashSet<String> = existing.into_iter().map(|(s, _)| s).collect();

    let (mut added, mut conflicts) = (0u32, 0u32);
    for term in terms {
        let pair = (term.source_text.clone(), term.target_text.clone());
        if pairs.contains(&pair) {
            continue; // already known, silently skip
        }
        if sources.contains(&term.source_text) {
            conflicts += 1; // local term wins, reported
            continue;
        }
        sqlx::query(
            "INSERT INTO glossary_terms \
                 (id, source_text, target_text, lang_pair, domain, project_id, auto_generated) \
             VALUES (?, ?, ?, ?, ?, ?, 0)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&term.source_text)
        .bind(&term.target_text)
        .bind(lang_pair)
        .bind(&term.domain)
        .bind(project_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| e.to_string())?;
        sources.insert(term.source_text.clone());
        pairs.insert(pair);
        added += 1;
    }
    Ok((added, conflicts))
}

/// Merge pack TM entries into the global TM. `DO NOTHING` on conflict — the
/// receiver's own TM always wins over an imported pack.
async fn import_tm_entries(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    lang_pair: &str,
    entries: &[PackTmEntry],
) -> Result<u32, String> {
    let mut added = 0u32;
    for entry in entries {
        let result = sqlx::query(
            "INSERT INTO tm_entries \
                 (id, source_hash, source_text, target_text, engine, lang_pair, confidence) \
             VALUES (?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(source_hash, lang_pair) DO NOTHING",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(tm::hash_source(&entry.source_text))
        .bind(&entry.source_text)
        .bind(&entry.target_text)
        .bind(&entry.engine)
        .bind(lang_pair)
        .bind(entry.confidence)
        .execute(&mut **tx)
        .await
        .map_err(|e| e.to_string())?;
        added += result.rows_affected() as u32;
    }
    Ok(added)
}

/// Write the pre-import backup pack under `<app_data>/backups/`.
async fn write_backup(
    app: &tauri::AppHandle,
    db: &sqlx::SqlitePool,
    project_id: &str,
    lang_pair: &str,
) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("backups");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    // "2026-07-07T12:59:59Z" → "20260707-125959" (filename-safe)
    let stamp: String = crate::utils::time::now_iso8601()
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect();
    let (date, time) = stamp.split_at(8);
    let path = dir.join(format!("backup-avant-import-{date}-{time}.h2s"));

    let backup = build_project_pack(db, project_id, lang_pair, false).await?;
    h2s_pack::write_pack(&path, &backup).map_err(|e| e.to_string())?;
    Ok(path)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Fresh migrated pool. The temp file is returned too: dropping it would
    /// delete the DB under the pool's next connections.
    async fn test_pool() -> (sqlx::SqlitePool, tempfile::NamedTempFile) {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let pool = crate::db::pool::init(tmp.path().to_str().unwrap())
            .await
            .unwrap();
        (pool, tmp)
    }

    /// Seed one project/file with segments covering every dry-run class:
    /// - s1: local target empty                        → Applicable
    /// - s2: local == pack (target + status)           → Identical
    /// - s3: local target differs, status translated   → Conflict
    /// - s4: local target differs, status reviewed     → Conflict (reviewed)
    /// - s5: local source differs from the pack's      → SourceChanged
    async fn seed(pool: &sqlx::SqlitePool) {
        sqlx::query(
            "INSERT INTO projects (id, name, engine, game_path) \
             VALUES ('p1', 'Test Game', 'wolf', '/tmp/nowhere')",
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
             VALUES ('f1', 'p1', 'CommonEvent.dat', '/x/CommonEvent.dat', 'wolf_dat')",
        )
        .execute(pool)
        .await
        .unwrap();
        for (id, key, source, target, status) in [
            ("s1", "/1", "あ", "", "untranslated"),
            ("s2", "/2", "い", "Same", "translated"),
            ("s3", "/3", "う", "Mine", "translated"),
            ("s4", "/4", "え", "Reviewed by me", "reviewed"),
            ("s5", "/5", "お(v2)", "Old translation", "translated"),
        ] {
            sqlx::query(
                "INSERT INTO segments (id, source_file_id, json_key, source_text, target_text, status) \
                 VALUES (?, 'f1', ?, ?, ?, ?)",
            )
            .bind(id)
            .bind(key)
            .bind(source)
            .bind(target)
            .bind(status)
            .execute(pool)
            .await
            .unwrap();
        }
    }

    fn pack_seg(key: &str, source: &str, target: &str, status: &str) -> PackSegment {
        PackSegment {
            file_name: "CommonEvent.dat".into(),
            json_key: key.into(),
            source_text: source.into(),
            target_text: target.into(),
            status: status.into(),
            qa_score: Some(100),
        }
    }

    fn sample_pack() -> H2sPack {
        H2sPack {
            manifest: PackManifest {
                format_version: FORMAT_VERSION,
                app_version: "0.4.9".into(),
                engine: "wolf".into(),
                game_title: "Test Game".into(),
                lang_pair: "ja-en".into(),
                created_at: "2026-07-07T00:00:00Z".into(),
                sender_project_id: "sender".into(),
                files: vec![],
            },
            segments: vec![
                pack_seg("/1", "あ", "Fill me", "translated"),
                pack_seg("/2", "い", "Same", "translated"),
                pack_seg("/3", "う", "Theirs", "reviewed"),
                pack_seg("/4", "え", "Theirs too", "translated"),
                pack_seg("/5", "お(v3)", "New source translation", "translated"),
                pack_seg("/999", "か", "Orphan", "translated"),
            ],
            glossary: vec![],
            tm: vec![],
        }
    }

    fn count_class(
        classified: &[(&PackSegment, SegClass, Option<&LocalSeg>)],
        class: SegClass,
    ) -> usize {
        classified.iter().filter(|(_, c, _)| *c == class).count()
    }

    #[tokio::test]
    async fn test_classify_all_classes() {
        let (pool, _tmp) = test_pool().await;
        seed(&pool).await;
        let pack = sample_pack();
        let index = load_local_index(&pool, "p1").await.unwrap();

        let classified = classify(&pack, &index);

        assert_eq!(count_class(&classified, SegClass::Applicable), 1);
        assert_eq!(count_class(&classified, SegClass::Identical), 1);
        assert_eq!(count_class(&classified, SegClass::Conflict), 2);
        assert_eq!(count_class(&classified, SegClass::SourceChanged), 1);
        assert_eq!(count_class(&classified, SegClass::Orphan), 1);
    }

    #[tokio::test]
    async fn test_ordinal_matching_for_duplicate_keys() {
        let (pool, _tmp) = test_pool().await;
        seed(&pool).await;
        // Two extra local segments sharing the SAME (file, key) — no UNIQUE
        // constraint guarantees this can't happen.
        for (id, target) in [("d1", ""), ("d2", "")] {
            sqlx::query(
                "INSERT INTO segments (id, source_file_id, json_key, source_text, target_text, status) \
                 VALUES (?, 'f1', '/dup', '同じ', ?, 'untranslated')",
            )
            .bind(id)
            .bind(target)
            .execute(&pool)
            .await
            .unwrap();
        }
        let mut pack = sample_pack();
        pack.segments = vec![
            pack_seg("/dup", "同じ", "First", "translated"),
            pack_seg("/dup", "同じ", "Second", "translated"),
            pack_seg("/dup", "同じ", "Third — exhausted", "translated"),
        ];
        let index = load_local_index(&pool, "p1").await.unwrap();

        let classified = classify(&pack, &index);

        // 1st and 2nd occurrences match d1/d2 in rowid order; 3rd is orphan.
        assert_eq!(count_class(&classified, SegClass::Applicable), 2);
        assert_eq!(count_class(&classified, SegClass::Orphan), 1);
        let locals: Vec<&str> = classified
            .iter()
            .filter_map(|(_, _, l)| l.map(|l| l.id.as_str()))
            .collect();
        assert_eq!(locals, vec!["d1", "d2"]);
    }

    async fn run_apply(
        pool: &sqlx::SqlitePool,
        policy: Policy,
        apply_source_changed: bool,
    ) -> ImportReport {
        let pack = sample_pack();
        let index = load_local_index(pool, "p1").await.unwrap();
        let classified = classify(&pack, &index);
        let mut report = ImportReport::default();
        let mut tx = pool.begin().await.unwrap();
        apply_segments(
            &mut tx,
            &classified,
            &policy,
            apply_source_changed,
            &mut report,
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        report
    }

    async fn segment_row(pool: &sqlx::SqlitePool, id: &str) -> (String, String) {
        sqlx::query_as("SELECT target_text, status FROM segments WHERE id = ?")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_apply_policy_fill_only_writes_empty_targets() {
        let (pool, _tmp) = test_pool().await;
        seed(&pool).await;

        let report = run_apply(&pool, Policy::Fill, false).await;

        assert_eq!(report.applied, 1);
        assert_eq!(report.skipped_conflicts, 2);
        assert_eq!(report.skipped_source_changed, 1);
        assert_eq!(report.identical, 1);
        assert_eq!(report.orphans, 1);
        assert_eq!(
            segment_row(&pool, "s1").await,
            ("Fill me".into(), "translated".into())
        );
        // Conflicts untouched under fill.
        assert_eq!(
            segment_row(&pool, "s3").await,
            ("Mine".into(), "translated".into())
        );
        assert_eq!(
            segment_row(&pool, "s4").await,
            ("Reviewed by me".into(), "reviewed".into())
        );
    }

    #[tokio::test]
    async fn test_apply_policy_overwrite_except_reviewed() {
        let (pool, _tmp) = test_pool().await;
        seed(&pool).await;

        let report = run_apply(&pool, Policy::OverwriteExceptReviewed, false).await;

        // s1 filled + s3 overwritten; s4 (reviewed) protected.
        assert_eq!(report.applied, 2);
        assert_eq!(report.skipped_conflicts, 1);
        assert_eq!(
            segment_row(&pool, "s3").await,
            ("Theirs".into(), "reviewed".into())
        );
        assert_eq!(
            segment_row(&pool, "s4").await,
            ("Reviewed by me".into(), "reviewed".into())
        );
    }

    #[tokio::test]
    async fn test_apply_policy_overwrite_all_and_source_changed() {
        let (pool, _tmp) = test_pool().await;
        seed(&pool).await;

        let report = run_apply(&pool, Policy::OverwriteAll, true).await;

        assert_eq!(report.applied, 3);
        assert_eq!(report.applied_source_changed, 1);
        assert_eq!(report.skipped_conflicts, 0);
        // Source-changed lands as needs_review, never the pack status.
        assert_eq!(
            segment_row(&pool, "s5").await,
            ("New source translation".into(), "needs_review".into())
        );
    }

    #[tokio::test]
    async fn test_apply_fill_is_idempotent() {
        let (pool, _tmp) = test_pool().await;
        seed(&pool).await;

        run_apply(&pool, Policy::Fill, false).await;
        let second = run_apply(&pool, Policy::Fill, false).await;

        // s1 now holds exactly the pack content → Identical on the 2nd pass.
        assert_eq!(second.applied, 0);
        assert_eq!(second.identical, 2);
    }

    #[tokio::test]
    async fn test_glossary_import_dedup_and_conflicts() {
        let (pool, _tmp) = test_pool().await;
        seed(&pool).await;
        // Existing GLOBAL term: same source as an incoming one, other target.
        sqlx::query(
            "INSERT INTO glossary_terms (id, source_text, target_text, lang_pair, domain, project_id) \
             VALUES ('g1', '勇者', 'Brave', 'ja-en', '', NULL)",
        )
        .execute(&pool)
        .await
        .unwrap();

        let terms = vec![
            PackGlossaryTerm {
                source_text: "勇者".into(),
                target_text: "Hero".into(),
                domain: "characters".into(),
            },
            PackGlossaryTerm {
                source_text: "魔王".into(),
                target_text: "Demon Lord".into(),
                domain: "characters".into(),
            },
        ];
        let mut tx = pool.begin().await.unwrap();
        let (added, conflicts) = import_glossary_terms(&mut tx, "p1", "ja-en", &terms)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        assert_eq!(added, 1, "only the new source is added");
        assert_eq!(
            conflicts, 1,
            "existing source with a different target is kept"
        );
        let (target, project_id): (String, Option<String>) = sqlx::query_as(
            "SELECT target_text, project_id FROM glossary_terms WHERE source_text = '魔王'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(target, "Demon Lord");
        assert_eq!(
            project_id.as_deref(),
            Some("p1"),
            "imported terms are PROJECT terms, never global"
        );
    }

    #[tokio::test]
    async fn test_tm_import_never_overwrites_local_entries() {
        let (pool, _tmp) = test_pool().await;
        seed(&pool).await;
        crate::core::tm::insert("こんにちは", "My own translation", "wolf", "ja-en", &pool)
            .await
            .unwrap();

        let entries = vec![
            PackTmEntry {
                source_text: "こんにちは".into(),
                target_text: "Their translation".into(),
                engine: "wolf".into(),
                confidence: 1.0,
            },
            PackTmEntry {
                source_text: "さようなら".into(),
                target_text: "Goodbye".into(),
                engine: "wolf".into(),
                confidence: 0.9,
            },
        ];
        let mut tx = pool.begin().await.unwrap();
        let added = import_tm_entries(&mut tx, "ja-en", &entries).await.unwrap();
        tx.commit().await.unwrap();

        assert_eq!(added, 1, "conflicting entry is DO NOTHING'd");
        let target: String = sqlx::query_scalar(
            "SELECT target_text FROM tm_entries WHERE source_text = 'こんにちは'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(target, "My own translation", "receiver's TM always wins");
    }
}
