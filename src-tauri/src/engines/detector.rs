//! Engine detector — identifies the game engine from a folder path.
//!
//! ## Detection logic (order matters — MV/MZ checked first)
//!
//! **RPG Maker MV/MZ**:
//!   - `<dir>/data/System.json` **or** `<dir>/www/data/System.json` exists
//!   - That `System.json` contains a `"gameTitle"` field
//!
//! **RPG Maker VX Ace**:
//!   - `<dir>/Data/System.rvdata2` exists (capital D — VX Ace convention)
//!   - Fallback: `<dir>/data/System.rvdata2` for games extracted on Linux
//!     where the directory case was not preserved.
//!
//! Returns `DetectionError::UnknownEngine` if no known engine is found.

use std::path::Path;

/// Supported game engines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Engine {
    MvMz,
    VxAce,
    Wolf,
}

impl Engine {
    /// Stable identifier stored in the `projects.engine` column.
    pub fn db_str(&self) -> &'static str {
        match self {
            Engine::MvMz => "mv_mz",
            Engine::VxAce => "vx_ace",
            Engine::Wolf => "wolf",
        }
    }

    /// Locate the engine's data directory relative to the game root.
    ///
    /// MV/MZ: `data/` or `www/data/`. VX Ace: `Data/` (lowercase fallback).
    /// Wolf reads from the game root directly, so its data dir is the game root.
    pub fn data_dir(&self, game_dir: &Path) -> Result<std::path::PathBuf, String> {
        match self {
            Engine::MvMz => find_data_dir(game_dir)
                .ok_or_else(|| "Cannot find data directory in game folder".to_string()),
            Engine::VxAce => find_vx_ace_data_dir(game_dir)
                .ok_or_else(|| "Cannot find Data/ directory in VX Ace game folder".to_string()),
            Engine::Wolf => Ok(game_dir.to_path_buf()),
        }
    }

    /// Read the game title from the engine's metadata file, or `None` if absent.
    pub fn game_title(&self, game_dir: &Path, data_dir: &Path) -> Option<String> {
        match self {
            Engine::MvMz => read_mv_mz_game_title(&data_dir.join("System.json")),
            Engine::VxAce => read_vx_ace_game_title(&data_dir.join("System.rvdata2")),
            Engine::Wolf => read_wolf_game_title(game_dir),
        }
    }
}

/// Read `gameTitle` from a `System.json` path (MV/MZ).
fn read_mv_mz_game_title(system_json_path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(system_json_path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    v.get("gameTitle")?.as_str().map(|s| s.to_string())
}

/// Read `game_title` from a `System.rvdata2` path (VX Ace, snake_case field).
fn read_vx_ace_game_title(system_rvdata2_path: &Path) -> Option<String> {
    let bytes = std::fs::read(system_rvdata2_path).ok()?;
    let mv: marshal_rs::Value = marshal_rs::load_utf8(&bytes, None).ok()?;
    let json: serde_json::Value = mv.into();
    json.get("game_title")?.as_str().map(|s| s.to_string())
}

/// Read `GameTitle` from `Game.ini` in a Wolf RPG game directory.
///
/// `Game.ini` uses Shift-JIS encoding for v2 games. We read as bytes and
/// attempt Shift-JIS decoding; falls back to UTF-8 then lossy on failure.
/// Returns `None` if the file is absent or the key is not found.
fn read_wolf_game_title(game_dir: &Path) -> Option<String> {
    let ini_path = game_dir.join("Game.ini");
    let bytes = std::fs::read(&ini_path).ok()?;

    // Try Shift-JIS first (most Wolf v2 games), then UTF-8, then lossy.
    let content = {
        use encoding_rs::SHIFT_JIS;
        let (decoded, _, had_errors) = SHIFT_JIS.decode(&bytes);
        if !had_errors {
            decoded.into_owned()
        } else {
            String::from_utf8(bytes.clone())
                .unwrap_or_else(|_| String::from_utf8_lossy(&bytes).into_owned())
        }
    };

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("GameTitle=") {
            let title = rest.trim().to_string();
            if !title.is_empty() {
                return Some(title);
            }
        }
    }
    None
}

