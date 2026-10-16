//! Pristine snapshots of the MV/MZ game files an export may rewrite.
//!
//! Export must start from the game's *original* files, never from whatever is
//! currently on disk: users unzip `hoshi2star.zip` over the game folder, so a
//! later export would otherwise inject into an already-translated file (and
//! linked rewrites or fill-ins could no longer be proven correct). Each
//! project keeps a snapshot directory:
//!
//! ```text
//! <data_dir>/originals/<project_id>/snapshot.json    manifest (sha256, provenance, drift)
//! <data_dir>/originals/<project_id>/<game-relative path>
//! ```
//!
//! For projects exported before snapshots existed, [`reconstruct_json`]
//! rebuilds the original exactly: Hoshi2Star only ever rewrote the string
//! leaves it extracted, so resetting each of those pointers to its recorded
//! `source_text` restores the file. A pointer whose disk value cannot be
//! explained is recorded as *drift* (the game itself changed) instead of being
//! guessed.
//!
//! Each entry also remembers the checksums of game-folder versions known to
//! correspond to it (the disk at capture, every export written from it). A
//! disk file matching none of them is rebuilt and compared: when the game
//! itself was updated, the snapshot follows (see `core::originals`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub const MANIFEST_FILE: &str = "snapshot.json";
const MANIFEST_VERSION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum OriginalsError {
    #[error("snapshot I/O for {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("snapshot manifest is unreadable: {0}")]
    Manifest(String),
    #[error("{0} is not valid JSON: {1}")]
    InvalidJson(String, String),
    #[error("no snapshot recorded for {0}")]
    Missing(String),
    #[error("snapshot of {rel} is corrupt (expected sha256 {expected}, found {actual})")]
    Corrupt {
        rel: String,
        expected: String,
        actual: String,
    },
    #[error("unsafe snapshot path: {0}")]
    UnsafePath(String),
}

/// How a snapshot file was obtained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    /// Disk bytes copied verbatim: every extracted pointer still held its source.
    Pristine,
    /// At least one pointer held a previous Hoshi2Star export and was reset.
    Reconstructed,
}

/// A core pointer whose disk value matched neither its source nor any
/// Hoshi2Star output — evidence that the game file itself changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriftedPointer {
    pub key: String,
    pub expected: String,
    pub found: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotEntry {
    pub sha256: String,
    pub provenance: Provenance,
    #[serde(default)]
    pub drift: Vec<DriftedPointer>,
    /// sha256 of game-folder bytes known to correspond to this original: the
    /// disk at capture, exports written from it, older exports found on disk.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub known_disk: Vec<String>,
}

/// Remembered game-folder checksums per file (oldest dropped first).
const KNOWN_DISK_LIMIT: usize = 16;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotManifest {
    pub version: u32,
    pub files: BTreeMap<String, SnapshotEntry>,
}

/// Filesystem store for one project's pristine files.
#[derive(Debug)]
pub struct SnapshotStore {
    root: PathBuf,
    manifest: SnapshotManifest,
}

