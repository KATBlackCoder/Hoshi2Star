//! Decrypt a Wolf RPG game still packed in `.wolf` archives (plan UWF).
//!
//! This is the **only** module of the app that imports the `unwolf` crate
//! (decision UWF-D11): everything about the Wolf archive format, the Pro keys
//! and the Pro protection lives in `unwolf`, behind its public API. A gap in
//! `unwolf` is fixed there, never worked around here. The rest of the app sees
//! [`unpack_game`], [`UnpackReport`], [`UnpackProgress`] and [`UnpackError`].
//!
//! What this module decides (the app's rules):
//! - only the archives holding text are extracted
//!   ([`super::sources::text_archives`], UWF-D2): `Data/BasicData.wolf` and
//!   `Data/MapData.wolf`, or the whole root `Data.wolf`; picture and sound
//!   archives stay packed;
//! - a Pro game is accepted when it is not protected, already unprotected, or
//!   protected by the v3.5 scheme, which is removed; Pro v1 and v2 are refused
//!   before the remaining archives are extracted (UWF-D5);
//! - the text archives are **moved**, never deleted, to
//!   `<game>/Hoshi2Star-archives/` with their relative path, once every one was
//!   extracted, the protection removed without failure and the game checked
//!   readable by the app (UWF-D3, UWF-D4). The game reads its archives before
//!   the loose files of `Data/`: while they stay in place, a translation would
//!   be ignored.
//!
//! Files already on disk are never overwritten (`Options.overwrite` stays
//! `false`): an output folder that exists is reported as already unpacked.

use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use unwolf::game::{Event, Options, Outcome, ProtectionStatus, Session};
use unwolf::protkey::ProVersion;
use unwolf::unprotect::FileStatus;

use crate::engines::detector::{
    find_wolf_data_dir, is_wolf_game_dir, wolf_database_version, WOLF_LAUNCHERS,
};
use crate::engines::wolf::sources::text_archives;

/// Folder of the game where the extracted text archives are moved (UWF-D3).
pub const ARCHIVES_FOLDER: &str = "Hoshi2Star-archives";

/// Stable marker at the start of every [`UnpackError`] message, followed by
/// the kind of failure (`wolf_unpack_cancelled`…), matched by the frontend.
pub const WOLF_UNPACK_ERROR: &str = "wolf_unpack";

/// State of the Wolf RPG Pro protection of the unpacked game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProProtection {
    /// Not a Wolf RPG Pro game.
    NotPro,
    /// A Pro game whose data files were already in plain form.
    AlreadyUnprotected,
    /// A Pro game protected by the v3.5 scheme, removed by this run.
    RemovedV35,
}

/// An archive that could not be extracted but holds no text (a root archive
/// next to `Data.wolf`): reported, not fatal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveFailure {
    pub archive: String,
    pub error: String,
}

/// What [`unpack_game`] did. Paths are relative to the game folder, with `/`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnpackReport {
    /// Archives extracted by this run.
    pub extracted: Vec<String>,
    /// Archives whose output folder already existed: left as they were.
    pub already_unpacked: Vec<String>,
    /// Archives `unwolf` never extracts (`Game.wolf`, `List.wolf`…).
    pub skipped: Vec<String>,
    /// Archives holding no text that could not be extracted.
    pub failed: Vec<ArchiveFailure>,
    /// `None` when there was nothing to unpack.
    pub protection: Option<ProProtection>,
    /// Files decrypted by the removal of the Pro protection (originals in
    /// `Data/BasicData/backup/`).
    pub unprotected_files: Vec<String>,
    /// Text archives moved to [`ARCHIVES_FOLDER`], by their former path.
    pub moved: Vec<String>,
    /// Warnings sent by `unwolf` (configuration file, Pro key not saved…).
    pub warnings: Vec<String>,
}

