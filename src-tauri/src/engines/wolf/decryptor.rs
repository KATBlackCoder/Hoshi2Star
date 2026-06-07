//! Wolf RPG DXA archive decryptor — F4-02 implementation.
//!
//! XOR-12 symmetric decryption for DXA archives produced by DxLib.
//! Covers DXA versions 5 (32-bit), 6, and 8 (64-bit).

use thiserror::Error;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum DecryptorError {
    #[error("not a DXA archive (invalid signature)")]
    InvalidSignature,
    #[error("unsupported DXA version: {0}")]
    UnsupportedVersion(u8),
    #[error("DXA header too short")]
    HeaderTooShort,
    #[error("cannot guess key (header fields not null)")]
    CannotGuessKey,
    #[error("LZSS compressed entries are not supported in F4 (packed_size > 0)")]
    UnsupportedCompression,
    /// All known XOR keys failed and GuessKeyV6 found no solution.
    /// This archive likely uses WolfX/v3.5+ ChaCha20 encryption, not yet supported.
    #[error(
        "archive may use WolfX encryption (v3.5+) which is not supported. \
         Use UberWolf to decrypt your game files first, \
         then open the Data/ folder directly (planned support: v0.5.0)"
    )]
    PossibleWolfX,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// One file extracted from a DXA archive.
#[derive(Debug, Clone)]
pub struct WolfFile {
    pub name: String,
    pub data: Vec<u8>,
    pub unpacked_size: u64,
}

/// A fully parsed DXA archive.
#[derive(Debug)]
pub struct WolfArchive {
    pub version: u8,
    pub code_page: Option<u32>,
    pub files: Vec<WolfFile>,
}

/// Parsed DXA file entry (one file inside the archive).
/// Fields vary between v5 (32-bit) and v6/v8 (64-bit).
#[derive(Debug, Clone)]
pub(crate) struct DxFileEntry {
    pub name_offset: u64,
    pub attributes: u32,
    pub data_offset: u64,
    pub unpacked_size: u64,
    pub packed_size: i64, // -1 = uncompressed
}

impl DxFileEntry {
    pub fn is_dir(&self) -> bool {
        self.attributes & 0x10 != 0
    }

    pub fn is_compressed(&self) -> bool {
        self.packed_size != -1 && self.packed_size as u64 != self.unpacked_size
    }
}

// ---------------------------------------------------------------------------
// Step 3 — Known key table
// ---------------------------------------------------------------------------

/// Known Wolf RPG XOR keys, indexed by game version string.
/// Sources: WolfDec DECRYPT_MODES table, GARbro, game-specific community docs.
// Wolf v3.10 / v3.173: NOT included — see ⚠️ in docs/plans/f4-02-wolf-decryptor.md §Step3
pub(crate) const WOLF_KEYS: &[(&str, [u8; 12])] = &[
    // Wolf v2.20 — most widespread (source: WolfDec DECRYPT_MODES)
    (
        "v2.20",
        [
            0x38, 0x50, 0x40, 0x28, 0x72, 0x4F, 0x21, 0x70, 0x3B, 0x73, 0x35, 0x38,
        ],
    ),
    // Wolf v2.01
    (
        "v2.01",
        [
            0x0F, 0x53, 0xE1, 0x3E, 0x04, 0x37, 0x12, 0x17, 0x60, 0x0F, 0x53, 0xE1,
        ],
    ),
    // Wolf v2.10
    (
        "v2.10",
        [
            0x4C, 0xD9, 0x2A, 0xB7, 0x28, 0x9B, 0xAC, 0x07, 0x3E, 0x77, 0xEC, 0x4C,
        ],
    ),
    // Wolf v2.255 — 月咲流ホノカ: key stored in plaintext at file[BaseOffset..BaseOffset+12]
    (
        "v2.255",
        [
            0xB8, 0x58, 0x8C, 0x7B, 0xCA, 0x3D, 0x6F, 0x3D, 0x8C, 0x34, 0xF8, 0x1A,
        ],
    ),
    // DXA_FLAG_NO_KEY: DxLib constant key (memset 0xAA → keyCreate → constant XOR)
    // TODO(F4-02): DXA_FLAG_NO_KEY value unknown — skip flag check for now
    (
        "no_key",
        [
            0x55, 0xAA, 0x20, 0x55, 0x55, 0x06, 0x55, 0xAA, 0x55, 0xD5, 0x7C, 0x66,
        ],
    ),
];

/// Look up a known Wolf RPG XOR key by version hint string.
/// Returns `None` for unknown versions — caller should then try GuessKeyV6 (Step 6).
pub fn known_key(version_hint: Option<&str>) -> Option<[u8; 12]> {
    let hint = version_hint?;
    WOLF_KEYS
        .iter()
        .find(|(name, _)| *name == hint)
        .map(|(_, key)| *key)
}

// ---------------------------------------------------------------------------
// Step 2 — KeyConv XOR-12
// ---------------------------------------------------------------------------

/// XOR `data` in place with `key`, starting at `key_pos = offset % 12`.
/// Symmetric: applying twice restores the original. Used for both encrypt and decrypt.
///
/// `offset` = position of `data[0]` in the DXA archive (determines the key start position).
pub(crate) fn key_conv(data: &mut [u8], offset: u64, key: &[u8; 12]) {
    // Wolf RPG bug: file data decryption offset = unpacked_size % 12
    // NOT the file position in the archive.
    // Source: docs/wolf-rpg-research.md §3 + ArcDX-reference.md §3.2
    let mut pos = (offset % 12) as usize;
    for byte in data.iter_mut() {
        *byte ^= key[pos];
        pos += 1;
        if pos == 12 {
            pos = 0;
        }
    }
}

