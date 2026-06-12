// Wolf RPG text extractor — F4-03/F4-05 implementation.

use super::dat_parser;
use super::decrypt::legacy_xor::extract_all;
use super::v3_format;
use crate::llm::tokenizer::{Engine as TokEngine, Tokenizer};
use std::collections::HashMap;
use std::path::Path;
use wolfrpg_map_parser::{command::Command, common_events_parser, Map};

// ---------------------------------------------------------------------------
// Command signature normalization
// ---------------------------------------------------------------------------

/// Replace unknown D2/D3 `CallCommonEvent` variant bytes before parsing.
///
/// `wolfrpg-map-parser` 0.6.x panics on `0x04D20000` / `0x09D20000`.
/// All D2 variants share the same binary layout — `argument_count` is read
/// from the data stream, not the signature nibble — so remapping the first
/// byte to a known variant is safe and produces identical parse output.
///
/// Signatures are stored big-endian: `[XX D2 00 00]` for CallCommonEvent,
/// `[XX D3 00 00]` for ReserveCommonEvent.
fn normalize_wolf_command_signatures(bytes: &mut [u8]) {
    // Known D2 variants: 0x03, 0x05, 0x06, 0x07, 0x0B
    // Known D3 variants: 0x03
    let len = bytes.len();
    if len < 4 {
        return;
    }
    let mut i = 0;
    while i + 3 < len {
        if bytes[i + 1] == 0xD2 && bytes[i + 2] == 0x00 && bytes[i + 3] == 0x00 {
            let v = bytes[i];
            if !matches!(v, 0x03 | 0x05 | 0x06 | 0x07 | 0x0B) {
                bytes[i] = 0x06; // remap to CallEvent1 (0x06D20000)
            }
        } else if bytes[i + 1] == 0xD3
            && bytes[i + 2] == 0x00
            && bytes[i + 3] == 0x00
            && bytes[i] != 0x03
        {
            bytes[i] = 0x03; // remap to ReserveEvent (0x03D30000)
        }
        i += 1;
    }
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum ExtractorError {
    #[error("decryptor error: {0}")]
    Decryptor(#[from] super::decrypt::legacy_xor::DecryptorError),
    #[error("map parser error: {0}")]
    MapParser(String),
    #[error("dat parse error in {file}: {reason}")]
    DatParse { file: String, reason: String },
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
        type_idx: usize,
        entry_idx: usize,
        field_name: String,
    },
    /// Dialogue text from CommonEvent.dat.
    CommonEventMessage {
        event_name: String,
        event_idx: usize,
        cmd_idx: usize,
    },
}

// ---------------------------------------------------------------------------
// File access helpers
// ---------------------------------------------------------------------------

/// Collect `.mps` files from `Data/MapData/` (unencrypted layout first),
/// falling back to decrypting `.wolf` archives when the directory is absent or empty.
///
/// Returns `Vec<(stem_name, raw_bytes)>`. Returns `Ok(vec![])` if no files are found.
pub(crate) fn load_mps_files(
    game_dir: &Path,
    _version: &crate::engines::detector::WolfVersion,
) -> Result<Vec<(String, Vec<u8>)>, ExtractorError> {
    // 1. Unencrypted layout: Data/MapData/*.mps (Option A — written by inject_all)
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

    // 2. Encrypted archives: Data/*.wolf and root Data.wolf
    Ok(extract_files_from_archives(game_dir, "mps"))
}

// ---------------------------------------------------------------------------
// Archive extraction helpers (F4-05)
// ---------------------------------------------------------------------------

/// Walk all `.wolf` archives in `Data/` (and root `Data.wolf`) and collect
/// files whose extension matches `ext` (lowercase, no dot).
/// Returns `Vec<(stem, bytes)>`.
fn extract_files_from_archives(game_dir: &Path, ext: &str) -> Vec<(String, Vec<u8>)> {
    let mut results = Vec::new();

    let archive_paths = collect_wolf_archive_paths(game_dir);
    for archive_path in archive_paths {
        let Ok(data) = std::fs::read(&archive_path) else {
            continue;
        };
        let Ok(archive) = extract_all(&data) else {
            continue;
        };
        for file in archive.files {
            let lower = file.name.to_lowercase();
            // Strip optional directory prefix (e.g. "MapData/Map001.mps" → "Map001.mps")
            let base = lower.rsplit('/').next().unwrap_or(&lower);
            if base.ends_with(&format!(".{ext}")) {
                let stem = Path::new(base)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                results.push((stem, file.data));
            }
        }
    }

    results
}

/// Walk all `.wolf` archives and pair `.project` + `.dat` files by stem.
/// Returns `Vec<(stem, project_bytes, dat_bytes)>`.
#[allow(clippy::type_complexity)]
fn extract_dat_pairs_from_archives(game_dir: &Path) -> Vec<(String, Vec<u8>, Vec<u8>)> {
    let mut project_map: HashMap<String, Vec<u8>> = HashMap::new();
    let mut dat_map: HashMap<String, Vec<u8>> = HashMap::new();

    let archive_paths = collect_wolf_archive_paths(game_dir);
    for archive_path in archive_paths {
        let Ok(data) = std::fs::read(&archive_path) else {
            continue;
        };
        let Ok(archive) = extract_all(&data) else {
            continue;
        };
        for file in archive.files {
            let lower = file.name.to_lowercase();
            let base = lower.rsplit('/').next().unwrap_or(&lower);
            let path = Path::new(base);
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let stem = stem.to_string();
            if stem == "SysDataBaseBasic" {
                continue;
            }
            match path.extension().and_then(|e| e.to_str()) {
                Some("project") => {
                    project_map.insert(stem, file.data);
                }
                Some("dat") => {
                    dat_map.insert(stem, file.data);
                }
                _ => {}
            }
        }
    }

    project_map
        .into_iter()
        .filter_map(|(stem, project_bytes)| {
            dat_map
                .remove(&stem)
                .map(|dat_bytes| (stem, project_bytes, dat_bytes))
        })
        .collect()
}

/// Collect paths of all `.wolf` archives: `Data/*.wolf` + root `Data.wolf`.
fn collect_wolf_archive_paths(game_dir: &Path) -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();

    // Multi-archive layout: Data/*.wolf
    let data_dir = game_dir.join("Data");
    if let Ok(entries) = std::fs::read_dir(&data_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("wolf") {
                paths.push(path);
            }
        }
    }

    // Single monolithic archive at root: Data.wolf
    let root_wolf = game_dir.join("Data.wolf");
    if root_wolf.exists() {
        paths.push(root_wolf);
    }

    paths
}