/// One extracted file, for the progress shown to the user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnpackProgress {
    /// The archive being extracted, relative to the game folder.
    pub archive: String,
    /// Number of the file in the archive, from 1 to `total`.
    pub index: u64,
    pub total: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum UnpackError {
    #[error("{WOLF_UNPACK_ERROR}_cancelled: the decryption was cancelled; no archive was moved")]
    Cancelled,
    #[error(
        "{WOLF_UNPACK_ERROR}_no_launcher: no Game.exe, GamePro.exe or Game.ini in {}",
        .0.display()
    )]
    NoLauncher(PathBuf),
    #[error(
        "{WOLF_UNPACK_ERROR}_not_writable: the game folder {} cannot be written: {source}",
        path.display()
    )]
    NotWritable {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// UWF-D4: a `Data/` beside the root `Data.wolf` may be partial; moving
    /// the archive would make the game lose its pictures and sounds.
    #[error(
        "{WOLF_UNPACK_ERROR}_data_folder_present: {archive} and a non-empty {folder}/ are both \
         in the game folder; the folder may be a partial extraction. Move one of them out of \
         the game folder, or decrypt the game with UberWolf"
    )]
    DataFolderPresent { archive: String, folder: String },
    #[error(
        "{WOLF_UNPACK_ERROR}_destination_exists: {0} already exists; move it out of the game \
         folder first"
    )]
    DestinationExists(String),
    #[error(
        "{WOLF_UNPACK_ERROR}_pro_unsupported: this Wolf RPG Pro protection ({0}) cannot be \
         removed; decrypt the game with UberWolf"
    )]
    ProUnsupported(String),
    #[error("{WOLF_UNPACK_ERROR}_extraction_failed: {archive}: {error}")]
    ExtractionFailed { archive: String, error: String },
    #[error("{WOLF_UNPACK_ERROR}_protection_failed: {0}")]
    Protection(String),
    #[error(
        "{WOLF_UNPACK_ERROR}_unprotect_failed: the Pro protection could not be removed from: {0}"
    )]
    UnprotectFailed(String),
    #[error(
        "{WOLF_UNPACK_ERROR}_not_readable: after extraction, {0}; the archives were left in place"
    )]
    NotReadable(String),
    #[error(
        "{WOLF_UNPACK_ERROR}_move_failed: {archive} could not be moved to {ARCHIVES_FOLDER}/ \
         ({source}); the archives were put back. Close the game and try again"
    )]
    MoveFailed {
        archive: String,
        #[source]
        source: std::io::Error,
    },
}

