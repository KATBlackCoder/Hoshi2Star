//! Where a Wolf RPG game reads its translatable files from (plan RW, RW-5).
//!
//! The engine reads its `.wolf` archives before any loose file of `Data/`
//! (verified in game on Wolf 2.23 and 3.658), so the archive copy of a file wins
//! and a loose file is used only for what no archive holds. [`WolfSources`] is
//! loaded once per operation and indexes the maps (`MapData/*.mps`) and the
//! `BasicData/` databases and common events by their path relative to `Data/`,
//! with the origin of each file — what the export needs to deliver a
//! translated file where the game will read it.
//!
//! Archives read: `Data.wolf` at the game root (paths relative to `Data/`),
//! `Data/MapData.wolf` and `Data/BasicData.wolf` (paths relative to those
//! folders). Only the direct children of `MapData/` and `BasicData/` are
//! decrypted: nested copies (sample data shipped inside some archives) are
//! ignored. An archive that cannot be read, or that holds maps none of which
//! sits under `MapData/`, refuses the operation: a project never opens
//! without its dialogue.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::decrypt::legacy_xor::extract_matching;
use crate::engines::detector::find_wolf_data_dir;

/// Stable marker at the start of [`SourcesError`] messages about archives,
/// matched by the frontend.
pub const WOLF_UNREADABLE_ARCHIVE: &str = "wolf_unreadable_archive";

#[derive(Debug, thiserror::Error)]
pub enum SourcesError {
    #[error("{marker}: {archive} cannot be read: {reason}", marker = WOLF_UNREADABLE_ARCHIVE)]
    UnreadableArchive { archive: String, reason: String },
    #[error(
        "{marker}: {archive} holds maps outside MapData/ that Hoshi2Star cannot place ({paths})",
        marker = WOLF_UNREADABLE_ARCHIVE
    )]
    UnplacedMaps { archive: String, paths: String },
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Where a file comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Origin {
    /// `Data/{logical path}` on disk.
    Loose,
    /// Inside an archive: the archive's path relative to the game folder
    /// (`Data.wolf`, `Data/MapData.wolf`) and the file's path inside it
    /// (`/MapData/Map001.mps`).
    Archive { archive: PathBuf, path: String },
}

#[derive(Debug, Clone)]
pub(crate) struct SourceFile {
    /// Path relative to `Data/`, e.g. `MapData/Map001.mps`.
    pub logical: String,
    pub bytes: Vec<u8>,
    pub origin: Origin,
}

impl SourceFile {
    /// File stem used in segment keys. Lowercase for an archive file, as the
    /// previous loaders did, so the keys of existing projects do not change.
    pub(crate) fn stem(&self) -> String {
        let name = self.logical.rsplit('/').next().unwrap_or(&self.logical);
        let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
        match self.origin {
            Origin::Loose => stem.to_string(),
            Origin::Archive { .. } => stem.to_lowercase(),
        }
    }
}

/// The translatable files of a game, by lowercase logical path.
#[derive(Debug, Default)]
pub(crate) struct WolfSources {
    files: BTreeMap<String, SourceFile>,
}

/// `true` for a direct child of `MapData/` in `.mps`, or of `BasicData/` in
/// `.dat` / `.project`.
fn is_wanted(logical: &str) -> bool {
    let lower = logical.to_ascii_lowercase();
    match lower.split_once('/') {
        Some(("mapdata", name)) => !name.contains('/') && name.ends_with(".mps"),
        Some(("basicdata", name)) => {
            !name.contains('/') && (name.ends_with(".dat") || name.ends_with(".project"))
        }
        _ => false,
    }
}

/// Entry of `dir` whose name matches `name` regardless of case.
fn find_case_insensitive(dir: &Path, name: &str) -> Option<PathBuf> {
    std::fs::read_dir(dir).ok()?.flatten().find_map(|entry| {
        let file_name = entry.file_name();
        (file_name.to_string_lossy().eq_ignore_ascii_case(name) && entry.path().is_file())
            .then(|| entry.path())
    })
}

/// Archives that may hold maps or databases: `(absolute path, path relative to
/// the game folder, prefix of their inner paths relative to Data/)`.
fn candidate_archives(game_dir: &Path) -> Vec<(PathBuf, PathBuf, &'static str)> {
    let mut archives = Vec::new();
    if let Some(path) = find_case_insensitive(game_dir, "Data.wolf") {
        let relative = PathBuf::from(path.file_name().unwrap_or_default());
        archives.push((path, relative, ""));
    }
    if let Some(data_dir) = find_wolf_data_dir(game_dir) {
        let data_name = PathBuf::from(data_dir.file_name().unwrap_or_default());
        for (name, prefix) in [
            ("MapData.wolf", "MapData/"),
            ("BasicData.wolf", "BasicData/"),
        ] {
            if let Some(path) = find_case_insensitive(&data_dir, name) {
                let relative = data_name.join(path.file_name().unwrap_or_default());
                archives.push((path, relative, prefix));
            }
        }
    }
    archives
}

