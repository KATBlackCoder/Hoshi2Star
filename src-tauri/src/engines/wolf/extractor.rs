// Wolf RPG text extractor — F4-03 implementation.
// helpers load_wolf_archive / find_wolf_file / load_mps_files become used in Step 4+
#![allow(dead_code)]

use super::decryptor::{extract_all, WolfFile};
use std::path::Path;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum ExtractorError {
    #[error("decryptor error: {0}")]
    Decryptor(#[from] super::decryptor::DecryptorError),
    #[error("map parser error: {0}")]
    MapParser(String),
    #[error("encoding error: {0}")]
    Encoding(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unsupported Wolf RPG version: {0}.{1}")]
    UnsupportedVersion(u8, u8),
}

// ---------------------------------------------------------------------------
// Public output types
// ---------------------------------------------------------------------------

/// A single translatable text unit extracted from a Wolf RPG game.
///
/// `key` uniquely addresses this segment for re-injection by the injector (F4-04).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WolfSegment {
    /// Unique key, e.g. "MapData/Map001/events/0/pages/0/42"
    /// or "Database/Actors/0/name"
    pub key: String,
    /// Source text in UTF-8 (decoded from Shift-JIS if the file was v2).
    pub source_text: String,
    /// Segment kind — carries context for the injector and CAT UI.
    pub kind: WolfSegmentKind,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WolfSegmentKind {
    /// Dialogue / in-game displayed text (from .mps map files).
    MapMessage {
        map_name: String,
        event_idx: usize,
        page_idx: usize,
        cmd_idx: usize,
    },
    /// Database field (from .dat database files).
    DatabaseField {
        db_name: String,
        entry_idx: usize,
        field_name: String,
    },
}

// ---------------------------------------------------------------------------
// File access helpers
// ---------------------------------------------------------------------------

/// Decrypt a `.wolf` DXA archive and return its file entries.
pub(crate) fn load_wolf_archive(archive_path: &Path) -> Result<Vec<WolfFile>, ExtractorError> {
    let data = std::fs::read(archive_path)?;
    let archive = extract_all(&data)?;
    Ok(archive.files)
}

/// Find a file in a decrypted archive by name (case-insensitive).
pub(crate) fn find_wolf_file<'a>(files: &'a [WolfFile], name: &str) -> Option<&'a WolfFile> {
    let lower = name.to_lowercase();
    files.iter().find(|f| f.name.to_lowercase() == lower)
}

/// Collect `.mps` files from `Data/MapData/` (unencrypted layout).
///
/// Returns `Vec<(stem_name, raw_bytes)>`. If no `.mps` files are found in the
/// directory the function returns `Err` — encrypted Wolf archives are handled
/// by F4-05 integration.
pub(crate) fn load_mps_files(
    game_dir: &Path,
    _version: &crate::engines::detector::WolfVersion,
) -> Result<Vec<(String, Vec<u8>)>, ExtractorError> {
    let map_dir = game_dir.join("Data").join("MapData");
    if map_dir.exists() {
        let mut result = Vec::new();
        for entry in std::fs::read_dir(&map_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("mps") {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                let bytes = std::fs::read(&path)?;
                result.push((name, bytes));
            }
        }
        if !result.is_empty() {
            return Ok(result);
        }
    }
    // Encrypted .wolf archives: deferred to F4-05 integration.
    Err(ExtractorError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "No .mps files found in Data/MapData/ — encrypted Wolf archives not yet supported in F4-03",
    )))
}
