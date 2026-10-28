//! Where a Wolf RPG game's translatable files are read from (plan UW).
//!
//! The game is decrypted with UberWolf first: Hoshi2Star reads and exports
//! only the loose files of `Data/`, never a `.wolf` archive. [`WolfSources`]
//! is loaded once per operation and indexes the maps (`MapData/*.mps`) and the
//! `BasicData/` databases and common events by their path relative to `Data/`.
//! Only the direct children of `MapData/` and `BasicData/` are read: nested
//! copies (sample data shipped by some games) are ignored.
//!
//! The game reads its `.wolf` archives before any loose file of `Data/`
//! (verified in game on Wolf 2.23 and 3.658): while `Data.wolf`,
//! `Data/MapData.wolf` or `Data/BasicData.wolf` is present, a translation
//! written to `Data/` would be ignored, so the operation is refused
//! (decision UW-D2). The other archives (pictures, sounds) hold no text and are
//! ignored.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::engines::detector::find_wolf_data_dir;

/// Stable marker at the start of [`SourcesError::ArchivePresent`], matched by
/// the frontend.
pub const WOLF_ARCHIVE_PRESENT: &str = "wolf_archive_present";

#[derive(Debug, thiserror::Error)]
pub enum SourcesError {
    #[error(
        "{marker}: {archives} still in the game folder: the game would read it instead of \
         the translated Data/ files. Decrypt the game with UberWolf, then move the .wolf \
         archives holding text out of the game folder",
        marker = WOLF_ARCHIVE_PRESENT
    )]
    ArchivePresent { archives: String },
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub(crate) struct SourceFile {
    /// Path relative to `Data/`, e.g. `MapData/Map001.mps`.
    pub logical: String,
    pub bytes: Vec<u8>,
}

