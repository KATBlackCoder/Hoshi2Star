// Wolf RPG binary injector — F4-04 implementation.
//
// Injection strategy (F4): Option A — write decrypted .mps/.dat directly to
// Data/MapData/ and Data/BasicData/.  Wolf RPG reads Data/ with priority over
// .wolf archives, so no re-encryption is required.  Option B (DXA re-pack) is
// deferred to F5.
//
// .mps injection: sequential scan + splice (Approach B).
//   wolfrpg-map-parser does not expose byte offsets in public structs
//   (confirmed Step 1 investigation).  We parse to get the ordered list of
//   translatable strings, then walk the raw bytes replacing each ReadString
//   payload in the same sequential order.
//
// .dat injection: parse_database() → modify string_values → serialize_dat().

use super::dat_parser::{self, DatFile, DatType, STRING_INDICATOR_PUB};
use super::dat_parser::{DAT_TYPE_SEPARATOR, DB_MAGIC_SJIS, DB_MAGIC_UTF8};
use super::encoding;
use super::v3_format::{self, V3FormatError};
use crate::engines::detector::WolfVersion;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use wolfrpg_map_parser::Map;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum InjectorError {
    #[error("key not found in Wolf data: {0}")]
    KeyNotFound(String),
    #[error("encoding error (UTF-8 to Shift-JIS failed): {0}")]
    Encoding(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("dat parse error: {0}")]
    DatParse(String),
    #[error("missing .project file for stem: {0}")]
    MissingProject(String),
    #[error("v3.x map format error: {0}")]
    V3Format(#[from] V3FormatError),
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// One (key, translated_text) pair to inject.
pub struct WolfTranslation {
    pub key: String,
    pub text: String,
}

/// Result of injecting one file.
pub struct InjectionResult {
    pub file_path: PathBuf,
    pub updated_count: usize,
}

/// Bucket for all `CommonEvents/*` segment keys.
///
/// Unlike maps/databases (one physical file per stem), every common event
/// shares the single `Data/BasicData/CommonEvent.dat`, so all `CommonEvents/*`
/// translations must collapse into one bucket — otherwise [`inject_all`] /
/// [`inject_all_to_memory`] would inject the file once per event name and drop
/// all but the last event's translations.
const COMMON_EVENTS_BUCKET: &str = "CommonEvents/CommonEvent";

/// Maps a Wolf segment key to the injection bucket (the `file_key` consumed by
/// [`inject_all`] / [`inject_all_to_memory`]).
///
/// - `MapData/{stem}/…`  → `MapData/{stem}`   (one `.mps` per stem)
/// - `Database/{stem}/…` → `Database/{stem}`  (one `.dat` pair per stem)
/// - `CommonEvents/…`    → [`COMMON_EVENTS_BUCKET`] (single physical file)
///
/// Returns `None` for keys that don't belong to any injectable Wolf file.
pub fn injection_bucket(segment_key: &str) -> Option<String> {
    let mut parts = segment_key.splitn(2, '/');
    let head = parts.next()?;
    match head {
        "CommonEvents" => {
            // Require at least one path segment after the head so bare
            // "CommonEvents" (no data) is not treated as injectable.
            parts.next()?;
            Some(COMMON_EVENTS_BUCKET.to_string())
        }
        "MapData" | "Database" => {
            let stem = parts.next()?.split('/').next()?;
            if stem.is_empty() {
                return None;
            }
            Some(format!("{head}/{stem}"))
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Step 3 — encode_for_wolf
// ---------------------------------------------------------------------------

fn encode_for_wolf(text: &str, version: &WolfVersion) -> Result<Vec<u8>, InjectorError> {
    if version.is_utf8() {
        Ok(text.as_bytes().to_vec())
    } else {
        encoding::encode_shiftjis(text).map_err(InjectorError::Encoding)
    }
}

// ---------------------------------------------------------------------------
// Step 2 — inject_map (.mps sequential scan + splice, Approach B)
// ---------------------------------------------------------------------------

/// Inject translations into a Wolf RPG `.mps` map file.
///
/// Approach B: parse the file to extract the ordered list of translatable
/// strings, then scan the raw bytes sequentially and splice each ReadString
/// whose source matches an entry in `translations`.
///
/// `map_name` must match the stem used in the segment keys (e.g. `"Map001"`).
pub fn inject_map(
    map_name: &str,
    bytes: &[u8],
    translations: &[WolfTranslation],
    version: &WolfVersion,
) -> Result<(Vec<u8>, InjectionResult), InjectorError> {
    let translation_map: HashMap<&str, &str> = translations
        .iter()
        .map(|t| (t.key.as_str(), t.text.as_str()))
        .collect();

    if v3_format::compression::is_lz4_v3(bytes) {
        let mut updated = 0usize;
        let patched = inject_map_v3(map_name, bytes, &translation_map, &mut updated)?;
        return Ok((
            patched,
            InjectionResult {
                file_path: PathBuf::from(format!("{map_name}.mps")),
                updated_count: updated,
            },
        ));
    }

    let bytes_owned = bytes.to_vec();
    let map = std::panic::catch_unwind(move || Map::parse(&bytes_owned)).map_err(|_| {
        InjectorError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "wolfrpg-map-parser panicked on mps bytes",
        ))
    })?;

    let mut updated = 0usize;
    let patched = patch_mps_strings(
        bytes,
        &map,
        &translation_map,
        map_name,
        version,
        &mut updated,
    )?;

    Ok((
        patched,
        InjectionResult {
            file_path: PathBuf::from(format!("{map_name}.mps")),
            updated_count: updated,
        },
    ))
}

/// Build an ordered list of (source_encoded, target_encoded) pairs for every
/// translatable command in the map, then walk the raw bytes and splice each
/// ReadString matching the source bytes.
///
/// The replacements are consumed in sequential order — guaranteed to match the
/// binary stream order because wolfrpg-map-parser preserves insertion order.
fn patch_mps_strings(
    original: &[u8],
    map: &Map,
    translations: &HashMap<&str, &str>,
    map_name: &str,
    version: &WolfVersion,
    updated: &mut usize,
) -> Result<Vec<u8>, InjectorError> {
    use wolfrpg_map_parser::command::Command;

    // Build ordered replacement list: (source_sjis, target_encoded).
    let mut replacements: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();

    for (event_idx, event) in map.events().iter().enumerate() {
        for (page_idx, page) in event.pages().iter().enumerate() {
            for (cmd_idx, command) in page.commands().iter().enumerate() {
                match command {
                    Command::ShowMessage(cmd) => {
                        let text = cmd.text();
                        if text.trim().is_empty() {
                            continue;
                        }
                        let key = format!(
                            "MapData/{map_name}/events/{event_idx}/pages/{page_idx}/{cmd_idx}"
                        );
                        let src = encode_for_wolf(text, version)?;
                        let dst = match translations.get(key.as_str()) {
                            Some(&t) => encode_for_wolf(t, version)?,
                            None => src.clone(),
                        };
                        replacements.push((src, dst));
                    }
                    Command::ShowChoice(cmd) => {
                        for (choice_idx, choice) in cmd.choices().iter().enumerate() {
                            if choice.trim().is_empty() {
                                continue;
                            }
                            let key = format!(
                                "MapData/{map_name}/events/{event_idx}/pages/{page_idx}/{cmd_idx}/choices/{choice_idx}"
                            );
                            let src = encode_for_wolf(choice, version)?;
                            let dst = match translations.get(key.as_str()) {
                                Some(&t) => encode_for_wolf(t, version)?,
                                None => src.clone(),
                            };
                            replacements.push((src, dst));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(splice_wolf_strings(original, &replacements, updated))
}

/// Sequential ReadString splice shared by the `.mps` and `CommonEvent.dat`
/// injectors (both v2.x binary formats store text as `size: u32 + bytes + 0x00`
/// frames in stream order).
///
/// Walks `original`, and for each `(src, dst)` replacement — consumed strictly
/// in order — splices the next frame whose payload matches `src`. Non-matching
/// bytes are copied through verbatim. Because `replacements` includes a no-op
/// `dst == src` entry for every translatable command (translated or not), each
/// frame is consumed in the exact order the parser emitted it, so duplicate
/// source strings can't de-align.
fn splice_wolf_strings(
    original: &[u8],
    replacements: &[(Vec<u8>, Vec<u8>)],
    updated: &mut usize,
) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::with_capacity(original.len());
    let mut pos = 0usize;
    let mut rep_idx = 0usize;

    while pos < original.len() {
        if rep_idx < replacements.len() {
            let (ref src, ref dst) = replacements[rep_idx];
            let frame_len = 4 + src.len() + 1;
            let expected_size = (src.len() + 1) as u32;

            if pos + frame_len <= original.len()
                && original[pos..pos + 4] == expected_size.to_le_bytes()
                && original[pos + 4..pos + 4 + src.len()] == src[..]
                && original[pos + 4 + src.len()] == 0x00
            {
                let new_size = (dst.len() + 1) as u32;
                out.extend_from_slice(&new_size.to_le_bytes());
                out.extend_from_slice(dst);
                out.push(0x00);
                if src != dst {
                    *updated += 1;
                }
                pos += frame_len;
                rep_idx += 1;
                continue;
            }
        }
        out.push(original[pos]);
        pos += 1;
    }

    out
}

/// Inject translations into a v3.x (LZ4-compressed) `.mps` map using the
/// in-house [`v3_format`] parser: parse to AST, replace `string_args` for
/// `ShowMessage`/`ShowChoice` commands by key, dump, recompress.
///
/// Same `key` format as [`super::extractor::extract_map_segments`]'s v3 path:
/// `MapData/{map_name}/events/{event_idx}/pages/{page_idx}/{cmd_idx}[/choices/{choice_idx}]`.
fn inject_map_v3(
    map_name: &str,
    bytes: &[u8],
    translations: &HashMap<&str, &str>,
    updated: &mut usize,
) -> Result<Vec<u8>, InjectorError> {
    let decompressed = v3_format::compression::decompress_v3(bytes)?;
    let mut map = v3_format::map::MapV3::parse(&decompressed)?;

    for (event_idx, event) in map.events.iter_mut().enumerate() {
        for (page_idx, page) in event.pages.iter_mut().enumerate() {
            for (cmd_idx, command) in page.commands.iter_mut().enumerate() {
                match command.cid {
                    v3_format::command::CID_MESSAGE => {
                        if let Some(text) = command.string_args.first_mut() {
                            if text.trim().is_empty() {
                                continue;
                            }
                            let key = format!(
                                "MapData/{map_name}/events/{event_idx}/pages/{page_idx}/{cmd_idx}"
                            );
                            if let Some(&t) = translations.get(key.as_str()) {
                                if t != text.as_str() {
                                    *updated += 1;
                                    *text = t.to_owned();
                                }
                            }
                        }
                    }
                    v3_format::command::CID_CHOICES => {
                        for (choice_idx, choice) in command.string_args.iter_mut().enumerate() {
                            if choice.trim().is_empty() {
                                continue;
                            }
                            let key = format!(
                                "MapData/{map_name}/events/{event_idx}/pages/{page_idx}/{cmd_idx}/choices/{choice_idx}"
                            );
                            if let Some(&t) = translations.get(key.as_str()) {
                                if t != choice.as_str() {
                                    *updated += 1;
                                    *choice = t.to_owned();
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    let dumped = map.dump()?;
    Ok(v3_format::compression::recompress_v3(&dumped)?)
}

// ---------------------------------------------------------------------------
// Step 4 — serialize_dat + inject_dat
// ---------------------------------------------------------------------------

fn write_wolf_string(buf: &mut Vec<u8>, encoded: &[u8]) {
    let size = (encoded.len() + 1) as u32;
    buf.extend_from_slice(&size.to_le_bytes());
    buf.extend_from_slice(encoded);
    buf.push(0x00);
}

/// Re-serialize a `DatFile` back to `.dat` binary format.
///
/// Must mirror `dat_parser::parse_dat_types` exactly:
///   indicator (1) + magic (9) + version (1) + type_count (4) + types + terminator (1).
///
/// The indicator and version bytes are preserved from `dat_original_header[0]`
/// and `dat_original_header[10]` so the output is byte-identical when no
/// translations are applied (round-trip identity guarantee) — except when
/// `version_byte == DAT_VERSION_LZ4` (Wolf RPG v3.x): the header still
/// declares LZ4, but the payload (type_count through the terminator) is
/// recompressed via `dat_parser::compress_lz4_dat` instead of written plain,
/// since the runtime always LZ4-decompresses bytes 11.. once it sees 0xC4 —
/// writing a plain payload there produces a "database corrupted or too old"
/// error even though the content itself is correct.
fn serialize_dat(
    dat: &DatFile,
    dat_original_header: &[u8],
    version: &WolfVersion,
) -> Result<Vec<u8>, InjectorError> {
    let indicator = dat_original_header[0];
    let version_byte = dat_original_header[10];
    let magic = if dat.is_utf8 {
        DB_MAGIC_UTF8
    } else {
        DB_MAGIC_SJIS
    };

    let mut payload = Vec::new();
    payload.extend_from_slice(&(dat.types.len() as u32).to_le_bytes());

    for dat_type in &dat.types {
        serialize_dat_type(&mut payload, dat_type, version, dat.is_utf8)?;
    }

    payload.push(version_byte);

    let mut buf = Vec::new();
    buf.push(indicator);
    buf.extend_from_slice(&magic);
    buf.push(version_byte);

    if version_byte == dat_parser::DAT_VERSION_LZ4 {
        buf.extend_from_slice(&dat_parser::compress_lz4_dat(&payload));
    } else {
        buf.extend_from_slice(&payload);
    }

    Ok(buf)
}

fn serialize_dat_type(
    buf: &mut Vec<u8>,
    dat_type: &DatType,
    version: &WolfVersion,
    _is_utf8: bool,
) -> Result<(), InjectorError> {
    buf.extend_from_slice(&DAT_TYPE_SEPARATOR);

    // unknown1: preserved as 0 for all synthetic and common real files.
    // Files with STRING_INDICATOR (0x0001_D4C0) would need special handling
    // but are not present in the test fixtures for F4-04.
    let unknown1: u32 = 0;
    buf.extend_from_slice(&unknown1.to_le_bytes());

    buf.extend_from_slice(&(dat_type.fields.len() as u32).to_le_bytes());

    if unknown1 == STRING_INDICATOR_PUB {
        write_wolf_string(buf, &[]);
    }

    for field in &dat_type.fields {
        buf.extend_from_slice(&field.index_info.to_le_bytes());
    }

    buf.extend_from_slice(&(dat_type.entries.len() as u32).to_le_bytes());

    let int_cnt = dat_type
        .fields
        .iter()
        .filter(|f| f.is_valid() && !f.is_string())
        .count();
    let str_cnt = dat_type
        .fields
        .iter()
        .filter(|f| f.is_valid() && f.is_string())
        .count();

    for entry in &dat_type.entries {
        for i in 0..int_cnt {
            let v = entry.int_values.get(i).copied().unwrap_or(0);
            buf.extend_from_slice(&v.to_le_bytes());
        }
        for i in 0..str_cnt {
            let s = entry.string_values.get(i).map(String::as_str).unwrap_or("");
            let encoded = encode_for_wolf(s, version)?;
            write_wolf_string(buf, &encoded);
        }
    }

    Ok(())
}

/// Inject translations into a Wolf RPG `.dat` database file.
///
/// Takes both `.project` (schema, read-only) and `.dat` (values to modify).
/// Returns only the new `.dat` bytes — the `.project` is never modified.
pub fn inject_dat(
    project_bytes: &[u8],
    dat_bytes: &[u8],
    translations: &[WolfTranslation],
    version: &WolfVersion,
) -> Result<(Vec<u8>, InjectionResult), InjectorError> {
    let mut db = dat_parser::parse_database(project_bytes, dat_bytes)
        .map_err(|e| InjectorError::DatParse(e.to_string()))?;

    let mut updated = 0usize;

    for t in translations {
        // Key: "Database/{db_name}/{type_idx}/{entry_idx}/{field_name}"
        let parts: Vec<&str> = t.key.splitn(5, '/').collect();
        if parts.len() < 5 {
            return Err(InjectorError::KeyNotFound(t.key.clone()));
        }
        let type_idx: usize = parts[2]
            .parse()
            .map_err(|_| InjectorError::KeyNotFound(t.key.clone()))?;
        let entry_idx: usize = parts[3]
            .parse()
            .map_err(|_| InjectorError::KeyNotFound(t.key.clone()))?;
        let field_name = parts[4];

        let dat_type = db
            .types
            .get_mut(type_idx)
            .ok_or_else(|| InjectorError::KeyNotFound(t.key.clone()))?;

        // Find the string slot index for this field name.
        let mut str_slot = 0usize;
        let mut found = false;
        for field in &dat_type.fields {
            if !field.is_valid() {
                continue;
            }
            if !field.is_string() {
                continue;
            }
            if field.name == field_name {
                found = true;
                break;
            }
            str_slot += 1;
        }

        if !found {
            return Err(InjectorError::KeyNotFound(t.key.clone()));
        }

        let entry = dat_type
            .entries
            .get_mut(entry_idx)
            .ok_or_else(|| InjectorError::KeyNotFound(t.key.clone()))?;

        let slot = entry
            .string_values
            .get_mut(str_slot)
            .ok_or_else(|| InjectorError::KeyNotFound(t.key.clone()))?;

        *slot = t.text.clone();
        updated += 1;
    }

    let new_bytes = serialize_dat(&db, dat_bytes, version)?;

    Ok((
        new_bytes,
        InjectionResult {
            file_path: PathBuf::new(),
            updated_count: updated,
        },
    ))
}

// ---------------------------------------------------------------------------
// Step 4b — inject_common_events (CommonEvent.dat)
// ---------------------------------------------------------------------------

/// Inject translations into a Wolf RPG `CommonEvent.dat` file.
///
/// Symmetric to [`inject_map`]: v2.x uses the sequential ReadString splice
/// (via [`splice_wolf_strings`]); v3.5 (Inko, LZ4-compressed) parses to the
/// in-house [`v3_format`] AST, replaces `string_args` by key, then dumps and
/// recompresses.
///
/// Keys match [`super::extractor::extract_common_events`]:
/// `CommonEvents/{event_name}/{event_idx}/{cmd_idx}[/choices/{choice_idx}]`
/// (flat command list — no `pages` level, unlike maps).
pub fn inject_common_events(
    bytes: &[u8],
    translations: &[WolfTranslation],
    version: &WolfVersion,
) -> Result<(Vec<u8>, InjectionResult), InjectorError> {
    let translation_map: HashMap<&str, &str> = translations
        .iter()
        .map(|t| (t.key.as_str(), t.text.as_str()))
        .collect();

    let mut updated = 0usize;
    let patched = if v3_format::common_events::is_lz4_v3(bytes) {
        inject_common_events_v3(bytes, &translation_map, &mut updated)?
    } else {
        let bytes_owned = bytes.to_vec();
        let events = std::panic::catch_unwind(move || {
            wolfrpg_map_parser::common_events_parser::parse_bytes(&bytes_owned)
        })
        .map_err(|_| {
            InjectorError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "wolfrpg-map-parser panicked on CommonEvent.dat bytes",
            ))
        })?;
        patch_common_events_strings(bytes, &events, &translation_map, version, &mut updated)?
    };

    Ok((
        patched,
        InjectionResult {
            file_path: PathBuf::from("CommonEvent.dat"),
            updated_count: updated,
        },
    ))
}

/// Build the ordered replacement list for every translatable command in a v2.x
/// `CommonEvent.dat` and splice it into the raw bytes — mirrors
/// [`patch_mps_strings`], but with the flat common-event command list.
fn patch_common_events_strings(
    original: &[u8],
    events: &[wolfrpg_map_parser::db_parser::common_event::CommonEvent],
    translations: &HashMap<&str, &str>,
    version: &WolfVersion,
    updated: &mut usize,
) -> Result<Vec<u8>, InjectorError> {
    use wolfrpg_map_parser::command::Command;

    let mut replacements: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();

    for (event_idx, event) in events.iter().enumerate() {
        let event_name = event.event_name();
        for (cmd_idx, command) in event.commands().iter().enumerate() {
            match command {
                Command::ShowMessage(cmd) => {
                    let text = cmd.text();
                    if text.trim().is_empty() {
                        continue;
                    }
                    let key = format!("CommonEvents/{event_name}/{event_idx}/{cmd_idx}");
                    let src = encode_for_wolf(text, version)?;
                    let dst = match translations.get(key.as_str()) {
                        Some(&t) => encode_for_wolf(t, version)?,
                        None => src.clone(),
                    };
                    replacements.push((src, dst));
                }
                Command::ShowChoice(cmd) => {
                    for (choice_idx, choice) in cmd.choices().iter().enumerate() {
                        if choice.trim().is_empty() {
                            continue;
                        }
                        let key = format!(
                            "CommonEvents/{event_name}/{event_idx}/{cmd_idx}/choices/{choice_idx}"
                        );
                        let src = encode_for_wolf(choice, version)?;
                        let dst = match translations.get(key.as_str()) {
                            Some(&t) => encode_for_wolf(t, version)?,
                            None => src.clone(),
                        };
                        replacements.push((src, dst));
                    }
                }
                _ => {}
            }
        }
    }

    Ok(splice_wolf_strings(original, &replacements, updated))
}

/// Inject translations into a v3.5 (LZ4-compressed) `CommonEvent.dat` using the
/// in-house [`v3_format`] parser — mirrors [`inject_map_v3`] with the flat
/// common-event command list.
fn inject_common_events_v3(
    bytes: &[u8],
    translations: &HashMap<&str, &str>,
    updated: &mut usize,
) -> Result<Vec<u8>, InjectorError> {
    let decompressed = v3_format::common_events::decompress(bytes)?;
    let mut common_events = v3_format::common_events::CommonEventsV3::parse(&decompressed)?;

    for (event_idx, event) in common_events.events.iter_mut().enumerate() {
        let event_name = event.name.clone();
        for (cmd_idx, command) in event.commands.iter_mut().enumerate() {
            match command.cid {
                v3_format::command::CID_MESSAGE => {
                    if let Some(text) = command.string_args.first_mut() {
                        if text.trim().is_empty() {
                            continue;
                        }
                        let key = format!("CommonEvents/{event_name}/{event_idx}/{cmd_idx}");
                        if let Some(&t) = translations.get(key.as_str()) {
                            if t != text.as_str() {
                                *updated += 1;
                                *text = t.to_owned();
                            }
                        }
                    }
                }
                v3_format::command::CID_CHOICES => {
                    for (choice_idx, choice) in command.string_args.iter_mut().enumerate() {
                        if choice.trim().is_empty() {
                            continue;
                        }
                        let key = format!(
                            "CommonEvents/{event_name}/{event_idx}/{cmd_idx}/choices/{choice_idx}"
                        );
                        if let Some(&t) = translations.get(key.as_str()) {
                            if t != choice.as_str() {
                                *updated += 1;
                                *choice = t.to_owned();
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    let dumped = common_events.dump()?;
    Ok(v3_format::common_events::recompress(&dumped)?)
}

// ---------------------------------------------------------------------------
// Step 5 — inject_all (Option A export)
// ---------------------------------------------------------------------------

/// Inject all translations into a Wolf RPG game directory (Option A: decrypted).
///
/// Writes patched `.mps` / `.dat` files to `Data/MapData/` and
/// `Data/BasicData/`.  Wolf RPG reads `Data/` with priority over `.wolf`
/// archives — no re-encryption needed (Option B deferred to F5).
///
/// Keys in `translations_by_file`:
///   `"MapData/{stem}"`   → translations for that `.mps`
///   `"Database/{stem}"`  → translations for that `.dat` pair
/// Inject translations and return `(relative_zip_path, bytes)` pairs — no disk I/O.
/// Used by the zip export path.
pub async fn inject_all_to_memory(
    game_dir: &Path,
    translations_by_file: &HashMap<String, Vec<WolfTranslation>>,
    version: &WolfVersion,
) -> Result<Vec<(String, Vec<u8>)>, InjectorError> {
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();

    for (file_key, translations) in translations_by_file {
        let parts: Vec<&str> = file_key.splitn(2, '/').collect();
        if parts.len() < 2 {
            continue;
        }
        let stem = parts[1];

        match parts[0] {
            "MapData" => {
                let Some(bytes) = super::extractor::load_mps_for_stem(game_dir, stem) else {
                    continue;
                };
                let (new_bytes, _) = inject_map(stem, &bytes, translations, version)?;
                entries.push((format!("Data/MapData/{stem}.mps"), new_bytes));
            }
            "Database" => {
                let Some((project_bytes, dat_bytes)) =
                    super::extractor::load_dat_for_stem(game_dir, stem)
                else {
                    return Err(InjectorError::MissingProject(stem.to_string()));
                };
                let (new_bytes, _) = inject_dat(&project_bytes, &dat_bytes, translations, version)?;
                entries.push((format!("Data/BasicData/{stem}.dat"), new_bytes));
            }
            "CommonEvents" => {
                // All CommonEvents/* keys share the single CommonEvent.dat.
                let Some(bytes) = super::extractor::load_common_event_bytes(game_dir) else {
                    continue;
                };
                let (new_bytes, _) = inject_common_events(&bytes, translations, version)?;
                entries.push(("Data/BasicData/CommonEvent.dat".to_string(), new_bytes));
            }
            _ => {}
        }
    }

    Ok(entries)
}

pub async fn inject_all(
    game_dir: &Path,
    translations_by_file: &HashMap<String, Vec<WolfTranslation>>,
    version: &WolfVersion,
) -> Result<Vec<InjectionResult>, InjectorError> {
    let map_dir = game_dir.join("Data").join("MapData");
    let db_dir = game_dir.join("Data").join("BasicData");

    std::fs::create_dir_all(&map_dir)?;
    std::fs::create_dir_all(&db_dir)?;

    let mut results = Vec::new();

    for (file_key, translations) in translations_by_file {
        let parts: Vec<&str> = file_key.splitn(2, '/').collect();
        if parts.len() < 2 {
            continue;
        }
        let stem = parts[1];

        match parts[0] {
            "MapData" => {
                // Load source bytes: Data/MapData/ first, then .wolf archives.
                let Some(bytes) = super::extractor::load_mps_for_stem(game_dir, stem) else {
                    continue;
                };
                let (new_bytes, mut result) = inject_map(stem, &bytes, translations, version)?;
                let out = map_dir.join(format!("{stem}.mps"));
                result.file_path = out.clone();
                std::fs::write(&out, &new_bytes)?;
                results.push(result);
            }
            "Database" => {
                // Load source bytes: Data/BasicData/ first, then .wolf archives.
                let Some((project_bytes, dat_bytes)) =
                    super::extractor::load_dat_for_stem(game_dir, stem)
                else {
                    return Err(InjectorError::MissingProject(stem.to_string()));
                };
                let (new_bytes, mut result) =
                    inject_dat(&project_bytes, &dat_bytes, translations, version)?;
                let out = db_dir.join(format!("{stem}.dat"));
                result.file_path = out.clone();
                // Write only .dat — .project is never modified.
                std::fs::write(&out, &new_bytes)?;
                results.push(result);
            }
            "CommonEvents" => {
                // All CommonEvents/* keys share the single CommonEvent.dat.
                let Some(bytes) = super::extractor::load_common_event_bytes(game_dir) else {
                    continue;
                };
                let (new_bytes, mut result) = inject_common_events(&bytes, translations, version)?;
                let out = db_dir.join("CommonEvent.dat");
                result.file_path = out.clone();
                std::fs::write(&out, &new_bytes)?;
                results.push(result);
            }
            _ => {}
        }
    }

    Ok(results)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::detector::WolfVersion;
    use crate::engines::wolf::dat_parser::{make_minimal_dat_pub, make_minimal_project_pub};

    fn v2() -> WolfVersion {
        WolfVersion { major: 2, minor: 0 }
    }

    fn v3() -> WolfVersion {
        WolfVersion { major: 3, minor: 0 }
    }

    // -----------------------------------------------------------------------
    // Helpers for .mps tests — duplicated from extractor tests (no shared
    // test utils module yet; acceptable for F4-04 scope)
    // -----------------------------------------------------------------------

    fn sjis_string(text: &str) -> Vec<u8> {
        use encoding_rs::SHIFT_JIS;
        let (enc, _, _) = SHIFT_JIS.encode(text);
        let len = (enc.len() + 1) as u32;
        let mut out = len.to_le_bytes().to_vec();
        out.extend_from_slice(&enc);
        out.push(0x00);
        out
    }

    fn make_show_message_cmd(text: &str) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&0x01650000u32.to_be_bytes()); // ShowMessage
        b.push(0x00);
        b.push(0x00);
        b.push(0x00);
        b.extend(sjis_string(text));
        b.push(0x00);
        b
    }

    fn make_page(commands_bytes: &[u8], command_count: u32) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(b"\x79\xff\xff\xff\xff");
        b.extend(sjis_string(""));
        b.push(0x02);
        b.push(0x00);
        b.push(0x00);
        b.push(0x00);
        b.push(0x00);
        b.extend_from_slice(&[0x00; 36]);
        b.extend_from_slice(&[0x00; 6]);
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&command_count.to_le_bytes());
        b.extend_from_slice(commands_bytes);
        b.extend_from_slice(&0u32.to_le_bytes());
        b.push(0x00);
        b.push(0x00);
        b.push(0x00);
        b.push(0x7a);
        b
    }

    fn make_event(page_bytes: &[u8]) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&0x6f393000u32.to_be_bytes());
        b.push(0x00);
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend(sjis_string("E"));
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(page_bytes);
        b.push(0x70);
        b
    }

    fn make_mps_with_event(event_bytes: &[u8]) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(
            b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x57\x4F\x4C\x46\x4D\x00\x00\x00\x00\x00",
        );
        b.extend_from_slice(&[0x00; 5]);
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&[0x00; 4]);
        b.extend_from_slice(&[0x00; 4]);
        b.extend_from_slice(&[0x00; 4]);
        b.extend_from_slice(event_bytes);
        b.push(0x66);
        b
    }

    // -----------------------------------------------------------------------
    // Step 3 — encode_for_wolf
    // -----------------------------------------------------------------------

    #[test]
    fn test_encode_french_accents_in_v2() {
        let result = encode_for_wolf("café", &v2());
        assert!(
            matches!(result, Err(InjectorError::Encoding(_))),
            "accented chars must fail for Wolf v2 (Shift-JIS)"
        );
    }

    #[test]
    fn test_encode_french_accents_in_v3() {
        let result = encode_for_wolf("café", &v3());
        assert!(result.is_ok(), "UTF-8 v3 should accept accented chars");
        assert_eq!(result.unwrap(), "café".as_bytes());
    }

    #[test]
    fn test_encode_ascii_both_versions() {
        assert!(encode_for_wolf("Hello", &v2()).is_ok());
        assert!(encode_for_wolf("Hello", &v3()).is_ok());
    }

    // -----------------------------------------------------------------------
    // Step 2 — inject_map
    // -----------------------------------------------------------------------

    #[test]
    fn test_inject_map_identity() {
        let cmd = make_show_message_cmd("テスト");
        let page = make_page(&cmd, 1);
        let event = make_event(&page);
        let mps = make_mps_with_event(&event);

        let (new_bytes, result) = inject_map("Map001", &mps, &[], &v2()).unwrap();
        assert_eq!(result.updated_count, 0);
        assert_eq!(
            new_bytes, mps,
            "identity injection must produce identical bytes"
        );
    }

    #[test]
    fn test_inject_map_translation() {
        let cmd = make_show_message_cmd("テスト");
        let page = make_page(&cmd, 1);
        let event = make_event(&page);
        let mps = make_mps_with_event(&event);

        // Use a same-byte-length replacement to keep the test simple.
        let translations = vec![WolfTranslation {
            key: "MapData/Map001/events/0/pages/0/0".to_string(),
            text: "Hello!!".to_string(), // 7 bytes (same as テスト in SJIS = 6 bytes... actually different)
        }];
        let (new_bytes, result) = inject_map("Map001", &mps, &translations, &v2()).unwrap();
        assert_eq!(result.updated_count, 1);

        // Re-parse to verify text changed.
        use wolfrpg_map_parser::{command::Command, Map};
        let re_parsed = Map::parse(&new_bytes);
        let text = re_parsed
            .events()
            .first()
            .and_then(|e| e.pages().first())
            .and_then(|p| p.commands().first())
            .and_then(|c| {
                if let Command::ShowMessage(m) = c {
                    Some(m.text().to_owned())
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(text, "Hello!!");
    }

    #[test]
    fn test_inject_map_wrong_key() {
        // A key that doesn't match any command is simply ignored (not an error).
        let cmd = make_show_message_cmd("テスト");
        let page = make_page(&cmd, 1);
        let event = make_event(&page);
        let mps = make_mps_with_event(&event);

        let translations = vec![WolfTranslation {
            key: "MapData/Map001/events/99/pages/0/0".to_string(),
            text: "Nope".to_string(),
        }];
        let (new_bytes, result) = inject_map("Map001", &mps, &translations, &v2()).unwrap();
        assert_eq!(result.updated_count, 0);
        assert_eq!(new_bytes, mps, "unknown key must leave file unchanged");
    }

    // -----------------------------------------------------------------------
    // Step 4 — inject_dat
    // -----------------------------------------------------------------------

    #[test]
    fn test_inject_dat_identity() {
        let project = make_minimal_project_pub("Type", "name", "entry");
        let dat = make_minimal_dat_pub("テスト");

        let (new_bytes, result) = inject_dat(&project, &dat, &[], &v2()).unwrap();
        assert_eq!(result.updated_count, 0);
        assert_eq!(
            new_bytes, dat,
            "identity injection must produce identical bytes"
        );
    }

    #[test]
    fn test_inject_dat_name() {
        let project = make_minimal_project_pub("Type", "name", "entry");
        let dat = make_minimal_dat_pub("テスト");

        let translations = vec![WolfTranslation {
            key: "Database/TestDB/0/0/name".to_string(),
            text: "Test".to_string(),
        }];
        let (new_bytes, result) = inject_dat(&project, &dat, &translations, &v2()).unwrap();
        assert_eq!(result.updated_count, 1);

        let db = dat_parser::parse_database(&project, &new_bytes).unwrap();
        assert_eq!(db.types[0].entries[0].string_values[0], "Test");
    }

    #[test]
    fn test_inject_dat_wrong_key() {
        let project = make_minimal_project_pub("Type", "name", "entry");
        let dat = make_minimal_dat_pub("テスト");

        let translations = vec![WolfTranslation {
            key: "Database/TestDB/99/0/name".to_string(),
            text: "Nope".to_string(),
        }];
        let result = inject_dat(&project, &dat, &translations, &v2());
        assert!(matches!(result, Err(InjectorError::KeyNotFound(_))));
    }

    /// Regression for the real-world "database corrupted or too old"
    /// crash: a v3.5 database (`BasicData/DataBase.dat`, version byte
    /// `0xC4`) must come back out LZ4-compressed, not plain bytes under an
    /// unchanged `0xC4` header — the Wolf RPG Editor runtime always
    /// LZ4-decompresses bytes 11.. once it sees that version byte.
    #[test]
    fn test_inject_dat_lz4_v3_recompresses() {
        const STRING_FIELD_START: u32 = 0x07D0; // dat_parser::STRING_FIELD_START

        // `make_minimal_project_pub` encodes names as Shift-JIS, so the
        // paired .dat below must use the matching SJIS magic — the LZ4
        // wrapping being tested here is orthogonal to string encoding.
        let project = make_minimal_project_pub("キャラ", "名前", "");

        let mut payload = Vec::new();
        payload.extend_from_slice(&1u32.to_le_bytes()); // type_count
        payload.extend_from_slice(&DAT_TYPE_SEPARATOR);
        payload.extend_from_slice(&0u32.to_le_bytes()); // unknown1
        payload.extend_from_slice(&1u32.to_le_bytes()); // fields_size
        payload.extend_from_slice(&STRING_FIELD_START.to_le_bytes());
        payload.extend_from_slice(&1u32.to_le_bytes()); // data_count
        payload.extend(sjis_string("ハルカ"));
        payload.push(0xC4); // terminator == version byte

        let compressed = lz4_flex::block::compress(&payload);
        let mut dat = vec![0x00]; // indicator = unencrypted
        dat.extend_from_slice(&DB_MAGIC_SJIS);
        dat.push(0xC4); // version = LZ4 (Wolf RPG v3.x)
        dat.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        dat.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
        dat.extend_from_slice(&compressed);

        let translations = vec![WolfTranslation {
            key: "Database/キャラ/0/0/名前".to_string(),
            text: "Haruka".to_string(),
        }];
        let (new_bytes, result) = inject_dat(&project, &dat, &translations, &v3()).unwrap();
        assert_eq!(result.updated_count, 1);

        // Header still declares LZ4 (0xC4) — the bytes right after the
        // header must therefore actually BE an LZ4 block, not the plain
        // DAT_TYPE_SEPARATOR that a non-recompressing bug would leave there.
        assert_eq!(new_bytes[10], 0xC4);
        assert_ne!(&new_bytes[15..19], DAT_TYPE_SEPARATOR.as_slice());

        // The real assertion: re-parsing through the same decompression path
        // the Wolf RPG Editor runtime uses must succeed and see the update.
        let reparsed = dat_parser::parse_database(&project, &new_bytes).unwrap();
        assert_eq!(reparsed.types[0].entries[0].string_values, ["Haruka"]);
    }

    // -----------------------------------------------------------------------
    // Step 6 — round-trip tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_round_trip_mps_identity() {
        let cmd = make_show_message_cmd("こんにちは");
        let page = make_page(&cmd, 1);
        let event = make_event(&page);
        let mps = make_mps_with_event(&event);

        let segs =
            crate::engines::wolf::extractor::extract_map_segments("RtMap", &mps, &v2()).unwrap();
        assert_eq!(segs.len(), 1);

        // Identity translations: inject source text as-is.
        let translations: Vec<WolfTranslation> = segs
            .iter()
            .map(|s| WolfTranslation {
                key: s.key.clone(),
                text: s.source_text.clone(),
            })
            .collect();

        let (new_bytes, result) = inject_map("RtMap", &mps, &translations, &v2()).unwrap();
        // Identity replacement: updated_count == 0 because src == dst.
        assert_eq!(result.updated_count, 0);
        assert_eq!(
            new_bytes, mps,
            "round-trip identity must produce identical bytes"
        );
    }

    #[test]
    fn test_round_trip_mps_translation() {
        let cmd = make_show_message_cmd("こんにちは");
        let page = make_page(&cmd, 1);
        let event = make_event(&page);
        let mps = make_mps_with_event(&event);

        let segs =
            crate::engines::wolf::extractor::extract_map_segments("RtMap", &mps, &v2()).unwrap();
        assert_eq!(segs.len(), 1);

        let translations = vec![WolfTranslation {
            key: segs[0].key.clone(),
            text: "Hello".to_string(),
        }];
        let (new_bytes, result) = inject_map("RtMap", &mps, &translations, &v2()).unwrap();
        assert_eq!(result.updated_count, 1);

        use wolfrpg_map_parser::{command::Command, Map};
        let re_parsed = Map::parse(&new_bytes);
        let text = re_parsed
            .events()
            .first()
            .and_then(|e| e.pages().first())
            .and_then(|p| p.commands().first())
            .and_then(|c| {
                if let Command::ShowMessage(m) = c {
                    Some(m.text().to_owned())
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(text, "Hello");
    }

    #[test]
    fn test_round_trip_dat_identity() {
        let project = make_minimal_project_pub("キャラ", "名前", "hero");
        let dat = make_minimal_dat_pub("テスト");

        let segs = crate::engines::wolf::extractor::extract_database_segments(
            "TestDB",
            &project,
            &dat,
            &v2(),
        )
        .unwrap();
        assert!(!segs.is_empty());

        let (new_bytes, result) = inject_dat(&project, &dat, &[], &v2()).unwrap();
        assert_eq!(result.updated_count, 0);
        assert_eq!(
            new_bytes, dat,
            "round-trip identity must produce identical bytes"
        );
    }

    #[test]
    fn test_round_trip_dat_translation() {
        let project = make_minimal_project_pub("キャラ", "名前", "hero");
        let dat = make_minimal_dat_pub("テスト");

        let segs = crate::engines::wolf::extractor::extract_database_segments(
            "TestDB",
            &project,
            &dat,
            &v2(),
        )
        .unwrap();
        assert_eq!(segs.len(), 1);

        let translations = vec![WolfTranslation {
            key: segs[0].key.clone(),
            text: "Test".to_string(),
        }];
        let (new_bytes, result) = inject_dat(&project, &dat, &translations, &v2()).unwrap();
        assert_eq!(result.updated_count, 1);

        let db = dat_parser::parse_database(&project, &new_bytes).unwrap();
        assert_eq!(db.types[0].entries[0].string_values[0], "Test");
    }

    // -----------------------------------------------------------------------
    // Step 5 — inject_all
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_inject_all_creates_files() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path();

        let basic_data = game_dir.join("Data").join("BasicData");
        std::fs::create_dir_all(&basic_data).unwrap();

        let project = make_minimal_project_pub("Type", "name", "e");
        let dat = make_minimal_dat_pub("テスト");
        std::fs::write(basic_data.join("TestDB.project"), &project).unwrap();
        std::fs::write(basic_data.join("TestDB.dat"), &dat).unwrap();

        let translations = vec![WolfTranslation {
            key: "Database/TestDB/0/0/name".to_string(),
            text: "Hello".to_string(),
        }];
        let mut by_file: HashMap<String, Vec<WolfTranslation>> = HashMap::new();
        by_file.insert("Database/TestDB".to_string(), translations);

        let results = inject_all(game_dir, &by_file, &v2()).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].updated_count, 1);

        let out_dat = basic_data.join("TestDB.dat");
        assert!(out_dat.exists(), "output .dat must be written");

        // .project must not be modified.
        let project_on_disk = std::fs::read(basic_data.join("TestDB.project")).unwrap();
        assert_eq!(project_on_disk, project, ".project must never be modified");
    }

    #[tokio::test]
    async fn test_inject_all_does_not_overwrite_wolf() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path();

        let data_dir = game_dir.join("Data");
        std::fs::create_dir_all(&data_dir).unwrap();
        let wolf_path = data_dir.join("MapData.wolf");
        std::fs::write(&wolf_path, b"fake wolf archive").unwrap();

        let by_file: HashMap<String, Vec<WolfTranslation>> = HashMap::new();
        let _ = inject_all(game_dir, &by_file, &v2()).await.unwrap();

        let content = std::fs::read(&wolf_path).unwrap();
        assert_eq!(content, b"fake wolf archive", ".wolf must not be modified");
    }

    fn test_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("test")
    }

    /// Round-trip injection on a real Inko (v3.x LZ4) `.mps` map: extract
    /// segments, inject a translation, decompress the result, and verify the
    /// translated text appears via re-parsing — and that LZ4 recompression
    /// stays self-consistent (`is_lz4_v3` + `decompress_v3` succeed).
    #[test]
    fn test_round_trip_mps_v3_translation() {
        let path = test_dir().join("Densyanai_Inko_ver2.0/Data/MapData/TitleMap.mps");
        if !path.exists() {
            return;
        }
        let bytes = std::fs::read(&path).unwrap();

        let segs = crate::engines::wolf::extractor::extract_map_segments("TitleMap", &bytes, &v3())
            .unwrap();
        assert!(!segs.is_empty());

        // Translate just the first segment.
        let translations = vec![WolfTranslation {
            key: segs[0].key.clone(),
            text: "Hello, world!".to_owned(),
        }];

        let (new_bytes, result) = inject_map("TitleMap", &bytes, &translations, &v3()).unwrap();
        assert_eq!(result.updated_count, 1);

        assert!(v3_format::compression::is_lz4_v3(&new_bytes));
        let decompressed = v3_format::compression::decompress_v3(&new_bytes).unwrap();
        let map = v3_format::map::MapV3::parse(&decompressed).unwrap();

        let new_segs =
            crate::engines::wolf::extractor::extract_map_segments("TitleMap", &new_bytes, &v3())
                .unwrap();
        assert_eq!(new_segs[0].source_text, "Hello, world!");

        // Round-trip: re-parsing must still satisfy parse(dump(x)) == x.
        let redumped = map.dump().unwrap();
        assert_eq!(redumped, decompressed);
    }

    // -----------------------------------------------------------------------
    // Common Events injection (Phase 1 — audit 2026-07-01)
    // -----------------------------------------------------------------------

    /// Build a temp game dir containing a synthetic v3.5 `CommonEvent.dat` with
    /// the given `(event_name, message)` events, and return `(tmp, game_dir)`.
    fn make_ce_game_dir(events: &[(&str, &str)]) -> (tempfile::TempDir, std::path::PathBuf) {
        use crate::engines::wolf::v3_format::common_events::{recompress, CommonEventsV3};

        let ce = CommonEventsV3::synthetic_messages(events);
        let decompressed = ce.dump().unwrap();
        let on_disk = recompress(&decompressed).unwrap();

        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().to_path_buf();
        let basic = game_dir.join("Data").join("BasicData");
        std::fs::create_dir_all(&basic).unwrap();
        std::fs::write(basic.join("CommonEvent.dat"), &on_disk).unwrap();
        (tmp, game_dir)
    }

    /// Reproduces the Phase 1 data-loss bug: Common Events translations are
    /// extracted and bucketed, but `inject_all_to_memory` silently drops them
    /// because there is no `"CommonEvents"` dispatch arm. Two distinct event
    /// names are used so the fix must also collapse them into a single bucket
    /// (one physical `CommonEvent.dat`) — a naive per-event bucket would keep
    /// only the last event's translation.
    #[tokio::test]
    async fn test_common_events_translations_survive_export() {
        use crate::engines::wolf::extractor::extract_common_events;

        let (_tmp, game_dir) = make_ce_game_dir(&[("EvA", "こんにちは"), ("EvB", "さようなら")]);
        let ce_bytes = std::fs::read(game_dir.join("Data/BasicData/CommonEvent.dat")).unwrap();

        // Extraction yields two CommonEvents segments (key parity source).
        let segs = extract_common_events(&ce_bytes, &v3()).unwrap();
        assert_eq!(segs.len(), 2, "two CommonEvents segments expected");

        // Bucket exactly the way the export pipeline does.
        let mut by_file: HashMap<String, Vec<WolfTranslation>> = HashMap::new();
        for seg in &segs {
            let bucket = injection_bucket(&seg.key).expect("wolf key must have a bucket");
            let text = format!("[T]{}", seg.source_text);
            by_file.entry(bucket).or_default().push(WolfTranslation {
                key: seg.key.clone(),
                text,
            });
        }

        let entries = inject_all_to_memory(&game_dir, &by_file, &v3())
            .await
            .unwrap();

        // Exactly one physical CommonEvent.dat entry, and BOTH translations
        // must survive into it.
        let ce_entries: Vec<_> = entries
            .iter()
            .filter(|(p, _)| p.ends_with("CommonEvent.dat"))
            .collect();
        assert_eq!(
            ce_entries.len(),
            1,
            "exactly one CommonEvent.dat entry expected, got {}",
            ce_entries.len()
        );

        let new_bytes = &ce_entries[0].1;
        let new_segs = extract_common_events(new_bytes, &v3()).unwrap();
        let texts: Vec<&str> = new_segs.iter().map(|s| s.source_text.as_str()).collect();
        assert!(
            texts.contains(&"[T]こんにちは"),
            "EvA translation missing: {texts:?}"
        );
        assert!(
            texts.contains(&"[T]さようなら"),
            "EvB translation missing: {texts:?}"
        );
    }

    /// All `CommonEvents/*` keys must collapse to a single bucket (one physical
    /// `CommonEvent.dat`), while Map/Database keys bucket per stem.
    #[test]
    fn test_injection_bucket_collapses_common_events() {
        assert_eq!(
            injection_bucket("CommonEvents/EvA/0/0").as_deref(),
            injection_bucket("CommonEvents/EvB/1/0").as_deref(),
            "different common-event names must share one bucket"
        );
        assert_eq!(
            injection_bucket("MapData/Map001/events/0/pages/0/0").as_deref(),
            Some("MapData/Map001"),
        );
        assert_eq!(
            injection_bucket("Database/DataBase/0/0/name").as_deref(),
            Some("Database/DataBase"),
        );
        assert_eq!(injection_bucket("nope").as_deref(), None);
    }

    // -- v2.x (Honoka) CommonEvent.dat synthetic fixture -------------------
    //
    // The real Honoka fixture isn't present in every checkout, so we build a
    // minimal-but-valid v2 `CommonEvent.dat` by hand (Shift-JIS, one event with
    // one ShowMessage + the Exit terminator). Layout mirrors the forked
    // `wolfrpg_map_parser::db_parser::models::common_event::CommonEvent::parse`.

    /// Exit command frame: signature `0x01000000` (BE) + 1 command padding +
    /// 3 bytes consumed by `parse_empty`.
    fn make_exit_cmd() -> Vec<u8> {
        let mut b = 0x0100_0000u32.to_be_bytes().to_vec();
        b.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        b
    }

    /// One v2 common event with a single ShowMessage command and the Exit
    /// terminator. `command_count` is 2 (ShowMessage + Exit, each counting 1).
    fn make_v2_common_event(id: u32, name: &str, message: &str) -> Vec<u8> {
        let mut b = Vec::new();
        b.push(0x8e); // EVENT_SIGNATURE
        b.extend_from_slice(&id.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes()); // event_type
        b.extend_from_slice(&[0u8; 5]); // padding
        b.push(0x00); // number_arguments_count
        b.push(0x00); // string_arguments_count
        b.extend(sjis_string(name)); // event_name
        b.extend_from_slice(&2u32.to_le_bytes()); // command_count
        b.extend(make_show_message_cmd(message));
        b.extend(make_exit_cmd());
        b.extend(sjis_string("")); // unknown string
        b.extend(sjis_string("")); // note
        b.push(0x00); // padding
        b.extend_from_slice(&0u32.to_le_bytes()); // argument_names_count
        b.extend_from_slice(&0u32.to_le_bytes()); // argument_types_count
        b.extend_from_slice(&0u32.to_le_bytes()); // db_options count
        b.extend_from_slice(&0u32.to_le_bytes()); // db_references count
        b.extend_from_slice(&0u32.to_le_bytes()); // argument_values len
        b.push(0x00); // padding
        b.extend_from_slice(&0u32.to_le_bytes()); // color
        for _ in 0..100 {
            b.extend(sjis_string("")); // var_names (fixed 100)
        }
        b.push(0x00); // padding
        b.extend_from_slice(&0u32.to_le_bytes()); // trailing "4 + len" block (len 0)
        b.push(0x00); // padding
        b.extend(sjis_string("")); // return_name
        b.extend_from_slice(&0u32.to_le_bytes()); // return_variable
        b.push(0x92); // END_SIGNATURE
        b
    }

    /// A full v2 `CommonEvent.dat`: 11-byte magic (version `0x8F`, non-LZ4) +
    /// event count + events.
    fn make_v2_common_event_dat(events: &[(&str, &str)]) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(b"\x00\x57\x00\x00\x4F\x4C\x00\x46\x43\x00\x8F");
        b.extend_from_slice(&(events.len() as u32).to_le_bytes());
        for (i, (name, msg)) in events.iter().enumerate() {
            b.extend(make_v2_common_event(i as u32 + 1, name, msg));
        }
        b
    }

    fn write_ce_dat(bytes: &[u8]) -> (tempfile::TempDir, std::path::PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().to_path_buf();
        let basic = game_dir.join("Data").join("BasicData");
        std::fs::create_dir_all(&basic).unwrap();
        std::fs::write(basic.join("CommonEvent.dat"), bytes).unwrap();
        (tmp, game_dir)
    }

    /// The synthetic v2 fixture must parse and yield the message segment — this
    /// guards the fixture itself before the round-trip tests rely on it.
    #[test]
    fn test_v2_common_event_fixture_extracts() {
        use crate::engines::wolf::extractor::extract_common_events;
        let bytes = make_v2_common_event_dat(&[("EvA", "Hello")]);
        assert!(
            !v3_format::common_events::is_lz4_v3(&bytes),
            "must be v2 (non-LZ4)"
        );
        let segs = extract_common_events(&bytes, &v2()).unwrap();
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].source_text, "Hello");
        assert_eq!(segs[0].key, "CommonEvents/EvA/0/0");
    }

    /// v2 (Honoka) end-to-end: two events → bucket → inject → re-extract, both
    /// translations survive into the single `CommonEvent.dat`.
    #[tokio::test]
    async fn test_v2_common_events_translations_survive_export() {
        use crate::engines::wolf::extractor::extract_common_events;

        let bytes = make_v2_common_event_dat(&[("EvA", "Hello"), ("EvB", "World")]);
        let (_tmp, game_dir) = write_ce_dat(&bytes);

        let segs = extract_common_events(&bytes, &v2()).unwrap();
        assert_eq!(segs.len(), 2);

        let mut by_file: HashMap<String, Vec<WolfTranslation>> = HashMap::new();
        for seg in &segs {
            let bucket = injection_bucket(&seg.key).unwrap();
            by_file.entry(bucket).or_default().push(WolfTranslation {
                key: seg.key.clone(),
                text: format!("[T]{}", seg.source_text),
            });
        }

        let entries = inject_all_to_memory(&game_dir, &by_file, &v2())
            .await
            .unwrap();
        let ce: Vec<_> = entries
            .iter()
            .filter(|(p, _)| p.ends_with("CommonEvent.dat"))
            .collect();
        assert_eq!(ce.len(), 1);

        let new_segs = extract_common_events(&ce[0].1, &v2()).unwrap();
        let texts: Vec<&str> = new_segs.iter().map(|s| s.source_text.as_str()).collect();
        assert!(texts.contains(&"[T]Hello"), "EvA missing: {texts:?}");
        assert!(texts.contains(&"[T]World"), "EvB missing: {texts:?}");
    }

    /// v2 identity: empty translations must reproduce the input byte-for-byte
    /// (no de-alignment on the sequential splice).
    #[test]
    fn test_v2_common_events_identity() {
        let bytes = make_v2_common_event_dat(&[("EvA", "Hello"), ("EvA", "Hello")]);
        let (out, result) = inject_common_events(&bytes, &[], &v2()).unwrap();
        assert_eq!(out, bytes, "identity injection must be byte-exact");
        assert_eq!(result.updated_count, 0);
    }

    // -- Real-file CommonEvent.dat injection (UberWolf-decrypted fixtures) --
    //
    // These read the real Honoka (v2) / Inko (v3.5) `CommonEvent.dat` and skip
    // gracefully when the fixture isn't present (same convention as the other
    // real-file tests). They are the definitive proof for Phase 1 on real data.

    /// Honoka (v2): empty injection is byte-exact, and a real translation
    /// survives re-extraction.
    #[test]
    fn test_real_honoka_common_events_inject() {
        use crate::engines::wolf::extractor::extract_common_events;
        let path = test_dir().join("月咲流ホノカver1.03/Data/BasicData/CommonEvent.dat");
        if !path.exists() {
            return;
        }
        let bytes = std::fs::read(&path).unwrap();
        assert!(
            !v3_format::common_events::is_lz4_v3(&bytes),
            "Honoka must be v2"
        );

        // Identity: no translations → byte-exact.
        let (identity, res) = inject_common_events(&bytes, &[], &v2()).unwrap();
        assert_eq!(identity, bytes, "v2 identity injection must be byte-exact");
        assert_eq!(res.updated_count, 0);

        // Translate the first segment and confirm it round-trips.
        let segs = extract_common_events(&bytes, &v2()).unwrap();
        assert!(
            !segs.is_empty(),
            "Honoka CommonEvent.dat must yield segments"
        );
        let target = "★ホシ2スター★";
        let translations = vec![WolfTranslation {
            key: segs[0].key.clone(),
            text: target.to_owned(),
        }];
        let (new_bytes, res) = inject_common_events(&bytes, &translations, &v2()).unwrap();
        assert_eq!(res.updated_count, 1);

        let new_segs = extract_common_events(&new_bytes, &v2()).unwrap();
        let hit = new_segs.iter().find(|s| s.key == segs[0].key).unwrap();
        assert_eq!(hit.source_text, target, "translation must round-trip");
        assert_eq!(
            new_segs.len(),
            segs.len(),
            "segment count must be preserved"
        );
    }

    /// Inko (v3.5, LZ4): the decompressed payload of an empty injection is
    /// byte-exact, and a real translation survives re-extraction.
    #[test]
    fn test_real_inko_common_events_inject() {
        use crate::engines::wolf::extractor::extract_common_events;
        use crate::engines::wolf::v3_format::common_events::decompress;
        let path = test_dir().join("Densyanai_Inko_ver2.0/Data/BasicData/CommonEvent.dat");
        if !path.exists() {
            return;
        }
        let bytes = std::fs::read(&path).unwrap();
        assert!(
            v3_format::common_events::is_lz4_v3(&bytes),
            "Inko must be v3.5 LZ4"
        );

        // Identity on the decompressed payload (LZ4 recompression isn't
        // byte-stable, but the underlying data must be — same invariant as
        // `test_real_inko_common_events_v3_round_trip`).
        let (identity, res) = inject_common_events(&bytes, &[], &v3()).unwrap();
        assert_eq!(res.updated_count, 0);
        assert_eq!(
            decompress(&identity).unwrap(),
            decompress(&bytes).unwrap(),
            "v3 identity injection must preserve the decompressed payload"
        );

        // Translate the first segment and confirm it round-trips.
        let segs = extract_common_events(&bytes, &v3()).unwrap();
        assert!(!segs.is_empty(), "Inko CommonEvent.dat must yield segments");
        let target = "★ホシ2スター★";
        let translations = vec![WolfTranslation {
            key: segs[0].key.clone(),
            text: target.to_owned(),
        }];
        let (new_bytes, res) = inject_common_events(&bytes, &translations, &v3()).unwrap();
        assert_eq!(res.updated_count, 1);

        let new_segs = extract_common_events(&new_bytes, &v3()).unwrap();
        let hit = new_segs.iter().find(|s| s.key == segs[0].key).unwrap();
        assert_eq!(hit.source_text, target, "translation must round-trip");
        assert_eq!(
            new_segs.len(),
            segs.len(),
            "segment count must be preserved"
        );
    }
}
