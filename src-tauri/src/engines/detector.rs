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

    // 2. Wolf RPG — Game.exe/Game.ini + BasicData/ or .wolf/.mps files
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

/// Detect the Wolf RPG version by reading the CodePage field from the first `.wolf` archive.
///
/// Tries plaintext headers first (e.g. Honoka), then cycles through known XOR keys.
/// Falls back to v2.0 (Shift-JIS) on any failure.
pub fn guess_wolf_version_from_structure(game_dir: &Path) -> WolfVersion {
    try_detect_wolf_version_from_dxa(game_dir).unwrap_or(WolfVersion { major: 2, minor: 0 })
}

fn find_first_wolf_file(game_dir: &Path) -> Option<std::path::PathBuf> {
    // Fully-packed single-archive distribution (e.g. Honoka Data.wolf at root)
    let root_wolf = game_dir.join("Data.wolf");
    if root_wolf.exists() {
        return Some(root_wolf);
    }
    // Multi-archive layout: Data/*.wolf
    let data_dir = find_wolf_data_dir(game_dir)?;
    std::fs::read_dir(&data_dir).ok()?.find_map(|e| {
        let path = e.ok()?.path();
        if path.extension()?.to_str()? == "wolf" {
            Some(path)
        } else {
            None
        }
    })
}

fn try_detect_wolf_version_from_dxa(game_dir: &Path) -> Option<WolfVersion> {
    use crate::engines::wolf::decrypt::legacy_xor::{
        code_page_to_wolf_version, read_header, read_signature, WOLF_KEYS,
    };

    let path = find_first_wolf_file(game_dir)?;
    let data = std::fs::read(&path).ok()?;

    // Check signature; v5 has no CodePage field — fall through to default v2.
    let version = read_signature(&data).ok()?;
    if version < 6 {
        return None;
    }

    if data.len() < 0x2C {
        return None;
    }

    // Step A: try reading code_page directly (plaintext header, e.g. Honoka)
    let cp_raw = u32::from_le_bytes([data[0x28], data[0x29], data[0x2A], data[0x2B]]);
    if matches!(cp_raw, 0 | 932 | 65001) {
        return Some(code_page_to_wolf_version(cp_raw));
    }

    // Step B: try each known XOR key
    for &(_, key) in WOLF_KEYS {
        if let Ok((_, _, _, _, _, Some(cp))) = read_header(&data, &key) {
            if matches!(cp, 0 | 932 | 65001) {
                return Some(code_page_to_wolf_version(cp));
            }
        }
    }

    None
}

/// Returns `true` if the directory looks like a Wolf RPG game root.
///
/// Criteria:
///   - `Game.exe` OR `Game.ini` present at root
///   - AND one of:
///     - `BasicData/` directory (unpacked, old layout)
///     - `Data.wolf` file at root (fully-packed single-archive distribution)
///     - `Data/*.wolf` files (unpacked multi-archive layout)
///     - `Data/MapData/*.mps` files (unpacked map data)
pub fn is_wolf_game_dir(game_dir: &Path) -> bool {
    let has_launcher = game_dir.join("Game.exe").exists() || game_dir.join("Game.ini").exists();
    if !has_launcher {
        return false;
    }
    game_dir.join("BasicData").is_dir()
        || game_dir.join("Data.wolf").exists()
        || has_wolf_archives(game_dir)
        || has_mps_files(game_dir)
}

fn has_wolf_archives(game_dir: &Path) -> bool {
    let data_dir = game_dir.join("Data");
    if !data_dir.is_dir() {
        return false;
    }
    std::fs::read_dir(&data_dir)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .any(|e| e.path().extension().and_then(|x| x.to_str()) == Some("wolf"))
        })
        .unwrap_or(false)
}

fn has_mps_files(game_dir: &Path) -> bool {
    let map_dir = game_dir.join("Data").join("MapData");
    if map_dir.is_dir() {
        return std::fs::read_dir(&map_dir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .any(|e| e.path().extension().and_then(|x| x.to_str()) == Some("mps"))
            })
            .unwrap_or(false);
    }
    false
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

    // --- Wolf RPG detection ---

    #[test]
    fn test_detect_wolf_with_game_exe_and_basic_data() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Game.exe"), b"mock").unwrap();
        std::fs::create_dir(dir.path().join("BasicData")).unwrap();

        let engine = detect_engine(dir.path()).unwrap();
        assert_eq!(engine, Engine::Wolf);
    }

    #[test]
    fn test_detect_wolf_with_game_exe_and_wolf_archives() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Game.exe"), b"mock").unwrap();
        let data_dir = dir.path().join("Data");
        std::fs::create_dir(&data_dir).unwrap();
        std::fs::write(data_dir.join("BasicData.wolf"), b"mock").unwrap();

        let engine = detect_engine(dir.path()).unwrap();
        assert_eq!(engine, Engine::Wolf);
    }

    #[test]
    fn test_detect_wolf_with_root_data_wolf_archive() {
        // Fully-packed distribution: single Data.wolf at root (no Data/ subdirectory)
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Game.exe"), b"mock").unwrap();
        std::fs::write(dir.path().join("Data.wolf"), b"mock").unwrap();

        let engine = detect_engine(dir.path()).unwrap();
        assert_eq!(engine, Engine::Wolf);
    }

    #[test]
    fn test_detect_wolf_with_game_ini() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Game.ini"), b"[Config]\nTitle=Test").unwrap();
        std::fs::create_dir(dir.path().join("BasicData")).unwrap();

        let engine = detect_engine(dir.path()).unwrap();
        assert_eq!(engine, Engine::Wolf);
    }

    #[test]
    fn test_detect_wolf_no_launcher() {
        // BasicData/ alone without Game.exe → UnknownEngine
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("BasicData")).unwrap();

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

        // MV/MZ is checked first — must win
        let engine = detect_engine(dir.path()).unwrap();
        assert_eq!(engine, Engine::MvMz);
    }

    #[test]
    fn test_detect_wolf_not_confused_with_mv() {
        // Game.exe + BasicData/ but NO data/System.json → Wolf
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Game.exe"), b"mock").unwrap();
        std::fs::create_dir(dir.path().join("BasicData")).unwrap();

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
}
