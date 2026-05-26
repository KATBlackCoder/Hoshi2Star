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

    // 2. VX Ace: look for Data/System.rvdata2 (capital D) or data/System.rvdata2 (fallback)
    if let Some(vx_dir) = find_vx_ace_data_dir(game_dir) {
        if is_vx_ace_data_dir(&vx_dir) {
            return Ok(Engine::VxAce);
        }
    }

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
}
