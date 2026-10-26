//! Project-level orchestration of pristine game-file snapshots.
//!
//! The snapshot format and reconstruction rules are engine-owned
//! (`engines::mv_mz::originals`); this module wires them to the database and
//! the per-installation data directory. Only RPG Maker MV/MZ projects are
//! snapshotted: Wolf and VX Ace exports keep reading the game folder.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use sqlx::SqlitePool;

use crate::engines::filter::{classify_file_type, FileClass};
use crate::engines::mv_mz::originals::{
    reconstruct_json, reconstruct_plugins_js, reverse_keke_edits, revert_locale_switch,
    same_content, sha256_hex, CoreSegment, PluginSegment, Provenance, Reconstruction,
    SnapshotStore,
};
use crate::engines::mv_mz::{keke, locale};

/// `true` for an id Hoshi2Star generates itself: a hyphenated UUID. Project
/// ids can come from `.hoshi2star.json` in the game folder, which is not
/// trusted, and they become a directory name below.
pub fn is_project_id(project_id: &str) -> bool {
    project_id.len() == 36 && uuid::Uuid::parse_str(project_id).is_ok()
}

/// Snapshot directory of one project. Refuses anything but a UUID, so a
/// crafted id can never point outside `<data_dir>/originals`.
pub fn snapshot_root(data_dir: &Path, project_id: &str) -> Result<PathBuf, String> {
    if !is_project_id(project_id) {
        return Err(format!(
            "invalid project id {project_id:?}: snapshots need a UUID"
        ));
    }
    Ok(data_dir.join("originals").join(project_id))
}

/// Path of `file_path` relative to the game folder with `/` separators — the
/// key used both inside snapshots and as the zip entry name.
pub fn game_relative_path(game_path: &str, file_path: &str, fallback: &str) -> String {
    Path::new(file_path)
        .strip_prefix(Path::new(game_path))
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| fallback.to_string())
}

/// Remove a project's snapshot (best effort: a leftover directory is harmless,
/// a failure here must never block deleting or re-creating the project).
pub fn remove_snapshot(data_dir: &Path, project_id: &str) {
    let root = match snapshot_root(data_dir, project_id) {
        Ok(root) => root,
        Err(error) => {
            log::warn!("snapshot not removed: {error}");
            return;
        }
    };
    if root.exists() {
        if let Err(error) = std::fs::remove_dir_all(&root) {
            log::warn!("could not remove snapshot {}: {error}", root.display());
        }
    }
}

/// Make sure every MV/MZ JSON source file of `project_id` has a pristine
/// snapshot, capturing (and if needed reconstructing) missing ones from disk.
/// Files already snapshotted are not re-read from the game; where a game
/// update must be noticed, use [`ensure_fresh_mv_mz_snapshot`].
pub async fn ensure_mv_mz_snapshot(
    pool: &SqlitePool,
    data_dir: &Path,
    project_id: &str,
) -> Result<SnapshotStore, String> {
    load_mv_mz_snapshot(pool, data_dir, project_id, false).await
}

/// [`ensure_mv_mz_snapshot`], plus a check of every snapshotted file against
/// the game folder. Disk bytes that are neither the original nor a known
/// export are rebuilt (previous exports undone, as on first capture) and
/// compared with the snapshot: when the game itself changed (an update), the
/// snapshot takes the new version, so an export never brings back an older
/// version of the game. Used by export, its preflight and the coverage scan.
pub async fn ensure_fresh_mv_mz_snapshot(
    pool: &SqlitePool,
    data_dir: &Path,
    project_id: &str,
) -> Result<SnapshotStore, String> {
    load_mv_mz_snapshot(pool, data_dir, project_id, true).await
}