// ---------------------------------------------------------------------------
// Step 4 — DXA v5 header (32-bit fields)
// ---------------------------------------------------------------------------

struct DxHeaderV5 {
    index_size: u32,
    base_offset: u32,
    index_offset: u32,
    file_table_offset: u32,
    dir_table_offset: u32,
}

/// Read and decrypt a DXA v5 header from raw archive bytes.
///
/// The encrypted body occupies `file[0x04..0x18]` (20 bytes, five u32 fields).
fn read_header_v5(data: &[u8], key: &[u8; 12]) -> Result<DxHeaderV5, DecryptorError> {
    if data.len() < 0x18 {
        return Err(DecryptorError::HeaderTooShort);
    }
    let mut buf = data[0x04..0x18].to_vec();
    key_conv(&mut buf, 4, key);

    let u32_at =
        |off: usize| u32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]]);

    Ok(DxHeaderV5 {
        index_size: u32_at(0x00),
        base_offset: u32_at(0x04),
        index_offset: u32_at(0x08),
        file_table_offset: u32_at(0x0C),
        dir_table_offset: u32_at(0x10),
    })
}

/// Read the DXA signature and return the archive version byte.
///
/// Valid versions: 5, 6, 8. Errors on invalid signature or unsupported version.
pub fn read_signature(data: &[u8]) -> Result<u8, DecryptorError> {
    if data.len() < 4 || &data[0..2] != b"DX" {
        return Err(DecryptorError::InvalidSignature);
    }
    let version = data[2];
    if version != 5 && version != 6 && version != 8 {
        return Err(DecryptorError::UnsupportedVersion(version));
    }
    Ok(version)
}

// ---------------------------------------------------------------------------
// Step 5 — DXA v6/v8 header (64-bit fields) + CodePage
// ---------------------------------------------------------------------------

use crate::engines::detector::WolfVersion;

struct DxHeaderV6 {
    index_size: u32,
    base_offset: i64,
    index_offset: i64,
    file_table_offset: i64,
    dir_table_offset: i64,
    code_page: u32,
}

/// Read and decrypt a DXA v6 or v8 header from raw archive bytes.
///
/// The encrypted body occupies `file[0x04..0x2C]` (40 bytes).
/// v6 and v8 share the same structure — CodePage is at body[0x24] = file[0x28].
fn read_header_v6(data: &[u8], key: &[u8; 12]) -> Result<DxHeaderV6, DecryptorError> {
    if data.len() < 0x2C {
        return Err(DecryptorError::HeaderTooShort);
    }
    let mut buf = data[0x04..0x2C].to_vec();
    key_conv(&mut buf, 4, key);

    let u32_at =
        |off: usize| u32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]]);
    let i64_at = |off: usize| {
        i64::from_le_bytes([
            buf[off],
            buf[off + 1],
            buf[off + 2],
            buf[off + 3],
            buf[off + 4],
            buf[off + 5],
            buf[off + 6],
            buf[off + 7],
        ])
    };

    Ok(DxHeaderV6 {
        index_size: u32_at(0x00),        // buf[0x00..0x04]
        base_offset: i64_at(0x04),       // buf[0x04..0x0C]
        index_offset: i64_at(0x0C),      // buf[0x0C..0x14]
        file_table_offset: i64_at(0x14), // buf[0x14..0x1C]
        dir_table_offset: i64_at(0x1C),  // buf[0x1C..0x24]
        code_page: u32_at(0x24),         // buf[0x24..0x28]  (file[0x28])
    })
}

/// Map a DXA CodePage value to a `WolfVersion`.
///
/// - `65001` (UTF-8) → v3+ (`major = 3`)
/// - `932` (Shift-JIS) or `0` (auto/legacy) → v2 (`major = 2`)
pub(crate) fn code_page_to_wolf_version(code_page: u32) -> WolfVersion {
    if code_page == 65001 {
        WolfVersion { major: 3, minor: 0 }
    } else {
        WolfVersion { major: 2, minor: 0 }
    }
}

/// Unified header reader — dispatches on version.
///
/// Returns `(version, base_offset, index_offset, file_table, dir_table, code_page)`.
/// `code_page` is `None` for v5 (no CodePage field in 32-bit headers).
#[allow(clippy::type_complexity)]
pub fn read_header(
    data: &[u8],
    key: &[u8; 12],
) -> Result<(u8, u64, u64, u64, u64, Option<u32>), DecryptorError> {
    let version = read_signature(data)?;
    match version {
        5 => {
            let h = read_header_v5(data, key)?;
            Ok((
                5,
                h.base_offset as u64,
                h.index_offset as u64,
                h.file_table_offset as u64,
                h.dir_table_offset as u64,
                None,
            ))
        }
        6 | 8 => {
            let h = read_header_v6(data, key)?;
            Ok((
                version,
                h.base_offset as u64,
                h.index_offset as u64,
                h.file_table_offset as u64,
                h.dir_table_offset as u64,
                Some(h.code_page),
            ))
        }
        _ => unreachable!("read_signature already validated version"),
    }
}

