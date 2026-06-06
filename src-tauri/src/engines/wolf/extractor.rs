// Wolf RPG text extractor — F4-03 implementation.
#![allow(dead_code)]

use super::decryptor::{extract_all, WolfFile};
use crate::llm::tokenizer::{Engine as TokEngine, Tokenizer};
use std::path::Path;
use wolfrpg_map_parser::{command::Command, Map};

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
    // Wrap the panicking parser in catch_unwind.
    let bytes_owned = bytes.to_vec();
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

fn is_translatable(text: &str) -> bool {
    !text.trim().is_empty() && !is_wolf_placeholder_only(text)
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
}