async fn load_mv_mz_snapshot(
    pool: &SqlitePool,
    data_dir: &Path,
    project_id: &str,
    check_disk: bool,
) -> Result<SnapshotStore, String> {
    let root = snapshot_root(data_dir, project_id)?;
    let (game_path, source_language, target_language): (String, String, String) = sqlx::query_as(
        "SELECT game_path, COALESCE(source_language, 'ja'), \
                    COALESCE(target_language, 'en') \
             FROM projects WHERE id = ?",
    )
    .bind(project_id)
    .fetch_one(pool)
    .await
    .map_err(|error| error.to_string())?;
    let target_locale = locale::target_locale(&target_language);

    let files: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT id, file_name, file_path, file_type FROM source_files WHERE project_id = ?",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?;

    let mut store = tokio::task::spawn_blocking(move || SnapshotStore::open(root))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;

    // Translated skill types renamed inside battle-command tags at export:
    // undone when a snapshot has to be rebuilt from an exported folder.
    let reverse_renames: HashMap<String, String> = sqlx::query_as::<_, (String, String)>(
        "SELECT s.target_text, s.source_text FROM segments s \
         JOIN source_files sf ON sf.id = s.source_file_id \
         WHERE sf.project_id = ? AND s.text_category = 'system.skill_types' \
           AND s.target_text != '' AND s.context_json LIKE '%keke_skill_type%'",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?
    .into_iter()
    .filter(|(target, source)| target != source)
    .collect();

    let mut changed = false;
    for (file_id, file_name, file_path, file_type) in files {
        let plugins_js = match classify_file_type(&file_type) {
            FileClass::Json => false,
            FileClass::PluginsJs => true,
            _ => continue,
        };
        let rel = game_relative_path(&game_path, &file_path, &file_name);
        let snapshotted = store.contains(&rel);
        if snapshotted && !check_disk {
            continue;
        }
        let read = tokio::task::spawn_blocking(move || {
            std::fs::read(&file_path).map(|disk| {
                let sha = sha256_hex(&disk);
                (disk, sha)
            })
        })
        .await
        .map_err(|error| error.to_string())?;
        let (disk, disk_sha) = match read {
            Ok(read) => read,
            // The snapshot alone exports a file the game folder no longer has.
            Err(_) if snapshotted => continue,
            Err(error) => return Err(format!("read {rel}: {error}")),
        };
        if snapshotted && store.is_known_disk(&rel, &disk_sha) {
            continue;
        }

        let recorded = recorded_segments(pool, &file_id, plugins_js).await?;
        let renames = reverse_renames.clone();
        let language = source_language.clone();
        let locale_target = (file_name == "System.json").then(|| target_locale.clone());
        let (store_back, outcome) = tokio::task::spawn_blocking(move || {
            let result =
                rebuild_original(&rel, &disk, &recorded, &language, &renames).and_then(|rebuilt| {
                    update_snapshot(
                        &mut store,
                        &rel,
                        rebuilt,
                        &disk_sha,
                        locale_target.as_deref(),
                    )
                });
            (store, result)
        })
        .await
        .map_err(|error| error.to_string())?;
        store = store_back;
        outcome?;
        changed = true;
    }

    if changed {
        store.save().map_err(|error| error.to_string())?;
    }
    Ok(store)
}

/// What the project recorded about one file's extracted text: enough to undo
/// a previous export found on disk.
enum Recorded {
    Json {
        core: Vec<CoreSegment>,
        /// Note pointer → `(occurrence, target, source)` Keke label segments.
        labels: HashMap<String, Vec<(usize, String, String)>>,
    },
    PluginsJs(Vec<PluginSegment>),
}

async fn recorded_segments(
    pool: &SqlitePool,
    file_id: &str,
    plugins_js: bool,
) -> Result<Recorded, String> {
    let rows: Vec<(String, String, String, Option<String>, String)> = sqlx::query_as(
        "SELECT json_key, source_text, target_text, context_json, segment_kind FROM segments \
         WHERE source_file_id = ?",
    )
    .bind(file_id)
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?;
    if plugins_js {
        return Ok(Recorded::PluginsJs(
            rows.into_iter()
                .map(|(key, source, target, context, _)| PluginSegment {
                    fill_in: crate::engines::mv_mz::coverage::plugins::context_is_fill_in(
                        context.as_deref(),
                    ),
                    key,
                    source,
                    target,
                })
                .collect(),
        ));
    }
    // Plain JSON pointers are reset by `reconstruct_json`; note-tag label
    // segments are reverted inside their note afterwards.
    let mut labels: HashMap<String, Vec<(usize, String, String)>> = HashMap::new();
    let mut core = Vec::new();
    for (key, source, target, _, kind) in rows {
        if let Some((occurrence, _, pointer)) = keke::parse_label_key(&key) {
            labels
                .entry(pointer)
                .or_default()
                .push((occurrence, target, source));
        } else {
            core.push(CoreSegment {
                key,
                source,
                target,
                kind,
            });
        }
    }
    Ok(Recorded::Json { core, labels })
}

/// The original of `label` rebuilt from game-folder bytes.
fn rebuild_original(
    label: &str,
    disk: &[u8],
    recorded: &Recorded,
    source_language: &str,
    reverse_renames: &HashMap<String, String>,
) -> Result<Reconstruction, String> {
    match recorded {
        Recorded::PluginsJs(segments) => reconstruct_plugins_js(label, disk, segments),
        Recorded::Json { core, labels } => reconstruct_json(label, disk, core, source_language)
            .and_then(|rebuilt| reverse_keke_edits(label, rebuilt, labels, reverse_renames)),
    }
    .map_err(|error| error.to_string())
}

/// Record `rebuilt` (from disk bytes with checksum `disk_sha`) in the
/// snapshot. A file already snapshotted keeps its original when `rebuilt` is
/// the same — the disk held an export not seen before — and takes `rebuilt`
/// otherwise: the game itself changed.
fn update_snapshot(
    store: &mut SnapshotStore,
    rel: &str,
    rebuilt: Reconstruction,
    disk_sha: &str,
    target_locale: Option<&str>,
) -> Result<(), String> {
    let mut rebuilt = rebuilt;
    if store.contains(rel) {
        let snapshot = store.read(rel).map_err(|error| error.to_string())?;
        if let Some(target) = target_locale {
            rebuilt = revert_locale_switch(rel, rebuilt, &snapshot, target)
                .map_err(|error| error.to_string())?;
        }
        if same_content(&rebuilt.bytes, &snapshot) {
            store.remember_disk(rel, disk_sha);
            return Ok(());
        }
        log::warn!(
            "{rel} changed in the game folder since it was snapshotted (game update?); \
             the snapshot now follows the new version"
        );
    }
    if !rebuilt.drift.is_empty() {
        log::warn!(
            "{rel}: {} extracted pointer(s) differ from the recorded source; \
             the game file changed since extraction",
            rebuilt.drift.len()
        );
    }
    store
        .put(rel, &rebuilt.bytes, rebuilt.provenance, rebuilt.drift)
        .map_err(|error| error.to_string())?;
    store.remember_disk(rel, disk_sha);
    Ok(())
}

/// Remember files Hoshi2Star just produced from the snapshot (the export zip,
/// a single-file injection), so finding them in the game folder later is not
/// mistaken for a game update. Best effort: forgetting one only costs a
/// rebuild-and-compare on the next check.
pub fn remember_written(store: &mut SnapshotStore, written: &[(String, String)]) {
    for (rel, sha) in written {
        store.remember_disk(rel, sha);
    }
    if let Err(error) = store.save() {
        log::warn!("could not record exported files in the snapshot: {error}");
    }
}

/// Snapshot a file Hoshi2Star has never written before (e.g. `js/plugins.js`
/// the first time plugin text is scanned) verbatim, if not already recorded.
/// Returns whether the store changed; the caller saves it.
pub fn capture_verbatim(
    store: &mut SnapshotStore,
    rel: &str,
    disk_path: &Path,
) -> Result<bool, String> {
    if store.contains(rel) {
        return Ok(false);
    }
    let bytes = std::fs::read(disk_path).map_err(|error| format!("read {rel}: {error}"))?;
    store
        .put(rel, &bytes, Provenance::Pristine, Vec::new())
        .map_err(|error| error.to_string())?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROJECT: &str = "1b4e28ba-2fa1-41d2-883f-0016d3cca427";

    async fn seeded(game: &Path, map_on_disk: &str) -> (SqlitePool, tempfile::NamedTempFile) {
        let db = tempfile::NamedTempFile::new().unwrap();
        let pool = crate::db::pool::init(db.path().to_str().unwrap())
            .await
            .unwrap();
        let data = game.join("data");
        std::fs::create_dir_all(&data).unwrap();
        std::fs::write(data.join("Map001.json"), map_on_disk).unwrap();
        sqlx::query(
            "INSERT INTO projects (id, name, engine, game_path) VALUES (?, 'T', 'mv_mz', ?)",
        )
        .bind(PROJECT)
        .bind(game.to_string_lossy().to_string())
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
             VALUES ('f1', ?, 'Map001.json', ?, 'map')",
        )
        .bind(PROJECT)
        .bind(data.join("Map001.json").to_string_lossy().to_string())
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO segments (id, source_file_id, json_key, source_text, target_text, status) \
             VALUES ('s1', 'f1', '/a', '勇者', 'Hero', 'translated')",
        )
        .execute(&pool)
        .await
        .unwrap();
        (pool, db)
    }

    #[tokio::test]
    async fn snapshot_reconstructs_an_already_exported_folder_once() {
        let temp = tempfile::tempdir().unwrap();
        let game = temp.path().join("game");
        // The user unzipped a previous export over the game: disk holds the target.
        let (pool, _db) = seeded(&game, r#"{"a":"\\FS[22]Hero","b":"不変"}"#).await;
        let data_dir = temp.path().join("app");

        let store = ensure_mv_mz_snapshot(&pool, &data_dir, PROJECT)
            .await
            .unwrap();
        assert_eq!(
            store.entry("data/Map001.json").unwrap().provenance,
            Provenance::Reconstructed
        );
        assert_eq!(
            store.read("data/Map001.json").unwrap(),
            r#"{"a":"勇者","b":"不変"}"#.as_bytes()
        );

        // Idempotent: later disk changes are never re-captured.
        std::fs::write(game.join("data").join("Map001.json"), r#"{"a":"garbage"}"#).unwrap();
        let again = ensure_mv_mz_snapshot(&pool, &data_dir, PROJECT)
            .await
            .unwrap();
        assert_eq!(
            again.read("data/Map001.json").unwrap(),
            r#"{"a":"勇者","b":"不変"}"#.as_bytes()
        );

        remove_snapshot(&data_dir, PROJECT);
        assert!(!snapshot_root(&data_dir, PROJECT).unwrap().exists());
    }

    #[tokio::test]
    async fn fresh_check_follows_a_game_update_but_not_an_export() {
        let temp = tempfile::tempdir().unwrap();
        let game = temp.path().join("game");
        let map = game.join("data").join("Map001.json");
        let (pool, _db) = seeded(&game, r#"{"a":"勇者","b":"不変"}"#).await;
        let data_dir = temp.path().join("app");
        let first = ensure_fresh_mv_mz_snapshot(&pool, &data_dir, PROJECT)
            .await
            .unwrap();
        assert_eq!(
            first.entry("data/Map001.json").unwrap().provenance,
            Provenance::Pristine
        );

        // Our export unzipped over the game (never recorded): same original.
        std::fs::write(&map, r#"{"a":"Hero","b":"不変"}"#).unwrap();
        let after_export = ensure_fresh_mv_mz_snapshot(&pool, &data_dir, PROJECT)
            .await
            .unwrap();
        assert_eq!(
            after_export.read("data/Map001.json").unwrap(),
            r#"{"a":"勇者","b":"不変"}"#.as_bytes()
        );
        assert!(after_export.is_known_disk(
            "data/Map001.json",
            &sha256_hex(r#"{"a":"Hero","b":"不変"}"#.as_bytes())
        ));

        // A game update (new content, our export of `/a` still on disk):
        // the snapshot follows, with the export undone.
        std::fs::write(&map, r#"{"a":"Hero","b":"不変","c":"新しい"}"#).unwrap();
        let cached = ensure_mv_mz_snapshot(&pool, &data_dir, PROJECT)
            .await
            .unwrap();
        assert_eq!(
            cached.read("data/Map001.json").unwrap(),
            r#"{"a":"勇者","b":"不変"}"#.as_bytes(),
            "the cheap variant never re-reads the game"
        );
        let updated = ensure_fresh_mv_mz_snapshot(&pool, &data_dir, PROJECT)
            .await
            .unwrap();
        let original: serde_json::Value =
            serde_json::from_slice(&updated.read("data/Map001.json").unwrap()).unwrap();
        assert_eq!(
            original,
            serde_json::json!({"a":"勇者","b":"不変","c":"新しい"})
        );
    }

    #[tokio::test]
    async fn remembered_exports_skip_the_rebuild() {
        let temp = tempfile::tempdir().unwrap();
        let game = temp.path().join("game");
        let map = game.join("data").join("Map001.json");
        let (pool, _db) = seeded(&game, r#"{"a":"勇者"}"#).await;
        let data_dir = temp.path().join("app");
        let mut store = ensure_fresh_mv_mz_snapshot(&pool, &data_dir, PROJECT)
            .await
            .unwrap();
        // A written file the rebuild alone could not explain (it would call
        // this Japanese value drift) is recognised by its checksum.
        let exported = r#"{"a":"勇者さま"}"#;
        remember_written(
            &mut store,
            &[(
                "data/Map001.json".to_string(),
                sha256_hex(exported.as_bytes()),
            )],
        );
        std::fs::write(&map, exported).unwrap();
        let again = ensure_fresh_mv_mz_snapshot(&pool, &data_dir, PROJECT)
            .await
            .unwrap();
        assert_eq!(
            again.read("data/Map001.json").unwrap(),
            r#"{"a":"勇者"}"#.as_bytes()
        );
        assert!(again.entry("data/Map001.json").unwrap().drift.is_empty());
    }

    #[test]
    fn crafted_project_ids_never_leave_the_snapshot_directory() {
        let temp = tempfile::tempdir().unwrap();
        let data_dir = temp.path().join("app");
        // A sibling the crafted ids would otherwise reach.
        let victim = temp.path().join("victim");
        std::fs::create_dir_all(&victim).unwrap();
        for bad in [
            "../../victim",
            r"..\..\victim",
            "p1",
            "",
            "{1b4e28ba-2fa1-41d2-883f-0016d3cca427}",
            "urn:uuid:1b4e28ba-2fa1-41d2-883f-0016d3cca427",
        ] {
            assert!(snapshot_root(&data_dir, bad).is_err(), "{bad}");
            remove_snapshot(&data_dir, bad);
        }
        assert!(victim.exists());
        assert!(snapshot_root(&data_dir, PROJECT).is_ok());
    }

    #[test]
    fn relative_paths_use_forward_slashes_and_fall_back_to_the_file_name() {
        let game = Path::new("C:/games/g");
        let file = game.join("www").join("data").join("Map001.json");
        assert_eq!(
            game_relative_path(
                &game.to_string_lossy(),
                &file.to_string_lossy(),
                "Map001.json"
            ),
            "www/data/Map001.json"
        );
        assert_eq!(
            game_relative_path("/elsewhere", "/other/Map001.json", "Map001.json"),
            "Map001.json"
        );
    }
}
