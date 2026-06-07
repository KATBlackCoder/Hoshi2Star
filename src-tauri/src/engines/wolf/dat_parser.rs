// Wolf RPG Database parser — handles .project (schema) + .dat (data values) file pairs.
// Based on WolfTL (github.com/Sinflower/WolfTL, MIT) format analysis.
//
// Wolf RPG databases require TWO separate files to parse:
//   *.project — schema: type names, field names, data-entry names
//   *.dat     — binary data: int and string values per entry
//
// String encoding: SJIS (v1/v2) or UTF-8 (v3+), detected from the .dat magic byte.
//
// Layout of an unencrypted .dat file:
//   byte  0      : indicator (0x00 = unencrypted)
//   bytes 1–9    : 9-byte magic (see DB_MAGIC_SJIS / DB_MAGIC_UTF8)
//   byte  10     : version (0xC4 = LZ4-compressed — not supported here)
//   bytes 11–14  : u32_le type_count
//   for each type:
//     4 bytes    : DAT_TYPE_SEPARATOR [0xFE, 0xFF, 0xFF, 0xFF]
//     4 bytes    : u32 unknown1
//     4 bytes    : u32 fields_size
//     if unknown1 == STRING_INDICATOR: ReadString (extra string)
//     fields_size × 4 bytes: u32 index_info per field
//     4 bytes    : u32 data_count
//     for each entry: int_cnt × u32, then str_cnt × ReadString
//   1 byte       : terminator (== version)

use super::encoding;
use std::io::{Cursor, Read};

// .dat magic — byte 5 = 0x00 (SJIS) or 0x55 (UTF-8)
pub const DB_MAGIC_SJIS: [u8; 9] = [0x57, 0x00, 0x00, 0x4F, 0x4C, 0x00, 0x46, 0x4D, 0x00];
pub const DB_MAGIC_UTF8: [u8; 9] = [0x57, 0x00, 0x00, 0x4F, 0x4C, 0x55, 0x46, 0x4D, 0x00];

// 4-byte separator that precedes each type's data section in the .dat
pub const DAT_TYPE_SEPARATOR: [u8; 4] = [0xFE, 0xFF, 0xFF, 0xFF];

// indexInfo thresholds (from WolfTL Field::IsString / IsValid)
const STRING_FIELD_START: u32 = 0x07D0;
const VALID_FIELD_START: u32 = 0x03E8;

// Signals that an extra string follows in the type data header
const STRING_INDICATOR: u32 = 0x0001_D4C0;
/// Public re-export for the injector (F4-04).
pub(crate) const STRING_INDICATOR_PUB: u32 = STRING_INDICATOR;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum DatParseError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("encoding error: {0}")]
    Encoding(String),
    #[error("invalid magic number")]
    InvalidMagic,
    #[error("unsupported format: {0}")]
    Unsupported(String),
    #[error("type count mismatch: project={project}, dat={dat}")]
    TypeCountMismatch { project: usize, dat: usize },
    #[error("type separator mismatch at type index {0}")]
    SeparatorMismatch(usize),
    #[error("zero-length string (corrupted file)")]
    ZeroLengthString,
}

// ---------------------------------------------------------------------------
// Public output types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DatField {
    pub name: String,
    pub index_info: u32,
}

impl DatField {
    /// Returns true when this field holds string data.
    pub fn is_string(&self) -> bool {
        self.index_info >= STRING_FIELD_START
    }

    /// Returns true when this field has a real value (not a padding/placeholder slot).
    pub fn is_valid(&self) -> bool {
        self.index_info >= VALID_FIELD_START
    }
}

