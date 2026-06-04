//! Project manifest — writes `.hoshi2star.json` at the game folder root.
//!
//! The manifest is optional: any read/write error is logged as a warning and
//! silently ignored. `open_project` must never fail because of a manifest error.

use crate::utils::time::now_iso8601;
use serde::{Deserialize, Serialize};
use std::io::{self, ErrorKind};
use std::path::Path;

// ---------------------------------------------------------------------------
// Version constant
// ---------------------------------------------------------------------------

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestStats {
    pub file_count: u32,
    pub segment_count: u32,
    pub translated_count: u32,
    pub glossary_term_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestData {
    pub project_id: String,
    pub game_title: String,
    pub engine: String,
    pub game_path: String,
    pub hoshi2star_version: String,
    pub created_at: String,
    pub last_opened_at: String,
    pub stats: ManifestStats,
}

impl ManifestData {
    pub fn new(
        project_id: String,
        game_title: String,
        engine: String,
        game_path: String,
        stats: ManifestStats,
    ) -> Self {
        let now = now_iso8601();
        Self {
            project_id,
            game_title,
            engine,
            game_path,
            hoshi2star_version: VERSION.to_string(),
            created_at: now.clone(),
            last_opened_at: now,
            stats,
        }
    }
}

// ---------------------------------------------------------------------------
// Public functions
// ---------------------------------------------------------------------------

/// Write `data` as pretty-printed JSON to `<game_path>/.hoshi2star.json`.
///
/// Uses `std::fs::write` (sync) — the manifest is < 1 KB and the Rust backend
/// is not subject to Tauri capabilities (those apply to the TS frontend only).
pub fn write_manifest(game_path: &str, data: &ManifestData) -> Result<(), io::Error> {
    let path = Path::new(game_path).join(".hoshi2star.json");
    let json = serde_json::to_string_pretty(data)
        .map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?;
    std::fs::write(path, json)
}

/// Read the manifest at `<game_path>/.hoshi2star.json`.
///
/// Returns `Ok(None)` if the file does not exist or contains invalid JSON
/// (corrupt manifest = normal scenario, must never block project open).
/// Other I/O errors are propagated.
pub fn read_manifest(game_path: &str) -> Result<Option<ManifestData>, io::Error> {
    let path = Path::new(game_path).join(".hoshi2star.json");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    match serde_json::from_str::<ManifestData>(&content) {
        Ok(data) => Ok(Some(data)),
        Err(e) => {
            log::warn!("manifest corrupt at {game_path}: {e}");
            Ok(None)
        }
    }
}

/// Update the stats fields of an existing manifest.
///
/// If no manifest exists at `game_path`, does nothing (silently returns `Ok(())`).
pub fn update_stats(game_path: &str, stats: ManifestStats) -> Result<(), io::Error> {
    match read_manifest(game_path)? {
        None => Ok(()),
        Some(mut data) => {
            data.stats = stats;
            data.last_opened_at = now_iso8601();
            write_manifest(game_path, &data)
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_stats() -> ManifestStats {
        ManifestStats {
            file_count: 10,
            segment_count: 500,
            translated_count: 200,
            glossary_term_count: 15,
        }
    }

    fn sample_manifest(dir: &str) -> ManifestData {
        ManifestData::new(
            "test-project-id".to_string(),
            "Test Game".to_string(),
            "mv_mz".to_string(),
            dir.to_string(),
            sample_stats(),
        )
    }

    #[test]
    fn test_write_read_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let game_path = dir.path().to_str().unwrap();
        let data = sample_manifest(game_path);

        write_manifest(game_path, &data).unwrap();

        let read = read_manifest(game_path).unwrap().expect("should be Some");
        assert_eq!(read.project_id, "test-project-id");
        assert_eq!(read.stats.segment_count, 500);
        assert_eq!(read.engine, "mv_mz");
    }

    #[test]
    fn test_read_absent_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let game_path = dir.path().to_str().unwrap();
        let result = read_manifest(game_path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_read_corrupt_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let game_path = dir.path().to_str().unwrap();
        let path = dir.path().join(".hoshi2star.json");
        std::fs::write(&path, "invalid json {{").unwrap();

        let result = read_manifest(game_path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_update_stats_updates_counts() {
        let dir = tempfile::tempdir().unwrap();
        let game_path = dir.path().to_str().unwrap();
        let data = sample_manifest(game_path);
        write_manifest(game_path, &data).unwrap();

        let new_stats = ManifestStats {
            file_count: 10,
            segment_count: 500,
            translated_count: 350,
            glossary_term_count: 15,
        };
        update_stats(game_path, new_stats).unwrap();

        let updated = read_manifest(game_path).unwrap().expect("should be Some");
        assert_eq!(updated.stats.translated_count, 350);
        assert_eq!(updated.project_id, "test-project-id");
    }
}
