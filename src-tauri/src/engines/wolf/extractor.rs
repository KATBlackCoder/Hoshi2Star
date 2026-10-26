// Wolf RPG text extractor — F4-03/F4-05 implementation.

use super::dat_parser;
use super::sources::{SourcesError, WolfSources};
use super::v3_format;
use crate::{engines::filter, llm::tokenizer::Engine as TokEngine};
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

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
    /// An archive the game reads cannot be read (plan RW, RW-5).
    #[error(transparent)]
    Sources(#[from] SourcesError),
    /// Maps or common events that could not be read (unknown container or
    /// parse failure). The project is refused as a whole rather than opened
    /// without their dialogue. Starts with [`WOLF_UNREADABLE_FILES`] so the
    /// frontend can show a dedicated message.
    #[error(
        "{marker}: some Wolf RPG maps or common events could not be read ({0})",
        marker = WOLF_UNREADABLE_FILES
    )]
    UnreadableWolfFiles(String),
}

/// Stable marker at the start of [`ExtractorError::UnreadableWolfFiles`],
/// matched by the frontend.
pub const WOLF_UNREADABLE_FILES: &str = "wolf_unreadable_files";

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
// Speaker name extraction (Wolf v2)
// ---------------------------------------------------------------------------

/// A character speaker name extracted from Wolf v2 `\E<name>\n` message prefixes.
pub struct SpeakerName {
    /// Canonical form after stripping inline Wolf codes (`\c[N]`, etc.).
    pub canonical: String,
    /// How many times this speaker appears across all segments.
    pub occurrences: usize,
}

/// Matches `\E<anything>\n` at the very start of a source text (Wolf v2 speaker prefix).
static RE_SPEAKER_PREFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\\E([^\n]+)\n").expect("RE_SPEAKER_PREFIX must compile"));

/// Matches inline Wolf display codes like `\c[2]`, `\C[1]`, `\f[0]`.
static RE_WOLF_INLINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\[a-zA-Z]+\[\d+\]").expect("RE_WOLF_INLINE must compile"));

fn is_valid_speaker_name(name: &str) -> bool {
    let t = name.trim();
    // Empty or too long (narration sentences are typically 13+ chars; real names ≤ 10).
    if t.is_empty() || t.chars().count() > 10 {
        return false;
    }
    // Must contain at least one Japanese or ASCII alphanumeric character.
    t.chars().any(|c| {
        matches!(c,
            '\u{3040}'..='\u{30FF}' // hiragana + katakana (incl. ー)
            | '\u{4E00}'..='\u{9FFF}' // CJK ideographs
            | 'A'..='Z' | 'a'..='z' | '0'..='9'
        )
    })
}

/// Extract and deduplicate Wolf v2 speaker names from a slice of raw source texts.
///
/// Only source texts that start with `\E<name>\n` are considered. The name is
/// canonicalised by stripping inline Wolf codes (`\c[N]` etc.) and trimming. Names
/// that are empty, exceed 10 chars, or contain no Japanese/ASCII alphanumeric chars
/// are discarded (they are typically narration sentences, not character names).
///
/// Returns speakers sorted by descending occurrence count, then alphabetically.
pub fn extract_wolf_speaker_names(source_texts: &[String]) -> Vec<SpeakerName> {
    let mut counts: HashMap<String, usize> = HashMap::new();

    for text in source_texts {
        if let Some(cap) = RE_SPEAKER_PREFIX.captures(text) {
            let raw = &cap[1];
            let canonical = RE_WOLF_INLINE.replace_all(raw, "").trim().to_string();
            if is_valid_speaker_name(&canonical) {
                *counts.entry(canonical).or_insert(0) += 1;
            }
        }
    }

    let mut names: Vec<SpeakerName> = counts
        .into_iter()
        .map(|(canonical, occurrences)| SpeakerName {
            canonical,
            occurrences,
        })
        .collect();

    names.sort_by(|a, b| {
        b.occurrences
            .cmp(&a.occurrences)
            .then(a.canonical.cmp(&b.canonical))
    });
    names
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
    // Maps / common events that cannot be read: refusing the whole project is
    // safer than opening it without their dialogue.
    let mut unreadable: Vec<String> = Vec::new();

    // Archives first, then loose files, as the game reads them.
    let sources = WolfSources::load(game_dir)?;

    // Maps
    for map in sources.maps() {
        let name = map.stem();
        match extract_map_segments(&name, &map.bytes, version) {
            Ok(segs) if !segs.is_empty() => {
                entries.push((format!("{name}.mps"), "wolf_map".to_string(), segs));
            }
            Ok(_) => {}
            Err(e) => unreadable.push(format!("{name}.mps ({e})")),
        }
    }

    // Databases
    for (project, dat) in sources.databases() {
        let name = dat.stem();
        match extract_database_segments(&name, &project.bytes, &dat.bytes, version) {
            Ok(segs) if !segs.is_empty() => {
                entries.push((format!("{name}.dat"), "wolf_database".to_string(), segs));
            }
            Ok(_) => {}
            Err(e) => eprintln!("warn: skipping {name}.dat — {e}"),
        }
    }

    // Common Events
    if let Some(common_events) = sources.common_events() {
        match extract_common_events(&common_events.bytes, version) {
            Ok(segs) if !segs.is_empty() => {
                entries.push((
                    "CommonEvent.dat".to_string(),
                    "wolf_common_events".to_string(),
                    segs,
                ));
            }
            Ok(_) => {}
            Err(e) => unreadable.push(format!("CommonEvent.dat ({e})")),
        }
    }

    reject_unreadable(unreadable)?;
    Ok(entries)
}