#[derive(Debug, Clone)]
pub struct DatEntry {
    /// Row name from the .project schema (used by injector F4-04).
    #[allow(dead_code)]
    pub name: String,
    /// Integer field values (preserved verbatim during injection).
    pub int_values: Vec<u32>,
    pub string_values: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DatType {
    /// Type name from the .project schema (used by injector F4-04).
    #[allow(dead_code)]
    pub name: String,
    /// Active fields (first `fields_size` entries from the .project schema).
    pub fields: Vec<DatField>,
    pub entries: Vec<DatEntry>,
}

#[derive(Debug, Clone)]
pub struct DatFile {
    pub types: Vec<DatType>,
    /// Encoding flag — used by the injector to write back the correct magic bytes.
    pub is_utf8: bool,
}

// ---------------------------------------------------------------------------
// Internal schema type (parsed from .project)
// ---------------------------------------------------------------------------

struct ProjectTypeInfo {
    name: String,
    field_names: Vec<String>,
    data_names: Vec<String>,
}

// ---------------------------------------------------------------------------
// Low-level binary readers (Cursor-based)
// ---------------------------------------------------------------------------

fn read_u8(c: &mut Cursor<&[u8]>) -> Result<u8, DatParseError> {
    let mut b = [0u8; 1];
    c.read_exact(&mut b).map_err(DatParseError::Io)?;
    Ok(b[0])
}

fn read_u32(c: &mut Cursor<&[u8]>) -> Result<u32, DatParseError> {
    let mut b = [0u8; 4];
    c.read_exact(&mut b).map_err(DatParseError::Io)?;
    Ok(u32::from_le_bytes(b))
}

fn read_bytes(c: &mut Cursor<&[u8]>, n: usize) -> Result<Vec<u8>, DatParseError> {
    let mut b = vec![0u8; n];
    c.read_exact(&mut b).map_err(DatParseError::Io)?;
    Ok(b)
}

/// Read a Wolf RPG length-prefixed string.
///
/// Format: `u32_le size` (includes null terminator) + `size` bytes.
/// The null terminator is stripped before decoding.
fn read_wolf_string(c: &mut Cursor<&[u8]>, is_utf8: bool) -> Result<String, DatParseError> {
    let size = read_u32(c)? as usize;
    if size == 0 {
        return Err(DatParseError::ZeroLengthString);
    }
    let bytes = read_bytes(c, size)?;
    // Strip the null terminator (last byte = 0x00).
    let text = if bytes.last() == Some(&0x00) {
        &bytes[..bytes.len() - 1]
    } else {
        &bytes[..]
    };
    if is_utf8 {
        String::from_utf8(text.to_vec()).map_err(|e| DatParseError::Encoding(e.to_string()))
    } else {
        encoding::decode_shiftjis(text).map_err(DatParseError::Encoding)
    }
}

// ---------------------------------------------------------------------------
// .project file parser
// ---------------------------------------------------------------------------

fn parse_project(bytes: &[u8], is_utf8: bool) -> Result<Vec<ProjectTypeInfo>, DatParseError> {
    let mut c = Cursor::new(bytes);
    let type_count = read_u32(&mut c)? as usize;
    let mut types = Vec::with_capacity(type_count);

    for _ in 0..type_count {
        let name = read_wolf_string(&mut c, is_utf8)?;

        let field_count = read_u32(&mut c)? as usize;
        let mut field_names = Vec::with_capacity(field_count);
        for _ in 0..field_count {
            field_names.push(read_wolf_string(&mut c, is_utf8)?);
        }

        let data_count = read_u32(&mut c)? as usize;
        let mut data_names = Vec::with_capacity(data_count);
        for _ in 0..data_count {
            data_names.push(read_wolf_string(&mut c, is_utf8)?);
        }

        // description
        let _ = read_wolf_string(&mut c, is_utf8)?;

        // field type list: field_type_list_size bytes, first field_count are type bytes
        let ftype_list_size = read_u32(&mut c)? as usize;
        let type_bytes_to_read = field_count.min(ftype_list_size);
        read_bytes(&mut c, type_bytes_to_read)?;
        let skip = ftype_list_size.saturating_sub(field_count);
        read_bytes(&mut c, skip)?;

        // unknown1: count + that many strings (one per field)
        let cnt = read_u32(&mut c)? as usize;
        for _ in 0..cnt {
            let _ = read_wolf_string(&mut c, is_utf8)?;
        }

        // unknown2: count + for each: (count + strings) — stringArgs per field
        let cnt = read_u32(&mut c)? as usize;
        for _ in 0..cnt {
            let n = read_u32(&mut c)? as usize;
            for _ in 0..n {
                let _ = read_wolf_string(&mut c, is_utf8)?;
            }
        }

        // unknown3: count + for each: (count + u32s) — int args per field
        let cnt = read_u32(&mut c)? as usize;
        for _ in 0..cnt {
            let n = read_u32(&mut c)? as usize;
            read_bytes(&mut c, n * 4)?;
        }

        // unknown4: count + u32s — default values per field
        let cnt = read_u32(&mut c)? as usize;
        read_bytes(&mut c, cnt * 4)?;

        types.push(ProjectTypeInfo {
            name,
            field_names,
            data_names,
        });
    }
    Ok(types)
}

// ---------------------------------------------------------------------------
// .dat data section parser
// ---------------------------------------------------------------------------

fn parse_dat_types(
    bytes: &[u8],
    project_types: &[ProjectTypeInfo],
    is_utf8: bool,
) -> Result<Vec<DatType>, DatParseError> {
    let mut c = Cursor::new(bytes);

    // Skip indicator (1) + magic (9) + version (1) = 11 bytes already validated.
    read_bytes(&mut c, 11)?;

    let type_count = read_u32(&mut c)? as usize;
    if type_count != project_types.len() {
        return Err(DatParseError::TypeCountMismatch {
            project: project_types.len(),
            dat: type_count,
        });
    }

    let mut dat_types = Vec::with_capacity(type_count);

    for (ti, proj) in project_types.iter().enumerate() {
        // Each type data section starts with the separator.
        let sep = read_bytes(&mut c, 4)?;
        if sep.as_slice() != DAT_TYPE_SEPARATOR {
            return Err(DatParseError::SeparatorMismatch(ti));
        }

        let unknown1 = read_u32(&mut c)?;
        let fields_size = read_u32(&mut c)? as usize;

        // Optional extra string present when unknown1 == STRING_INDICATOR.
        if unknown1 == STRING_INDICATOR {
            let _ = read_wolf_string(&mut c, is_utf8)?;
        }

        // Read indexInfo for each active field; borrow name from project schema.
        let mut fields = Vec::with_capacity(fields_size);
        for i in 0..fields_size {
            let index_info = read_u32(&mut c)?;
            let name = proj
                .field_names
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("field_{i}"));
            fields.push(DatField { name, index_info });
        }

        // Count valid int and string slots to know how many values to read per entry.
        let (int_cnt, str_cnt) = fields.iter().fold((0usize, 0usize), |(ic, sc), f| {
            if !f.is_valid() {
                (ic, sc)
            } else if f.is_string() {
                (ic, sc + 1)
            } else {
                (ic + 1, sc)
            }
        });

        let data_count = read_u32(&mut c)? as usize;
        let mut entries = Vec::with_capacity(data_count);

        for i in 0..data_count {
            let name = proj
                .data_names
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("entry_{i}"));

            // All int values come first, then all string values.
            let mut int_values = Vec::with_capacity(int_cnt);
            for _ in 0..int_cnt {
                int_values.push(read_u32(&mut c)?);
            }
            let mut string_values = Vec::with_capacity(str_cnt);
            for _ in 0..str_cnt {
                string_values.push(read_wolf_string(&mut c, is_utf8)?);
            }

            entries.push(DatEntry {
                name,
                int_values,
                string_values,
            });
        }

