//! `.h2s` exchange pack — share a project's translation state between users.
//!
//! A pack is a ZIP archive holding only translation DATA (never game files):
//! - `manifest.json`  (required) — format version, engine, identity, stats
//! - `segments.json`  (required) — translated segments keyed by
//!   `(file_name, json_key, source_text)`; segment UUIDs are local to each
//!   database and never cross machines
//! - `glossary.json`  (optional) — project-visible terms
//! - `tm.json`        (optional) — TM entries (opt-in at export)
//!
//! Reading is defensive: entries are decompressed in memory with a hard size
//! cap (zip-bomb guard), archive paths are never extracted to disk, unknown
//! JSON fields are ignored (forward compatibility with minor versions) and a
//! pack written by a NEWER format version is rejected with a dedicated error.

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::io::{Read as _, Seek, Write as _};
use std::path::Path;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Version of the pack format itself (independent of the app version).
pub const FORMAT_VERSION: u32 = 1;

/// Hard cap on the decompressed size of a single pack entry (zip-bomb guard).
pub const MAX_ENTRY_BYTES: u64 = 256 * 1024 * 1024;

const MANIFEST_ENTRY: &str = "manifest.json";
const SEGMENTS_ENTRY: &str = "segments.json";
const GLOSSARY_ENTRY: &str = "glossary.json";
const TM_ENTRY: &str = "tm.json";