/// Load raw bytes for a single `.mps` file by stem.
///
/// Tries `Data/MapData/{stem}.mps` first, then falls back to decrypting archives.
pub(crate) fn load_mps_for_stem(game_dir: &Path, stem: &str) -> Option<Vec<u8>> {
    let unencrypted = game_dir
        .join("Data")
        .join("MapData")
        .join(format!("{stem}.mps"));
    if unencrypted.exists() {
        return std::fs::read(&unencrypted).ok();
    }
    let target = format!("{}.mps", stem.to_lowercase());
    for ap in collect_wolf_archive_paths(game_dir) {
        let Ok(data) = std::fs::read(&ap) else {
            continue;
        };
        let Ok(archive) = extract_all(&data) else {
            continue;
        };
        for file in archive.files {
            let lower = file.name.to_lowercase();
            let base = lower.rsplit('/').next().unwrap_or(&lower);
            if base == target {
                return Some(file.data);
            }
        }
    }
    None
}

/// Load `.project` + `.dat` bytes for a single database stem.
///
/// Tries `Data/BasicData/{stem}.project` + `.dat` first, then archives.
pub(crate) fn load_dat_for_stem(game_dir: &Path, stem: &str) -> Option<(Vec<u8>, Vec<u8>)> {
    let proj = game_dir
        .join("Data")
        .join("BasicData")
        .join(format!("{stem}.project"));
    let dat = game_dir
        .join("Data")
        .join("BasicData")
        .join(format!("{stem}.dat"));
    if proj.exists() && dat.exists() {
        if let (Ok(pb), Ok(db)) = (std::fs::read(&proj), std::fs::read(&dat)) {
            return Some((pb, db));
        }
    }
    // Fallback: scan archives for both files
    let proj_target = format!("{}.project", stem.to_lowercase());
    let dat_target = format!("{}.dat", stem.to_lowercase());
    let mut project_bytes: Option<Vec<u8>> = None;
    let mut dat_bytes: Option<Vec<u8>> = None;
    for ap in collect_wolf_archive_paths(game_dir) {
        let Ok(data) = std::fs::read(&ap) else {
            continue;
        };
        let Ok(archive) = extract_all(&data) else {
            continue;
        };
        for file in archive.files {
            let lower = file.name.to_lowercase();
            let base = lower.rsplit('/').next().unwrap_or(&lower);
            if base == proj_target {
                project_bytes = Some(file.data);
            } else if base == dat_target {
                dat_bytes = Some(file.data);
            }
        }
        if project_bytes.is_some() && dat_bytes.is_some() {
            break;
        }
    }
    project_bytes.zip(dat_bytes)
}

// ---------------------------------------------------------------------------
// Project-level extractor with file grouping (F4-05)
// ---------------------------------------------------------------------------

/// Extract ALL translatable segments grouped by source file.
///
/// Returns `Vec<(file_name, file_type, segments)>` where:
///  - `file_name` is e.g. `"Map001.mps"` or `"Actors.dat"` (display name)
///  - `file_type` is `"wolf_map"` or `"wolf_database"`
pub fn extract_all_wolf(
    game_dir: &Path,
    version: &crate::engines::detector::WolfVersion,
) -> Result<Vec<(String, String, Vec<WolfSegment>)>, ExtractorError> {
    let mut entries: Vec<(String, String, Vec<WolfSegment>)> = Vec::new();

    // Maps
    if let Ok(mps_files) = load_mps_files(game_dir, version) {
        for (name, bytes) in mps_files {
            match extract_map_segments(&name, &bytes, version) {
                Ok(segs) if !segs.is_empty() => {
                    entries.push((format!("{name}.mps"), "wolf_map".to_string(), segs));
                }
                Ok(_) => {}
                Err(e) => eprintln!("warn: skipping {name}.mps — {e}"),
            }
        }
    }

    // Databases
    match load_dat_files(game_dir) {
        Ok(dat_files) => {
            for (name, project_bytes, dat_bytes) in dat_files {
                match extract_database_segments(&name, &project_bytes, &dat_bytes, version) {
                    Ok(segs) if !segs.is_empty() => {
                        entries.push((format!("{name}.dat"), "wolf_database".to_string(), segs));
                    }
                    Ok(_) => {}
                    Err(e) => eprintln!("warn: skipping {name}.dat — {e}"),
                }
            }
        }
        Err(e) => eprintln!("warn: could not load .dat files — {e}"),
    }

    // Common Events
    if let Some(bytes) = load_common_event_bytes(game_dir) {
        match extract_common_events(&bytes, version) {
            Ok(segs) if !segs.is_empty() => {
                entries.push((
                    "CommonEvent.dat".to_string(),
                    "wolf_common_events".to_string(),
                    segs,
                ));
            }
            Ok(_) => {}
            Err(e) => log::warn!("[h2s] CommonEvent extraction failed: {e}"),
        }
    }

    Ok(entries)
}

// ---------------------------------------------------------------------------
// .mps extraction
// ---------------------------------------------------------------------------