impl SnapshotStore {
    /// Open (or start) the snapshot rooted at `root`. A missing directory is an
    /// empty snapshot; an unreadable manifest is an error, never a silent reset.
    pub fn open(root: PathBuf) -> Result<Self, OriginalsError> {
        let manifest_path = root.join(MANIFEST_FILE);
        let manifest = match std::fs::read(&manifest_path) {
            Ok(bytes) => serde_json::from_slice::<SnapshotManifest>(&bytes)
                .map_err(|error| OriginalsError::Manifest(error.to_string()))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => SnapshotManifest {
                version: MANIFEST_VERSION,
                files: BTreeMap::new(),
            },
            Err(source) => {
                return Err(OriginalsError::Io {
                    path: manifest_path.display().to_string(),
                    source,
                })
            }
        };
        if manifest.version != MANIFEST_VERSION {
            return Err(OriginalsError::Manifest(format!(
                "unsupported manifest version {}",
                manifest.version
            )));
        }
        Ok(Self { root, manifest })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn entry(&self, rel: &str) -> Option<&SnapshotEntry> {
        self.manifest.files.get(rel)
    }

    pub fn entries(&self) -> impl Iterator<Item = (&str, &SnapshotEntry)> {
        self.manifest
            .files
            .iter()
            .map(|(rel, entry)| (rel.as_str(), entry))
    }

    /// `true` when `rel` is recorded *and* its file exists.
    pub fn contains(&self, rel: &str) -> bool {
        self.manifest.files.contains_key(rel)
            && self
                .file_path(rel)
                .map(|path| path.is_file())
                .unwrap_or(false)
    }

    /// Store `bytes` as the original of `rel` (written atomically) and record it.
    pub fn put(
        &mut self,
        rel: &str,
        bytes: &[u8],
        provenance: Provenance,
        drift: Vec<DriftedPointer>,
    ) -> Result<(), OriginalsError> {
        let path = self.file_path(rel)?;
        write_atomic(&path, bytes)?;
        self.manifest.files.insert(
            rel.to_string(),
            SnapshotEntry {
                sha256: sha256_hex(bytes),
                provenance,
                drift,
                known_disk: Vec::new(),
            },
        );
        Ok(())
    }

    /// `true` when game-folder bytes with checksum `disk_sha256` are this
    /// original itself or a version already known to correspond to it.
    pub fn is_known_disk(&self, rel: &str, disk_sha256: &str) -> bool {
        self.manifest.files.get(rel).is_some_and(|entry| {
            entry.sha256 == disk_sha256 || entry.known_disk.iter().any(|sha| sha == disk_sha256)
        })
    }

    /// Record that game-folder bytes with checksum `disk_sha256` correspond to
    /// the original of `rel` (an export of it, or the disk it was taken from).
    /// The caller saves the store.
    pub fn remember_disk(&mut self, rel: &str, disk_sha256: &str) {
        let Some(entry) = self.manifest.files.get_mut(rel) else {
            return;
        };
        if entry.sha256 == disk_sha256 {
            return;
        }
        entry.known_disk.retain(|sha| sha != disk_sha256);
        entry.known_disk.push(disk_sha256.to_string());
        if entry.known_disk.len() > KNOWN_DISK_LIMIT {
            let excess = entry.known_disk.len() - KNOWN_DISK_LIMIT;
            entry.known_disk.drain(..excess);
        }
    }

    /// Read the original of `rel`, verifying its recorded checksum.
    pub fn read(&self, rel: &str) -> Result<Vec<u8>, OriginalsError> {
        let entry = self
            .manifest
            .files
            .get(rel)
            .ok_or_else(|| OriginalsError::Missing(rel.to_string()))?;
        let path = self.file_path(rel)?;
        let bytes = std::fs::read(&path).map_err(|source| OriginalsError::Io {
            path: path.display().to_string(),
            source,
        })?;
        let actual = sha256_hex(&bytes);
        if actual != entry.sha256 {
            return Err(OriginalsError::Corrupt {
                rel: rel.to_string(),
                expected: entry.sha256.clone(),
                actual,
            });
        }
        Ok(bytes)
    }

    /// Persist the manifest (atomically).
    pub fn save(&self) -> Result<(), OriginalsError> {
        let bytes = serde_json::to_vec_pretty(&self.manifest)
            .map_err(|error| OriginalsError::Manifest(error.to_string()))?;
        write_atomic(&self.root.join(MANIFEST_FILE), &bytes)
    }

    fn file_path(&self, rel: &str) -> Result<PathBuf, OriginalsError> {
        let mut path = self.root.clone();
        for part in rel.split('/') {
            if part.is_empty()
                || part == "."
                || part == ".."
                || part.contains('\\')
                || part.contains(':')
            {
                return Err(OriginalsError::UnsafePath(rel.to_string()));
            }
            path.push(part);
        }
        Ok(path)
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), OriginalsError> {
    let io = |source| OriginalsError::Io {
        path: path.display().to_string(),
        source,
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(io)?;
    }
    let temporary = path.with_extension("h2s-tmp");
    std::fs::write(&temporary, bytes).map_err(io)?;
    std::fs::rename(&temporary, path).map_err(io)
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

// ---------------------------------------------------------------------------
// Reconstruction
// ---------------------------------------------------------------------------

/// One core-extracted pointer with what Hoshi2Star knows about it.
#[derive(Debug, Clone)]
pub struct CoreSegment {
    pub key: String,
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reconstruction {
    pub bytes: Vec<u8>,
    pub provenance: Provenance,
    pub drift: Vec<DriftedPointer>,
}

/// `\FS[n]` font prefix Hoshi2Star may have prepended to Map targets at export.
static RE_EXPORT_FONT_PREFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\\FS\[\d+\]").expect("RE_EXPORT_FONT_PREFIX must compile"));

/// Rebuild the original of an MV/MZ JSON file from its on-disk bytes.
///
/// Each core pointer is classified by its disk value:
/// - equal to `source` → untouched;
/// - equal to `target`, or `\FS[n]` + `target` → a previous export, reset;
/// - otherwise, when `source` contains source-language script but the disk
///   value does not → a stale export of an older target (a game update would
///   replace Japanese with Japanese), reset;
/// - otherwise → drift, left as found and reported.
///
/// When nothing is reset the disk bytes are returned verbatim.
pub fn reconstruct_json(
    file_label: &str,
    disk: &[u8],
    segments: &[CoreSegment],
    source_language: &str,
) -> Result<Reconstruction, OriginalsError> {
    let text = std::str::from_utf8(disk)
        .map_err(|error| OriginalsError::InvalidJson(file_label.to_string(), error.to_string()))?;
    let mut json: Value = serde_json::from_str(text)
        .map_err(|error| OriginalsError::InvalidJson(file_label.to_string(), error.to_string()))?;

    let mut reset = false;
    let mut drift = Vec::new();
    for segment in segments {
        match json.pointer_mut(&segment.key) {
            Some(Value::String(current)) => {
                if *current == segment.source {
                    continue;
                }
                let was_exported = !segment.target.is_empty()
                    && (*current == segment.target
                        || RE_EXPORT_FONT_PREFIX.replace(current, "") == segment.target);
                let stale_export =
                    crate::core::qa::contains_source_script(&segment.source, source_language)
                        && !crate::core::qa::contains_source_script(current, source_language);
                if was_exported || stale_export {
                    *current = segment.source.clone();
                    reset = true;
                } else {
                    drift.push(DriftedPointer {
                        key: segment.key.clone(),
                        expected: segment.source.clone(),
                        found: Some(current.clone()),
                    });
                }
            }
            Some(other) => drift.push(DriftedPointer {
                key: segment.key.clone(),
                expected: segment.source.clone(),
                found: Some(other.to_string()),
            }),
            None => drift.push(DriftedPointer {
                key: segment.key.clone(),
                expected: segment.source.clone(),
                found: None,
            }),
        }
    }

    if !reset {
        return Ok(Reconstruction {
            bytes: disk.to_vec(),
            provenance: Provenance::Pristine,
            drift,
        });
    }
    let bytes = serde_json::to_vec(&json)
        .map_err(|error| OriginalsError::InvalidJson(file_label.to_string(), error.to_string()))?;
    Ok(Reconstruction {
        bytes,
        provenance: Provenance::Reconstructed,
        drift,
    })
}

/// Revert exported Keke command-tag edits in a reconstructed data file:
/// `labels` maps a note pointer to `(occurrence, target, source)` label
/// segments, `reverse_renames` maps translated skill-type names back.
pub fn reverse_keke_edits(
    file_label: &str,
    reconstruction: Reconstruction,
    labels: &std::collections::HashMap<String, Vec<(usize, String, String)>>,
    reverse_renames: &std::collections::HashMap<String, String>,
) -> Result<Reconstruction, OriginalsError> {
    if labels.is_empty() && reverse_renames.is_empty() {
        return Ok(reconstruction);
    }
    let invalid = |reason: String| OriginalsError::InvalidJson(file_label.to_string(), reason);
    let mut json: Value = serde_json::from_slice(&reconstruction.bytes)
        .map_err(|error| invalid(error.to_string()))?;
    let mut changed = false;
    if let Some(entries) = json.as_array_mut() {
        for (index, entry) in entries.iter_mut().enumerate() {
            let pointer = format!("/{index}/note");
            let Some(Value::String(note)) = entry.get_mut("note") else {
                continue;
            };
            let edits = labels.get(&pointer).map(Vec::as_slice).unwrap_or(&[]);
            if let Some(restored) =
                crate::engines::mv_mz::keke::reverse_note_edits(note, edits, reverse_renames)
            {
                *note = restored;
                changed = true;
            }
        }
    }
    if !changed {
        return Ok(reconstruction);
    }
    Ok(Reconstruction {
        bytes: serde_json::to_vec(&json).map_err(|error| invalid(error.to_string()))?,
        provenance: Provenance::Reconstructed,
        drift: reconstruction.drift,
    })
}

/// Undo an export's language switch in a `System.json` rebuilt from disk: a
/// `locale` equal to the export target while the snapshot holds another one
/// was written by Hoshi2Star, not by the game.
pub fn revert_locale_switch(
    file_label: &str,
    reconstruction: Reconstruction,
    snapshot: &[u8],
    target_locale: &str,
) -> Result<Reconstruction, OriginalsError> {
    let invalid = |reason: String| OriginalsError::InvalidJson(file_label.to_string(), reason);
    let original: Value =
        serde_json::from_slice(snapshot).map_err(|error| invalid(error.to_string()))?;
    let Some(before) = original.get("locale").and_then(Value::as_str) else {
        return Ok(reconstruction);
    };
    let mut json: Value = serde_json::from_slice(&reconstruction.bytes)
        .map_err(|error| invalid(error.to_string()))?;
    match json.get_mut("locale") {
        Some(Value::String(current)) if current != before && current == target_locale => {
            *current = before.to_string();
        }
        _ => return Ok(reconstruction),
    }
    Ok(Reconstruction {
        bytes: serde_json::to_vec(&json).map_err(|error| invalid(error.to_string()))?,
        provenance: Provenance::Reconstructed,
        drift: reconstruction.drift,
    })
}

/// Same original: byte-identical, or equal once parsed as JSON (a rebuilt
/// file is re-serialized compactly, an untouched one keeps its formatting).
pub fn same_content(left: &[u8], right: &[u8]) -> bool {
    if left == right {
        return true;
    }
    match (
        serde_json::from_slice::<Value>(left),
        serde_json::from_slice::<Value>(right),
    ) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

/// One translated `plugins.js` parameter, as recorded in the project.
#[derive(Debug, Clone)]
pub struct PluginSegment {
    pub key: String,
    pub source: String,
    pub target: String,
    /// The original field was empty (the source is the plugin's default).
    pub fill_in: bool,
}

/// Rebuild the original `plugins.js` from its on-disk text: every recorded
/// parameter holding its target is reset to the source (or to empty for a
/// fill-in); anything else unexplained is drift. Untouched text is kept
/// verbatim — the edits are span surgery, like the export itself.
pub fn reconstruct_plugins_js(
    file_label: &str,
    disk: &[u8],
    segments: &[PluginSegment],
) -> Result<Reconstruction, OriginalsError> {
    use crate::engines::mv_mz::plugins_js::{parse_plugin_key, PluginsJs};

    let invalid = |reason: String| OriginalsError::InvalidJson(file_label.to_string(), reason);
    let text = std::str::from_utf8(disk).map_err(|error| invalid(error.to_string()))?;
    let plugins = PluginsJs::parse(text).map_err(|error| invalid(error.to_string()))?;

    let mut edits = Vec::new();
    let mut drift = Vec::new();
    for segment in segments {
        let expected = if segment.fill_in {
            String::new()
        } else {
            segment.source.clone()
        };
        let current = parse_plugin_key(&segment.key).and_then(|(name, occurrence, path)| {
            plugins
                .get(&name, occurrence, &path)
                .ok()
                .map(|value| (name, occurrence, path, value))
        });
        match current {
            Some((_, _, _, value)) if value == expected => {}
            Some((name, occurrence, path, value))
                if !segment.target.is_empty() && value == segment.target =>
            {
                edits.push((name, occurrence, path, expected));
            }
            Some((_, _, _, value)) => drift.push(DriftedPointer {
                key: segment.key.clone(),
                expected,
                found: Some(value),
            }),
            None => drift.push(DriftedPointer {
                key: segment.key.clone(),
                expected,
                found: None,
            }),
        }
    }
    if edits.is_empty() {
        return Ok(Reconstruction {
            bytes: disk.to_vec(),
            provenance: Provenance::Pristine,
            drift,
        });
    }
    let rebuilt = plugins
        .apply_edits(&edits)
        .map_err(|error| invalid(error.to_string()))?;
    Ok(Reconstruction {
        bytes: rebuilt.into_bytes(),
        provenance: Provenance::Reconstructed,
        drift,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(key: &str, source: &str, target: &str) -> CoreSegment {
        CoreSegment {
            key: key.into(),
            source: source.into(),
            target: target.into(),
        }
    }

    #[test]
    fn untouched_file_is_returned_byte_for_byte() {
        let disk = b"{ \"a\" : \"\xe5\x8b\x87\xe8\x80\x85\", \"n\": 1.50 }";
        let rec = reconstruct_json("X.json", disk, &[seg("/a", "勇者", "Hero")], "ja").unwrap();
        assert_eq!(rec.bytes, disk);
        assert_eq!(rec.provenance, Provenance::Pristine);
        assert!(rec.drift.is_empty());
    }

    #[test]
    fn exported_values_including_font_prefixed_ones_are_reset_exactly() {
        let original = r#"{"a":"勇者","b":"行こう","c":"そのまま","note":"<tag:鞭>"}"#;
        let exported = r#"{"a":"Hero","b":"\\FS[22]Let's go","c":"そのまま","note":"<tag:鞭>"}"#;
        let rec = reconstruct_json(
            "Map001.json",
            exported.as_bytes(),
            &[
                seg("/a", "勇者", "Hero"),
                seg("/b", "行こう", "Let's go"),
                seg("/c", "そのまま", ""),
            ],
            "ja",
        )
        .unwrap();
        assert_eq!(rec.provenance, Provenance::Reconstructed);
        assert!(rec.drift.is_empty());
        assert_eq!(String::from_utf8(rec.bytes).unwrap(), original);
    }

    #[test]
    fn stale_english_export_is_reset_but_japanese_change_is_drift() {
        // The target was edited after the last export: disk holds an older
        // English value → still a Hoshi2Star export.
        let disk = r#"{"a":"Old hero","b":"新しい台詞"}"#;
        let rec = reconstruct_json(
            "Map002.json",
            disk.as_bytes(),
            &[seg("/a", "勇者", "Hero"), seg("/b", "古い台詞", "Old line")],
            "ja",
        )
        .unwrap();
        let value: Value = serde_json::from_slice(&rec.bytes).unwrap();
        assert_eq!(value["a"], "勇者");
        // A Japanese→Japanese change is the game itself changing: never guessed.
        assert_eq!(value["b"], "新しい台詞");
        assert_eq!(
            rec.drift,
            vec![DriftedPointer {
                key: "/b".into(),
                expected: "古い台詞".into(),
                found: Some("新しい台詞".into()),
            }]
        );
    }

    #[test]
    fn missing_or_non_string_pointers_are_drift() {
        let rec = reconstruct_json(
            "X.json",
            br#"{"a":1}"#,
            &[seg("/a", "勇者", "Hero"), seg("/gone", "消えた", "")],
            "ja",
        )
        .unwrap();
        assert_eq!(rec.drift.len(), 2);
        assert_eq!(rec.drift[1].found, None);
        assert_eq!(rec.provenance, Provenance::Pristine);
    }

    #[test]
    fn store_round_trips_and_detects_corruption() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("originals").join("p1");
        let mut store = SnapshotStore::open(root.clone()).unwrap();
        assert!(!store.contains("data/System.json"));
        store
            .put(
                "data/System.json",
                b"{\"x\":1}",
                Provenance::Pristine,
                vec![],
            )
            .unwrap();
        store.save().unwrap();

        let reopened = SnapshotStore::open(root.clone()).unwrap();
        assert!(reopened.contains("data/System.json"));
        assert_eq!(reopened.read("data/System.json").unwrap(), b"{\"x\":1}");

        std::fs::write(root.join("data").join("System.json"), b"{\"x\":2}").unwrap();
        assert!(matches!(
            reopened.read("data/System.json"),
            Err(OriginalsError::Corrupt { .. })
        ));
        assert!(matches!(
            reopened.read("data/Nope.json"),
            Err(OriginalsError::Missing(_))
        ));
    }

    #[test]
    fn store_rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = SnapshotStore::open(dir.path().to_path_buf()).unwrap();
        for bad in ["../x.json", "data/../../x", "/abs", "C:/x", "a\\b"] {
            assert!(
                matches!(
                    store.put(bad, b"{}", Provenance::Pristine, vec![]),
                    Err(OriginalsError::UnsafePath(_))
                ),
                "{bad}"
            );
        }
    }

    #[test]
    fn exported_plugins_js_is_reconstructed_by_span_surgery() {
        let original = "// header\r\nvar $plugins =\r\n[\r\n{\"name\":\"A\",\"status\":true,\"description\":\"\",\"parameters\":{\"Cmd\":\"図鑑\",\"Rows\":\"[\\\"{\\\\\\\"label\\\\\\\":\\\\\\\"\\\\\\\"}\\\"]\"}},\r\n{\"name\":\"B\",\"status\":true,\"description\":\"\",\"parameters\":{\"x\":\"未\"}}\r\n];\r\n";
        let plugins = crate::engines::mv_mz::plugins_js::PluginsJs::parse(original).unwrap();
        let exported = plugins
            .apply_edits(&[
                ("A".into(), 1, vec!["Cmd".into()], "Bestiary".into()),
                (
                    "A".into(),
                    1,
                    vec!["Rows".into(), "0".into(), "label".into()],
                    "Kills".into(),
                ),
            ])
            .unwrap();
        let key = |path: &[&str]| {
            crate::engines::mv_mz::plugins_js::plugin_key(
                "A",
                1,
                &path.iter().map(|p| p.to_string()).collect::<Vec<_>>(),
            )
        };
        let rec = reconstruct_plugins_js(
            "js/plugins.js",
            exported.as_bytes(),
            &[
                PluginSegment {
                    key: key(&["Cmd"]),
                    source: "図鑑".into(),
                    target: "Bestiary".into(),
                    fill_in: false,
                },
                PluginSegment {
                    key: key(&["Rows", "0", "label"]),
                    source: "倒した数".into(),
                    target: "Kills".into(),
                    fill_in: true,
                },
            ],
        )
        .unwrap();
        assert_eq!(rec.provenance, Provenance::Reconstructed);
        assert!(rec.drift.is_empty());
        assert_eq!(String::from_utf8(rec.bytes).unwrap(), original);
    }

    #[test]
    fn known_disk_checksums_are_bounded_and_reset_by_a_new_capture() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = SnapshotStore::open(dir.path().to_path_buf()).unwrap();
        store
            .put("data/A.json", b"{}", Provenance::Pristine, vec![])
            .unwrap();
        assert!(store.is_known_disk("data/A.json", &sha256_hex(b"{}")));
        for index in 0..=KNOWN_DISK_LIMIT {
            store.remember_disk("data/A.json", &format!("export-{index}"));
        }
        assert!(!store.is_known_disk("data/A.json", "export-0"));
        assert!(store.is_known_disk("data/A.json", "export-1"));
        store.save().unwrap();

        let mut reopened = SnapshotStore::open(dir.path().to_path_buf()).unwrap();
        assert!(reopened.is_known_disk("data/A.json", "export-1"));
        reopened
            .put("data/A.json", br#"{"v":2}"#, Provenance::Pristine, vec![])
            .unwrap();
        assert!(!reopened.is_known_disk("data/A.json", "export-1"));
        assert!(!reopened.is_known_disk("data/Missing.json", "export-1"));
    }

    #[test]
    fn locale_switch_is_reverted_only_when_it_is_the_export_target() {
        let snapshot = br#"{"locale":"ja_JP","gameTitle":"T"}"#;
        let rebuilt = |disk: &str| Reconstruction {
            bytes: disk.as_bytes().to_vec(),
            provenance: Provenance::Pristine,
            drift: vec![],
        };
        let ours = revert_locale_switch(
            "System.json",
            rebuilt(r#"{"locale":"en_US","gameTitle":"T"}"#),
            snapshot,
            "en_US",
        )
        .unwrap();
        assert!(same_content(&ours.bytes, snapshot));
        // Any other language is the game's own change: kept.
        let theirs = revert_locale_switch(
            "System.json",
            rebuilt(r#"{"locale":"zh_CN","gameTitle":"T"}"#),
            snapshot,
            "en_US",
        )
        .unwrap();
        assert!(!same_content(&theirs.bytes, snapshot));
    }

    #[test]
    fn same_content_ignores_json_formatting_but_not_values() {
        assert!(same_content(
            b"[\nnull,\n{\"a\":1}\n]",
            br#"[null,{"a":1}]"#
        ));
        assert!(same_content(br#"{"a":1,"b":2}"#, br#"{"b":2,"a":1}"#));
        assert!(!same_content(br#"{"a":1}"#, br#"{"a":2}"#));
        assert!(!same_content(b"var x = 1;", b"var x = 1; "));
    }

    #[test]
    fn unreadable_manifest_is_an_error_not_a_reset() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(MANIFEST_FILE), b"not json").unwrap();
        assert!(matches!(
            SnapshotStore::open(dir.path().to_path_buf()),
            Err(OriginalsError::Manifest(_))
        ));
    }
}