/// Fail with one [`ExtractorError::UnreadableWolfFiles`] naming the first
/// unreadable files, so the project is never opened without its dialogue.
fn reject_unreadable(unreadable: Vec<String>) -> Result<(), ExtractorError> {
    const SHOWN: usize = 3;
    match unreadable.len() {
        0 => Ok(()),
        count => {
            let mut files = unreadable[..count.min(SHOWN)].join(", ");
            if count > SHOWN {
                files.push_str(&format!(" and {} more", count - SHOWN));
            }
            Err(ExtractorError::UnreadableWolfFiles(files))
        }
    }
}

// ---------------------------------------------------------------------------
// .mps extraction
// ---------------------------------------------------------------------------

/// Extract translatable segments from a `.mps` map file with the in-house
/// [`v3_format`] parser: v3.x LZ4 maps and maps stored in clear (v2.x Shift-JIS,
/// v3.x `0x64` UTF-8) share the same flat model; the string encoding comes from
/// the map's own flag byte.
///
/// `map_name` is the file stem (e.g. `"Map001"`).
/// `_version` is reserved for future per-version branching.
pub fn extract_map_segments(
    map_name: &str,
    bytes: &[u8],
    _version: &crate::engines::detector::WolfVersion,
) -> Result<Vec<WolfSegment>, ExtractorError> {
    let decompressed = v3_format::compression::unpack_map(bytes)
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
    filter::needs_translation(text, TokEngine::Wolf) && !is_resource_path(text.trim())
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

                if !is_translatable(value) {
                    continue;
                }
                // Wolf RPG auto-replaces these values at runtime; they are never player-facing.
                if value.contains("自動ｼｽﾃﾑ初期化") {
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

/// Extract translatable segments from `CommonEvent.dat` bytes, via
/// [`v3_format::common_events`]: v3.5 LZ4 files and files stored in clear
/// (v2.x) share the same flat model. Each event's commands are a flat list
/// (no pages, unlike `.mps` events).
pub fn extract_common_events(
    bytes: &[u8],
    _version: &crate::engines::detector::WolfVersion,
) -> Result<Vec<WolfSegment>, ExtractorError> {
    let decompressed = v3_format::common_events::unpack(bytes)
        .map_err(|e| ExtractorError::MapParser(format!("CommonEvent.dat: {e}")))?;
    let common_events = v3_format::common_events::CommonEventsV3::parse(&decompressed)
        .map_err(|e| ExtractorError::MapParser(format!("CommonEvent.dat: {e}")))?;

    let mut segments = Vec::new();

    for (event_idx, event) in common_events.events.iter().enumerate() {
        let event_name = &event.name;
        if event_name.starts_with("X[") || event_name.starts_with("zz") {
            continue;
        }
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
/// Reads the files the game reads ([`WolfSources`]: archives first, then
/// loose `Data/` files):
///   - `MapData/*.mps`        → `extract_map_segments`
///   - `BasicData/*.dat`      → `extract_database_segments` (pairs with `.project`)
///   - `BasicData/CommonEvent.dat` → `extract_common_events`
///
/// A `.dat` parse error is logged as a warning; an unreadable archive, map or
/// `CommonEvent.dat` refuses the extraction.
pub fn extract_wolf_project(
    game_dir: &Path,
    version: &crate::engines::detector::WolfVersion,
) -> Result<Vec<WolfSegment>, ExtractorError> {
    let mut segments = Vec::new();
    let mut unreadable: Vec<String> = Vec::new();

    let sources = WolfSources::load(game_dir)?;

    // --- Maps ---
    for map in sources.maps() {
        let name = map.stem();
        match extract_map_segments(&name, &map.bytes, version) {
            Ok(segs) => segments.extend(segs),
            Err(e) => unreadable.push(format!("{name}.mps ({e})")),
        }
    }

    // --- Databases ---
    for (project, dat) in sources.databases() {
        let name = dat.stem();
        match extract_database_segments(&name, &project.bytes, &dat.bytes, version) {
            Ok(segs) => segments.extend(segs),
            Err(e) => eprintln!("warn: skipping {name}.dat — {e}"),
        }
    }

    // --- Common Events ---
    if let Some(common_events) = sources.common_events() {
        match extract_common_events(&common_events.bytes, version) {
            Ok(segs) => segments.extend(segs),
            Err(e) => unreadable.push(format!("CommonEvent.dat ({e})")),
        }
    }

    reject_unreadable(unreadable)?;
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

    fn v2() -> WolfVersion {
        WolfVersion { major: 2, minor: 0 }
    }

    fn v3() -> WolfVersion {
        WolfVersion { major: 3, minor: 0 }
    }

    /// Japanese texts the translatable filter keeps.
    const MESSAGE: &str = "こんにちは、旅の人";
    const CHOICES: [&str; 2] = ["はい、行きます", "いいえ、残ります"];

    /// On-disk bytes of a synthetic map, in the container its `version` selects.
    fn synthetic_map_bytes(version: u32, is_utf8: bool) -> Vec<u8> {
        let map = v3_format::map::MapV3::synthetic_messages(version, is_utf8, &[MESSAGE], &CHOICES);
        v3_format::compression::pack_map(map.dump().unwrap()).unwrap()
    }

    /// v2.x (Shift-JIS `0x64`) and v3.x `0x64` maps are stored in clear and read
    /// with the same flat model as the LZ4 v3.5 maps (plan RW, RW-4).
    #[test]
    fn test_extract_map_segments_all_containers() {
        for (version, is_utf8) in [(0x64, false), (0x64, true), (0x67, true)] {
            let bytes = synthetic_map_bytes(version, is_utf8);
            assert_eq!(
                v3_format::compression::is_plain_map(&bytes),
                version < 0x65,
                "version {version:#x}"
            );
            let segs = extract_map_segments("Map001", &bytes, &v2()).unwrap();
            let texts: Vec<&str> = segs.iter().map(|s| s.source_text.as_str()).collect();
            assert_eq!(texts, [MESSAGE, CHOICES[0], CHOICES[1]]);
            assert_eq!(segs[0].key, "MapData/Map001/events/0/pages/0/0");
            assert_eq!(segs[2].key, "MapData/Map001/events/0/pages/0/1/choices/1");
        }
    }

    #[test]
    fn test_extract_all_wolf_reads_a_v2_game() {
        let game = tempfile::tempdir().unwrap();
        let map_dir = game.path().join("Data").join("MapData");
        let basic = game.path().join("Data").join("BasicData");
        std::fs::create_dir_all(&map_dir).unwrap();
        std::fs::create_dir_all(&basic).unwrap();
        std::fs::write(map_dir.join("Map001.mps"), synthetic_map_bytes(0x64, false)).unwrap();
        let common_events =
            v3_format::common_events::CommonEventsV3::synthetic_messages_v2(&[("Ev001", MESSAGE)]);
        std::fs::write(basic.join("CommonEvent.dat"), common_events.dump().unwrap()).unwrap();

        let entries = extract_all_wolf(game.path(), &v2()).unwrap();
        let map = entries
            .iter()
            .find(|(file, _, _)| file == "Map001.mps")
            .unwrap();
        assert_eq!(map.1, "wolf_map");
        assert_eq!(map.2.len(), 3);
        let ce = entries
            .iter()
            .find(|(file, _, _)| file == "CommonEvent.dat")
            .unwrap();
        assert_eq!(ce.2[0].source_text, MESSAGE);
    }

    /// A map that cannot be read refuses the whole project (never opened
    /// without its dialogue), with the dedicated marker.
    #[test]
    fn test_extract_all_wolf_refuses_unreadable_maps() {
        let game = tempfile::tempdir().unwrap();
        let map_dir = game.path().join("Data").join("MapData");
        std::fs::create_dir_all(&map_dir).unwrap();
        // Signature of a map in clear, then a zero-length string: parse error.
        std::fs::write(map_dir.join("Map001.mps"), make_empty_mps()).unwrap();
        std::fs::write(map_dir.join("Map002.mps"), make_empty_mps()).unwrap();

        match extract_all_wolf(game.path(), &v2()) {
            Err(error @ ExtractorError::UnreadableWolfFiles(_)) => {
                let message = error.to_string();
                assert!(message.starts_with(WOLF_UNREADABLE_FILES), "{message}");
                assert!(
                    message.contains("Map001.mps") && message.contains("Map002.mps"),
                    "{message}"
                );
            }
            other => panic!("unreadable maps must refuse the project, got {other:?}"),
        }
    }

    #[test]
    fn test_reject_unreadable_lists_at_most_three_files() {
        let files = (1..=5).map(|i| format!("Map00{i}.mps")).collect();
        match reject_unreadable(files) {
            Err(ExtractorError::UnreadableWolfFiles(text)) => {
                assert_eq!(text, "Map001.mps, Map002.mps, Map003.mps and 2 more");
            }
            other => panic!("unexpected {other:?}"),
        }
        assert!(reject_unreadable(Vec::new()).is_ok());
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
        b.push(0x89); // file terminator
        b
    }

    #[test]
    fn test_extract_common_events_stored_in_clear() {
        let common_events =
            v3_format::common_events::CommonEventsV3::synthetic_messages_v2(&[("Ev001", MESSAGE)]);
        let bytes = common_events.dump().unwrap();
        assert!(v3_format::common_events::is_plain(&bytes));
        let segs = extract_common_events(&bytes, &v2()).unwrap();
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].source_text, MESSAGE);
        // An empty v2 file (no events) also reads, with nothing to translate.
        assert!(extract_common_events(&make_empty_common_event_dat(), &v2())
            .unwrap()
            .is_empty());
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
    fn test_extract_database_skips_auto_init() {
        // Values containing 自動ｼｽﾃﾑ初期化 are auto-replaced at runtime — not player-facing.
        let project = make_db_project("System", "description");
        let dat = make_db_dat_string("ここの値は「自動ｼｽﾃﾑ初期化」処理でセットされます");
        let segs = extract_database_segments("CDataBase", &project, &dat, &v2()).unwrap();
        assert!(
            segs.is_empty(),
            "auto-init placeholder values must be skipped"
        );
    }

    #[test]
    fn test_extract_common_events_invalid_magic_no_panic() {
        // Not a v3.x file → explicit error, never a process panic.
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

    /// Honoka (Wolf v2.225): its v2 maps and common events are read again
    /// (RW-4), no file is refused.
    #[test]
    fn test_real_honoka_v2_is_read() {
        let game_dir = test_dir().join("月咲流ホノカver1.03");
        if !game_dir.exists() {
            return;
        }
        let entries = extract_all_wolf(&game_dir, &v2()).unwrap();
        assert!(entries.iter().any(|(_, kind, _)| kind == "wolf_map"));
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
        // Engine-internal (X[) and archived/disabled (zz) events must be filtered out
        assert!(
            !segs.iter().any(|s| {
                if let WolfSegmentKind::CommonEventMessage { event_name, .. } = &s.kind {
                    event_name.starts_with("X[") || event_name.starts_with("zz")
                } else {
                    false
                }
            }),
            "X[ and zz events must not appear in extracted segments"
        );
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
            if p.extension().is_some_and(|e| e == "mps") {
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

    /// Honoka's `Data.wolf` (v8) bundles a second, bogus copy of
    /// `DataBase.dat`/`CDataBase.dat`/`SysDatabase.dat`/`SysDataBaseBasic.dat`
    /// inside a nested "complete initial state" sample folder
    /// (`データ集/（完全初期状態データ）/Data/BasicData/`). Before path-based
    /// filtering, the bogus 257-byte `DataBase.dat` (indicator byte 0xbd)
    /// would shadow the real 85738-byte one, surfacing as a spurious
    /// "protected database" error — and `CDataBase.dat`'s bogus copy (also
    /// 257 bytes, but with a *valid* header) would silently shadow the real
    /// 90631-byte one.
    #[test]
    fn test_real_honoka_archive_database_pairs() {
        let game_dir = test_dir().join("月咲流ホノカver1.03(.wolf)");
        if !game_dir.exists() {
            return;
        }
        let sources = WolfSources::load(&game_dir).unwrap();
        let by_stem: HashMap<String, usize> = sources
            .databases()
            .into_iter()
            .map(|(_project, dat)| (dat.stem(), dat.bytes.len()))
            .collect();

        assert_eq!(
            by_stem.get("database").copied(),
            Some(85738),
            "must pick the real BasicData/DataBase.dat, not the 257-byte sample copy"
        );
        assert_eq!(
            by_stem.get("cdatabase").copied(),
            Some(90631),
            "must pick the real BasicData/CDataBase.dat, not the 257-byte sample copy"
        );
        assert!(
            !by_stem.contains_key("sysdatabasebasic"),
            "SysDataBaseBasic must be skipped (engine-internal, non-translatable)"
        );
    }

    // -----------------------------------------------------------------------
    // Speaker name extraction tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_speaker_plain_name() {
        let texts = vec!["\\Eほのか\n「こんにちは」".to_string()];
        let names = extract_wolf_speaker_names(&texts);
        assert_eq!(names.len(), 1);
        assert_eq!(names[0].canonical, "ほのか");
        assert_eq!(names[0].occurrences, 1);
    }

    #[test]
    fn test_speaker_colored_name_stripped() {
        // \c[2]ほのか → canonical must be "ほのか"
        let texts = vec!["\\E\\c[2]ほのか\n「セリフ」".to_string()];
        let names = extract_wolf_speaker_names(&texts);
        assert_eq!(names.len(), 1);
        assert_eq!(names[0].canonical, "ほのか");
    }

    #[test]
    fn test_speaker_deduplication_and_count() {
        let texts = vec![
            "\\Eほのか\n「A」".to_string(),
            "\\Eほのか\n「B」".to_string(),
            "\\E男\n「C」".to_string(),
        ];
        let names = extract_wolf_speaker_names(&texts);
        assert_eq!(names.len(), 2);
        assert_eq!(names[0].canonical, "ほのか");
        assert_eq!(names[0].occurrences, 2);
    }

    #[test]
    fn test_speaker_punctuation_only_filtered() {
        // ……？ — no alphanumeric Japanese → must be filtered
        let texts = vec!["\\E……？\n「…」".to_string()];
        let names = extract_wolf_speaker_names(&texts);
        assert!(names.is_empty(), "punctuation-only must be filtered");
    }

    #[test]
    fn test_speaker_long_narration_filtered() {
        // >10 chars → must be filtered (narration sentence, not a name)
        let texts = vec!["\\Eここは150年の歴史を持つ道場\n続き".to_string()];
        let names = extract_wolf_speaker_names(&texts);
        assert!(
            names.is_empty(),
            "narration sentences longer than 10 chars must be filtered"
        );
    }

    #[test]
    fn test_speaker_no_newline_no_match() {
        // \E without a following \n → no match
        let texts = vec!["\\Eほのか".to_string()];
        let names = extract_wolf_speaker_names(&texts);
        assert!(names.is_empty());
    }

    #[test]
    fn test_speaker_mid_text_no_match() {
        // \E not at position 0 → not a speaker prefix
        let texts = vec!["普通のテキスト\\Eほのか\n続き".to_string()];
        let names = extract_wolf_speaker_names(&texts);
        assert!(
            names.is_empty(),
            "\\E mid-text must not be treated as a speaker prefix"
        );
    }
}