/// Archives the game reads first and an export rebuilds when they hold a
/// translated file (relative paths, `/` separators), found without reading them.
pub(crate) fn archive_paths(game_dir: &Path) -> Vec<String> {
    candidate_archives(game_dir)
        .iter()
        .map(|(_, relative, _)| display_path(relative))
        .collect()
}

/// `archive` relative path with `/` separators (zip entry name, messages).
pub(crate) fn display_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

impl WolfSources {
    pub(crate) fn load(game_dir: &Path) -> Result<Self, SourcesError> {
        let mut sources = Self::default();

        for (absolute, relative, prefix) in candidate_archives(game_dir) {
            let archive_name = display_path(&relative);
            let data = std::fs::read(&absolute)?;
            let unplaced = RefCell::new(Vec::new());
            let wanted = |inner: &str| {
                let logical = format!("{prefix}{}", inner.trim_start_matches('/'));
                let keep = is_wanted(&logical);
                if !keep && logical.to_ascii_lowercase().ends_with(".mps") {
                    unplaced.borrow_mut().push(inner.to_string());
                }
                keep
            };
            let archive =
                extract_matching(&data, &wanted).map_err(|e| SourcesError::UnreadableArchive {
                    archive: archive_name.clone(),
                    reason: e.to_string(),
                })?;
            drop(data);

            let unplaced = unplaced.into_inner();
            let placed_maps = archive
                .files
                .iter()
                .any(|file| file.path.to_ascii_lowercase().ends_with(".mps"));
            if !placed_maps && !unplaced.is_empty() {
                return Err(SourcesError::UnplacedMaps {
                    archive: archive_name,
                    paths: unplaced
                        .iter()
                        .take(3)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", "),
                });
            }

            for file in archive.files {
                let logical = format!("{prefix}{}", file.path.trim_start_matches('/'));
                sources
                    .files
                    .entry(logical.to_ascii_lowercase())
                    .or_insert(SourceFile {
                        logical,
                        bytes: file.data,
                        origin: Origin::Archive {
                            archive: relative.clone(),
                            path: file.path,
                        },
                    });
            }
        }

        if let Some(data_dir) = find_wolf_data_dir(game_dir) {
            for folder in ["MapData", "BasicData"] {
                let Ok(entries) = std::fs::read_dir(data_dir.join(folder)) else {
                    continue;
                };
                for entry in entries.flatten() {
                    let logical = format!("{folder}/{}", entry.file_name().to_string_lossy());
                    let key = logical.to_ascii_lowercase();
                    if !is_wanted(&logical) || sources.files.contains_key(&key) {
                        continue;
                    }
                    let bytes = std::fs::read(entry.path())?;
                    sources.files.insert(
                        key,
                        SourceFile {
                            logical,
                            bytes,
                            origin: Origin::Loose,
                        },
                    );
                }
            }
        }