        dat_types.push(DatType {
            name: proj.name.clone(),
            fields,
            entries,
        });
    }

    // terminator byte (should equal version, but we don't re-validate here)
    let _ = read_u8(&mut c)?;

    Ok(dat_types)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a Wolf RPG Database from its two component files.
///
/// `project_bytes` — contents of the `.project` schema file (no magic, no header).
/// `dat_bytes`     — contents of the `.dat` data file (indicator + magic + version + data).
///
/// Encoding (SJIS vs UTF-8) is detected automatically from the `.dat` magic.
/// Only unencrypted, non-LZ4 databases are supported in F4-03.
pub fn parse_database(project_bytes: &[u8], dat_bytes: &[u8]) -> Result<DatFile, DatParseError> {
    if dat_bytes.len() < 11 {
        return Err(DatParseError::Io(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            ".dat file is too short",
        )));
    }

    // Byte 0: indicator (0x00 = unencrypted; other values → encrypted, unsupported in F4-03).
    if dat_bytes[0] != 0x00 {
        return Err(DatParseError::Unsupported(
            "encrypted database not supported in F4-03 (deferred to F4-05)".into(),
        ));
    }

    // Bytes 1–9: 9-byte magic.
    let magic = &dat_bytes[1..10];
    let is_utf8 = if magic == DB_MAGIC_UTF8 {
        true
    } else if magic == DB_MAGIC_SJIS {
        false
    } else {
        return Err(DatParseError::InvalidMagic);
    };

    // Byte 10: version. 0xC4 means LZ4-compressed (not supported here).
    let version = dat_bytes[10];
    if version == 0xC4 {
        return Err(DatParseError::Unsupported(
            "LZ4-compressed database (v3.5) not supported in F4-03 (deferred to F4-05)".into(),
        ));
    }

    let project_types = parse_project(project_bytes, is_utf8)?;
    let types = parse_dat_types(dat_bytes, &project_types, is_utf8)?;

    Ok(DatFile { types, is_utf8 })
}

// ---------------------------------------------------------------------------
// Test helpers (pub(crate) so injector tests can reuse them)
// ---------------------------------------------------------------------------