#[derive(Debug, thiserror::Error)]
pub enum DetectionError {
    #[error("could not identify game engine in directory")]
    UnknownEngine,
    #[error("I/O error while detecting engine: {0}")]
    Io(#[from] std::io::Error),
}

/// Detect the game engine from a game directory.
///
/// MV/MZ is checked first; VX Ace second. Returns the first match.
pub fn detect_engine(game_dir: &Path) -> Result<Engine, DetectionError> {
    // 1. MV/MZ: look for data/System.json or www/data/System.json
    if let Some(data_dir) = find_data_dir(game_dir) {
        let system_path = data_dir.join("System.json");
        if system_path.exists() {
            let content = std::fs::read_to_string(&system_path)?;
            if is_mv_mz_system(&content) {
                return Ok(Engine::MvMz);
            }
        }
    }

    // 2. Wolf RPG — Game.exe/GamePro.exe/Game.ini + decrypted BasicData/
    if is_wolf_game_dir(game_dir) {
        return Ok(Engine::Wolf);
    }

    // 3. VX Ace détection désactivée temporairement —
    //    code conservé dans engines/vx_ace/ pour réactivation future.
    //    Priorité actuelle : Wolf RPG (F4).
    //    Réactiver en décommentant ce bloc quand VX Ace sera
    //    la prochaine priorité.
    // if let Some(vx_dir) = find_vx_ace_data_dir(game_dir) {
    //     if is_vx_ace_data_dir(&vx_dir) {
    //         return Ok(Engine::VxAce);
    //     }
    // }

    Err(DetectionError::UnknownEngine)
}

/// Check whether a `System.json` content belongs to an RPG Maker MV/MZ game.
///
/// This function is pure (no I/O) and is the primary unit-testable surface.
pub fn is_mv_mz_system(content: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(content)
        .ok()
        .and_then(|v| v.get("gameTitle").cloned())
        .is_some()
}

/// Find the `data/` directory in a game folder (MV/MZ can use `data/` or `www/data/`).
pub fn find_data_dir(game_dir: &Path) -> Option<std::path::PathBuf> {
    let candidates = [game_dir.join("data"), game_dir.join("www").join("data")];
    candidates.into_iter().find(|p| p.is_dir())
}

/// Find the VX Ace `Data/` directory.
///
/// VX Ace uses `Data/` (capital D) on Windows. On Linux (case-sensitive fs),
/// we try `Data/` first, then `data/` as a fallback for games extracted
/// without preserving the original Windows casing.
pub fn find_vx_ace_data_dir(game_dir: &Path) -> Option<std::path::PathBuf> {
    let candidates = [game_dir.join("Data"), game_dir.join("data")];
    candidates.into_iter().find(|p| p.is_dir())
}

/// Returns `true` if `dir` is a VX Ace data directory.
///
/// Criterion: `System.rvdata2` exists in the directory.
pub fn is_vx_ace_data_dir(dir: &Path) -> bool {
    dir.join("System.rvdata2").exists()
}

/// Wolf RPG engine version.
///
/// Determines the text encoding used during extraction and injection.
/// v2 and below use Shift-JIS (cp932); v3+ use UTF-8.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WolfVersion {
    pub major: u8,
    pub minor: u8,
}

impl WolfVersion {
    pub fn is_utf8(&self) -> bool {
        self.major >= 3
    }
}

/// Find the Wolf RPG `Data/` directory in a game folder.
///
/// Wolf RPG uses `Data/` (capital D, Windows convention).
/// Tries `Data/` first, then `data/` as a Linux case-insensitive fallback.
pub fn find_wolf_data_dir(game_dir: &Path) -> Option<std::path::PathBuf> {
    let candidates = [game_dir.join("Data"), game_dir.join("data")];
    candidates.into_iter().find(|p| p.is_dir())
}

/// Stable marker at the start of [`WolfVersionError`] messages, matched by the
/// frontend.
pub const WOLF_NO_DATABASE: &str = "wolf_no_database";

/// The databases whose 9-byte magic declares the text encoding, in the order
/// they are read.
const WOLF_DATABASES: [&str; 3] = ["SysDatabase.dat", "DataBase.dat", "CDataBase.dat"];

