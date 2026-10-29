// Wolf RPG binary injector — F4-04 implementation.
//
// Delivery (plan UW): the game is decrypted with UberWolf, files are read from
// and delivered as loose Data/ files; a .wolf archive holding text refuses the
// export (the game would read it instead).
//
// .mps / CommonEvent.dat injection: in-house `v3_format` AST, for the v3.x
//   LZ4 layout and the layout stored in clear (v2.x, v3.x `0x64` maps); the
//   file is written back in the container it was read from (plan RW, RW-4).
//
// .dat injection: parse_database() → modify string_values → serialize_dat().

use super::dat_parser::{self, DatFile, DatType};
use super::dat_parser::{DAT_TYPE_SEPARATOR, DB_MAGIC_SJIS, DB_MAGIC_UTF8};
use super::encoding;
use super::integrity;
use super::sources::{SourcesError, WolfSources};
use super::v3_format::{self, V3FormatError};
use crate::engines::detector::WolfVersion;
use crate::engines::mv_mz::integrity::IntegrityIssue;
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
    /// An archive holding text is still in the game folder (plan UW, UW-D2).
    #[error(transparent)]
    Sources(#[from] SourcesError),
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
    /// Translations folded to fit a Shift-JIS file (plan RW, decision D3).
    pub folded_count: usize,
}

/// `text` as a file of this encoding can hold it: folded for Shift-JIS
/// (plan RW, decision D3: é → e…), as is for UTF-8. Counts the texts folded.
fn text_for_file<'a>(text: &'a str, is_utf8: bool, folded: &mut usize) -> Cow<'a, str> {
    if is_utf8 {
        return Cow::Borrowed(text);
    }
    let fitted = encoding::fold_for_shift_jis(text);
    if fitted != text {
        *folded += 1;
    }
    fitted
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

fn encode_for_wolf(text: &str, is_utf8: bool) -> Result<Vec<u8>, InjectorError> {
    if is_utf8 {
        Ok(text.as_bytes().to_vec())
    } else {
        encoding::encode_shiftjis(text).map_err(InjectorError::Encoding)
    }
}

// ---------------------------------------------------------------------------
// Step 2 — inject_map (.mps, v3.x only)
// ---------------------------------------------------------------------------

/// Inject translations into a Wolf RPG `.mps` map file (v3.x LZ4 layout or
/// layout stored in clear). A map that cannot be read returns an error: nothing
/// is written rather than a partially patched file.
///
/// `map_name` must match the stem used in the segment keys (e.g. `"Map001"`).
pub fn inject_map(
    map_name: &str,
    bytes: &[u8],
    translations: &[WolfTranslation],
    _version: &WolfVersion,
) -> Result<(Vec<u8>, InjectionResult), InjectorError> {
    let translation_map: HashMap<&str, &str> = translations
        .iter()
        .map(|t| (t.key.as_str(), t.text.as_str()))
        .collect();

    let (mut updated, mut folded) = (0usize, 0usize);
    let patched = inject_map_v3(map_name, bytes, &translation_map, &mut updated, &mut folded)?;
    Ok((
        patched,
        InjectionResult {
            file_path: PathBuf::from(format!("{map_name}.mps")),
            updated_count: updated,
            folded_count: folded,
        },
    ))
}