/// Extract the text archives of the Wolf RPG game in `game_dir`, remove a Pro
/// v3.5 protection, check the result and move the archives aside.
///
/// `config_path` is where `unwolf` keeps the archive keys of Pro games
/// (`UberWolfConfig.json`, an absolute path in the app's data folder).
/// `on_progress` is called after each extracted file; setting `cancel` stops
/// the run at the next file, and nothing is moved then. A game without any
/// text archive gives an empty report.
///
/// Synchronous and long: call it from a blocking task.
pub fn unpack_game(
    game_dir: &Path,
    config_path: &Path,
    on_progress: &mut dyn FnMut(&UnpackProgress),
    cancel: &AtomicBool,
) -> Result<UnpackReport, UnpackError> {
    // `unwolf` reports archives by absolute path: compare like with like.
    let game_dir = &std::path::absolute(game_dir).unwrap_or_else(|_| game_dir.to_path_buf());
    let archives = text_archives(game_dir);
    if archives.is_empty() {
        return Ok(UnpackReport::default());
    }
    check_before_writing(game_dir, &archives)?;
    let launcher = WOLF_LAUNCHERS
        .iter()
        .map(|name| game_dir.join(name))
        .find(|path| path.is_file())
        .ok_or_else(|| UnpackError::NoLauncher(game_dir.to_path_buf()))?;
    if let Some(parent) = config_path.parent() {
        if let Err(error) = std::fs::create_dir_all(parent) {
            // `unwolf` then keeps a Pro key for this run only, with a warning.
            log::warn!("cannot create {}: {error}", parent.display());
        }
    }

    let warnings = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&warnings);
    let mut session = Session::new(
        Options::new(config_path),
        Box::new(move |event: &Event<'_>| {
            if let Event::Warning(error) = event {
                if let Ok(mut list) = sink.lock() {
                    list.push(error.to_string());
                }
            }
        }),
    );
    session
        .init_game(&launcher)
        .map_err(|error| UnpackError::ExtractionFailed {
            archive: relative(game_dir, &launcher),
            error: error.to_string(),
        })?;

    let mut report = UnpackReport::default();
    let root_archive = archives.iter().any(|a| !a.contains('/'));
    let mut on_file = |file: &unwolf::game::FileProgress<'_>| {
        on_progress(&UnpackProgress {
            archive: relative(game_dir, file.archive),
            index: file.index,
            total: file.total,
        });
        if cancel.load(Ordering::SeqCst) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    };

    // 1. The archives holding `BasicData` first: the Pro check reads its
    //    `Game.dat`, and a refused protection must stop before the rest.
    let (first, rest): (Vec<PathBuf>, Vec<PathBuf>) = if root_archive {
        let outcomes = session
            .unpack_data_with_progress(&mut on_file)
            .map_err(|error| UnpackError::ExtractionFailed {
                archive: archives.join(", "),
                error: error.to_string(),
            })?;
        record(game_dir, &archives, outcomes, &mut report, cancel)?;
        (Vec::new(), Vec::new())
    } else {
        archives
            .iter()
            .map(|a| game_dir.join(a))
            .partition(|path| is_basic_data_archive(path))
    };
    if !first.is_empty() {
        let outcomes = session.unpack_files_with_progress(&first, &mut on_file);
        record(game_dir, &archives, outcomes, &mut report, cancel)?;
    }
    check_cancel(cancel)?;

    // 2. Pro protection (UWF-D5): the state comes from `unwolf`.
    let protection = match session
        .protection_status()
        .map_err(|error| UnpackError::Protection(error.to_string()))?
    {
        ProtectionStatus::NotPro => ProProtection::NotPro,
        ProtectionStatus::Unprotected => ProProtection::AlreadyUnprotected,
        ProtectionStatus::Protected(ProVersion::V35) => ProProtection::RemovedV35,
        ProtectionStatus::Protected(version) => {
            return Err(UnpackError::ProUnsupported(format!("{version:?}")));
        }
        other => return Err(UnpackError::ProUnsupported(format!("{other:?}"))),
    };

    // 3. The remaining text archives.
    if !rest.is_empty() {
        let outcomes = session.unpack_files_with_progress(&rest, &mut on_file);
        record(game_dir, &archives, outcomes, &mut report, cancel)?;
    }
    check_cancel(cancel)?;

    // 4. Remove the v3.5 protection; any file left protected stops the run.
    if protection == ProProtection::RemovedV35 {
        let unprotected = session
            .unprotect(ProVersion::V35)
            .map_err(|error| UnpackError::Protection(error.to_string()))?;
        let mut failures = Vec::new();
        for file in unprotected.files {
            match file.status {
                FileStatus::Done => report.unprotected_files.push(file.name),
                FileStatus::NotProtected => {}
                FileStatus::Failed(error) => failures.push(format!("{} ({error})", file.name)),
                other => failures.push(format!("{} ({other:?})", file.name)),
            }
        }
        if !failures.is_empty() {
            return Err(UnpackError::UnprotectFailed(failures.join(", ")));
        }
    }
    report.protection = Some(protection);

    // The archives are memory-mapped while the session lives.
    drop(session);
    report.warnings = warnings.lock().map(|w| w.clone()).unwrap_or_default();

    // 5. Readable by the app (UWF-D4), then 6. moved aside (UWF-D3).
    check_readable(game_dir)?;
    check_cancel(cancel)?;
    report.moved = move_aside(game_dir, &archives)?;
    Ok(report)
}