#[cfg(test)]
fn sjis_string_inner(text: &str) -> Vec<u8> {
    use encoding_rs::SHIFT_JIS;
    let (encoded, _, _) = SHIFT_JIS.encode(text);
    let sjis: &[u8] = &encoded;
    let len = (sjis.len() + 1) as u32;
    let mut out = len.to_le_bytes().to_vec();
    out.extend_from_slice(sjis);
    out.push(0x00);
    out
}

/// Build a minimal .project with one type, one field, one data entry.
/// Available to all test modules via `dat_parser::make_minimal_project_pub`.
#[cfg(test)]
pub(crate) fn make_minimal_project_pub(
    type_name: &str,
    field_name: &str,
    entry_name: &str,
) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&1u32.to_le_bytes()); // type_count = 1
    b.extend(sjis_string_inner(type_name));
    b.extend_from_slice(&1u32.to_le_bytes()); // field_count = 1
    b.extend(sjis_string_inner(field_name));
    b.extend_from_slice(&1u32.to_le_bytes()); // data_count = 1
    if entry_name.is_empty() {
        b.extend_from_slice(&1u32.to_le_bytes());
        b.push(0x00);
    } else {
        b.extend(sjis_string_inner(entry_name));
    }
    // description: empty
    b.extend_from_slice(&1u32.to_le_bytes());
    b.push(0x00);
    // field_type_list_size = 1, one type byte = 0
    b.extend_from_slice(&1u32.to_le_bytes());
    b.push(0x00);
    // unknown1: 1 entry, 1 empty string
    b.extend_from_slice(&1u32.to_le_bytes());
    b.extend_from_slice(&1u32.to_le_bytes());
    b.push(0x00);
    // unknown2: 1 entry, 0 string args
    b.extend_from_slice(&1u32.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());
    // unknown3: 1 entry, 0 int args
    b.extend_from_slice(&1u32.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());
    // unknown4: 1 entry, default value = 0
    b.extend_from_slice(&1u32.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());
    b
}

