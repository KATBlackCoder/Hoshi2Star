//! Engine detector — identifies the game engine from a folder path.
//!
//! ## Detection logic
//! **RPG Maker MV/MZ**:
//!   - `<dir>/data/System.json` **or** `<dir>/www/data/System.json` exists
//!   - That `System.json` contains a `"gameTitle"` field
//!
//! Returns `Engine::MvMz` on success or `DetectionError::UnknownEngine` if
//! no known engine is found.

use std::path::Path;

/// Supported game engines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Engine {
    MvMz,
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
/// Reads `System.json` from `<dir>/data/` or `<dir>/www/data/` and checks
/// for the `gameTitle` field which is present in all MV/MZ games.
pub fn detect_engine(game_dir: &Path) -> Result<Engine, DetectionError> {
    let data_dir = find_data_dir(game_dir).ok_or(DetectionError::UnknownEngine)?;

    let system_path = data_dir.join("System.json");
    if system_path.exists() {
        let content = std::fs::read_to_string(&system_path)?;
        if is_mv_mz_system(&content) {
            return Ok(Engine::MvMz);
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
}