/// Refusals that need no extraction: the folder is writable, no destination in
/// [`ARCHIVES_FOLDER`] is taken, and a root `Data.wolf` has no `Data/` beside
/// it (UWF-D4).
fn check_before_writing(game_dir: &Path, archives: &[String]) -> Result<(), UnpackError> {
    tempfile::tempfile_in(game_dir).map_err(|source| UnpackError::NotWritable {
        path: game_dir.to_path_buf(),
        source,
    })?;
    for archive in archives {
        let destination = game_dir.join(ARCHIVES_FOLDER).join(archive);
        if destination.exists() {
            return Err(UnpackError::DestinationExists(format!(
                "{ARCHIVES_FOLDER}/{archive}"
            )));
        }
    }
    let root_archive = archives.iter().find(|a| !a.contains('/'));
    let data_dir = find_wolf_data_dir(game_dir);
    if let (Some(archive), Some(data_dir)) = (root_archive, data_dir) {
        if std::fs::read_dir(&data_dir).is_ok_and(|mut entries| entries.next().is_some()) {
            return Err(UnpackError::DataFolderPresent {
                archive: archive.clone(),
                folder: relative(game_dir, &data_dir),
            });
        }
    }
    Ok(())
}

fn check_cancel(cancel: &AtomicBool) -> Result<(), UnpackError> {
    if cancel.load(Ordering::SeqCst) {
        Err(UnpackError::Cancelled)
    } else {
        Ok(())
    }
}

/// `BasicData.<ext>`, whatever the case.
fn is_basic_data_archive(path: &Path) -> bool {
    path.file_stem()
        .is_some_and(|stem| stem.to_string_lossy().eq_ignore_ascii_case("BasicData"))
}

/// Add the outcomes of one `unwolf` call to `report`. A text archive that
/// failed stops the run; another archive is only reported.
fn record(
    game_dir: &Path,
    text: &[String],
    outcomes: Vec<(PathBuf, Outcome)>,
    report: &mut UnpackReport,
    cancel: &AtomicBool,
) -> Result<(), UnpackError> {
    for (path, outcome) in outcomes {
        let archive = relative(game_dir, &path);
        let holds_text = text.iter().any(|t| t.eq_ignore_ascii_case(&archive));
        match outcome {
            Outcome::Extracted { .. } => report.extracted.push(archive),
            Outcome::AlreadyUnpacked => report.already_unpacked.push(archive),
            Outcome::Skipped => report.skipped.push(archive),
            Outcome::Failed(_) if cancel.load(Ordering::SeqCst) => {
                return Err(UnpackError::Cancelled);
            }
            Outcome::Failed(error) if holds_text => {
                return Err(UnpackError::ExtractionFailed {
                    archive,
                    error: error.to_string(),
                });
            }
            Outcome::Failed(error) => report.failed.push(ArchiveFailure {
                archive,
                error: error.to_string(),
            }),
            other if holds_text => {
                return Err(UnpackError::ExtractionFailed {
                    archive,
                    error: format!("{other:?}"),
                });
            }
            other => report.failed.push(ArchiveFailure {
                archive,
                error: format!("{other:?}"),
            }),
        }
    }
    Ok(())
}

/// The game opens in the app only with a launcher and `Data/BasicData/` files
/// ([`is_wolf_game_dir`]), a database that declares the encoding and at least
/// one map: checked before the archives leave, whatever was extracted now or
/// earlier (UWF-D4: `unwolf` does not check that an existing folder is
/// complete).
fn check_readable(game_dir: &Path) -> Result<(), UnpackError> {
    if !is_wolf_game_dir(game_dir) {
        return Err(UnpackError::NotReadable(
            "Data/BasicData holds no .project file nor CommonEvent.dat".into(),
        ));
    }
    if wolf_database_version(game_dir).is_none() {
        return Err(UnpackError::NotReadable(
            "no database of Data/BasicData declares its encoding".into(),
        ));
    }
    let has_map = find_wolf_data_dir(game_dir)
        .and_then(|data_dir| std::fs::read_dir(data_dir.join("MapData")).ok())
        .is_some_and(|entries| {
            entries.flatten().any(|entry| {
                entry.path().is_file()
                    && entry
                        .path()
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("mps"))
            })
        });
    if !has_map {
        return Err(UnpackError::NotReadable(
            "Data/MapData holds no .mps map".into(),
        ));
    }
    Ok(())
}