/// Build a minimal unencrypted SJIS .dat with one type, one string field, one entry.
/// Available to all test modules via `dat_parser::make_minimal_dat_pub`.
#[cfg(test)]
pub(crate) fn make_minimal_dat_pub(string_value: &str) -> Vec<u8> {
    let version: u8 = 0xC1;
    let mut b = Vec::new();
    b.push(0x00); // indicator = unencrypted
    b.extend_from_slice(&DB_MAGIC_SJIS);
    b.push(version);
    b.extend_from_slice(&1u32.to_le_bytes()); // type_count = 1
    b.extend_from_slice(&DAT_TYPE_SEPARATOR);
    b.extend_from_slice(&0u32.to_le_bytes()); // unknown1
    b.extend_from_slice(&1u32.to_le_bytes()); // fields_size = 1
    b.extend_from_slice(&STRING_FIELD_START.to_le_bytes()); // string field
    b.extend_from_slice(&1u32.to_le_bytes()); // data_count = 1
    b.extend(sjis_string_inner(string_value));
    b.push(version); // terminator
    b
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use encoding_rs::SHIFT_JIS;

    /// Encode `text` to Shift-JIS and wrap in a Wolf length-prefixed string.
    /// Format: u32_le(len) where len = sjis_bytes.len() + 1, then sjis_bytes + 0x00 null.
    fn sjis_string(text: &str) -> Vec<u8> {
        let (encoded, _, _) = SHIFT_JIS.encode(text);
        let sjis: &[u8] = &encoded;
        let len = (sjis.len() + 1) as u32;
        let mut out = len.to_le_bytes().to_vec();
        out.extend_from_slice(sjis);
        out.push(0x00);
        out
    }

    /// Build a minimal .project file with one type, one field, one data entry.
    fn make_minimal_project(type_name: &str, field_name: &str, entry_name: &str) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&1u32.to_le_bytes()); // type_count = 1
        b.extend(sjis_string(type_name));
        b.extend_from_slice(&1u32.to_le_bytes()); // field_count = 1
        b.extend(sjis_string(field_name));
        b.extend_from_slice(&1u32.to_le_bytes()); // data_count = 1
                                                  // data entry name: use empty-string encoding if empty, otherwise sjis
        if entry_name.is_empty() {
            b.extend_from_slice(&1u32.to_le_bytes()); // size = 1 (just null)
            b.push(0x00);
        } else {
            b.extend(sjis_string(entry_name));
        }
        // description: empty
        b.extend_from_slice(&1u32.to_le_bytes());
        b.push(0x00);
        // field_type_list_size = 1, one type byte = 0
        b.extend_from_slice(&1u32.to_le_bytes());
        b.push(0x00);
        // unknown1: 1 entry, 1 empty string
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes());
        b.push(0x00);
        // unknown2: 1 entry, 0 string args
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        // unknown3: 1 entry, 0 int args
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        // unknown4: 1 entry, default value = 0
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b
    }

    /// Build a minimal unencrypted SJIS .dat file with one type, one string field, one entry.
    fn make_minimal_dat(string_value: &str) -> Vec<u8> {
        let version: u8 = 0xC1;
        let mut b = Vec::new();
        b.push(0x00); // indicator = unencrypted
        b.extend_from_slice(&DB_MAGIC_SJIS); // 9-byte magic
        b.push(version); // version
        b.extend_from_slice(&1u32.to_le_bytes()); // type_count = 1
                                                  // Type section
        b.extend_from_slice(&DAT_TYPE_SEPARATOR);
        b.extend_from_slice(&0u32.to_le_bytes()); // unknown1 (not STRING_INDICATOR)
        b.extend_from_slice(&1u32.to_le_bytes()); // fields_size = 1
        b.extend_from_slice(&STRING_FIELD_START.to_le_bytes()); // field[0].indexInfo = string, index 0
        b.extend_from_slice(&1u32.to_le_bytes()); // data_count = 1
                                                  // entry[0]: 0 int values, 1 string value
        b.extend(sjis_string(string_value));
        b.push(version); // terminator
        b
    }

    #[test]
    fn test_parse_database_synthetic_string_field() {
        let project = make_minimal_project("キャラ", "名前", "");
        let dat = make_minimal_dat("テスト");
        let db = parse_database(&project, &dat).unwrap();
        assert_eq!(db.types.len(), 1);
        assert_eq!(db.types[0].name, "キャラ");
        assert_eq!(db.types[0].fields.len(), 1);
        assert_eq!(db.types[0].fields[0].name, "名前");
        assert!(db.types[0].fields[0].is_string());
        assert_eq!(db.types[0].entries.len(), 1);
        assert_eq!(db.types[0].entries[0].string_values, ["テスト"]);
        assert!(!db.is_utf8);
    }

    #[test]
    fn test_parse_database_int_field() {
        let version: u8 = 0xC1;
        let project = make_minimal_project("Vars", "hp", "hero");
        // Build a .dat with an int field (indexInfo = VALID_FIELD_START, which is int)
        let mut dat = Vec::new();
        dat.push(0x00);
        dat.extend_from_slice(&DB_MAGIC_SJIS);
        dat.push(version);
        dat.extend_from_slice(&1u32.to_le_bytes()); // type_count
        dat.extend_from_slice(&DAT_TYPE_SEPARATOR);
        dat.extend_from_slice(&0u32.to_le_bytes()); // unknown1
        dat.extend_from_slice(&1u32.to_le_bytes()); // fields_size = 1
        dat.extend_from_slice(&VALID_FIELD_START.to_le_bytes()); // indexInfo = int
        dat.extend_from_slice(&1u32.to_le_bytes()); // data_count = 1
        dat.extend_from_slice(&42u32.to_le_bytes()); // int value = 42
        dat.push(version);

        let db = parse_database(&project, &dat).unwrap();
        assert_eq!(db.types[0].fields[0].is_string(), false);
        assert_eq!(db.types[0].fields[0].is_valid(), true);
        assert_eq!(db.types[0].entries[0].int_values, [42u32]);
        assert!(db.types[0].entries[0].string_values.is_empty());
    }

    #[test]
    fn test_parse_database_rejects_encrypted() {
        let mut dat = vec![0x01]; // indicator != 0x00 → encrypted
        dat.extend_from_slice(&DB_MAGIC_SJIS);
        dat.push(0xC1);
        let project = make_minimal_project("T", "F", "");
        let err = parse_database(&project, &dat).unwrap_err();
        assert!(matches!(err, DatParseError::Unsupported(_)));
    }

    #[test]
    fn test_parse_database_rejects_lz4() {
        let mut dat = vec![0x00];
        dat.extend_from_slice(&DB_MAGIC_SJIS);
        dat.push(0xC4); // LZ4 version
        let project = make_minimal_project("T", "F", "");
        let err = parse_database(&project, &dat).unwrap_err();
        assert!(matches!(err, DatParseError::Unsupported(_)));
    }

    #[test]
    fn test_parse_database_invalid_magic() {
        let mut dat = vec![0x00];
        dat.extend_from_slice(&[0xFF; 9]); // garbage magic
        dat.push(0xC1);
        let project = make_minimal_project("T", "F", "");
        let err = parse_database(&project, &dat).unwrap_err();
        assert!(matches!(err, DatParseError::InvalidMagic));
    }
}