// ---------------------------------------------------------------------------
// Step 7 — TOC parsing
// ---------------------------------------------------------------------------

/// Decrypt and parse the DXA index (TOC) block into a list of file entries.
///
/// `index_data` is decrypted in place. Entries with the directory attribute
/// (`attributes & 0x10`) are filtered out — only file entries are returned.
///
/// Entry layout (all little-endian):
/// - v5 (0x2C bytes): u32 name\_offset, u32 attributes, 24 B timestamps,
///   u32 data\_offset, u32 unpacked\_size, i32 packed\_size
/// - v6/v8 (0x40 bytes): i64 name\_offset, u64 attributes, 24 B timestamps,
///   i64 data\_offset, i64 unpacked\_size, i64 packed\_size
///
/// `file_table` and `dir_table` are offsets *within the TOC* (not absolute file offsets).
pub(crate) fn parse_index(
    index_data: &mut [u8],
    key: &[u8; 12],
    version: u8,
    index_offset: u64,
    file_table: u64,
    dir_table: u64,
) -> Result<Vec<DxFileEntry>, DecryptorError> {
    // TOC decryption offset: v5 uses position in archive; v6+ always starts at 0
    let toc_key_offset = if version <= 5 { index_offset % 12 } else { 0 };
    key_conv(index_data, toc_key_offset, key);

    let entry_size: usize = if version <= 5 { 0x2C } else { 0x40 };
    let ft = file_table as usize;
    let dt = dir_table as usize;

    if dt <= ft || dt > index_data.len() {
        return Ok(vec![]);
    }
    let n_entries = (dt - ft) / entry_size;
    let mut entries = Vec::with_capacity(n_entries);

    for i in 0..n_entries {
        let base = ft + i * entry_size;
        if base + entry_size > index_data.len() {
            break;
        }

        let entry = if version <= 5 {
            let u32_at = |off: usize| -> u32 {
                u32::from_le_bytes(index_data[base + off..base + off + 4].try_into().unwrap())
            };
            let i32_at = |off: usize| -> i32 {
                i32::from_le_bytes(index_data[base + off..base + off + 4].try_into().unwrap())
            };
            DxFileEntry {
                name_offset: u32_at(0x00) as u64,
                attributes: u32_at(0x04),
                data_offset: u32_at(0x20) as u64,
                unpacked_size: u32_at(0x24) as u64,
                packed_size: i32_at(0x28) as i64,
            }
        } else {
            let u64_at = |off: usize| -> u64 {
                u64::from_le_bytes(index_data[base + off..base + off + 8].try_into().unwrap())
            };
            let i64_at = |off: usize| -> i64 {
                i64::from_le_bytes(index_data[base + off..base + off + 8].try_into().unwrap())
            };
            DxFileEntry {
                name_offset: u64_at(0x00),
                attributes: (u64_at(0x08) & 0xFFFF_FFFF) as u32,
                data_offset: i64_at(0x28) as u64,
                unpacked_size: i64_at(0x30) as u64,
                packed_size: i64_at(0x38),
            }
        };

        if !entry.is_dir() {
            entries.push(entry);
        }
    }

    Ok(entries)
}

// ---------------------------------------------------------------------------
// Step 6 — GuessKeyV6 (known-plaintext attack on null high bytes)
// ---------------------------------------------------------------------------

/// Recover the XOR key from a raw (encrypted) DXA v6/v8 header.
///
/// DXA v6 fields are 64-bit values whose upper 4 bytes are zero in plaintext
/// (practical archive sizes stay well under 4 GB). XOR with zero reveals the key bytes.
///
/// Three non-overlapping field pairs map to the 12-byte key:
/// - `file[0x0C..0x10]` (base_offset high)  → `key[0..4]`
/// - `file[0x1C..0x20]` (file_table high)   → `key[4..8]`
/// - `file[0x14..0x18]` (index_offset high) → `key[8..12]`
///
/// Two cross-checks validate the hypothesis:
/// - `file[0x24..0x28]` (dir_table high)    must equal `key[0..4]`
/// - `file[0x2C..0x30]` (post-header zeros) must equal `key[8..12]`
///
/// Returns `None` if the header is too short, validations fail, or the derived
/// key does not produce a plausible header (valid version + small index_size).
pub fn guess_key_v6(raw_header: &[u8]) -> Option<[u8; 12]> {
    if raw_header.len() < 0x30 {
        return None;
    }
    let read4 = |pos: usize| -> [u8; 4] { raw_header[pos..pos + 4].try_into().unwrap() };

    let high_base = read4(0x0C); // → key[0..4]
    let high_idx = read4(0x14); // → key[8..12]
    let high_ftbl = read4(0x1C); // → key[4..8]
    let high_dtbl = read4(0x24); // must == high_base  (validation 1)
    let post_hdr = read4(0x2C); // must == high_idx   (validation 2)

    if high_base != high_dtbl || high_idx != post_hdr {
        return None;
    }

    let mut key = [0u8; 12];
    key[0..4].copy_from_slice(&high_base);
    key[4..8].copy_from_slice(&high_ftbl);
    key[8..12].copy_from_slice(&high_idx);

    // Validate: version byte in {5, 6, 8} and index_size < 16 MB
    let version = raw_header[0x02];
    if version != 5 && version != 6 && version != 8 {
        return None;
    }
    let mut body = raw_header[0x04..0x2C].to_vec();
    key_conv(&mut body, 4, &key);
    let index_size = u32::from_le_bytes(body[0..4].try_into().unwrap());
    if index_size >= 0x0100_0000 {
        return None;
    }

    Some(key)
}