/// Move each archive to [`ARCHIVES_FOLDER`] with its relative path. When one
/// move fails (an archive locked by the running game on Windows), those
/// already moved are put back and the error is returned.
fn move_aside(game_dir: &Path, archives: &[String]) -> Result<Vec<String>, UnpackError> {
    let mut moved: Vec<(PathBuf, PathBuf)> = Vec::new();
    for archive in archives {
        let from = game_dir.join(archive);
        let to = game_dir.join(ARCHIVES_FOLDER).join(archive);
        let result = to
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::rename(&from, &to));
        if let Err(source) = result {
            for (from, to) in moved.iter().rev() {
                if let Err(error) = std::fs::rename(to, from) {
                    log::error!(
                        "cannot put {} back to {}: {error}",
                        to.display(),
                        from.display()
                    );
                }
            }
            return Err(UnpackError::MoveFailed {
                archive: archive.clone(),
                source,
            });
        }
        moved.push((from, to));
    }
    Ok(archives.to_vec())
}

/// `path` relative to `game_dir`, with `/` separators.
fn relative(game_dir: &Path, path: &Path) -> String {
    path.strip_prefix(game_dir)
        .unwrap_or(path)
        .components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
pub(crate) mod tests {
    //! Archives are made with `unwolf::dxa::pack_directory` and the
    //! "Wolf RPG v2.225" mode (DXA v8). The Pro branches (key derivation,
    //! protection removal, refusal of v1 and v2) cannot be fabricated: they
    //! are checked on real games only (plan UWF, UWF-0 and UWF-V).

    use super::*;
    use crate::engines::detector::{detect_engine, Engine};
    use crate::engines::wolf::dat_parser::DB_MAGIC_UTF8;
    use crate::engines::wolf::sources::refuse_text_archives;

    const MAP: &[u8] = b"map one";
    const PICTURE: &[u8] = b"png";

    /// The loose `Data/` of a decrypted Wolf 3.x game: a database declaring
    /// UTF-8, its project, the common events, one map and one picture.
    fn write_loose_data(data: &Path) {
        let basic = data.join("BasicData");
        std::fs::create_dir_all(&basic).unwrap();
        let mut database = vec![0x00];
        database.extend_from_slice(&DB_MAGIC_UTF8);
        database.push(0xC1);
        std::fs::write(basic.join("SysDatabase.dat"), database).unwrap();
        std::fs::write(basic.join("SysDatabase.project"), b"project").unwrap();
        std::fs::write(basic.join("CommonEvent.dat"), b"common").unwrap();
        let maps = data.join("MapData");
        std::fs::create_dir_all(&maps).unwrap();
        std::fs::write(maps.join("Map001.mps"), MAP).unwrap();
        let pictures = data.join("Picture");
        std::fs::create_dir_all(&pictures).unwrap();
        std::fs::write(pictures.join("face.png"), PICTURE).unwrap();
    }

    /// Replace the folder `dir` by the archive `<dir>.wolf`.
    fn pack(dir: &Path) {
        let mode = &unwolf::modes::default_modes()[3];
        unwolf::dxa::pack_directory(dir, &dir.with_extension("wolf"), mode).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// A game packed per folder: `Data/BasicData.wolf`, `Data/MapData.wolf`
    /// and `Data/Picture.wolf`.
    pub(crate) fn split_game() -> tempfile::TempDir {
        let game = tempfile::tempdir().unwrap();
        std::fs::write(game.path().join("Game.exe"), b"mock").unwrap();
        let data = game.path().join("Data");
        write_loose_data(&data);
        for folder in ["BasicData", "MapData", "Picture"] {
            pack(&data.join(folder));
        }
        game
    }

    /// A game packed in one root `Data.wolf`.
    fn root_game() -> tempfile::TempDir {
        let game = tempfile::tempdir().unwrap();
        std::fs::write(game.path().join("Game.exe"), b"mock").unwrap();
        write_loose_data(&game.path().join("Data"));
        pack(&game.path().join("Data"));
        game
    }

    /// Run [`unpack_game`]; `cancel_at` sets the cancel flag on that progress
    /// call (1-based).
    fn run(
        game: &Path,
        cancel_at: Option<usize>,
    ) -> (Result<UnpackReport, UnpackError>, Vec<UnpackProgress>) {
        let config = tempfile::tempdir().unwrap();
        let cancel = AtomicBool::new(false);
        let mut progress = Vec::new();
        let result = unpack_game(
            game,
            &config.path().join("unwolf").join("UberWolfConfig.json"),
            &mut |p| {
                progress.push(p.clone());
                if cancel_at == Some(progress.len()) {
                    cancel.store(true, Ordering::SeqCst);
                }
            },
            &cancel,
        );
        (result, progress)
    }

    /// Every path under `root`, relative, sorted.
    fn tree(root: &Path) -> Vec<String> {
        fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
            for entry in std::fs::read_dir(dir).unwrap().flatten() {
                out.push(relative(root, &entry.path()));
                if entry.path().is_dir() {
                    walk(root, &entry.path(), out);
                }
            }
        }
        let mut out = Vec::new();
        walk(root, root, &mut out);
        out.sort();
        out
    }

    #[test]
    fn split_game_is_readable_and_its_text_archives_are_moved_aside() {
        let game = split_game();
        let (result, progress) = run(game.path(), None);
        let report = result.unwrap();

        assert_eq!(
            report.extracted,
            ["Data/BasicData.wolf", "Data/MapData.wolf"],
            "BasicData first, the picture archive is not extracted"
        );
        assert_eq!(report.protection, Some(ProProtection::NotPro));
        assert_eq!(report.moved, ["Data/MapData.wolf", "Data/BasicData.wolf"]);
        assert!(report.failed.is_empty() && report.already_unpacked.is_empty());

        let root = game.path();
        for archive in ["Data/MapData.wolf", "Data/BasicData.wolf"] {
            assert!(!root.join(archive).exists(), "{archive} left in place");
            assert!(root.join(ARCHIVES_FOLDER).join(archive).is_file());
        }
        assert!(root.join("Data/Picture.wolf").is_file());
        assert!(!root.join("Data/Picture").exists());
        assert_eq!(
            std::fs::read(root.join("Data/MapData/Map001.mps")).unwrap(),
            MAP
        );
        assert!(refuse_text_archives(root).is_ok());
        assert_eq!(detect_engine(root).unwrap(), Engine::Wolf);
        assert!(!tree(root).iter().any(|path| path.contains("unwolf-tmp")));

        assert!(progress.iter().any(|p| p.archive == "Data/BasicData.wolf"));
        assert!(progress.iter().any(|p| p.archive == "Data/MapData.wolf"));
        assert!(progress.iter().all(|p| p.index >= 1 && p.index <= p.total));
    }

    #[test]
    fn root_data_archive_is_extracted_whole_and_moved_aside() {
        let game = root_game();
        let report = run(game.path(), None).0.unwrap();

        let root = game.path();
        assert_eq!(report.extracted, ["Data.wolf"]);
        assert_eq!(report.moved, ["Data.wolf"]);
        assert!(!root.join("Data.wolf").exists());
        assert!(root.join(ARCHIVES_FOLDER).join("Data.wolf").is_file());
        assert_eq!(
            std::fs::read(root.join("Data/Picture/face.png")).unwrap(),
            PICTURE,
            "the game reads everything from Data/ once Data.wolf is gone"
        );
        assert!(refuse_text_archives(root).is_ok());
        assert_eq!(detect_engine(root).unwrap(), Engine::Wolf);
    }

    #[test]
    fn root_data_archive_beside_a_data_folder_is_refused_without_writing() {
        let game = root_game();
        let root = game.path();
        std::fs::create_dir_all(root.join("Data/MapData")).unwrap();
        std::fs::write(root.join("Data/MapData/Map001.mps"), b"partial").unwrap();
        let before = tree(root);

        let error = run(root, None).0.unwrap_err();
        assert!(
            matches!(error, UnpackError::DataFolderPresent { .. }),
            "{error}"
        );
        assert!(error
            .to_string()
            .starts_with("wolf_unpack_data_folder_present"));
        assert_eq!(tree(root), before);
    }

    #[test]
    fn corrupt_text_archive_stops_before_anything_is_moved() {
        let game = split_game();
        let root = game.path();
        std::fs::write(root.join("Data/MapData.wolf"), b"not an archive").unwrap();

        let error = run(root, None).0.unwrap_err();
        match &error {
            UnpackError::ExtractionFailed { archive, .. } => {
                assert_eq!(archive, "Data/MapData.wolf");
            }
            other => panic!("unexpected error: {other}"),
        }
        assert!(root.join("Data/MapData.wolf").is_file());
        assert!(root.join("Data/BasicData.wolf").is_file());
        assert!(!root.join(ARCHIVES_FOLDER).exists());
    }

    #[test]
    fn cancelled_run_moves_nothing() {
        let game = split_game();
        let root = game.path();

        let error = run(root, Some(1)).0.unwrap_err();
        assert!(matches!(error, UnpackError::Cancelled), "{error}");
        assert!(root.join("Data/MapData.wolf").is_file());
        assert!(root.join("Data/BasicData.wolf").is_file());
        assert!(!root.join(ARCHIVES_FOLDER).exists());
        assert!(
            !root.join("Data/BasicData").exists(),
            "the archive stopped leaves no partial output"
        );
    }

    #[test]
    fn taken_destination_is_refused_before_extraction() {
        let game = split_game();
        let root = game.path();
        let taken = root.join(ARCHIVES_FOLDER).join("Data/MapData.wolf");
        std::fs::create_dir_all(taken.parent().unwrap()).unwrap();
        std::fs::write(&taken, b"older copy").unwrap();

        let error = run(root, None).0.unwrap_err();
        assert!(
            matches!(error, UnpackError::DestinationExists(_)),
            "{error}"
        );
        assert!(!root.join("Data/BasicData").exists());
        assert!(!root.join("Data/MapData").exists());
        assert_eq!(std::fs::read(&taken).unwrap(), b"older copy");
    }

    #[test]
    fn loose_files_already_there_are_never_overwritten() {
        let game = split_game();
        let root = game.path();
        std::fs::create_dir_all(root.join("Data/MapData")).unwrap();
        std::fs::write(root.join("Data/MapData/Map001.mps"), b"mine").unwrap();

        let report = run(root, None).0.unwrap();
        assert_eq!(report.already_unpacked, ["Data/MapData.wolf"]);
        assert_eq!(report.extracted, ["Data/BasicData.wolf"]);
        assert_eq!(
            std::fs::read(root.join("Data/MapData/Map001.mps")).unwrap(),
            b"mine"
        );
        assert_eq!(report.moved, ["Data/MapData.wolf", "Data/BasicData.wolf"]);
    }

    #[test]
    fn game_unreadable_after_extraction_keeps_its_archives() {
        let game = tempfile::tempdir().unwrap();
        let root = game.path();
        std::fs::write(root.join("Game.exe"), b"mock").unwrap();
        let data = root.join("Data");
        write_loose_data(&data);
        std::fs::remove_file(data.join("MapData/Map001.mps")).unwrap();
        std::fs::write(data.join("MapData/readme.txt"), b"no map").unwrap();
        for folder in ["BasicData", "MapData"] {
            pack(&data.join(folder));
        }

        let error = run(root, None).0.unwrap_err();
        assert!(matches!(error, UnpackError::NotReadable(_)), "{error}");
        assert!(root.join("Data/MapData.wolf").is_file());
        assert!(root.join("Data/BasicData.wolf").is_file());
        assert!(!root.join(ARCHIVES_FOLDER).exists());
    }

    #[test]
    fn game_without_launcher_is_refused() {
        let game = split_game();
        std::fs::remove_file(game.path().join("Game.exe")).unwrap();

        let error = run(game.path(), None).0.unwrap_err();
        assert!(matches!(error, UnpackError::NoLauncher(_)), "{error}");
        assert!(!game.path().join("Data/BasicData").exists());
    }

    #[test]
    fn second_run_on_an_unpacked_game_has_nothing_to_do() {
        let game = split_game();
        run(game.path(), None).0.unwrap();
        let before = tree(game.path());

        let report = run(game.path(), None).0.unwrap();
        assert_eq!(report, UnpackReport::default());
        assert_eq!(tree(game.path()), before);
    }
}