/// Extract translatable segments from a `.mps` map file.
///
/// `map_name` is the file stem (e.g. `"Map001"`). `bytes` is the raw file
/// content — the crate handles Shift-JIS decoding internally.
/// `_version` is reserved for future per-version branching.
///
/// `Map::parse` panics on invalid bytes; we capture that with `catch_unwind`
/// and convert it to `ExtractorError::MapParser`.
pub fn extract_map_segments(
    map_name: &str,
    bytes: &[u8],
    _version: &crate::engines::detector::WolfVersion,
) -> Result<Vec<WolfSegment>, ExtractorError> {
    if v3_format::compression::is_lz4_v3(bytes) {
        return extract_map_segments_v3(map_name, bytes);
    }

    // Wrap the panicking parser in catch_unwind.
    let mut bytes_owned = bytes.to_vec();
    normalize_wolf_command_signatures(&mut bytes_owned);
    let map = std::panic::catch_unwind(move || Map::parse(&bytes_owned)).map_err(|e| {
        let msg = e
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| e.downcast_ref::<String>().map(|s| s.as_str()))
            .unwrap_or("unknown panic");
        ExtractorError::MapParser(format!("{map_name}: {msg}"))
    })?;

    let mut segments = Vec::new();

    for (event_idx, event) in map.events().iter().enumerate() {
        for (page_idx, page) in event.pages().iter().enumerate() {
            for (cmd_idx, command) in page.commands().iter().enumerate() {
                match command {
                    Command::ShowMessage(cmd) => {
                        let text = cmd.text();
                        if is_translatable(text) {
                            segments.push(WolfSegment {
                                key: format!(
                                    "MapData/{map_name}/events/{event_idx}/pages/{page_idx}/{cmd_idx}"
                                ),
                                source_text: text.to_owned(),
                                kind: WolfSegmentKind::MapMessage {
                                    map_name: map_name.to_owned(),
                                    event_idx,
                                    page_idx,
                                    cmd_idx,
                                },
                            });
                        }
                    }
                    Command::ShowChoice(cmd) => {
                        for (choice_idx, choice) in cmd.choices().iter().enumerate() {
                            if is_translatable(choice) {
                                segments.push(WolfSegment {
                                    key: format!(
                                        "MapData/{map_name}/events/{event_idx}/pages/{page_idx}/{cmd_idx}/choices/{choice_idx}"
                                    ),
                                    source_text: choice.clone(),
                                    kind: WolfSegmentKind::MapMessage {
                                        map_name: map_name.to_owned(),
                                        event_idx,
                                        page_idx,
                                        cmd_idx,
                                    },
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(segments)
}

/// Extract translatable segments from a v3.x (LZ4-compressed) `.mps` map,
/// using the in-house [`v3_format`] parser. Same `key`/`kind` shape as the
/// v2.x path, so the injector and CAT UI don't need to branch on version.
fn extract_map_segments_v3(
    map_name: &str,
    bytes: &[u8],
) -> Result<Vec<WolfSegment>, ExtractorError> {
    let decompressed = v3_format::compression::decompress_v3(bytes)
        .map_err(|e| ExtractorError::MapParser(format!("{map_name}: {e}")))?;
    let map = v3_format::map::MapV3::parse(&decompressed)
        .map_err(|e| ExtractorError::MapParser(format!("{map_name}: {e}")))?;

    let mut segments = Vec::new();

    for (event_idx, event) in map.events.iter().enumerate() {
        for (page_idx, page) in event.pages.iter().enumerate() {
            for (cmd_idx, command) in page.commands.iter().enumerate() {
                match command.cid {
                    v3_format::command::CID_MESSAGE => {
                        if let Some(text) = command.string_args.first() {
                            if is_translatable(text) {
                                segments.push(WolfSegment {
                                    key: format!(
                                        "MapData/{map_name}/events/{event_idx}/pages/{page_idx}/{cmd_idx}"
                                    ),
                                    source_text: text.clone(),
                                    kind: WolfSegmentKind::MapMessage {
                                        map_name: map_name.to_owned(),
                                        event_idx,
                                        page_idx,
                                        cmd_idx,
                                    },
                                });
                            }
                        }
                    }
                    v3_format::command::CID_CHOICES => {
                        for (choice_idx, choice) in command.string_args.iter().enumerate() {
                            if is_translatable(choice) {
                                segments.push(WolfSegment {
                                    key: format!(
                                        "MapData/{map_name}/events/{event_idx}/pages/{page_idx}/{cmd_idx}/choices/{choice_idx}"
                                    ),
                                    source_text: choice.clone(),
                                    kind: WolfSegmentKind::MapMessage {
                                        map_name: map_name.to_owned(),
                                        event_idx,
                                        page_idx,
                                        cmd_idx,
                                    },
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(segments)
}

fn is_translatable(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.is_empty() && !is_wolf_placeholder_only(text) && !is_resource_path(trimmed)
}

/// Matches a trailing resource file extension (images, audio, video, fonts)
/// commonly used in Wolf RPG database/event fields for graphic, BGM/SE, and
/// font references — these are file paths, not translatable text.
static RE_RESOURCE_EXTENSION: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    regex::Regex::new(r"(?i)\.(png|jpe?g|bmp|gif|ogg|wav|mp3|mid|midi|ttf|otf|avi|webm|mp4)$")
        .unwrap()
});

/// Returns true if `text` ends with a known resource file extension.
fn is_resource_path(text: &str) -> bool {
    RE_RESOURCE_EXTENSION.is_match(text)
}

fn is_wolf_placeholder_only(text: &str) -> bool {
    let tok = Tokenizer::tokenize(text, TokEngine::Wolf);
    if tok.map.is_empty() {
        return false;
    }
    // Strip every ⟦ph_N⟧ token and check whether any real text remains.
    let bare = tok
        .map
        .keys()
        .fold(tok.text.clone(), |s, k| s.replace(k.as_str(), ""));
    bare.trim().is_empty()
}

/// Returns true if `text` contains at least one Japanese character
/// (hiragana, katakana, or CJK unified ideograph).
fn contains_japanese(text: &str) -> bool {
    text.chars().any(|c| {
        matches!(c,
            '\u{3041}'..='\u{3096}'   // hiragana
            | '\u{30A0}'..='\u{30FF}' // katakana
            | '\u{4E00}'..='\u{9FFF}' // CJK unified ideographs
        )
    })
}

/// Returns true if the field name is in the set of known translatable field names.
fn is_known_translatable_field(name: &str) -> bool {
    matches!(
        name,
        "name" | "名前" | "description" | "説明" | "note" | "備考" | "message" | "text"
    )
}

// ---------------------------------------------------------------------------
// .dat Database extraction
// ---------------------------------------------------------------------------

/// Collect `.dat` / `.project` pairs from `Data/BasicData/` (unencrypted layout first),
/// falling back to decrypting `.wolf` archives when the directory is absent or empty.
///
/// Returns `Vec<(stem_name, project_bytes, dat_bytes)>`.
#[allow(clippy::type_complexity)]
pub(crate) fn load_dat_files(
    game_dir: &Path,
) -> Result<Vec<(String, Vec<u8>, Vec<u8>)>, ExtractorError> {
    let basic_data_dir = game_dir.join("Data").join("BasicData");

    if basic_data_dir.exists() {
        let mut result = Vec::new();
        for entry in std::fs::read_dir(&basic_data_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("project") {
                continue;
            }
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            // WolfTL skips SysDataBaseBasic — it only contains engine internals.
            if stem == "SysDataBaseBasic" {
                continue;
            }
            let dat_path = path.with_extension("dat");
            if !dat_path.exists() {
                continue;
            }
            let project_bytes = std::fs::read(&path)?;
            let dat_bytes = std::fs::read(&dat_path)?;
            result.push((stem, project_bytes, dat_bytes));
        }
        if !result.is_empty() {
            return Ok(result);
        }
    }

    // 2. Encrypted archives: extract .project + .dat pairs by stem
    Ok(extract_dat_pairs_from_archives(game_dir))
}

/// Extract translatable string segments from a Wolf RPG Database file pair.
///
/// Only fields that are of String type (indexInfo ≥ 0x07D0) are considered.
/// A value is extracted if the field name is a known translatable name OR if the
/// value contains Japanese characters.  Empty and placeholder-only values are
/// skipped.
pub fn extract_database_segments(
    db_name: &str,
    project_bytes: &[u8],
    dat_bytes: &[u8],
    _version: &crate::engines::detector::WolfVersion,
) -> Result<Vec<WolfSegment>, ExtractorError> {
    let db = dat_parser::parse_database(project_bytes, dat_bytes).map_err(|e| {
        ExtractorError::DatParse {
            file: db_name.to_owned(),
            reason: e.to_string(),
        }
    })?;

    let mut segments = Vec::new();

    for (type_idx, dat_type) in db.types.iter().enumerate() {
        for (entry_idx, entry) in dat_type.entries.iter().enumerate() {
            // Walk string fields in declaration order; string_values are stored in
            // the same sequential order — one value per string-typed active field.
            let mut str_pos = 0usize;
            for field in &dat_type.fields {
                if !field.is_valid() {
                    continue;
                }
                if !field.is_string() {
                    continue;
                }
                let Some(value) = entry.string_values.get(str_pos) else {
                    str_pos += 1;
                    continue;
                };
                str_pos += 1;

                if value.trim().is_empty() {
                    continue;
                }
                if is_wolf_placeholder_only(value) {
                    continue;
                }
                if is_resource_path(value.trim()) {
                    continue;
                }
                if !is_known_translatable_field(&field.name) && !contains_japanese(value) {
                    continue;
                }

                segments.push(WolfSegment {
                    key: format!("Database/{db_name}/{type_idx}/{entry_idx}/{}", field.name),
                    source_text: value.clone(),
                    kind: WolfSegmentKind::DatabaseField {
                        db_name: db_name.to_owned(),
                        type_idx,
                        entry_idx,
                        field_name: field.name.clone(),
                    },
                });
            }
        }
    }

    Ok(segments)
}

// ---------------------------------------------------------------------------
// CommonEvents extraction
// ---------------------------------------------------------------------------

/// Load raw bytes for `CommonEvent.dat`.
///
/// Tries `Data/BasicData/CommonEvent.dat` first, then scans `.wolf` archives.
pub(crate) fn load_common_event_bytes(game_dir: &Path) -> Option<Vec<u8>> {
    let unencrypted = game_dir
        .join("Data")
        .join("BasicData")
        .join("CommonEvent.dat");
    if unencrypted.exists() {
        return std::fs::read(&unencrypted).ok();
    }
    for ap in collect_wolf_archive_paths(game_dir) {
        let Ok(data) = std::fs::read(&ap) else {
            continue;
        };
        let Ok(archive) = extract_all(&data) else {
            continue;
        };
        for file in archive.files {
            let lower = file.name.to_lowercase();
            let base = lower.rsplit('/').next().unwrap_or(&lower);
            if base == "commonevent.dat" {
                return Some(file.data);
            }
        }
    }
    None
}

/// Extract translatable segments from `CommonEvent.dat` bytes.
///
/// Uses `wolfrpg_map_parser::common_events_parser::parse_bytes` wrapped in
/// `catch_unwind` — the parser panics on malformed files, same as `Map::parse`.
pub fn extract_common_events(
    bytes: &[u8],
    _version: &crate::engines::detector::WolfVersion,
) -> Result<Vec<WolfSegment>, ExtractorError> {
    if v3_format::common_events::is_lz4_v3(bytes) {
        return extract_common_events_v3(bytes);
    }

    let bytes_owned = bytes.to_vec();
    let events = std::panic::catch_unwind(move || common_events_parser::parse_bytes(&bytes_owned))
        .map_err(|e| {
            let msg = e
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| e.downcast_ref::<String>().map(|s| s.as_str()))
                .unwrap_or("unknown panic");
            ExtractorError::MapParser(format!("CommonEvent.dat: {msg}"))
        })?;

    let mut segments = Vec::new();

    for (event_idx, event) in events.iter().enumerate() {
        let event_name = event.event_name();
        for (cmd_idx, command) in event.commands().iter().enumerate() {
            match command {
                Command::ShowMessage(cmd) => {
                    let text = cmd.text();
                    if is_translatable(text) {
                        segments.push(WolfSegment {
                            key: format!("CommonEvents/{event_name}/{event_idx}/{cmd_idx}"),
                            source_text: text.to_owned(),
                            kind: WolfSegmentKind::CommonEventMessage {
                                event_name: event_name.to_owned(),
                                event_idx,
                                cmd_idx,
                            },
                        });
                    }
                }
                Command::ShowChoice(cmd) => {
                    for (choice_idx, choice) in cmd.choices().iter().enumerate() {
                        if is_translatable(choice) {
                            segments.push(WolfSegment {
                                key: format!(
                                    "CommonEvents/{event_name}/{event_idx}/{cmd_idx}/choices/{choice_idx}"
                                ),
                                source_text: choice.clone(),
                                kind: WolfSegmentKind::CommonEventMessage {
                                    event_name: event_name.to_owned(),
                                    event_idx,
                                    cmd_idx,
                                },
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(segments)
}

/// Extracts translatable segments from a v3.5 (Inko) `CommonEvent.dat`, via
/// [`v3_format::common_events`]. Each event's commands are a flat list (no
/// pages, unlike `.mps` events).
fn extract_common_events_v3(bytes: &[u8]) -> Result<Vec<WolfSegment>, ExtractorError> {
    let decompressed = v3_format::common_events::decompress(bytes)
        .map_err(|e| ExtractorError::MapParser(format!("CommonEvent.dat: {e}")))?;
    let common_events = v3_format::common_events::CommonEventsV3::parse(&decompressed)
        .map_err(|e| ExtractorError::MapParser(format!("CommonEvent.dat: {e}")))?;

    let mut segments = Vec::new();

    for (event_idx, event) in common_events.events.iter().enumerate() {
        let event_name = &event.name;
        for (cmd_idx, command) in event.commands.iter().enumerate() {
            match command.cid {
                v3_format::command::CID_MESSAGE => {
                    if let Some(text) = command.string_args.first() {
                        if is_translatable(text) {
                            segments.push(WolfSegment {
                                key: format!("CommonEvents/{event_name}/{event_idx}/{cmd_idx}"),
                                source_text: text.clone(),
                                kind: WolfSegmentKind::CommonEventMessage {
                                    event_name: event_name.clone(),
                                    event_idx,
                                    cmd_idx,
                                },
                            });
                        }
                    }
                }
                v3_format::command::CID_CHOICES => {
                    for (choice_idx, choice) in command.string_args.iter().enumerate() {
                        if is_translatable(choice) {
                            segments.push(WolfSegment {
                                key: format!(
                                    "CommonEvents/{event_name}/{event_idx}/{cmd_idx}/choices/{choice_idx}"
                                ),
                                source_text: choice.clone(),
                                kind: WolfSegmentKind::CommonEventMessage {
                                    event_name: event_name.clone(),
                                    event_idx,
                                    cmd_idx,
                                },
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(segments)
}

// ---------------------------------------------------------------------------
// Project-level orchestrator
// ---------------------------------------------------------------------------

/// Extract ALL translatable segments from a Wolf RPG game directory.
///
/// Scans `game_dir/Data/` for unencrypted files:
///   - `MapData/*.mps`        → `extract_map_segments`
///   - `BasicData/*.dat`      → `extract_database_segments` (pairs with `.project`)
///   - `BasicData/CommonEvent.dat` → `extract_common_events` (stub in F4-03)
///
/// Encrypted `.wolf` archives are deferred to F4-05.  Individual file parse
/// errors are logged as warnings and do not abort the overall extraction.
pub fn extract_wolf_project(
    game_dir: &Path,
    version: &crate::engines::detector::WolfVersion,
) -> Result<Vec<WolfSegment>, ExtractorError> {
    let mut segments = Vec::new();

    // --- Maps ---
    // Err = no .mps files found (encrypted layout deferred to F4-05).
    if let Ok(mps_files) = load_mps_files(game_dir, version) {
        for (name, bytes) in mps_files {
            match extract_map_segments(&name, &bytes, version) {
                Ok(segs) => segments.extend(segs),
                Err(e) => eprintln!("warn: skipping {name}.mps — {e}"),
            }
        }
    }

    // --- Databases ---
    match load_dat_files(game_dir) {
        Ok(dat_files) => {
            for (name, project_bytes, dat_bytes) in dat_files {
                match extract_database_segments(&name, &project_bytes, &dat_bytes, version) {
                    Ok(segs) => segments.extend(segs),
                    Err(e) => eprintln!("warn: skipping {name}.dat — {e}"),
                }
            }
        }
        Err(e) => eprintln!("warn: could not load .dat files — {e}"),
    }

    // --- Common Events ---
    if let Some(bytes) = load_common_event_bytes(game_dir) {
        match extract_common_events(&bytes, version) {
            Ok(segs) => segments.extend(segs),
            Err(e) => log::warn!("[h2s] CommonEvent extraction failed: {e}"),
        }
    }

    Ok(segments)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::detector::WolfVersion;

    /// Build a minimal valid .mps with zero events.
    fn make_empty_mps() -> Vec<u8> {
        let mut b = Vec::new();
        // MAP_SIGNATURE (20 bytes)
        b.extend_from_slice(
            b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x57\x4F\x4C\x46\x4D\x00\x00\x00\x00\x00",
        );
        b.extend_from_slice(&[0x00; 5]); // unknown
        b.extend_from_slice(&0u32.to_le_bytes()); // skippable = 0
        b.extend_from_slice(&1u32.to_le_bytes()); // tileset
        b.extend_from_slice(&1u32.to_le_bytes()); // width = 1
        b.extend_from_slice(&1u32.to_le_bytes()); // height = 1
        b.extend_from_slice(&0u32.to_le_bytes()); // event_count = 0
        b.extend_from_slice(&[0x00; 4]); // layer1 (1×1×4)
        b.extend_from_slice(&[0x00; 4]); // layer2
        b.extend_from_slice(&[0x00; 4]); // layer3
        b.push(0x66); // map end
        b
    }

    /// Encode a string as the length-prefixed SJIS format used by wolfrpg-map-parser.
    /// Format: u32_le(len) + sjis_bytes + 0x00 null, where len = sjis_bytes.len() + 1.
    fn encode_mps_string(text: &str) -> Vec<u8> {
        use encoding_rs::SHIFT_JIS;
        let (encoded, _, _) = SHIFT_JIS.encode(text);
        let sjis: &[u8] = &encoded;
        let len = sjis.len() + 1; // +1 for null terminator
        let mut out = Vec::new();
        out.extend_from_slice(&(len as u32).to_le_bytes());
        out.extend_from_slice(sjis);
        out.push(0x00); // null terminator
        out
    }

    /// Build ShowMessage command bytes (the full Command wrapper).
    fn make_show_message_cmd(text: &str) -> Vec<u8> {
        let mut b = Vec::new();
        // Command header: signature BE + 1 padding byte
        b.extend_from_slice(&0x01650000u32.to_be_bytes()); // ShowMessage
        b.push(0x00); // padding
                      // ShowTextCommand body: 2 unknown bytes + string + 1 end byte
        b.push(0x00);
        b.push(0x00);
        b.extend(encode_mps_string(text));
        b.push(0x00); // command end byte
        b
    }

    /// Build a page with a given set of pre-encoded command bytes.
    fn make_page(commands_bytes: &[u8], command_count: u32) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(b"\x79\xff\xff\xff\xff"); // PAGE_SIGNATURE
        b.extend(encode_mps_string("")); // icon = ""
        b.push(0x02); // icon_row raw (→ 0 after (>>1)-1)
        b.push(0x00); // icon_column
        b.push(0x00); // icon_opacity
        b.push(0x00); // icon_blend
        b.push(0x00); // event_trigger
        b.extend_from_slice(&[0x00; 36]); // conditions
        b.extend_from_slice(&[0x00; 6]); // animation+move fields
        b.extend_from_slice(&0u32.to_le_bytes()); // move_count = 0
        b.extend_from_slice(&command_count.to_le_bytes()); // command_count
        b.extend_from_slice(commands_bytes);
        b.extend_from_slice(&0u32.to_le_bytes()); // unknown2
        b.push(0x00); // shadow_graphic
        b.push(0x00); // range_extension_x
        b.push(0x00); // range_extension_y
        b.push(0x7a); // page end
        b
    }

    /// Build an event with a single page.
    fn make_event(page_bytes: &[u8]) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&0x6f393000u32.to_be_bytes()); // EVENT_SIGNATURE
        b.push(0x00); // padding
        b.extend_from_slice(&1u32.to_le_bytes()); // id
        b.extend(encode_mps_string("E")); // name
        b.extend_from_slice(&0u32.to_le_bytes()); // position_x
        b.extend_from_slice(&0u32.to_le_bytes()); // position_y
        b.extend_from_slice(&1u32.to_le_bytes()); // page_count = 1
        b.extend_from_slice(&0u32.to_le_bytes()); // unknown1
        b.extend_from_slice(page_bytes);
        b.push(0x70); // event end
        b
    }

    /// Build a full .mps wrapping the given event bytes.
    fn make_mps_with_event(event_bytes: &[u8]) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(
            b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x57\x4F\x4C\x46\x4D\x00\x00\x00\x00\x00",
        );
        b.extend_from_slice(&[0x00; 5]); // unknown
        b.extend_from_slice(&0u32.to_le_bytes()); // skippable = 0
        b.extend_from_slice(&1u32.to_le_bytes()); // tileset
        b.extend_from_slice(&1u32.to_le_bytes()); // width = 1
        b.extend_from_slice(&1u32.to_le_bytes()); // height = 1
        b.extend_from_slice(&1u32.to_le_bytes()); // event_count = 1
        b.extend_from_slice(&[0x00; 4]); // layer1
        b.extend_from_slice(&[0x00; 4]); // layer2
        b.extend_from_slice(&[0x00; 4]); // layer3
        b.extend_from_slice(event_bytes);
        b.push(0x66); // map end
        b
    }

    fn v2() -> WolfVersion {
        WolfVersion { major: 2, minor: 0 }
    }

    fn v3() -> WolfVersion {
        WolfVersion { major: 3, minor: 0 }
    }

    #[test]
    fn test_extract_map_empty() {
        let bytes = make_empty_mps();
        let segments = extract_map_segments("TestMap", &bytes, &v2()).unwrap();
        assert!(segments.is_empty());
    }

    #[test]
    fn test_extract_map_single_message() {
        let cmd = make_show_message_cmd("テスト");
        let page = make_page(&cmd, 1);
        let event = make_event(&page);
        let mps = make_mps_with_event(&event);

        let segments = extract_map_segments("Map001", &mps, &v2()).unwrap();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].source_text, "テスト");
        assert_eq!(segments[0].key, "MapData/Map001/events/0/pages/0/0");
    }

    #[test]
    fn test_extract_map_placeholder_only() {
        // \v[1] is a Wolf placeholder — should be filtered out
        let cmd = make_show_message_cmd("\\v[1]");
        let page = make_page(&cmd, 1);
        let event = make_event(&page);
        let mps = make_mps_with_event(&event);

        let segments = extract_map_segments("Map001", &mps, &v2()).unwrap();
        assert!(
            segments.is_empty(),
            "placeholder-only segment must be filtered"
        );
    }

    // -----------------------------------------------------------------------
    // Database extraction tests (Step 6)
    // -----------------------------------------------------------------------

    /// Encode `text` as a Wolf length-prefixed SJIS string (for test helpers).
    fn sjis_string(text: &str) -> Vec<u8> {
        use encoding_rs::SHIFT_JIS;
        let (enc, _, _) = SHIFT_JIS.encode(text);
        let len = (enc.len() + 1) as u32;
        let mut out = len.to_le_bytes().to_vec();
        out.extend_from_slice(&enc);
        out.push(0x00);
        out
    }

    /// Empty Wolf string (1-byte null only).
    fn empty_wolf_string() -> Vec<u8> {
        let mut v = 1u32.to_le_bytes().to_vec();
        v.push(0x00);
        v
    }

    /// Build a minimal unencrypted SJIS .project with one type, one string field,
    /// and one data entry.
    fn make_db_project(type_name: &str, field_name: &str) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&1u32.to_le_bytes()); // type_count = 1
        b.extend(sjis_string(type_name));
        b.extend_from_slice(&1u32.to_le_bytes()); // field_count = 1
        b.extend(sjis_string(field_name));
        b.extend_from_slice(&1u32.to_le_bytes()); // data_count = 1
        b.extend(empty_wolf_string()); // entry name = ""
        b.extend(empty_wolf_string()); // description = ""
                                       // field_type_list_size = 1, type byte = 0
        b.extend_from_slice(&1u32.to_le_bytes());
        b.push(0x00);
        // unknown1/2/3/4 each: count=1, then zero items (counts only, no data)
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend(empty_wolf_string()); // unknown1 string for field
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes()); // unknown2 args = 0
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes()); // unknown3 args = 0
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes()); // unknown4 default = 0
        b
    }

    /// Build a minimal unencrypted SJIS .dat with one type, one string field
    /// (indexInfo = STRING_FIELD_START), one entry.
    fn make_db_dat_string(value: &str) -> Vec<u8> {
        use crate::engines::wolf::dat_parser::{DAT_TYPE_SEPARATOR, DB_MAGIC_SJIS};
        let ver: u8 = 0xC1;
        let mut b = Vec::new();
        b.push(0x00); // indicator
        b.extend_from_slice(&DB_MAGIC_SJIS);
        b.push(ver);
        b.extend_from_slice(&1u32.to_le_bytes()); // type_count
        b.extend_from_slice(&DAT_TYPE_SEPARATOR);
        b.extend_from_slice(&0u32.to_le_bytes()); // unknown1
        b.extend_from_slice(&1u32.to_le_bytes()); // fields_size = 1
        b.extend_from_slice(&0x07D0u32.to_le_bytes()); // indexInfo = STRING_FIELD_START
        b.extend_from_slice(&1u32.to_le_bytes()); // data_count = 1
        b.extend(sjis_string(value));
        b.push(ver); // terminator
        b
    }

    /// Build a .dat with one int field (indexInfo = VALID_FIELD_START = 0x03E8).
    fn make_db_dat_int(int_value: u32) -> Vec<u8> {
        use crate::engines::wolf::dat_parser::{DAT_TYPE_SEPARATOR, DB_MAGIC_SJIS};
        let ver: u8 = 0xC1;
        let mut b = Vec::new();
        b.push(0x00);
        b.extend_from_slice(&DB_MAGIC_SJIS);
        b.push(ver);
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&DAT_TYPE_SEPARATOR);
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&0x03E8u32.to_le_bytes()); // int field
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&int_value.to_le_bytes());
        b.push(ver);
        b
    }

    #[test]
    fn test_extract_database_name_field_japanese() {
        let project = make_db_project("Items", "name");
        let dat = make_db_dat_string("テスト"); // Japanese value → always extracted
        let segs = extract_database_segments("UserDB", &project, &dat, &v2()).unwrap();
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].source_text, "テスト");
        assert!(segs[0].key.starts_with("Database/UserDB/"));
        assert!(segs[0].key.ends_with("/name"));
    }

    #[test]
    fn test_extract_database_known_field_ascii_extracted() {
        // Field named "name" is a known translatable field even without Japanese.
        let project = make_db_project("Items", "name");
        let dat = make_db_dat_string("Hero");
        let segs = extract_database_segments("UserDB", &project, &dat, &v2()).unwrap();
        assert_eq!(
            segs.len(),
            1,
            "known translatable field name must be extracted"
        );
    }

    #[test]
    fn test_extract_database_skips_int_field() {
        let project = make_db_project("Vars", "hp");
        let dat = make_db_dat_int(100);
        let segs = extract_database_segments("UserDB", &project, &dat, &v2()).unwrap();
        assert!(segs.is_empty(), "integer fields must be skipped");
    }

    #[test]
    fn test_extract_database_skips_empty_value() {
        // An empty string value should not produce a segment.
        let project = make_db_project("Items", "name");
        let dat = make_db_dat_string(""); // empty → skip
        let segs = extract_database_segments("UserDB", &project, &dat, &v2()).unwrap();
        assert!(segs.is_empty(), "empty string must be skipped");
    }

    // -----------------------------------------------------------------------
    // Step 9 — key uniqueness + format validation
    // -----------------------------------------------------------------------

    #[test]
    fn test_key_uniqueness_single_mps() {
        // Two distinct messages in the same map must produce distinct keys.
        let cmd1 = make_show_message_cmd("こんにちは");
        let cmd2 = make_show_message_cmd("さようなら");
        let mut cmds = cmd1;
        cmds.extend(cmd2);
        let page = make_page(&cmds, 2);
        let event = make_event(&page);
        let mps = make_mps_with_event(&event);

        let segs = extract_map_segments("Map001", &mps, &v2()).unwrap();
        assert_eq!(segs.len(), 2);
        let key0 = &segs[0].key;
        let key1 = &segs[1].key;
        assert_ne!(key0, key1, "two distinct commands must have distinct keys");
    }

    #[test]
    fn test_key_format_mps() {
        let cmd = make_show_message_cmd("テスト");
        let page = make_page(&cmd, 1);
        let event = make_event(&page);
        let mps = make_mps_with_event(&event);
        let segs = extract_map_segments("TitleMap", &mps, &v2()).unwrap();
        assert_eq!(segs.len(), 1);
        assert!(
            segs[0].key.starts_with("MapData/TitleMap/events/"),
            "key must follow MapData/{{map}}/events/... format"
        );
    }

    #[test]
    fn test_key_format_dat() {
        let project = make_db_project("Chars", "名前");
        let dat = make_db_dat_string("テスト");
        let segs = extract_database_segments("CDB", &project, &dat, &v2()).unwrap();
        assert_eq!(segs.len(), 1);
        assert!(
            segs[0].key.starts_with("Database/CDB/"),
            "key must follow Database/{{db}}/... format"
        );
    }

    #[test]
    fn test_extract_wolf_project_empty_dir() {
        // A directory with no Data/ subdirectory → Ok(vec![]).
        let dir = std::path::Path::new("/tmp/hoshi2star_nonexistent_game_dir");
        let segs = extract_wolf_project(dir, &v2()).unwrap();
        assert!(segs.is_empty(), "empty game dir must yield no segments");
    }

    // -----------------------------------------------------------------------
    // CommonEvents extraction tests
    // -----------------------------------------------------------------------

    /// Build a minimal valid CommonEvent.dat with zero events.
    fn make_empty_common_event_dat() -> Vec<u8> {
        // Magic = b"\x00\x57\x00\x00\x4F\x4C\x00\x46\x43\x00\x8F" (11 bytes)
        let mut b = Vec::new();
        b.extend_from_slice(b"\x00\x57\x00\x00\x4F\x4C\x00\x46\x43\x00\x8F");
        b.extend_from_slice(&0u32.to_le_bytes()); // event_count = 0
        b
    }

    #[test]
    fn test_extract_common_events_empty() {
        let bytes = make_empty_common_event_dat();
        let segs = extract_common_events(&bytes, &v2()).unwrap();
        assert!(segs.is_empty(), "zero events must yield no segments");
    }

    #[test]
    fn test_is_resource_path() {
        for path in [
            "Picture/title.png",
            "title.PNG",
            "se001.ogg",
            "bgm.mp3",
            "voice.WAV",
            "MS Gothic.ttf",
            "movie.webm",
        ] {
            assert!(is_resource_path(path), "{path:?} should be a resource path");
        }

        for text in ["こんにちは", "Hello, world!", "Lv.5", "Mr. Smith"] {
            assert!(
                !is_resource_path(text),
                "{text:?} should not be a resource path"
            );
        }
    }

    #[test]
    fn test_is_translatable_skips_resource_paths() {
        assert!(!is_translatable("Picture/title.png"));
        assert!(!is_translatable("se001.ogg"));
        assert!(is_translatable("こんにちは"));
    }

    #[test]
    fn test_extract_common_events_invalid_magic_no_panic() {
        // Invalid magic → parser panics → caught → ExtractorError, not a process panic.
        let bytes = b"garbage data that is definitely not a common event file".to_vec();
        let result = extract_common_events(&bytes, &v2());
        assert!(result.is_err(), "invalid magic must return Err, not panic");
    }

    // -----------------------------------------------------------------------
    // Integration tests against UberWolf-decrypted game files in test/
    // Run with: cargo test --manifest-path src-tauri/Cargo.toml test_real_
    // -----------------------------------------------------------------------

    fn test_dir() -> std::path::PathBuf {
        // When running under cargo, the crate root is src-tauri/
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("test")
    }

    // CommonEvent.dat:
    //   Honoka v2.225: fixed via forked wolfrpg-map-parser (fix/wolf-v3-format), which
    //                  adds the 0x04D20000/0x09D20000 CallCommonEvent signatures.
    //   Inko v2.292:   header (byte6=0x55/0x93) and v3.x payload (LZ4-compressed,
    //                  UTF-8 strings, same 0x8E per-event signature as v2.x) now
    //                  decode correctly via the same fork — but command parsing still
    //                  panics on v3.x-only opcodes. See test below.
    #[test]
    fn test_real_honoka_common_events() {
        let path = test_dir().join("月咲流ホノカver1.03/Data/BasicData/CommonEvent.dat");
        if !path.exists() {
            return;
        }
        let bytes = std::fs::read(&path).unwrap();
        let segs = extract_common_events(&bytes, &v2())
            .unwrap_or_else(|e| panic!("Honoka CommonEvent.dat must parse: {e:?}"));
        eprintln!("Honoka CommonEvent.dat → {} segments", segs.len());
        assert!(
            !segs.is_empty(),
            "Honoka CommonEvent.dat must yield segments"
        );
    }

    #[test]
    fn test_real_inko_common_events_v3() {
        let path = test_dir().join("Densyanai_Inko_ver2.0/Data/BasicData/CommonEvent.dat");
        if !path.exists() {
            return;
        }
        let bytes = std::fs::read(&path).unwrap();
        let segs = extract_common_events(&bytes, &v3())
            .unwrap_or_else(|e| panic!("Inko CommonEvent.dat must parse: {e:?}"));
        eprintln!("Inko CommonEvent.dat → {} segments", segs.len());
        assert!(!segs.is_empty(), "Inko CommonEvent.dat must yield segments");
        assert!(
            segs.iter()
                .any(|s| !s.source_text.trim().is_empty() && !s.source_text.contains('\u{FFFD}')),
            "Inko CommonEvent.dat should yield at least one readable, non-empty segment"
        );
    }

    // Maps (.mps) — known crate limitations:
    //   Honoka v2.225: parses successfully (signature normalisation fixes unknown D2 cmds).
    //   Inko v2.292:   Different map signature at bytes 16-19 (0x55000000 vs 0x00000000).
    //                  Crate hard-codes v2.225 map signature → incompatible.
    #[test]
    fn test_real_honoka_maps() {
        let map_dir = test_dir().join("月咲流ホノカver1.03/Data/MapData");
        if !map_dir.exists() {
            return;
        }
        let mut total = 0usize;
        let mut errors = 0usize;
        for entry in std::fs::read_dir(&map_dir).unwrap().flatten() {
            let p = entry.path();
            if p.extension().map_or(false, |e| e == "mps") {
                let bytes = std::fs::read(&p).unwrap();
                let name = p.file_name().unwrap().to_string_lossy().into_owned();
                match extract_map_segments(&name, &bytes, &v2()) {
                    Ok(segs) => total += segs.len(),
                    Err(_) => errors += 1,
                }
            }
        }
        eprintln!("Honoka maps → {total} segments, {errors} files errored");
        assert_eq!(errors, 0, "all Honoka .mps files must parse");
        assert!(total > 0, "Honoka maps must yield segments");
    }

    #[test]
    fn test_real_inko_maps_v3() {
        let map_dir = test_dir().join("Densyanai_Inko_ver2.0/Data/MapData");
        if !map_dir.exists() {
            return;
        }
        let mut total = 0usize;
        let mut errors = 0usize;
        for entry in std::fs::read_dir(&map_dir).unwrap().flatten() {
            let p = entry.path();
            if p.extension().map_or(false, |e| e == "mps") {
                let bytes = std::fs::read(&p).unwrap();
                let name = p.file_name().unwrap().to_string_lossy().into_owned();
                match extract_map_segments(&name, &bytes, &v2()) {
                    Ok(segs) => total += segs.len(),
                    Err(e) => {
                        eprintln!("{name}: {e}");
                        errors += 1;
                    }
                }
            }
        }
        eprintln!("Inko v3.x maps → {total} segments, {errors} files errored");
        assert_eq!(errors, 0, "all Inko v3.x .mps files must parse");
        assert!(total > 0, "Inko v3.x maps must yield segments");

        // Spot-check: at least one segment must be readable, non-empty Japanese
        // text (not mojibake / replacement characters).
        let title_map = std::fs::read(map_dir.join("TitleMap.mps")).unwrap();
        let segments = extract_map_segments("TitleMap", &title_map, &v2()).unwrap();
        assert!(
            segments
                .iter()
                .any(|s| !s.source_text.trim().is_empty() && !s.source_text.contains('\u{FFFD}')),
            "TitleMap should yield at least one readable, non-empty segment"
        );
    }

    /// Inko's `DataBase.dat`/`CDataBase.dat`/`SysDatabase.dat` are LZ4-compressed
    /// (version byte 0xC4, Wolf RPG v3.x) but otherwise UTF-8-encoded with the same
    /// `.project`/`.dat` schema as Honoka — `dat_parser::decompress_lz4_dat` makes
    /// them parse like any uncompressed database.
    #[test]
    fn test_real_inko_database_segments() {
        let basic_data_dir = test_dir().join("Densyanai_Inko_ver2.0/Data/BasicData");
        if !basic_data_dir.exists() {
            return;
        }
        let mut total = 0usize;
        for db_name in ["DataBase", "CDataBase", "SysDatabase"] {
            let project_path = basic_data_dir.join(format!("{db_name}.project"));
            let dat_path = basic_data_dir.join(format!("{db_name}.dat"));
            let project_bytes = std::fs::read(&project_path).unwrap();
            let dat_bytes = std::fs::read(&dat_path).unwrap();
            let segs = extract_database_segments(db_name, &project_bytes, &dat_bytes, &v2())
                .unwrap_or_else(|e| panic!("Inko {db_name}.dat must parse (LZ4): {e:?}"));
            eprintln!("Inko {db_name}.dat → {} segments", segs.len());
            total += segs.len();
        }
        assert!(total > 0, "Inko databases must yield segments");
    }
}