        Ok(sources)
    }

    /// The file at `logical` (relative to `Data/`, any case).
    pub(crate) fn get(&self, logical: &str) -> Option<&SourceFile> {
        self.files.get(&logical.to_ascii_lowercase())
    }

    /// Every map, in path order.
    pub(crate) fn maps(&self) -> impl Iterator<Item = &SourceFile> {
        self.files
            .iter()
            .filter(|(key, _)| key.starts_with("mapdata/") && key.ends_with(".mps"))
            .map(|(_, file)| file)
    }

    /// Every database as `(project, dat)`, except the engine-internal
    /// `SysDataBaseBasic` (as WolfTL).
    pub(crate) fn databases(&self) -> Vec<(&SourceFile, &SourceFile)> {
        self.files
            .iter()
            .filter(|(key, _)| key.starts_with("basicdata/") && key.ends_with(".project"))
            .filter(|(key, _)| key.as_str() != "basicdata/sysdatabasebasic.project")
            .filter_map(|(key, project)| {
                let dat_key = format!("{}.dat", key.trim_end_matches(".project"));
                self.files.get(&dat_key).map(|dat| (project, dat))
            })
            .collect()
    }

    /// `BasicData/CommonEvent.dat`.
    pub(crate) fn common_events(&self) -> Option<&SourceFile> {
        self.get("BasicData/CommonEvent.dat")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::wolf::decrypt::legacy_xor::test_archives::{make_v6_dx_archive, DxFile};
    use crate::engines::wolf::decrypt::legacy_xor::WOLF_KEYS;

    fn key() -> [u8; 12] {
        WOLF_KEYS
            .iter()
            .find(|(name, _)| *name == "v2.20")
            .map(|(_, key)| *key)
            .unwrap()
    }

    fn archive(files: &[DxFile]) -> Vec<u8> {
        make_v6_dx_archive(&key(), files, 932, true).bytes
    }

    fn file<'a>(dir: Option<&'a str>, name: &'a str, content: &'a [u8]) -> DxFile<'a> {
        DxFile {
            dir,
            name,
            content,
            compress: false,
        }
    }

    fn write(path: &Path, bytes: &[u8]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    #[test]
    fn test_archive_wins_over_loose_file_and_loose_fills_the_rest() {
        let game = tempfile::tempdir().unwrap();
        let data = game.path().join("Data");
        write(
            &game.path().join("Data.wolf"),
            &archive(&[file(Some("MapData"), "Map001.mps", b"from archive")]),
        );
        write(&data.join("MapData").join("Map001.mps"), b"loose copy");
        write(&data.join("MapData").join("Map002.mps"), b"loose only");

        let sources = WolfSources::load(game.path()).unwrap();
        let map1 = sources.get("MapData/Map001.mps").unwrap();
        assert_eq!(map1.bytes, b"from archive");
        assert_eq!(
            map1.origin,
            Origin::Archive {
                archive: PathBuf::from("Data.wolf"),
                path: "/MapData/Map001.mps".into(),
            }
        );
        assert_eq!(map1.stem(), "map001", "archive stems stay lowercase");
        let map2 = sources.get("mapdata/map002.mps").unwrap();
        assert_eq!(map2.bytes, b"loose only");
        assert_eq!(map2.origin, Origin::Loose);
        assert_eq!(map2.stem(), "Map002");
        assert_eq!(sources.maps().count(), 2);
    }

    #[test]
    fn test_split_archives_and_database_pairs() {
        let game = tempfile::tempdir().unwrap();
        let data = game.path().join("Data");
        write(
            &data.join("MapData.wolf"),
            &archive(&[file(None, "Map001.mps", b"map")]),
        );
        write(
            &data.join("BasicData.wolf"),
            &archive(&[
                file(None, "DataBase.project", b"project"),
                file(None, "DataBase.dat", b"dat"),
                file(None, "SysDataBaseBasic.project", b"project"),
                file(None, "SysDataBaseBasic.dat", b"dat"),
                file(None, "CommonEvent.dat", b"events"),
            ]),
        );
        write(&data.join("Picture.wolf"), b"never read");

        let sources = WolfSources::load(game.path()).unwrap();
        let map = sources.maps().next().unwrap();
        assert_eq!(map.logical, "MapData/Map001.mps");
        assert_eq!(
            map.origin,
            Origin::Archive {
                archive: PathBuf::from("Data").join("MapData.wolf"),
                path: "/Map001.mps".into(),
            }
        );
        let databases = sources.databases();
        assert_eq!(databases.len(), 1);
        assert_eq!(databases[0].0.bytes, b"project");
        assert_eq!(databases[0].1.bytes, b"dat");
        assert_eq!(databases[0].1.stem(), "database");
        assert_eq!(sources.common_events().unwrap().bytes, b"events");
    }

    #[test]
    fn test_archive_with_maps_outside_mapdata_is_refused() {
        let game = tempfile::tempdir().unwrap();
        write(
            &game.path().join("Data.wolf"),
            &archive(&[file(Some("Sample"), "Map001.mps", b"map")]),
        );
        match WolfSources::load(game.path()) {
            Err(error @ SourcesError::UnplacedMaps { .. }) => {
                let message = error.to_string();
                assert!(message.starts_with(WOLF_UNREADABLE_ARCHIVE), "{message}");
                assert!(message.contains("/Sample/Map001.mps"), "{message}");
            }
            other => panic!("expected UnplacedMaps, got {other:?}"),
        }
    }

    #[test]
    fn test_unreadable_archive_is_refused() {
        let game = tempfile::tempdir().unwrap();
        let mut header = vec![0u8; 64];
        header[0..4].copy_from_slice(&[b'D', b'X', 8, 0]);
        header[44..48].copy_from_slice(&(350u32 << 16).to_le_bytes());
        write(&game.path().join("Data.wolf"), &header);
        write(
            &game.path().join("Data").join("MapData").join("Map001.mps"),
            b"decrypted next to the archive",
        );
        match WolfSources::load(game.path()) {
            Err(SourcesError::UnreadableArchive { archive, reason }) => {
                assert_eq!(archive, "Data.wolf");
                assert!(reason.contains("350"), "{reason}");
            }
            other => panic!("expected UnreadableArchive, got {other:?}"),
        }
    }
}