const VALID_STATUSES: [&str; 4] = ["untranslated", "translated", "reviewed", "needs_review"];

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum H2sPackError {
    /// The file is not a readable ZIP or has no `manifest.json` — the most
    /// common misuse is picking the `export_project` ZIP meant for players.
    #[error("not a Hoshi2Star pack (missing or unreadable manifest.json)")]
    NotAPack,
    /// Pack written by a newer app — the user must update to import it.
    #[error("pack format version {0} is newer than supported version {FORMAT_VERSION}")]
    UnsupportedVersion(u32),
    /// A required entry is absent from the archive.
    #[error("pack entry missing: {0}")]
    MissingEntry(&'static str),
    /// Decompressed entry exceeds [`MAX_ENTRY_BYTES`].
    #[error("pack entry too large: {0}")]
    TooLarge(&'static str),
    /// Entry exists but its JSON does not parse into the expected shape.
    #[error("invalid JSON in {0}: {1}")]
    InvalidJson(&'static str, String),
    /// A segment carries a status outside the four known values.
    #[error("invalid segment status: {0:?}")]
    InvalidStatus(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("zip error: {0}")]
    Zip(String),
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackFileStat {
    pub file_name: String,
    pub file_type: String,
    pub segment_count: u32,
    pub translated_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackManifest {
    pub format_version: u32,
    pub app_version: String,
    pub engine: String,
    pub game_title: String,
    pub lang_pair: String,
    pub created_at: String,
    pub sender_project_id: String,
    #[serde(default)]
    pub files: Vec<PackFileStat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackSegment {
    pub file_name: String,
    pub json_key: String,
    pub source_text: String,
    pub target_text: String,
    pub status: String,
    #[serde(default)]
    pub qa_score: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackGlossaryTerm {
    pub source_text: String,
    pub target_text: String,
    #[serde(default)]
    pub domain: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackTmEntry {
    pub source_text: String,
    pub target_text: String,
    pub engine: String,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
}

fn default_confidence() -> f64 {
    1.0
}

#[derive(Debug, Clone)]
pub struct H2sPack {
    pub manifest: PackManifest,
    pub segments: Vec<PackSegment>,
    pub glossary: Vec<PackGlossaryTerm>,
    pub tm: Vec<PackTmEntry>,
}

// ---------------------------------------------------------------------------
// Write
// ---------------------------------------------------------------------------

/// Write `pack` as a `.h2s` ZIP archive at `path`.
///
/// Optional sections (`glossary.json`, `tm.json`) are omitted when empty.
pub fn write_pack(path: &Path, pack: &H2sPack) -> Result<(), H2sPackError> {
    let file = std::fs::File::create(path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let mut write_entry = |name: &str, bytes: Vec<u8>| -> Result<(), H2sPackError> {
        zip.start_file(name, options)
            .map_err(|e| H2sPackError::Zip(e.to_string()))?;
        zip.write_all(&bytes)?;
        Ok(())
    };

    write_entry(MANIFEST_ENTRY, to_json(MANIFEST_ENTRY, &pack.manifest)?)?;
    write_entry(SEGMENTS_ENTRY, to_json(SEGMENTS_ENTRY, &pack.segments)?)?;
    if !pack.glossary.is_empty() {
        write_entry(GLOSSARY_ENTRY, to_json(GLOSSARY_ENTRY, &pack.glossary)?)?;
    }
    if !pack.tm.is_empty() {
        write_entry(TM_ENTRY, to_json(TM_ENTRY, &pack.tm)?)?;
    }

    zip.finish().map_err(|e| H2sPackError::Zip(e.to_string()))?;
    Ok(())
}

fn to_json<T: Serialize>(entry: &'static str, value: &T) -> Result<Vec<u8>, H2sPackError> {
    serde_json::to_vec_pretty(value).map_err(|e| H2sPackError::InvalidJson(entry, e.to_string()))
}

// ---------------------------------------------------------------------------
// Read
// ---------------------------------------------------------------------------

/// Read and validate a `.h2s` pack from `path`.
///
/// Segments with an empty `target_text` are dropped on read: the format never
/// ships them (nothing to apply) and this removes the "overwrite with empty"
/// hazard even against a hand-crafted pack.
pub fn read_pack(path: &Path) -> Result<H2sPack, H2sPackError> {
    let file = std::fs::File::open(path)?;
    let mut zip = zip::ZipArchive::new(file).map_err(|_| H2sPackError::NotAPack)?;

    let manifest: PackManifest =
        read_json_entry(&mut zip, MANIFEST_ENTRY)?.ok_or(H2sPackError::NotAPack)?;
    if manifest.format_version > FORMAT_VERSION {
        return Err(H2sPackError::UnsupportedVersion(manifest.format_version));
    }

    let mut segments: Vec<PackSegment> = read_json_entry(&mut zip, SEGMENTS_ENTRY)?
        .ok_or(H2sPackError::MissingEntry(SEGMENTS_ENTRY))?;
    for seg in &segments {
        if !VALID_STATUSES.contains(&seg.status.as_str()) {
            return Err(H2sPackError::InvalidStatus(seg.status.clone()));
        }
    }
    segments.retain(|s| !s.target_text.is_empty());

    let glossary: Vec<PackGlossaryTerm> =
        read_json_entry(&mut zip, GLOSSARY_ENTRY)?.unwrap_or_default();
    let tm: Vec<PackTmEntry> = read_json_entry(&mut zip, TM_ENTRY)?.unwrap_or_default();

    Ok(H2sPack {
        manifest,
        segments,
        glossary,
        tm,
    })
}

/// Read one known entry into memory (size-capped) and parse it as JSON.
///
/// Returns `Ok(None)` when the entry is absent — optional sections.
fn read_json_entry<T: DeserializeOwned, R: std::io::Read + Seek>(
    zip: &mut zip::ZipArchive<R>,
    name: &'static str,
) -> Result<Option<T>, H2sPackError> {
    let entry = match zip.by_name(name) {
        Ok(e) => e,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => return Err(H2sPackError::Zip(e.to_string())),
    };
    // The declared size can lie: cap the actual read too.
    if entry.size() > MAX_ENTRY_BYTES {
        return Err(H2sPackError::TooLarge(name));
    }
    let mut buf = Vec::new();
    entry.take(MAX_ENTRY_BYTES + 1).read_to_end(&mut buf)?;
    if buf.len() as u64 > MAX_ENTRY_BYTES {
        return Err(H2sPackError::TooLarge(name));
    }
    serde_json::from_slice(&buf)
        .map(Some)
        .map_err(|e| H2sPackError::InvalidJson(name, e.to_string()))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_pack() -> H2sPack {
        H2sPack {
            manifest: PackManifest {
                format_version: FORMAT_VERSION,
                app_version: "0.4.9".into(),
                engine: "wolf".into(),
                game_title: "Test Game".into(),
                lang_pair: "ja-en".into(),
                created_at: "2026-07-07T00:00:00Z".into(),
                sender_project_id: "sender-uuid".into(),
                files: vec![PackFileStat {
                    file_name: "CommonEvent.dat".into(),
                    file_type: "wolf_dat".into(),
                    segment_count: 2,
                    translated_count: 2,
                }],
            },
            segments: vec![
                PackSegment {
                    file_name: "CommonEvent.dat".into(),
                    json_key: "/1/name".into(),
                    source_text: "こんにちは".into(),
                    target_text: "Hello".into(),
                    status: "translated".into(),
                    qa_score: Some(100),
                },
                PackSegment {
                    file_name: "CommonEvent.dat".into(),
                    json_key: "/2/name".into(),
                    source_text: "さようなら".into(),
                    target_text: "Goodbye".into(),
                    status: "reviewed".into(),
                    qa_score: None,
                },
            ],
            glossary: vec![PackGlossaryTerm {
                source_text: "勇者".into(),
                target_text: "Hero".into(),
                domain: "characters".into(),
            }],
            tm: vec![],
        }
    }

    #[test]
    fn test_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.h2s");
        let pack = sample_pack();

        write_pack(&path, &pack).unwrap();
        let read = read_pack(&path).unwrap();

        assert_eq!(read.manifest.engine, "wolf");
        assert_eq!(read.manifest.format_version, FORMAT_VERSION);
        assert_eq!(read.segments.len(), 2);
        assert_eq!(read.segments[1].status, "reviewed");
        assert_eq!(read.glossary.len(), 1);
        assert!(read.tm.is_empty());
    }

    #[test]
    fn test_not_a_zip_is_not_a_pack() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bogus.h2s");
        std::fs::write(&path, b"definitely not a zip").unwrap();

        assert!(matches!(read_pack(&path), Err(H2sPackError::NotAPack)));
    }

    #[test]
    fn test_zip_without_manifest_is_not_a_pack() {
        // Simulates the classic misuse: picking the export_project ZIP
        // (game files for players) instead of a pack.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("game.zip");
        let file = std::fs::File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("Data/Map001.json", options).unwrap();
        zip.write_all(b"{}").unwrap();
        zip.finish().unwrap();

        assert!(matches!(read_pack(&path), Err(H2sPackError::NotAPack)));
    }

    #[test]
    fn test_newer_format_version_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("future.h2s");
        let mut pack = sample_pack();
        pack.manifest.format_version = FORMAT_VERSION + 1;
        write_pack(&path, &pack).unwrap();

        assert!(matches!(
            read_pack(&path),
            Err(H2sPackError::UnsupportedVersion(v)) if v == FORMAT_VERSION + 1
        ));
    }

    #[test]
    fn test_invalid_status_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad-status.h2s");
        let mut pack = sample_pack();
        pack.segments[0].status = "hacked".into();
        write_pack(&path, &pack).unwrap();

        assert!(matches!(
            read_pack(&path),
            Err(H2sPackError::InvalidStatus(s)) if s == "hacked"
        ));
    }

    #[test]
    fn test_empty_targets_dropped_on_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty-target.h2s");
        let mut pack = sample_pack();
        pack.segments[0].target_text = String::new();
        write_pack(&path, &pack).unwrap();

        let read = read_pack(&path).unwrap();
        assert_eq!(read.segments.len(), 1);
        assert_eq!(read.segments[0].target_text, "Goodbye");
    }

    #[test]
    fn test_unknown_fields_ignored_forward_compat() {
        // A future minor version may add fields — they must not break v1.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("future-minor.h2s");
        let manifest = r#"{
            "formatVersion": 1, "appVersion": "9.9.9", "engine": "wolf",
            "gameTitle": "T", "langPair": "ja-en", "createdAt": "x",
            "senderProjectId": "p", "files": [], "newFancyField": 42
        }"#;
        let segments = r#"[{
            "fileName": "A.dat", "jsonKey": "/1", "sourceText": "あ",
            "targetText": "a", "status": "translated", "shinyExtra": true
        }]"#;
        let file = std::fs::File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("manifest.json", options).unwrap();
        zip.write_all(manifest.as_bytes()).unwrap();
        zip.start_file("segments.json", options).unwrap();
        zip.write_all(segments.as_bytes()).unwrap();
        zip.finish().unwrap();

        let read = read_pack(&path).unwrap();
        assert_eq!(read.segments.len(), 1);
        assert_eq!(read.manifest.app_version, "9.9.9");
    }

    #[test]
    fn test_missing_segments_entry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("no-segments.h2s");
        let file = std::fs::File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("manifest.json", options).unwrap();
        zip.write_all(
            serde_json::to_vec(&sample_pack().manifest)
                .unwrap()
                .as_slice(),
        )
        .unwrap();
        zip.finish().unwrap();

        assert!(matches!(
            read_pack(&path),
            Err(H2sPackError::MissingEntry("segments.json"))
        ));
    }
}