#[derive(Debug, thiserror::Error)]
pub enum WolfVersionError {
    /// An archive holding text is still in the game folder (plan UW, UW-D2):
    /// checked first, as a half-decrypted `Data/` may lack its databases.
    #[error(transparent)]
    ArchivePresent(#[from] crate::engines::wolf::sources::SourcesError),
    /// No database of `Data/BasicData/` declares an encoding (plan UW, UW-D4):
    /// guessing one would write Shift-JIS into UTF-8 files, or the reverse.
    #[error(
        "{marker}: no readable Wolf RPG database (SysDatabase.dat, DataBase.dat, CDataBase.dat) \
         in Data/BasicData: decrypt the game with UberWolf first",
        marker = WOLF_NO_DATABASE
    )]
    NoDatabase,
}

/// Detect the Wolf RPG version (text encoding) from the game's decrypted
/// `Data/BasicData/` (plan UW, UW-D4): the magic of the first readable database
/// among [`WOLF_DATABASES`] (any case). Pro games may ship without
/// `SysDatabase.dat`, hence the other two. Without any, the game is refused:
/// there is no default encoding. A text archive still in the game folder is
/// refused first ([`crate::engines::wolf::sources::refuse_text_archives`]).
pub fn guess_wolf_version_from_structure(game_dir: &Path) -> Result<WolfVersion, WolfVersionError> {
    crate::engines::wolf::sources::refuse_text_archives(game_dir)?;
    let basic_data = find_wolf_data_dir(game_dir)
        .map(|data_dir| data_dir.join("BasicData"))
        .ok_or(WolfVersionError::NoDatabase)?;
    let entries: Vec<std::path::PathBuf> = std::fs::read_dir(&basic_data)
        .map(|entries| entries.flatten().map(|entry| entry.path()).collect())
        .unwrap_or_default();
    WOLF_DATABASES
        .iter()
        .filter_map(|name| {
            entries.iter().find(|path| {
                path.file_name()
                    .is_some_and(|file| file.to_string_lossy().eq_ignore_ascii_case(name))
            })
        })
        .find_map(|path| wolf_version_from_database(&read_database_header(path)?))
        .ok_or(WolfVersionError::NoDatabase)
}

/// The first 10 bytes of a database (indicator + magic): a `DataBase.dat` can
/// weigh megabytes.
fn read_database_header(path: &Path) -> Option<[u8; 10]> {
    use std::io::Read;

    let mut header = [0u8; 10];
    std::fs::File::open(path)
        .ok()?
        .read_exact(&mut header)
        .ok()?;
    Some(header)
}

/// Version declared by a Wolf database (`SysDatabase.dat`, `DataBase.dat`,
/// `CDataBase.dat`): byte 5 of its 9-byte magic, `0x00` Shift-JIS / `0x55` UTF-8
/// — the same signal [`super::wolf::dat_parser::parse_database`] uses per file.
fn wolf_version_from_database(bytes: &[u8]) -> Option<WolfVersion> {
    use crate::engines::wolf::dat_parser::{DB_MAGIC_SJIS, DB_MAGIC_UTF8};

    let magic = bytes.get(1..10)?;
    if magic == DB_MAGIC_UTF8 {
        Some(WolfVersion { major: 3, minor: 0 })
    } else if magic == DB_MAGIC_SJIS {
        Some(WolfVersion { major: 2, minor: 0 })
    } else {
        None
    }
}

/// Launchers of a Wolf RPG game: `Game.exe`, `GamePro.exe` (Wolf Pro) or
/// `Game.ini` (plan UW, UW-D3).
const WOLF_LAUNCHERS: [&str; 3] = ["Game.exe", "GamePro.exe", "Game.ini"];

/// Returns `true` if the directory looks like a Wolf RPG game root whose data
/// was decrypted by UberWolf (plan UW, UW-D3).
///
/// Criteria:
///   - `Game.exe`, `GamePro.exe` or `Game.ini` present at root
///   - AND `Data/BasicData/` (or `BasicData/`) holding at least one `.project`
///     or `CommonEvent.dat`.
///
/// A game still packed in `.wolf` archives is not recognised: it must be
/// decrypted with UberWolf first.
pub fn is_wolf_game_dir(game_dir: &Path) -> bool {
    let has_launcher = WOLF_LAUNCHERS
        .iter()
        .any(|name| game_dir.join(name).exists());
    if !has_launcher {
        return false;
    }
    let data_basic_data = find_wolf_data_dir(game_dir).map(|data_dir| data_dir.join("BasicData"));
    data_basic_data
        .into_iter()
        .chain(std::iter::once(game_dir.join("BasicData")))
        .any(|dir| has_wolf_basic_data_files(&dir))
}