impl SourceFile {
    /// File stem used in segment keys, as named on disk.
    pub(crate) fn stem(&self) -> String {
        let name = self.logical.rsplit('/').next().unwrap_or(&self.logical);
        let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
        stem.to_string()
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

/// The archives holding text the game would read before `Data/`, relative to
/// the game folder with `/` separators: `Data.wolf`, `Data/MapData.wolf`,
/// `Data/BasicData.wolf` (any case). Found without reading them.
pub(crate) fn text_archives(game_dir: &Path) -> Vec<String> {
    let mut archives = Vec::new();
    if let Some(path) = find_case_insensitive(game_dir, "Data.wolf") {
        archives.push(file_name(&path));
    }
    if let Some(data_dir) = find_wolf_data_dir(game_dir) {
        for name in ["MapData.wolf", "BasicData.wolf"] {
            if let Some(path) = find_case_insensitive(&data_dir, name) {
                archives.push(format!("{}/{}", file_name(&data_dir), file_name(&path)));
            }
        }
    }
    archives
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

impl WolfSources {
    /// Read the maps, databases and common events of `Data/`; refused while
    /// an archive holding text is present ([`text_archives`]).
    pub(crate) fn load(game_dir: &Path) -> Result<Self, SourcesError> {
        let archives = text_archives(game_dir);
        if !archives.is_empty() {
            return Err(SourcesError::ArchivePresent {
                archives: archives.join(", "),
            });
        }

        let mut sources = Self::default();
        let Some(data_dir) = find_wolf_data_dir(game_dir) else {
            return Ok(sources);
        };
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
                sources.files.insert(key, SourceFile { logical, bytes });
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

    fn write(path: &Path, bytes: &[u8]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    /// A decrypted game: `Data/MapData/` and `Data/BasicData/`.
    fn decrypted_game() -> tempfile::TempDir {
        let game = tempfile::tempdir().unwrap();
        let data = game.path().join("Data");
        write(&data.join("MapData").join("Map001.mps"), b"map 1");
        write(&data.join("MapData").join("TitleMap.mps"), b"title");
        write(&data.join("MapData").join("notes.txt"), b"not a map");
        write(
            &data.join("MapData").join("Old").join("Map009.mps"),
            b"nested",
        );
        let basic_data = data.join("BasicData");
        write(&basic_data.join("DataBase.project"), b"project");
        write(&basic_data.join("DataBase.dat"), b"dat");
        write(&basic_data.join("CDataBase.project"), b"orphan project");
        write(&basic_data.join("SysDataBaseBasic.project"), b"project");
        write(&basic_data.join("SysDataBaseBasic.dat"), b"dat");
        write(&basic_data.join("CommonEvent.dat"), b"events");
        game
    }

    #[test]
    fn test_loose_files_are_read_from_data() {
        let game = decrypted_game();
        let sources = WolfSources::load(game.path()).unwrap();

        let maps: Vec<_> = sources.maps().map(|map| map.logical.as_str()).collect();
        assert_eq!(maps, ["MapData/Map001.mps", "MapData/TitleMap.mps"]);
        let map = sources.get("mapdata/map001.mps").unwrap();
        assert_eq!(map.bytes, b"map 1");
        assert_eq!(map.stem(), "Map001", "stems keep their case");
        assert_eq!(
            sources.get("MapData/TitleMap.mps").unwrap().stem(),
            "TitleMap"
        );

        let databases = sources.databases();
        assert_eq!(
            databases.len(),
            1,
            "orphans and SysDataBaseBasic are skipped"
        );
        assert_eq!(databases[0].0.bytes, b"project");
        assert_eq!(databases[0].1.bytes, b"dat");
        assert_eq!(databases[0].1.stem(), "DataBase");
        assert_eq!(sources.common_events().unwrap().bytes, b"events");
    }

    #[test]
    fn test_game_without_data_has_no_files() {
        let game = tempfile::tempdir().unwrap();
        let sources = WolfSources::load(game.path()).unwrap();
        assert_eq!(sources.maps().count(), 0);
        assert!(sources.databases().is_empty());
        assert!(sources.common_events().is_none());
    }

    /// Decision UW-D2: an archive holding text refuses the operation, whatever
    /// its case, even next to a decrypted `Data/`.
    #[test]
    fn test_text_archive_is_refused() {
        for (archive, shown) in [
            ("Data.wolf", "Data.wolf"),
            ("data.WOLF", "data.WOLF"),
            ("Data/MapData.wolf", "Data/MapData.wolf"),
            ("Data/BasicData.wolf", "Data/BasicData.wolf"),
        ] {
            let game = decrypted_game();
            write(&game.path().join(archive), b"never read");
            match WolfSources::load(game.path()) {
                Err(error @ SourcesError::ArchivePresent { .. }) => {
                    let message = error.to_string();
                    assert!(message.starts_with(WOLF_ARCHIVE_PRESENT), "{message}");
                    assert!(message.contains(shown), "{message}");
                    assert!(message.contains("UberWolf"), "{message}");
                }
                other => panic!("{archive}: expected ArchivePresent, got {other:?}"),
            }
        }
    }

    #[test]
    fn test_every_text_archive_is_listed() {
        let game = decrypted_game();
        write(&game.path().join("Data.wolf"), b"");
        write(&game.path().join("Data").join("MapData.wolf"), b"");
        write(&game.path().join("Data").join("BasicData.wolf"), b"");
        assert_eq!(
            text_archives(game.path()),
            ["Data.wolf", "Data/MapData.wolf", "Data/BasicData.wolf"]
        );
    }

    /// Archives of pictures or sounds hold no text: ignored.
    #[test]
    fn test_other_archives_are_ignored() {
        let game = decrypted_game();
        let data = game.path().join("Data");
        write(&data.join("Picture.wolf"), b"never read");
        write(&data.join("BGM.wolf"), b"never read");
        assert!(text_archives(game.path()).is_empty());
        let sources = WolfSources::load(game.path()).unwrap();
        assert_eq!(sources.maps().count(), 2);
    }
}