/// Inject translations into a `.mps` map using the in-house [`v3_format`]
/// parser: unpack, parse to AST, replace `string_args` for
/// `ShowMessage`/`ShowChoice` commands by key, dump, pack back into the
/// original container.
///
/// Same `key` format as [`super::extractor::extract_map_segments`]:
/// `MapData/{map_name}/events/{event_idx}/pages/{page_idx}/{cmd_idx}[/choices/{choice_idx}]`.
fn inject_map_v3(
    map_name: &str,
    bytes: &[u8],
    translations: &HashMap<&str, &str>,
    updated: &mut usize,
    folded: &mut usize,
) -> Result<Vec<u8>, InjectorError> {
    let decompressed = v3_format::compression::unpack_map(bytes)?;
    let mut map = v3_format::map::MapV3::parse(&decompressed)?;
    let is_utf8 = map.is_utf8();

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
                                let t = text_for_file(t, is_utf8, folded);
                                if t != text.as_str() {
                                    *updated += 1;
                                    *text = t.into_owned();
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
                                let t = text_for_file(t, is_utf8, folded);
                                if t != choice.as_str() {
                                    *updated += 1;
                                    *choice = t.into_owned();
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
    Ok(v3_format::compression::pack_map(dumped)?)
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
fn serialize_dat(dat: &DatFile, dat_original_header: &[u8]) -> Result<Vec<u8>, InjectorError> {
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
        serialize_dat_type(&mut payload, dat_type, dat.is_utf8)?;
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

/// `is_utf8` drives string encoding directly from the `.dat`'s own magic
/// byte (see [`serialize_dat`]) rather than from the externally-guessed
/// `WolfVersion` — that guess once defaulted to v2.0/Shift-JIS even when a
/// specific database was genuinely UTF-8, silently corrupting any non-ASCII
/// translated character while the header still declared UTF-8.
fn serialize_dat_type(
    buf: &mut Vec<u8>,
    dat_type: &DatType,
    is_utf8: bool,
) -> Result<(), InjectorError> {
    buf.extend_from_slice(&DAT_TYPE_SEPARATOR);

    // unknown1 and its optional string (STRING_INDICATOR) as read.
    buf.extend_from_slice(&dat_type.unknown1.to_le_bytes());

    buf.extend_from_slice(&(dat_type.fields.len() as u32).to_le_bytes());

    if let Some(text) = &dat_type.unknown1_string {
        let encoded = encode_for_wolf(text, is_utf8)?;
        write_wolf_string(buf, &encoded);
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
            let encoded = encode_for_wolf(s, is_utf8)?;
            write_wolf_string(buf, &encoded);
        }
    }

    Ok(())
}

/// Inject translations into a Wolf RPG `.dat` database file.
///
/// Takes both `.project` (schema, read-only) and `.dat` (values to modify).
/// Returns only the new `.dat` bytes — the `.project` is never modified.
///
/// `_version` is accepted (unused) only so this function keeps the same
/// signature as [`inject_map`]/[`inject_common_events`] for the [`inject_all`]
/// dispatch loop: unlike `.mps`/`CommonEvent.dat`, a database's own magic byte
/// (`dat.is_utf8`, threaded through to [`serialize_dat_type`]) already says
/// definitively whether it's Shift-JIS or UTF-8, so the externally-guessed
/// game version is not needed here.
pub fn inject_dat(
    project_bytes: &[u8],
    dat_bytes: &[u8],
    translations: &[WolfTranslation],
    _version: &WolfVersion,
) -> Result<(Vec<u8>, InjectionResult), InjectorError> {
    let mut db = dat_parser::parse_database(project_bytes, dat_bytes)
        .map_err(|e| InjectorError::DatParse(e.to_string()))?;
    let is_utf8 = db.is_utf8;

    let (mut updated, mut folded) = (0usize, 0usize);

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

        *slot = text_for_file(&t.text, is_utf8, &mut folded).into_owned();
        updated += 1;
    }

    let new_bytes = serialize_dat(&db, dat_bytes)?;

    Ok((
        new_bytes,
        InjectionResult {
            file_path: PathBuf::new(),
            updated_count: updated,
            folded_count: folded,
        },
    ))
}

// ---------------------------------------------------------------------------
// Step 4b — inject_common_events (CommonEvent.dat)
// ---------------------------------------------------------------------------

/// Inject translations into a Wolf RPG `CommonEvent.dat` file.
///
/// Symmetric to [`inject_map`]: the file (v3.5 LZ4, or stored in clear) parses
/// to the in-house [`v3_format`] AST, `string_args` are replaced by key, then
/// it is dumped and packed back into its original container.
///
/// Keys match [`super::extractor::extract_common_events`]:
/// `CommonEvents/{event_name}/{event_idx}/{cmd_idx}[/choices/{choice_idx}]`
/// (flat command list — no `pages` level, unlike maps).
pub fn inject_common_events(
    bytes: &[u8],
    translations: &[WolfTranslation],
    _version: &WolfVersion,
) -> Result<(Vec<u8>, InjectionResult), InjectorError> {
    let translation_map: HashMap<&str, &str> = translations
        .iter()
        .map(|t| (t.key.as_str(), t.text.as_str()))
        .collect();

    let (mut updated, mut folded) = (0usize, 0usize);
    let patched = inject_common_events_v3(bytes, &translation_map, &mut updated, &mut folded)?;

    Ok((
        patched,
        InjectionResult {
            file_path: PathBuf::from("CommonEvent.dat"),
            updated_count: updated,
            folded_count: folded,
        },
    ))
}

/// Inject translations into a `CommonEvent.dat` using the in-house
/// [`v3_format`] parser — mirrors [`inject_map_v3`] with the flat
/// common-event command list.
fn inject_common_events_v3(
    bytes: &[u8],
    translations: &HashMap<&str, &str>,
    updated: &mut usize,
    folded: &mut usize,
) -> Result<Vec<u8>, InjectorError> {
    let decompressed = v3_format::common_events::unpack(bytes)?;
    let mut common_events = v3_format::common_events::CommonEventsV3::parse(&decompressed)?;
    let is_utf8 = common_events.is_utf8();

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
                            let t = text_for_file(t, is_utf8, folded);
                            if t != text.as_str() {
                                *updated += 1;
                                *text = t.into_owned();
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
                            let t = text_for_file(t, is_utf8, folded);
                            if t != choice.as_str() {
                                *updated += 1;
                                *choice = t.into_owned();
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    let dumped = common_events.dump()?;
    Ok(v3_format::common_events::pack(dumped)?)
}

// ---------------------------------------------------------------------------
// Step 5 — inject_all (Option A export)
// ---------------------------------------------------------------------------

/// Output of [`inject_all_to_memory`].
pub struct WolfExport {
    /// `(relative zip path, bytes)` pairs.
    pub entries: Vec<(String, Vec<u8>)>,
    /// Integrity issues: the export must not be written unless this is empty.
    pub issues: Vec<IntegrityIssue>,
    /// Translations folded to fit Shift-JIS files (plan RW, decision D3).
    pub folded_texts: usize,
}

/// Inject translations and return `(relative_zip_path, bytes)` pairs — no disk
/// write. Used by the zip export path.
///
/// Files are read from the decrypted `Data/` ([`WolfSources`], plan UW) and
/// delivered as loose files under `Data/` (`Data/MapData/Map001.mps`…).
///
/// Every produced file is checked against the very bytes it was derived from
/// ([`super::integrity`]): the returned issues must be empty for the export to
/// be written.
pub async fn inject_all_to_memory(
    game_dir: &Path,
    translations_by_file: &HashMap<String, Vec<WolfTranslation>>,
    version: &WolfVersion,
) -> Result<WolfExport, InjectorError> {
    let sources = WolfSources::load(game_dir)?;
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    let mut issues: Vec<IntegrityIssue> = Vec::new();
    let mut folded_texts = 0usize;

    for (file_key, translations) in translations_by_file {
        let parts: Vec<&str> = file_key.splitn(2, '/').collect();
        if parts.len() < 2 {
            continue;
        }
        let stem = parts[1];
        let planned: HashMap<String, String> = translations
            .iter()
            .map(|t| (t.key.clone(), t.text.clone()))
            .collect();

        match parts[0] {
            "MapData" => {
                let Some(map) = sources.get(&format!("MapData/{stem}.mps")) else {
                    continue;
                };
                let (new_bytes, result) = inject_map(stem, &map.bytes, translations, version)?;
                folded_texts += result.folded_count;
                let label = format!("Data/{}", map.logical);
                issues.extend(integrity::verify_map_edit(
                    &label, stem, &map.bytes, &new_bytes, &planned,
                ));
                entries.push((label, new_bytes));
            }
            "Database" => {
                let (Some(project), Some(dat)) = (
                    sources.get(&format!("BasicData/{stem}.project")),
                    sources.get(&format!("BasicData/{stem}.dat")),
                ) else {
                    return Err(InjectorError::MissingProject(stem.to_string()));
                };
                let (new_bytes, result) =
                    inject_dat(&project.bytes, &dat.bytes, translations, version)?;
                folded_texts += result.folded_count;
                let label = format!("Data/{}", dat.logical);
                issues.extend(integrity::verify_dat_edit(
                    &label,
                    stem,
                    &project.bytes,
                    &dat.bytes,
                    &new_bytes,
                    &planned,
                ));
                entries.push((label, new_bytes));
            }
            "CommonEvents" => {
                // All CommonEvents/* keys share the single CommonEvent.dat.
                let Some(common_events) = sources.common_events() else {
                    continue;
                };
                let (new_bytes, result) =
                    inject_common_events(&common_events.bytes, translations, version)?;
                folded_texts += result.folded_count;
                let label = format!("Data/{}", common_events.logical);
                issues.extend(integrity::verify_common_events_edit(
                    &label,
                    &common_events.bytes,
                    &new_bytes,
                    &planned,
                ));
                entries.push((label, new_bytes));
            }
            _ => {}
        }
    }
    Ok(WolfExport {
        entries,
        issues,
        folded_texts,
    })
}

/// Inject all translations into a Wolf RPG game directory as loose files
/// (`Data/MapData/`, `Data/BasicData/`), read where the game reads them.
///
/// Used by the debug single-file command only; the zip export is
/// [`inject_all_to_memory`]. Refused like it, before any folder is created,
/// while an archive holding text is in the game folder (plan UW, UW-D2).
///
/// Keys in `translations_by_file`:
///   `"MapData/{stem}"`   → translations for that `.mps`
///   `"Database/{stem}"`  → translations for that `.dat` pair
pub async fn inject_all(
    game_dir: &Path,
    translations_by_file: &HashMap<String, Vec<WolfTranslation>>,
    version: &WolfVersion,
) -> Result<Vec<InjectionResult>, InjectorError> {
    let sources = WolfSources::load(game_dir)?;
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
                // Source bytes from the decrypted Data/.
                let Some(map) = sources.get(&format!("MapData/{stem}.mps")) else {
                    continue;
                };
                let (new_bytes, mut result) = inject_map(stem, &map.bytes, translations, version)?;
                let out = map_dir.join(format!("{stem}.mps"));
                result.file_path = out.clone();
                std::fs::write(&out, &new_bytes)?;
                results.push(result);
            }
            "Database" => {
                // Source bytes from the decrypted Data/.
                let (Some(project), Some(dat)) = (
                    sources.get(&format!("BasicData/{stem}.project")),
                    sources.get(&format!("BasicData/{stem}.dat")),
                ) else {
                    return Err(InjectorError::MissingProject(stem.to_string()));
                };
                let (new_bytes, mut result) =
                    inject_dat(&project.bytes, &dat.bytes, translations, version)?;
                let out = db_dir.join(format!("{stem}.dat"));
                result.file_path = out.clone();
                // Write only .dat — .project is never modified.
                std::fs::write(&out, &new_bytes)?;
                results.push(result);
            }
            "CommonEvents" => {
                // All CommonEvents/* keys share the single CommonEvent.dat.
                let Some(common_events) = sources.common_events() else {
                    continue;
                };
                let (new_bytes, mut result) =
                    inject_common_events(&common_events.bytes, translations, version)?;
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

    fn sjis_string(text: &str) -> Vec<u8> {
        use encoding_rs::SHIFT_JIS;
        let (enc, _, _) = SHIFT_JIS.encode(text);
        let len = (enc.len() + 1) as u32;
        let mut out = len.to_le_bytes().to_vec();
        out.extend_from_slice(&enc);
        out.push(0x00);
        out
    }

    /// Japanese texts the extractor's translatable filter keeps.
    const MESSAGE: &str = "こんにちは、旅の人";
    const CHOICES: [&str; 2] = ["はい、行きます", "いいえ、残ります"];

    /// On-disk bytes of a synthetic map, in the container its `version` selects.
    fn synthetic_map_bytes(version: u32, is_utf8: bool) -> Vec<u8> {
        let map = v3_format::map::MapV3::synthetic_messages(version, is_utf8, &[MESSAGE], &CHOICES);
        v3_format::compression::pack_map(map.dump().unwrap()).unwrap()
    }

    // -----------------------------------------------------------------------
    // Step 3 — encode_for_wolf
    // -----------------------------------------------------------------------

    #[test]
    fn test_encode_french_accents_in_v2() {
        let result = encode_for_wolf("café", v2().is_utf8());
        assert!(
            matches!(result, Err(InjectorError::Encoding(_))),
            "accented chars must fail for Wolf v2 (Shift-JIS)"
        );
    }

    /// Plan RW, decision D3: a Shift-JIS (v2) file gets its accents folded and
    /// counted; a UTF-8 file keeps them.
    #[test]
    fn test_accents_are_folded_for_shift_jis_files_only() {
        use crate::engines::wolf::extractor::extract_map_segments;

        let accented = "Élève, déjà là « oui »";
        let translations = vec![WolfTranslation {
            key: "MapData/Map001/events/0/pages/0/0".to_string(),
            text: accented.to_string(),
        }];

        let (sjis, result) = inject_map(
            "Map001",
            &synthetic_map_bytes(0x64, false),
            &translations,
            &v2(),
        )
        .unwrap();
        assert_eq!(result.folded_count, 1);
        let segs = extract_map_segments("Map001", &sjis, &v2()).unwrap();
        assert_eq!(segs[0].source_text, "Eleve, deja la \" oui \"");

        let (utf8, result) = inject_map(
            "Map001",
            &synthetic_map_bytes(0x67, true),
            &translations,
            &v3(),
        )
        .unwrap();
        assert_eq!(result.folded_count, 0);
        let segs = extract_map_segments("Map001", &utf8, &v3()).unwrap();
        assert_eq!(segs[0].source_text, accented);

        // Shift-JIS database: folded too.
        let project = make_minimal_project_pub("Type", "name", "entry");
        let dat = make_minimal_dat_pub("こんにちは");
        let translations = vec![WolfTranslation {
            key: "Database/DataBase/0/0/name".to_string(),
            text: "Héros".to_string(),
        }];
        let (new_dat, result) = inject_dat(&project, &dat, &translations, &v2()).unwrap();
        assert_eq!(result.folded_count, 1);
        let parsed = dat_parser::parse_database(&project, &new_dat).unwrap();
        assert_eq!(parsed.types[0].entries[0].string_values[0], "Heros");
    }

    #[test]
    fn test_encode_french_accents_in_v3() {
        let result = encode_for_wolf("café", v3().is_utf8());
        assert!(result.is_ok(), "UTF-8 v3 should accept accented chars");
        assert_eq!(result.unwrap(), "café".as_bytes());
    }

    #[test]
    fn test_encode_ascii_both_versions() {
        assert!(encode_for_wolf("Hello", v2().is_utf8()).is_ok());
        assert!(encode_for_wolf("Hello", v3().is_utf8()).is_ok());
    }

    // -----------------------------------------------------------------------
    // Step 2 — maps / common events stored in clear (v2.x, v3.x 0x64): RW-4
    // -----------------------------------------------------------------------

    /// Without translations a map is written back byte for byte; with one, it
    /// stays in its container and only the translated string changes.
    #[test]
    fn test_inject_map_keeps_its_container() {
        for (version, is_utf8) in [(0x64, false), (0x64, true), (0x67, true)] {
            let bytes = synthetic_map_bytes(version, is_utf8);
            let (unchanged, result) = inject_map("Map001", &bytes, &[], &v2()).unwrap();
            assert_eq!(result.updated_count, 0);
            if version < 0x65 {
                assert_eq!(unchanged, bytes, "version {version:#x}");
            }

            let translations = vec![
                WolfTranslation {
                    key: "MapData/Map001/events/0/pages/0/0".to_string(),
                    text: "Hello, traveller".to_string(),
                },
                WolfTranslation {
                    key: "MapData/Map001/events/0/pages/0/1/choices/1".to_string(),
                    text: "No, I stay".to_string(),
                },
            ];
            let (patched, result) = inject_map("Map001", &bytes, &translations, &v2()).unwrap();
            assert_eq!(result.updated_count, 2);
            assert_eq!(
                v3_format::compression::is_plain_map(&patched),
                version < 0x65,
                "version {version:#x}"
            );
            let segs =
                crate::engines::wolf::extractor::extract_map_segments("Map001", &patched, &v2())
                    .unwrap();
            let texts: Vec<&str> = segs.iter().map(|s| s.source_text.as_str()).collect();
            assert_eq!(texts, ["Hello, traveller", CHOICES[0], "No, I stay"]);
        }
    }

    #[test]
    fn test_inject_common_events_stored_in_clear() {
        let common_events =
            v3_format::common_events::CommonEventsV3::synthetic_messages_v2(&[("Ev001", MESSAGE)]);
        let bytes = common_events.dump().unwrap();

        let (unchanged, _) = inject_common_events(&bytes, &[], &v2()).unwrap();
        assert_eq!(unchanged, bytes);

        let translations = vec![WolfTranslation {
            key: "CommonEvents/Ev001/0/0".to_string(),
            text: "Hello, traveller".to_string(),
        }];
        let (patched, result) = inject_common_events(&bytes, &translations, &v2()).unwrap();
        assert_eq!(result.updated_count, 1);
        assert!(v3_format::common_events::is_plain(&patched));
        let segs = crate::engines::wolf::extractor::extract_common_events(&patched, &v2()).unwrap();
        assert_eq!(segs[0].source_text, "Hello, traveller");
    }

    #[tokio::test]
    async fn test_export_of_a_v2_map_writes_it_in_clear() {
        let tmp = tempfile::tempdir().unwrap();
        let map_dir = tmp.path().join("Data").join("MapData");
        std::fs::create_dir_all(&map_dir).unwrap();
        std::fs::write(map_dir.join("Map001.mps"), synthetic_map_bytes(0x64, false)).unwrap();
        let mut by_file: HashMap<String, Vec<WolfTranslation>> = HashMap::new();
        by_file.insert(
            "MapData/Map001".to_string(),
            vec![WolfTranslation {
                key: "MapData/Map001/events/0/pages/0/0".to_string(),
                text: "Hello".to_string(),
            }],
        );

        let WolfExport {
            entries, issues, ..
        } = inject_all_to_memory(tmp.path(), &by_file, &v2())
            .await
            .unwrap();
        assert!(issues.is_empty(), "{issues:?}");
        let (path, bytes) = entries
            .iter()
            .find(|(path, _)| path == "Data/MapData/Map001.mps")
            .unwrap();
        assert_eq!(path, "Data/MapData/Map001.mps");
        assert!(v3_format::compression::is_plain_map(bytes));
    }

    /// A decrypted game exports loose `Data/` files only, its stems as named on
    /// disk (plan UW): a map and `CommonEvent.dat`, never an archive.
    #[tokio::test]
    async fn test_export_delivers_loose_data_files_only() {
        let tmp = tempfile::tempdir().unwrap();
        let map_dir = tmp.path().join("Data").join("MapData");
        std::fs::create_dir_all(&map_dir).unwrap();
        std::fs::write(
            map_dir.join("TitleMap.mps"),
            synthetic_map_bytes(0x64, false),
        )
        .unwrap();
        // Archives without text (pictures, sounds) are left alone.
        std::fs::write(tmp.path().join("Data").join("Picture.wolf"), b"pictures").unwrap();
        let mut by_file: HashMap<String, Vec<WolfTranslation>> = HashMap::new();
        by_file.insert(
            "MapData/TitleMap".to_string(),
            vec![WolfTranslation {
                key: "MapData/TitleMap/events/0/pages/0/0".to_string(),
                text: "Hello, traveller".to_string(),
            }],
        );

        let WolfExport {
            entries, issues, ..
        } = inject_all_to_memory(tmp.path(), &by_file, &v2())
            .await
            .unwrap();
        assert!(issues.is_empty(), "{issues:?}");
        let names: Vec<&str> = entries.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["Data/MapData/TitleMap.mps"]);
        let segs =
            crate::engines::wolf::extractor::extract_map_segments("TitleMap", &entries[0].1, &v2())
                .unwrap();
        assert_eq!(segs[0].source_text, "Hello, traveller");
    }

    /// A game still holding a text archive is refused before anything is
    /// produced (plan UW, UW-D2): the game would read the archive instead.
    #[tokio::test]
    async fn test_export_with_a_text_archive_is_refused() {
        use super::super::sources::WOLF_ARCHIVE_PRESENT;

        let tmp = tempfile::tempdir().unwrap();
        let map_dir = tmp.path().join("Data").join("MapData");
        std::fs::create_dir_all(&map_dir).unwrap();
        std::fs::write(map_dir.join("Map001.mps"), synthetic_map_bytes(0x64, false)).unwrap();
        std::fs::write(tmp.path().join("Data.wolf"), b"archive").unwrap();
        let mut by_file: HashMap<String, Vec<WolfTranslation>> = HashMap::new();
        by_file.insert(
            "MapData/Map001".to_string(),
            vec![WolfTranslation {
                key: "MapData/Map001/events/0/pages/0/0".to_string(),
                text: "Hello".to_string(),
            }],
        );

        let error = match inject_all_to_memory(tmp.path(), &by_file, &v2()).await {
            Ok(_) => panic!("a text archive must refuse the export"),
            Err(error) => error,
        };
        assert!(
            matches!(
                error,
                InjectorError::Sources(SourcesError::ArchivePresent { ref archives })
                    if archives == "Data.wolf"
            ),
            "expected ArchivePresent, got {error:?}"
        );
        assert!(
            error.to_string().starts_with(WOLF_ARCHIVE_PRESENT),
            "{error}"
        );
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

    /// Regression for a real-world silent-corruption case: a UTF-8 database
    /// (whose game-wide encoding guess used to fall back to v2.0/Shift-JIS)
    /// must encode
    /// non-ASCII translated text as UTF-8 from its own `.dat` magic byte,
    /// never from the externally-guessed (and here deliberately wrong)
    /// `WolfVersion` — otherwise a full-width `．` (U+FF0E) gets written as
    /// Shift-JIS bytes `81 44` under a header that still declares UTF-8,
    /// which the Wolf RPG Editor runtime cannot parse back.
    #[test]
    fn test_inject_dat_utf8_file_ignores_wrong_external_version() {
        const STRING_FIELD_START: u32 = 0x07D0; // dat_parser::STRING_FIELD_START

        let project = make_minimal_project_pub("Type", "name", "entry");

        let version_byte: u8 = 0xC1;
        let mut dat = vec![0x00]; // indicator = unencrypted
        dat.extend_from_slice(&DB_MAGIC_UTF8);
        dat.push(version_byte);
        dat.extend_from_slice(&1u32.to_le_bytes()); // type_count = 1
        dat.extend_from_slice(&DAT_TYPE_SEPARATOR);
        dat.extend_from_slice(&0u32.to_le_bytes()); // unknown1
        dat.extend_from_slice(&1u32.to_le_bytes()); // fields_size = 1
        dat.extend_from_slice(&STRING_FIELD_START.to_le_bytes());
        dat.extend_from_slice(&1u32.to_le_bytes()); // data_count = 1
        dat.extend_from_slice(&2u32.to_le_bytes()); // "x\0"
        dat.extend_from_slice(b"x\0");
        dat.push(version_byte); // terminator

        let translations = vec![WolfTranslation {
            key: "Database/Type/0/0/name".to_string(),
            text: "cast Ice．Edge!".to_string(), // U+FF0E, non-ASCII
        }];

        // v2() (Shift-JIS) stands in for the wrong game-wide guess an
        // unpacked v3.x distribution used to produce — it must be ignored.
        let (new_bytes, result) = inject_dat(&project, &dat, &translations, &v2()).unwrap();
        assert_eq!(result.updated_count, 1);

        let db = dat_parser::parse_database(&project, &new_bytes).unwrap();
        assert_eq!(db.types[0].entries[0].string_values[0], "cast Ice．Edge!");
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
    fn test_round_trip_dat_identity() {
        let project = make_minimal_project_pub("キャラ", "名前", "hero");
        let dat = make_minimal_dat_pub("テスト");

        let segs = crate::engines::wolf::extractor::extract_database_segments(
            "TestDB",
            &project,
            &dat,
            &std::collections::HashSet::new(),
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
            &std::collections::HashSet::new(),
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

    /// A text archive still in the game folder (plan UW, UW-D2) refuses the
    /// injection, and the `.wolf` file is left untouched on disk.
    #[tokio::test]
    async fn test_inject_all_does_not_overwrite_wolf() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path();

        let data_dir = game_dir.join("Data");
        std::fs::create_dir_all(&data_dir).unwrap();
        let wolf_path = data_dir.join("MapData.wolf");
        std::fs::write(&wolf_path, b"fake wolf archive").unwrap();

        let by_file: HashMap<String, Vec<WolfTranslation>> = HashMap::new();
        let err = match inject_all(game_dir, &by_file, &v2()).await {
            Ok(_) => panic!("a text archive must refuse the injection"),
            Err(err) => err,
        };
        assert!(
            matches!(
                err,
                InjectorError::Sources(SourcesError::ArchivePresent { ref archives })
                    if archives == "Data/MapData.wolf"
            ),
            "expected ArchivePresent, got {err:?}"
        );

        let content = std::fs::read(&wolf_path).unwrap();
        assert_eq!(content, b"fake wolf archive", ".wolf must not be modified");
        assert!(
            !data_dir.join("MapData").exists() && !data_dir.join("BasicData").exists(),
            "no folder is created before the refusal"
        );
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

        let WolfExport {
            entries, issues, ..
        } = inject_all_to_memory(&game_dir, &by_file, &v3())
            .await
            .unwrap();
        assert!(issues.is_empty(), "{issues:?}");

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

    // -- Real-file CommonEvent.dat injection (UberWolf-decrypted fixtures) --
    //
    // Reads the real Inko (v3.5) `CommonEvent.dat` and skips gracefully when
    // the fixture isn't present (same convention as the other real-file tests).

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