// ---------------------------------------------------------------------------
// Step 8 — extract_all helpers
// ---------------------------------------------------------------------------

/// Check whether a candidate key produces a plausible decrypted header.
fn is_valid_key(data: &[u8], version: u8, key: &[u8; 12]) -> bool {
    if version <= 5 {
        if data.len() < 0x18 {
            return false;
        }
        let mut buf = data[0x04..0x18].to_vec();
        key_conv(&mut buf, 4, key);
        let index_size = u32::from_le_bytes(buf[0..4].try_into().unwrap());
        index_size < 0x0100_0000
    } else {
        if data.len() < 0x2C {
            return false;
        }
        let mut buf = data[0x04..0x2C].to_vec();
        key_conv(&mut buf, 4, key);
        let index_size = u32::from_le_bytes(buf[0..4].try_into().unwrap());
        let code_page = u32::from_le_bytes(buf[0x24..0x28].try_into().unwrap());
        index_size < 0x0100_0000 && (code_page == 0 || code_page == 932 || code_page == 65001)
    }
}

/// Try each known Wolf key then GuessKeyV6; return first that validates.
fn find_key(data: &[u8], version: u8) -> Result<[u8; 12], DecryptorError> {
    for &(_, key) in WOLF_KEYS {
        if is_valid_key(data, version, &key) {
            return Ok(key);
        }
    }
    if let Some(key) = guess_key_v6(data) {
        return Ok(key);
    }
    Err(DecryptorError::PossibleWolfX)
}

/// Read only the `index_size` field from the decrypted header (without returning all fields).
fn read_index_size(data: &[u8], key: &[u8; 12]) -> Result<u64, DecryptorError> {
    let version = read_signature(data)?;
    if version <= 5 {
        Ok(read_header_v5(data, key)?.index_size as u64)
    } else {
        Ok(read_header_v6(data, key)?.index_size as u64)
    }
}

/// Decode a null-terminated byte slice from the TOC name table into a UTF-8 `String`.
fn decode_name(raw: &[u8], code_page: u32) -> String {
    if code_page == 65001 {
        String::from_utf8_lossy(raw).into_owned()
    } else {
        let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(raw);
        decoded.into_owned()
    }
}

// ---------------------------------------------------------------------------
// Step 8 — extract_all (complete DXA extraction pipeline)
// ---------------------------------------------------------------------------