/// `true` if `dir` holds a `.project` file or `CommonEvent.dat` (any case).
fn has_wolf_basic_data_files(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries.flatten().any(|entry| {
                let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
                name.ends_with(".project") || name == "commonevent.dat"
            })
        })
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- is_mv_mz_system (pure, no I/O) ---

    #[test]
    fn test_is_mv_mz_system_valid() {
        let json = r#"{"gameTitle": "勇者の物語", "encryptionKey": ""}"#;
        assert!(is_mv_mz_system(json));
    }

    #[test]
    fn test_is_mv_mz_system_empty_title_still_valid() {
        // An empty string is a valid JSON string — gameTitle exists, so it's MV/MZ
        let json = r#"{"gameTitle": ""}"#;
        assert!(is_mv_mz_system(json));
    }

    #[test]
    fn test_is_mv_mz_system_missing_game_title() {
        let json = r#"{"title": "Not an MV game"}"#;
        assert!(!is_mv_mz_system(json));
    }

    #[test]
    fn test_is_mv_mz_system_invalid_json() {
        assert!(!is_mv_mz_system("not json at all"));
    }

    #[test]
    fn test_is_mv_mz_system_empty_string() {
        assert!(!is_mv_mz_system(""));
    }

    // --- detect_engine (requires filesystem) ---

    #[test]
    fn test_detect_engine_mv_data_dir() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().join("data");
        std::fs::create_dir(&data_dir).unwrap();
        std::fs::write(
            data_dir.join("System.json"),
            r#"{"gameTitle": "テストゲーム"}"#,
        )
        .unwrap();

        let engine = detect_engine(dir.path()).unwrap();
        assert_eq!(engine, Engine::MvMz);
    }

    #[test]
    fn test_detect_engine_mv_www_data_dir() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().join("www").join("data");
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::write(data_dir.join("System.json"), r#"{"gameTitle": "MV Game"}"#).unwrap();

        let engine = detect_engine(dir.path()).unwrap();
        assert_eq!(engine, Engine::MvMz);
    }

    #[test]
    fn test_detect_engine_unknown_no_data_dir() {
        let dir = tempfile::tempdir().unwrap();
        let result = detect_engine(dir.path());
        assert!(matches!(result, Err(DetectionError::UnknownEngine)));
    }

    #[test]
    fn test_detect_engine_unknown_no_game_title() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().join("data");
        std::fs::create_dir(&data_dir).unwrap();
        std::fs::write(data_dir.join("System.json"), r#"{"title": "wrong"}"#).unwrap();

        let result = detect_engine(dir.path());
        assert!(matches!(result, Err(DetectionError::UnknownEngine)));
    }

    // --- VX Ace detection ---

    #[test]
    #[ignore = "VX Ace désactivé — réactiver en F5"]
    fn test_detect_vx_ace_data_dir_capital() {
        // VX Ace canonical: Data/ with capital D
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().join("Data");
        std::fs::create_dir(&data_dir).unwrap();
        std::fs::write(data_dir.join("System.rvdata2"), b"mock").unwrap();

        let engine = detect_engine(dir.path()).unwrap();
        assert_eq!(engine, Engine::VxAce);
    }

    #[test]
    #[ignore = "VX Ace désactivé — réactiver en F5"]
    fn test_detect_vx_ace_fallback_lowercase() {
        // Linux fallback: data/ (lowercase) when game was extracted without preserving case
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().join("data");
        std::fs::create_dir(&data_dir).unwrap();
        std::fs::write(data_dir.join("System.rvdata2"), b"mock").unwrap();

        let engine = detect_engine(dir.path()).unwrap();
        assert_eq!(engine, Engine::VxAce);
    }

    #[test]
    fn test_detect_mv_mz_not_confused_with_vx_ace() {
        // data/ with System.json → MvMz, even if rvdata2 present (MV/MZ checked first)
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().join("data");
        std::fs::create_dir(&data_dir).unwrap();
        std::fs::write(data_dir.join("System.json"), r#"{"gameTitle": "MV Game"}"#).unwrap();
        std::fs::write(data_dir.join("System.rvdata2"), b"mock").unwrap();

        let engine = detect_engine(dir.path()).unwrap();
        assert_eq!(engine, Engine::MvMz);
    }

    #[test]
    fn test_detect_unknown_neither() {
        let dir = tempfile::tempdir().unwrap();
        // No data directory at all
        let result = detect_engine(dir.path());
        assert!(matches!(result, Err(DetectionError::UnknownEngine)));
    }

    #[test]
    fn test_detect_vx_ace_no_system_rvdata2() {
        // Data/ exists but System.rvdata2 missing → not VX Ace
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().join("Data");
        std::fs::create_dir(&data_dir).unwrap();
        // Only some other rvdata2 file, not System
        std::fs::write(data_dir.join("Actors.rvdata2"), b"mock").unwrap();

        let result = detect_engine(dir.path());
        assert!(matches!(result, Err(DetectionError::UnknownEngine)));
    }

    // --- Wolf RPG detection (plan UW, UW-D3) ---

    /// A decrypted Wolf game: `launcher` at the root and `Data/BasicData/{file}`.
    fn wolf_game(launcher: &str, basic_data_file: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(launcher), b"mock").unwrap();
        let basic_data = dir.path().join("Data").join("BasicData");
        std::fs::create_dir_all(&basic_data).unwrap();
        std::fs::write(basic_data.join(basic_data_file), b"mock").unwrap();
        dir
    }

    #[test]
    fn test_detect_wolf_with_each_launcher() {
        for launcher in ["Game.exe", "GamePro.exe", "Game.ini"] {
            let dir = wolf_game(launcher, "DataBase.project");
            assert_eq!(
                detect_engine(dir.path()).unwrap(),
                Engine::Wolf,
                "{launcher}"
            );
        }
    }

    /// Wolf Pro games (3.714, 3.723) only ship `GamePro.exe`.
    #[test]
    fn test_detect_wolf_pro_launcher_alone() {
        let dir = wolf_game("GamePro.exe", "CommonEvent.dat");
        assert!(!dir.path().join("Game.exe").exists());
        assert!(!dir.path().join("Game.ini").exists());
        assert_eq!(detect_engine(dir.path()).unwrap(), Engine::Wolf);
    }

    #[test]
    fn test_detect_wolf_with_root_basic_data() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Game.exe"), b"mock").unwrap();
        std::fs::create_dir(dir.path().join("BasicData")).unwrap();
        std::fs::write(
            dir.path().join("BasicData").join("SysDatabase.project"),
            b"mock",
        )
        .unwrap();

        assert_eq!(detect_engine(dir.path()).unwrap(), Engine::Wolf);
    }

    /// A game still packed in `.wolf` archives is not a Wolf game for the app:
    /// it must be decrypted with UberWolf first.
    #[test]
    fn test_detect_wolf_packed_game_is_not_detected() {
        let root_archive = tempfile::tempdir().unwrap();
        std::fs::write(root_archive.path().join("Game.exe"), b"mock").unwrap();
        std::fs::write(root_archive.path().join("Data.wolf"), b"mock").unwrap();
        assert!(matches!(
            detect_engine(root_archive.path()),
            Err(DetectionError::UnknownEngine)
        ));

        let split_archives = tempfile::tempdir().unwrap();
        std::fs::write(split_archives.path().join("Game.exe"), b"mock").unwrap();
        let data = split_archives.path().join("Data");
        std::fs::create_dir(&data).unwrap();
        std::fs::write(data.join("BasicData.wolf"), b"mock").unwrap();
        std::fs::write(data.join("MapData.wolf"), b"mock").unwrap();
        assert!(matches!(
            detect_engine(split_archives.path()),
            Err(DetectionError::UnknownEngine)
        ));
    }

    #[test]
    fn test_detect_wolf_empty_basic_data_is_not_detected() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Game.exe"), b"mock").unwrap();
        std::fs::create_dir_all(dir.path().join("Data").join("BasicData")).unwrap();
        std::fs::create_dir_all(dir.path().join("Data").join("MapData")).unwrap();
        std::fs::write(
            dir.path().join("Data").join("MapData").join("Map001.mps"),
            b"mock",
        )
        .unwrap();

        assert!(matches!(
            detect_engine(dir.path()),
            Err(DetectionError::UnknownEngine)
        ));
    }

    #[test]
    fn test_detect_wolf_no_launcher() {
        // A decrypted BasicData/ alone without a launcher → UnknownEngine
        let dir = wolf_game("Readme.txt", "DataBase.project");

        let result = detect_engine(dir.path());
        assert!(matches!(result, Err(DetectionError::UnknownEngine)));
    }

    #[test]
    fn test_detect_mv_not_confused_with_wolf() {
        // data/System.json with gameTitle → MvMz even if Game.exe + BasicData present
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().join("data");
        std::fs::create_dir(&data_dir).unwrap();
        std::fs::write(data_dir.join("System.json"), r#"{"gameTitle": "MV Game"}"#).unwrap();
        std::fs::write(dir.path().join("Game.exe"), b"mock").unwrap();
        std::fs::create_dir(dir.path().join("BasicData")).unwrap();
        std::fs::write(
            dir.path().join("BasicData").join("DataBase.project"),
            b"mock",
        )
        .unwrap();

        // MV/MZ is checked first — must win
        let engine = detect_engine(dir.path()).unwrap();
        assert_eq!(engine, Engine::MvMz);
    }

    #[test]
    fn test_detect_wolf_not_confused_with_mv() {
        // Wolf layout but NO data/System.json → Wolf
        let dir = wolf_game("Game.exe", "DataBase.project");

        let engine = detect_engine(dir.path()).unwrap();
        assert_eq!(engine, Engine::Wolf);
    }

    // --- WolfVersion ---

    #[test]
    fn test_wolf_version_is_utf8() {
        assert!(WolfVersion { major: 3, minor: 0 }.is_utf8());
        assert!(WolfVersion { major: 4, minor: 0 }.is_utf8());
    }

    #[test]
    fn test_wolf_version_is_shiftjis() {
        assert!(!WolfVersion { major: 2, minor: 0 }.is_utf8());
        assert!(!WolfVersion { major: 1, minor: 0 }.is_utf8());
    }

    #[test]
    fn test_find_wolf_data_dir_capital_d() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("Data")).unwrap();

        let result = find_wolf_data_dir(dir.path()).unwrap();
        assert_eq!(result, dir.path().join("Data"));
    }

    #[test]
    fn test_find_wolf_data_dir_lowercase_fallback() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("data")).unwrap();

        let result = find_wolf_data_dir(dir.path()).unwrap();
        assert_eq!(result, dir.path().join("data"));
    }

    // --- Encoding from the decrypted databases (plan UW, UW-D4) ---

    fn database_bytes(magic: &[u8; 9]) -> Vec<u8> {
        let mut bytes = vec![0x00]; // indicator = unencrypted
        bytes.extend_from_slice(magic);
        bytes.push(0xC1); // version (content irrelevant to detection)
        bytes
    }

    /// A game folder with `Data/BasicData/{name}` holding `bytes` for each file.
    fn game_with_databases(files: &[(&str, Vec<u8>)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let basic_data = dir.path().join("Data").join("BasicData");
        std::fs::create_dir_all(&basic_data).unwrap();
        for (name, bytes) in files {
            std::fs::write(basic_data.join(name), bytes).unwrap();
        }
        dir
    }

    /// Regression: an unpacked v3.x/UTF-8 game must not silently fall back to
    /// v2.0/Shift-JIS — that mismatch is what caused `encode_for_wolf` to
    /// Shift-JIS-encode non-ASCII translated characters into a database whose
    /// header still declared UTF-8.
    #[test]
    fn test_guess_wolf_version_loose_utf8_database() {
        use crate::engines::wolf::dat_parser::DB_MAGIC_UTF8;

        let game = game_with_databases(&[("SysDatabase.dat", database_bytes(&DB_MAGIC_UTF8))]);
        assert_eq!(
            guess_wolf_version_from_structure(game.path()).unwrap(),
            WolfVersion { major: 3, minor: 0 }
        );
    }

    #[test]
    fn test_guess_wolf_version_loose_sjis_database() {
        use crate::engines::wolf::dat_parser::DB_MAGIC_SJIS;

        let game = game_with_databases(&[("SysDatabase.dat", database_bytes(&DB_MAGIC_SJIS))]);
        assert_eq!(
            guess_wolf_version_from_structure(game.path()).unwrap(),
            WolfVersion { major: 2, minor: 0 }
        );
    }

    /// Wolf Pro game without `SysDatabase.dat` (ポンコツ 3.714): the encoding
    /// comes from `DataBase.dat`.
    #[test]
    fn test_guess_wolf_version_from_database_dat_alone() {
        use crate::engines::wolf::dat_parser::DB_MAGIC_UTF8;

        let game = game_with_databases(&[
            ("DataBase.project", b"project".to_vec()),
            ("DataBase.dat", database_bytes(&DB_MAGIC_UTF8)),
        ]);
        assert_eq!(
            guess_wolf_version_from_structure(game.path()).unwrap(),
            WolfVersion { major: 3, minor: 0 }
        );
    }

    #[test]
    fn test_guess_wolf_version_from_cdatabase_any_case() {
        use crate::engines::wolf::dat_parser::DB_MAGIC_SJIS;

        let game = game_with_databases(&[("cdatabase.DAT", database_bytes(&DB_MAGIC_SJIS))]);
        assert_eq!(
            guess_wolf_version_from_structure(game.path()).unwrap(),
            WolfVersion { major: 2, minor: 0 }
        );
    }

    /// An unreadable `SysDatabase.dat` (unknown magic) does not hide the next base.
    #[test]
    fn test_guess_wolf_version_skips_an_unreadable_database() {
        use crate::engines::wolf::dat_parser::DB_MAGIC_UTF8;

        let game = game_with_databases(&[
            ("SysDatabase.dat", b"garbage".to_vec()),
            ("CDataBase.dat", database_bytes(&DB_MAGIC_UTF8)),
        ]);
        assert_eq!(
            guess_wolf_version_from_structure(game.path()).unwrap(),
            WolfVersion { major: 3, minor: 0 }
        );
    }

    /// No readable database: refused, never a Shift-JIS default (replaces the
    /// former `test_guess_wolf_version_no_signal_defaults_to_v2`).
    #[test]
    fn test_guess_wolf_version_without_database_is_an_error() {
        let empty = tempfile::tempdir().unwrap();
        let only_events = game_with_databases(&[
            ("CommonEvent.dat", b"events".to_vec()),
            ("SysDatabase.dat", b"garbage".to_vec()),
        ]);
        for game in [&empty, &only_events] {
            match guess_wolf_version_from_structure(game.path()) {
                Err(error @ WolfVersionError::NoDatabase) => {
                    assert!(error.to_string().starts_with(WOLF_NO_DATABASE), "{error}");
                }
                other => panic!("expected NoDatabase, got {other:?}"),
            }
        }
    }

    /// An archive is never read for the encoding, and a text archive is
    /// reported before a missing database (half-decrypted `Data/`).
    #[test]
    fn test_guess_wolf_version_reports_text_archives_first() {
        use crate::engines::wolf::sources::WOLF_ARCHIVE_PRESENT;

        let packed = tempfile::tempdir().unwrap();
        std::fs::write(packed.path().join("Data.wolf"), b"DX\x08\x00").unwrap();
        let half = game_with_databases(&[("DataBase.project", b"project".to_vec())]);
        std::fs::write(half.path().join("Data").join("BasicData.wolf"), b"").unwrap();
        for game in [&packed, &half] {
            match guess_wolf_version_from_structure(game.path()) {
                Err(error @ WolfVersionError::ArchivePresent(_)) => {
                    assert!(
                        error.to_string().starts_with(WOLF_ARCHIVE_PRESENT),
                        "{error}"
                    );
                }
                other => panic!("expected ArchivePresent, got {other:?}"),
            }
        }
    }
}