/// Extract all files from a raw DXA archive (`.wolf` file bytes).
///
/// Sequence:
/// 1. Verify `"DX"` signature and supported version.
/// 2. Discover the XOR key: try `WOLF_KEYS` table, then `guess_key_v6`.
/// 3. Decrypt and read the archive header.
/// 4. Decrypt and parse the TOC (file/directory tables).
/// 5. For each non-directory entry: decrypt file bytes and decode the filename.
///
/// Returns `Err(UnsupportedCompression)` if any entry has `packed_size > 0`
/// (LZSS compressed). LZSS support is deferred to a post-F4 phase.
pub fn extract_all(data: &[u8]) -> Result<WolfArchive, DecryptorError> {
    // Step 1 — signature check
    let version = read_signature(data)?;

    // Step 2 — key discovery
    let key = find_key(data, version)?;

    // Step 3 — read header (offsets, code_page)
    let (_, base_offset, index_offset, file_table, dir_table, code_page) = read_header(data, &key)?;
    let index_size = read_index_size(data, &key)?;

    // Step 4 — extract and decrypt TOC
    let toc_start = index_offset as usize;
    let toc_end = toc_start + index_size as usize;
    if toc_end > data.len() {
        return Err(DecryptorError::HeaderTooShort);
    }
    let mut toc_data = data[toc_start..toc_end].to_vec();

    // parse_index decrypts toc_data in place; toc_data is readable after the call
    let entries = parse_index(
        &mut toc_data,
        &key,
        version,
        index_offset,
        file_table,
        dir_table,
    )?;

    // Step 5 — extract and decrypt each file
    let code_page_val = code_page.unwrap_or(932);
    let mut files = Vec::with_capacity(entries.len());

    for entry in &entries {
        if entry.is_compressed() {
            // LZSS not implemented in F4 — none of the targeted test archives use it
            return Err(DecryptorError::UnsupportedCompression);
        }

        let start = base_offset as usize + entry.data_offset as usize;
        let len = entry.unpacked_size as usize;
        if start.saturating_add(len) > data.len() {
            return Err(DecryptorError::HeaderTooShort);
        }
        let mut file_data = data[start..start + len].to_vec();
        // Wolf RPG bug: decryption offset = unpacked_size, not archive position
        key_conv(&mut file_data, entry.unpacked_size, &key);

        // Decode filename: null-terminated at toc_data[name_offset..]
        let ns = entry.name_offset as usize;
        let name_len = toc_data[ns..]
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(toc_data.len().saturating_sub(ns));
        let name = decode_name(&toc_data[ns..ns + name_len], code_page_val);

        files.push(WolfFile {
            name,
            data: file_data,
            unpacked_size: entry.unpacked_size,
        });
    }

    Ok(WolfArchive {
        version,
        code_page,
        files,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Step 2: key_conv ---

    #[test]
    fn test_key_conv_symmetric() {
        let key = [
            0x38u8, 0x50, 0x40, 0x28, 0x72, 0x4F, 0x21, 0x70, 0x3B, 0x73, 0x35, 0x38,
        ];
        let original = b"Hello, Wolf!".to_vec();
        let mut data = original.clone();
        key_conv(&mut data, 0, &key);
        assert_ne!(data, original, "XOR must change data");
        key_conv(&mut data, 0, &key);
        assert_eq!(data, original, "XOR twice must restore original");
    }

    #[test]
    fn test_key_conv_known_vector() {
        // data=[0x00], offset=0, key=v2.20 → data[0] = key[0] = 0x38
        let key = [
            0x38u8, 0x50, 0x40, 0x28, 0x72, 0x4F, 0x21, 0x70, 0x3B, 0x73, 0x35, 0x38,
        ];
        let mut data = [0x00u8];
        key_conv(&mut data, 0, &key);
        assert_eq!(data[0], 0x38);
    }

    #[test]
    fn test_key_conv_offset_wraps() {
        // offset=12 → key_pos = 12 % 12 = 0 → same XOR result as offset=0
        let key = [
            0x38u8, 0x50, 0x40, 0x28, 0x72, 0x4F, 0x21, 0x70, 0x3B, 0x73, 0x35, 0x38,
        ];
        let mut a = [0x00u8];
        let mut b = [0x00u8];
        key_conv(&mut a, 0, &key);
        key_conv(&mut b, 12, &key);
        assert_eq!(a[0], b[0]);
    }

    #[test]
    fn test_key_conv_wolf_offset_bug() {
        // Wolf RPG bug: file data uses unpacked_size as key offset, not archive position.
        // Verify: encrypt(data, unpacked_size) then decrypt(data, unpacked_size) restores original.
        let key = [
            0x38u8, 0x50, 0x40, 0x28, 0x72, 0x4F, 0x21, 0x70, 0x3B, 0x73, 0x35, 0x38,
        ];
        let unpacked_size: u64 = 1024;
        let original = vec![0xAAu8; 16];
        let mut data = original.clone();
        key_conv(&mut data, unpacked_size, &key);
        assert_ne!(data, original);
        key_conv(&mut data, unpacked_size, &key);
        assert_eq!(data, original);
    }

    // --- Step 3: known_key ---

    #[test]
    fn test_known_key_v220() {
        let key = known_key(Some("v2.20")).unwrap();
        assert_eq!(key[0], 0x38);
        assert_eq!(key[1], 0x50);
        assert_eq!(key[11], 0x38);
    }

    #[test]
    fn test_known_key_unknown() {
        assert!(known_key(Some("v9.99")).is_none());
    }

    // --- Step 4: read_signature + read_header_v5 ---

    fn make_v5_header(key: &[u8; 12]) -> Vec<u8> {
        let index_size: u32 = 0x1000;
        let base_offset: u32 = 0x18;
        let index_offset: u32 = 0x5000;
        let file_table: u32 = 0x00;
        let dir_table: u32 = 0x40;

        let mut body = Vec::with_capacity(20);
        body.extend_from_slice(&index_size.to_le_bytes());
        body.extend_from_slice(&base_offset.to_le_bytes());
        body.extend_from_slice(&index_offset.to_le_bytes());
        body.extend_from_slice(&file_table.to_le_bytes());
        body.extend_from_slice(&dir_table.to_le_bytes());
        key_conv(&mut body, 4, key);

        let mut hdr = vec![b'D', b'X', 5u8, 0u8];
        hdr.extend_from_slice(&body);
        hdr
    }

    #[test]
    fn test_read_header_v5_synthetic() {
        let key = [
            0x38u8, 0x50, 0x40, 0x28, 0x72, 0x4F, 0x21, 0x70, 0x3B, 0x73, 0x35, 0x38,
        ];
        let hdr = make_v5_header(&key);

        assert_eq!(read_signature(&hdr).unwrap(), 5);
        let h = read_header_v5(&hdr, &key).unwrap();
        assert_eq!(h.index_size, 0x1000);
        assert_eq!(h.base_offset, 0x18);
        assert_eq!(h.index_offset, 0x5000);
        assert_eq!(h.file_table_offset, 0x00);
        assert_eq!(h.dir_table_offset, 0x40);
    }

    // --- Step 5: read_header_v6 + CodePage → WolfVersion ---

    fn make_v6_header(key: &[u8; 12], code_page: u32) -> Vec<u8> {
        let index_size: u32 = 0x2000;
        let base_offset: i64 = 0x2C;
        let index_offset: i64 = 0x8000;
        let file_table: i64 = 0x00;
        let dir_table: i64 = 0x80;

        let mut body = Vec::with_capacity(40);
        body.extend_from_slice(&index_size.to_le_bytes()); // 0x00..0x04
        body.extend_from_slice(&base_offset.to_le_bytes()); // 0x04..0x0C
        body.extend_from_slice(&index_offset.to_le_bytes()); // 0x0C..0x14
        body.extend_from_slice(&file_table.to_le_bytes()); // 0x14..0x1C
        body.extend_from_slice(&dir_table.to_le_bytes()); // 0x1C..0x24
        body.extend_from_slice(&code_page.to_le_bytes()); // 0x24..0x28
        key_conv(&mut body, 4, key);

        let mut hdr = vec![b'D', b'X', 6u8, 0u8];
        hdr.extend_from_slice(&body);
        hdr
    }

    #[test]
    fn test_read_header_v6_synthetic() {
        let key = [
            0x38u8, 0x50, 0x40, 0x28, 0x72, 0x4F, 0x21, 0x70, 0x3B, 0x73, 0x35, 0x38,
        ];
        let hdr = make_v6_header(&key, 932);

        assert_eq!(read_signature(&hdr).unwrap(), 6);
        let h = read_header_v6(&hdr, &key).unwrap();
        assert_eq!(h.base_offset, 0x2C);
        assert_eq!(h.index_offset, 0x8000);
        assert_eq!(h.file_table_offset, 0x00);
        assert_eq!(h.dir_table_offset, 0x80);
        assert_eq!(h.code_page, 932);
    }

    #[test]
    fn test_code_page_932_is_shiftjis() {
        assert!(!code_page_to_wolf_version(932).is_utf8());
    }

    #[test]
    fn test_code_page_65001_is_utf8() {
        assert!(code_page_to_wolf_version(65001).is_utf8());
    }

    // --- Step 6: guess_key_v6 ---

    /// Build a 48-byte buffer (file[0x00..0x30]) that simulates an encrypted DXA v6 header,
    /// including the 4 post-header zero bytes that guess_key_v6 uses for validation.
    fn make_v6_header_for_guess(key: &[u8; 12]) -> Vec<u8> {
        let index_size: u32 = 0x2000;
        let base_offset: i64 = 0x2C;
        let index_offset: i64 = 0x8000;
        let file_table: i64 = 0x00;
        let dir_table: i64 = 0x80;
        let code_page: u32 = 932;

        // Build plaintext for file[0x04..0x30] = 44 bytes
        // (40 bytes formal header + 4 post-header zeros)
        let mut body = Vec::with_capacity(44);
        body.extend_from_slice(&index_size.to_le_bytes()); // body[0x00..0x04]
        body.extend_from_slice(&base_offset.to_le_bytes()); // body[0x04..0x0C]
        body.extend_from_slice(&index_offset.to_le_bytes()); // body[0x0C..0x14]
        body.extend_from_slice(&file_table.to_le_bytes()); // body[0x14..0x1C]
        body.extend_from_slice(&dir_table.to_le_bytes()); // body[0x1C..0x24]
        body.extend_from_slice(&code_page.to_le_bytes()); // body[0x24..0x28]
        body.extend_from_slice(&[0u8; 4]); // body[0x28..0x2C] → post-header zeros at file[0x2C..0x30]
        key_conv(&mut body, 4, key);

        let mut hdr = vec![b'D', b'X', 6u8, 0u8];
        hdr.extend_from_slice(&body);
        hdr // 4 + 44 = 48 bytes = file[0x00..0x30]
    }

    #[test]
    fn test_guess_key_v6_synthetic() {
        let key = [
            0x38u8, 0x50, 0x40, 0x28, 0x72, 0x4F, 0x21, 0x70, 0x3B, 0x73, 0x35, 0x38,
        ];
        let hdr = make_v6_header_for_guess(&key);
        let found = guess_key_v6(&hdr).expect("should recover key from null high bytes");
        assert_eq!(found, key);
    }

    #[test]
    fn test_guess_key_v6_random_data() {
        // 32 bytes < 0x30 — length guard returns None immediately
        let short = vec![0xABu8; 32];
        assert!(guess_key_v6(&short).is_none());
    }

    // --- Step 7: parse_index ---

    #[test]
    fn test_parse_index_v5_synthetic() {
        let key = [0u8; 12]; // zero key → XOR no-op → no encryption
        let version = 5u8;
        let index_offset: u64 = 0x5000; // chosen so index_offset % 12 = 0 with this zero key

        // Name table: two null-terminated ASCII names
        let name_table: Vec<u8> = b"file1.dat\0file2.dat\0".to_vec(); // offsets 0 and 10
        let file_table = name_table.len() as u64; // 20

        // Two v5 file entries (0x2C bytes each)
        let mut entries = vec![0u8; 2 * 0x2C];
        // Entry 0: name=0, attrs=0, data_offset=0, unpacked=100, packed=-1
        entries[0x00..0x04].copy_from_slice(&0u32.to_le_bytes());
        entries[0x04..0x08].copy_from_slice(&0u32.to_le_bytes());
        // timestamps [0x08..0x20] = zeros already
        entries[0x20..0x24].copy_from_slice(&0u32.to_le_bytes());
        entries[0x24..0x28].copy_from_slice(&100u32.to_le_bytes());
        entries[0x28..0x2C].copy_from_slice(&(-1i32).to_le_bytes());
        // Entry 1: name=10, attrs=0, data_offset=100, unpacked=200, packed=-1
        let e = 0x2C;
        entries[e + 0x00..e + 0x04].copy_from_slice(&10u32.to_le_bytes());
        entries[e + 0x04..e + 0x08].copy_from_slice(&0u32.to_le_bytes());
        entries[e + 0x20..e + 0x24].copy_from_slice(&100u32.to_le_bytes());
        entries[e + 0x24..e + 0x28].copy_from_slice(&200u32.to_le_bytes());
        entries[e + 0x28..e + 0x2C].copy_from_slice(&(-1i32).to_le_bytes());

        let dir_table = file_table + entries.len() as u64;
        let mut toc = name_table;
        toc.extend_from_slice(&entries);

        let result =
            parse_index(&mut toc, &key, version, index_offset, file_table, dir_table).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name_offset, 0);
        assert_eq!(result[0].unpacked_size, 100);
        assert_eq!(result[0].packed_size, -1);
        assert_eq!(result[1].name_offset, 10);
        assert_eq!(result[1].data_offset, 100);
        assert_eq!(result[1].unpacked_size, 200);
    }

    #[test]
    fn test_parse_index_v6_synthetic() {
        let key = [0u8; 12]; // zero key → XOR no-op
        let version = 6u8;
        let index_offset: u64 = 0; // v6 offset is always 0 for key_conv

        let name_table: Vec<u8> = b"alpha.dat\0beta.bin\0".to_vec(); // offsets 0 and 10
        let file_table = name_table.len() as u64; // 19

        // Two v6/v8 file entries (0x40 bytes each)
        let mut entries = vec![0u8; 2 * 0x40];
        // Entry 0: name=0, attrs=0, data_offset=0, unpacked=500, packed=-1
        entries[0x00..0x08].copy_from_slice(&0i64.to_le_bytes()); // name_offset
        entries[0x08..0x10].copy_from_slice(&0u64.to_le_bytes()); // attributes
                                                                  // timestamps [0x10..0x28] = zeros
        entries[0x28..0x30].copy_from_slice(&0i64.to_le_bytes()); // data_offset
        entries[0x30..0x38].copy_from_slice(&500i64.to_le_bytes()); // unpacked_size
        entries[0x38..0x40].copy_from_slice(&(-1i64).to_le_bytes()); // packed_size
                                                                     // Entry 1: name=10, attrs=0, data_offset=500, unpacked=1000, packed=-1
        let e = 0x40;
        entries[e + 0x00..e + 0x08].copy_from_slice(&10i64.to_le_bytes());
        entries[e + 0x08..e + 0x10].copy_from_slice(&0u64.to_le_bytes());
        entries[e + 0x28..e + 0x30].copy_from_slice(&500i64.to_le_bytes());
        entries[e + 0x30..e + 0x38].copy_from_slice(&1000i64.to_le_bytes());
        entries[e + 0x38..e + 0x40].copy_from_slice(&(-1i64).to_le_bytes());

        let dir_table = file_table + entries.len() as u64;
        let mut toc = name_table;
        toc.extend_from_slice(&entries);

        let result =
            parse_index(&mut toc, &key, version, index_offset, file_table, dir_table).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name_offset, 0);
        assert_eq!(result[0].unpacked_size, 500);
        assert_eq!(result[0].packed_size, -1);
        assert_eq!(result[1].name_offset, 10);
        assert_eq!(result[1].data_offset, 500);
        assert_eq!(result[1].unpacked_size, 1000);
    }

    // --- Step 8: extract_all ---

    /// Build a minimal DXA v5 archive with a single uncompressed file.
    ///
    /// Layout: header (0x18 B) | encrypted file data | encrypted TOC
    fn make_v5_archive(key: &[u8; 12], file_name: &str, content: &[u8]) -> Vec<u8> {
        let unpacked_size = content.len() as u32;

        // Name table: file_name + null terminator
        let mut name_table = file_name.as_bytes().to_vec();
        name_table.push(0);
        let file_table: u32 = name_table.len() as u32;

        // v5 file entry (0x2C bytes)
        let mut entry = vec![0u8; 0x2C];
        entry[0x00..0x04].copy_from_slice(&0u32.to_le_bytes()); // name_offset = 0
        entry[0x04..0x08].copy_from_slice(&0u32.to_le_bytes()); // attributes = 0
                                                                // timestamps [0x08..0x20] = zeros
        entry[0x20..0x24].copy_from_slice(&0u32.to_le_bytes()); // data_offset = 0
        entry[0x24..0x28].copy_from_slice(&unpacked_size.to_le_bytes());
        entry[0x28..0x2C].copy_from_slice(&(-1i32).to_le_bytes()); // packed_size = -1

        let dir_table: u32 = file_table + 0x2C;

        // Build plaintext TOC
        let mut toc = name_table;
        toc.extend_from_slice(&entry);
        let index_size: u32 = toc.len() as u32;

        // Archive layout offsets
        let base_offset: u32 = 0x18; // v5 header = 4 sig + 20 body
        let index_offset: u32 = base_offset + content.len() as u32;

        // Encrypt header body (file[0x04..0x18])
        let mut hdr_body = vec![0u8; 20];
        hdr_body[0x00..0x04].copy_from_slice(&index_size.to_le_bytes());
        hdr_body[0x04..0x08].copy_from_slice(&base_offset.to_le_bytes());
        hdr_body[0x08..0x0C].copy_from_slice(&index_offset.to_le_bytes());
        hdr_body[0x0C..0x10].copy_from_slice(&file_table.to_le_bytes());
        hdr_body[0x10..0x14].copy_from_slice(&dir_table.to_le_bytes());
        key_conv(&mut hdr_body, 4, key);

        // Encrypt file data (Wolf bug: offset = unpacked_size)
        let mut file_data = content.to_vec();
        key_conv(&mut file_data, unpacked_size as u64, key);

        // Encrypt TOC (v5: key offset = index_offset % 12)
        key_conv(&mut toc, index_offset as u64 % 12, key);

        let mut archive = vec![b'D', b'X', 5u8, 0u8];
        archive.extend_from_slice(&hdr_body);
        archive.extend_from_slice(&file_data);
        archive.extend_from_slice(&toc);
        archive
    }

    /// Build a minimal DXA v6 archive with a single uncompressed file.
    ///
    /// Layout: header (0x2C B) | encrypted file data | encrypted TOC
    fn make_v6_archive(key: &[u8; 12], file_name: &str, content: &[u8], code_page: u32) -> Vec<u8> {
        let unpacked_size = content.len() as i64;

        let mut name_table = file_name.as_bytes().to_vec();
        name_table.push(0);
        let file_table: i64 = name_table.len() as i64;

        // v6 file entry (0x40 bytes)
        let mut entry = vec![0u8; 0x40];
        entry[0x00..0x08].copy_from_slice(&0i64.to_le_bytes()); // name_offset = 0
        entry[0x08..0x10].copy_from_slice(&0u64.to_le_bytes()); // attributes = 0
                                                                // timestamps [0x10..0x28] = zeros
        entry[0x28..0x30].copy_from_slice(&0i64.to_le_bytes()); // data_offset = 0
        entry[0x30..0x38].copy_from_slice(&unpacked_size.to_le_bytes());
        entry[0x38..0x40].copy_from_slice(&(-1i64).to_le_bytes());

        let dir_table: i64 = file_table + 0x40;

        let mut toc = name_table;
        toc.extend_from_slice(&entry);
        let index_size: u32 = toc.len() as u32;

        let base_offset: i64 = 0x2C; // v6 header size
        let index_offset: i64 = base_offset + content.len() as i64;

        // Encrypt header body (file[0x04..0x2C] = 40 bytes)
        let mut hdr_body = vec![0u8; 40];
        hdr_body[0x00..0x04].copy_from_slice(&index_size.to_le_bytes());
        hdr_body[0x04..0x0C].copy_from_slice(&base_offset.to_le_bytes());
        hdr_body[0x0C..0x14].copy_from_slice(&index_offset.to_le_bytes());
        hdr_body[0x14..0x1C].copy_from_slice(&file_table.to_le_bytes());
        hdr_body[0x1C..0x24].copy_from_slice(&dir_table.to_le_bytes());
        hdr_body[0x24..0x28].copy_from_slice(&code_page.to_le_bytes());
        key_conv(&mut hdr_body, 4, key);

        let mut file_data = content.to_vec();
        key_conv(&mut file_data, unpacked_size as u64, key);

        // Encrypt TOC (v6: key offset = 0)
        key_conv(&mut toc, 0, key);

        let mut archive = vec![b'D', b'X', 6u8, 0u8];
        archive.extend_from_slice(&hdr_body);
        archive.extend_from_slice(&file_data);
        archive.extend_from_slice(&toc);
        archive
    }

    #[test]
    fn test_extract_all_synthetic_v5() {
        let key = WOLF_KEYS
            .iter()
            .find(|(n, _)| *n == "v2.20")
            .map(|(_, k)| *k)
            .unwrap();
        let content = b"Hello, Wolf RPG!";
        let archive = make_v5_archive(&key, "game.dat", content);

        let result = extract_all(&archive).unwrap();
        assert_eq!(result.version, 5);
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.files[0].name, "game.dat");
        assert_eq!(result.files[0].data.as_slice(), content.as_slice());
    }

    #[test]
    fn test_extract_all_synthetic_v6() {
        let key = WOLF_KEYS
            .iter()
            .find(|(n, _)| *n == "v2.20")
            .map(|(_, k)| *k)
            .unwrap();
        let content = b"Wolf RPG v6 content";
        let archive = make_v6_archive(&key, "data.bin", content, 932);

        let result = extract_all(&archive).unwrap();
        assert_eq!(result.version, 6);
        assert_eq!(result.code_page, Some(932));
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.files[0].name, "data.bin");
        assert_eq!(result.files[0].data.as_slice(), content.as_slice());
    }

    #[test]
    fn test_extract_all_round_trip() {
        // Use v2.10 key to exercise a different code path than the v2.20 tests
        let key = WOLF_KEYS
            .iter()
            .find(|(n, _)| *n == "v2.10")
            .map(|(_, k)| *k)
            .unwrap();
        let original: &[u8] = b"Round-trip binary payload \xAB\xCD\xEF";
        let archive = make_v5_archive(&key, "payload.bin", original);

        let result = extract_all(&archive).unwrap();
        assert_eq!(result.files[0].data.as_slice(), original);
    }

    #[test]
    fn test_extract_all_no_key() {
        let key = WOLF_KEYS
            .iter()
            .find(|(n, _)| *n == "no_key")
            .map(|(_, k)| *k)
            .unwrap();
        let content = b"constant-key archive content";
        let archive = make_v5_archive(&key, "nokey.dat", content);

        let result = extract_all(&archive).unwrap();
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.files[0].data.as_slice(), content.as_slice());
    }

    #[test]
    fn test_extract_all_bad_signature() {
        let data = vec![0xABu8, 0xCD, 0xEF, 0x00, 0x11, 0x22, 0x33, 0x44];
        assert!(matches!(
            extract_all(&data).unwrap_err(),
            DecryptorError::InvalidSignature
        ));
    }
}
