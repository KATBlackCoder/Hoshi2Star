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
    #[error("a compressed archive entry could not be decoded (corrupt or unsupported LZ/Huffman stream)")]
    UnsupportedCompression,
    /// All known XOR keys failed and GuessKeyV6 found no solution.
    /// This archive likely uses WolfX/v3.5+ ChaCha20 encryption, not yet supported.
    #[error(
        "archive may use WolfX encryption (v3.5+) which is not supported. \
         Use UberWolf to decrypt your game files first, \
         then open the Data/ folder directly (planned support: v0.5.0)"
    )]
    PossibleWolfX,
    /// DXA v8 archive packed by WOLF RPG Editor 3.31+ ("new crypt": encrypted
    /// header addresses, AES-CTR data). The crypt version is the high 16 bits
    /// of `DARC_HEAD::Flags`. Not read natively (plan RW, decision D2).
    #[error(
        "this archive uses the newer Wolf RPG encryption (crypt version {0}, editor 3.31+), \
         which Hoshi2Star does not read. Decrypt the game with UberWolf first, \
         then open the resulting Data/ folder"
    )]
    UnsupportedWolfCrypt(u16),
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
    /// Full path from the archive root, starting with `/` (e.g. `/BasicData/DataBase.dat`).
    /// v6/v8 paths are rebuilt from the directory tree; v5 archives (no tree
    /// reconstruction) give `/` + `name`.
    pub path: String,
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
    pub packed_size: i64, // -1 = no LZ compression
    /// Offset of this entry within the file table (needed to find its parent
    /// `DARC_DIRECTORY` and rebuild the full path).
    pub entry_offset: u64,
}

impl DxFileEntry {
    pub fn is_dir(&self) -> bool {
        self.attributes & 0x10 != 0
    }

    /// DxLib stores `PressDataSize = -1` for an uncompressed entry; any other
    /// value is the size of the LZ stream on disk (`DXArchiveVer6.cpp`, decode).
    pub fn is_compressed(&self) -> bool {
        self.packed_size != -1
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
    read_header_for_version(data, version, key)
}

/// [`read_header`] for a version already established by the caller — needed when
/// the signature itself is encrypted (see [`read_signature_any`]).
#[allow(clippy::type_complexity)]
fn read_header_for_version(
    data: &[u8],
    version: u8,
    key: &[u8; 12],
) -> Result<(u8, u64, u64, u64, u64, Option<u32>), DecryptorError> {
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
        other => Err(DecryptorError::UnsupportedVersion(other)),
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
                entry_offset: (i * entry_size) as u64,
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
                entry_offset: (i * entry_size) as u64,
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

    // Validate: version byte in {5, 6, 8} and index_size < 16 MB. DxLib v6
    // decrypts the whole DARC_HEAD from offset 0, so on real archives the
    // signature itself is encrypted: read the version through the key then.
    let version = if raw_header[0..2] == *b"DX" {
        raw_header[0x02]
    } else {
        decrypt_signature(raw_header, &key)?
    };
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

/// Decrypt the 4-byte `DX` signature with `key` (key position 0) and return the
/// DXA version when it reads `DX` + a known version.
fn decrypt_signature(data: &[u8], key: &[u8; 12]) -> Option<u8> {
    if data.len() < 4 {
        return None;
    }
    let mut sig = [data[0], data[1], data[2], data[3]];
    key_conv(&mut sig, 0, key);
    (sig[0..2] == *b"DX" && matches!(sig[2], 5 | 6) && sig[3] == 0).then_some(sig[2])
}

/// Read the DXA version from a plaintext signature, or — when the signature is
/// itself encrypted (DxLib v6 encrypts the header from offset 0, e.g. Wolf 2.23
/// `Data.wolf`) — from the signature decrypted with a known or guessed key.
///
/// Returns the version and, for an encrypted signature, the key that revealed it.
fn read_signature_any(data: &[u8]) -> Result<(u8, Option<[u8; 12]>), DecryptorError> {
    match read_signature(data) {
        Err(DecryptorError::InvalidSignature) => {}
        other => return other.map(|version| (version, None)),
    }
    WOLF_KEYS
        .iter()
        .map(|&(_, key)| key)
        .chain(guess_key_v6(data))
        .find_map(|key| {
            let version = decrypt_signature(data, &key)?;
            is_valid_key(data, version, &key).then_some((version, Some(key)))
        })
        .ok_or(DecryptorError::InvalidSignature)
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
/// 1. Read the signature — plaintext, or encrypted with the header key
///    (DxLib v6 encrypts the whole header from offset 0: [`read_signature_any`]).
/// 2. Discover the XOR key: try `WOLF_KEYS` table, then `guess_key_v6`.
/// 3. Decrypt and read the archive header.
/// 4. Decrypt and parse the TOC (file/directory tables).
/// 5. For each non-directory entry: decrypt (and LZ-decode) its bytes and rebuild
///    its name and full path.
pub fn extract_all(data: &[u8]) -> Result<WolfArchive, DecryptorError> {
    extract_matching(data, &|_| true)
}

/// [`extract_all`] restricted to the files whose full path (`/BasicData/SysDatabase.dat`)
/// satisfies `wanted`: the others are listed but neither decrypted nor decoded.
pub fn extract_matching(
    data: &[u8],
    wanted: &dyn Fn(&str) -> bool,
) -> Result<WolfArchive, DecryptorError> {
    // Step 1 — signature check (an encrypted signature also yields its key)
    let (version, signature_key) = read_signature_any(data)?;

    // DXA v8 has a plaintext header and Huffman+LZ compressed TOC — handled separately
    if version == 8 {
        return extract_all_v8(data, wanted);
    }

    // Step 2 — key discovery
    let header_key = match signature_key {
        Some(key) => key,
        None => find_key(data, version)?,
    };

    // Step 3 — read header (offsets, code_page)
    let (_, base_offset, index_offset, file_table, dir_table, code_page) =
        read_header_for_version(data, version, &header_key)?;
    let index_size = if version <= 5 {
        read_header_v5(data, &header_key)?.index_size as u64
    } else {
        read_header_v6(data, &header_key)?.index_size as u64
    };

    // Step 2b — Wolf v2.255 in-archive key.
    // When the header is unencrypted (header_key == [0;12]), the real XOR key
    // is stored in plaintext at file[base_offset..base_offset+12].
    // The header key (all-zero) must NOT be used for TOC/file decryption.
    let data_key = if header_key == [0u8; 12] {
        let ks = base_offset as usize;
        if data.len() >= ks.saturating_add(12) {
            let mut k = [0u8; 12];
            k.copy_from_slice(&data[ks..ks + 12]);
            k
        } else {
            header_key
        }
    } else {
        header_key
    };

    // Step 4 — extract and decrypt TOC (offsets are attacker-controlled u64s:
    // do the bound check in u64 so a forged value cannot wrap)
    let toc_end = index_offset
        .checked_add(index_size)
        .filter(|&end| end <= data.len() as u64)
        .ok_or(DecryptorError::HeaderTooShort)?;
    let toc_start = index_offset as usize;
    let mut toc_data = data[toc_start..toc_end as usize].to_vec();

    // parse_index decrypts toc_data in place; toc_data is readable after the call
    let entries = parse_index(
        &mut toc_data,
        &data_key,
        version,
        index_offset,
        file_table,
        dir_table,
    )?;

    // v6 keeps the DxLib name table and directory tree (same layout as v8):
    // rebuild real names and full paths. v5 keeps its historical raw-name reading
    // (no real v5 archive has been checked against the DxLib layout yet).
    let dirs = if version >= 6 {
        parse_darc_dirs(&toc_data, dir_table as usize)
    } else {
        Vec::new()
    };

    // Step 5 — extract, decrypt and (LZ-)decode each file
    let code_page_val = code_page.unwrap_or(932);
    let mut files = Vec::with_capacity(entries.len());

    for entry in &entries {
        let ns = usize::try_from(entry.name_offset)
            .ok()
            .filter(|&ns| ns <= toc_data.len())
            .ok_or(DecryptorError::HeaderTooShort)?;
        let (name, path) = if version >= 6 {
            let name = decode_name(read_original_name(&toc_data, ns), code_page_val);
            let parent_idx = find_parent_dir(&dirs, entry.entry_offset, 0x40);
            let path = build_file_path(
                &toc_data,
                &name,
                parent_idx,
                &dirs,
                file_table as usize,
                code_page_val,
            );
            (name, path)
        } else {
            // Decode filename: null-terminated at toc_data[name_offset..]
            let name_len = toc_data[ns..]
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(toc_data.len().saturating_sub(ns));
            let name = decode_name(&toc_data[ns..ns + name_len], code_page_val);
            let path = format!("/{name}");
            (name, path)
        };
        if !wanted(&path) {
            continue;
        }

        let start = base_offset
            .checked_add(entry.data_offset)
            .and_then(|v| usize::try_from(v).ok())
            .ok_or(DecryptorError::HeaderTooShort)?;
        let len =
            usize::try_from(entry.unpacked_size).map_err(|_| DecryptorError::HeaderTooShort)?;
        let stored = if entry.is_compressed() {
            usize::try_from(entry.packed_size).map_err(|_| DecryptorError::HeaderTooShort)?
        } else {
            len
        };
        let end = start
            .checked_add(stored)
            .filter(|&end| end <= data.len())
            .ok_or(DecryptorError::HeaderTooShort)?;
        let mut file_data = data[start..end].to_vec();
        // Wolf RPG bug: decryption offset = unpacked_size, not archive position
        // (also for the LZ stream of a compressed entry, `DXArchiveVer6.cpp`).
        key_conv(&mut file_data, entry.unpacked_size, &data_key);
        if entry.is_compressed() {
            file_data = lz_decode(&file_data)
                .filter(|decoded| decoded.len() == len)
                .ok_or(DecryptorError::UnsupportedCompression)?;
        }

        files.push(WolfFile {
            name,
            path,
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
// DXA v8 — Huffman decoder (port of DxLib Huffman.cpp::Huffman_Decode)
// ---------------------------------------------------------------------------

/// Sanity ceiling for decompressed output claimed by an archive header.
/// A forged size field must not drive the allocation (`orig_size` can claim up
/// to `u64::MAX` → "capacity overflow" panic; `dest_size` up to 4 GiB → OOM).
/// 1 GiB is far above any real Wolf RPG file.
const MAX_DECODE_OUTPUT: usize = 1 << 30;

/// Decode DxLib Huffman-compressed data.
///
/// Header uses MSB-first bit stream; compressed body uses LSB-first.
/// Returns the decompressed bytes, or `None` if the data is malformed.
fn huffman_decode(data: &[u8]) -> Option<Vec<u8>> {
    if data.is_empty() {
        return None;
    }

    // --- MSB-first bit reader for the header ---
    struct HdrBits<'a> {
        buf: &'a [u8],
        byte_pos: usize,
        bit_pos: u8,
    }
    impl<'a> HdrBits<'a> {
        fn read(&mut self, n: u8) -> u64 {
            let mut result = 0u64;
            for i in 0..n {
                if self.byte_pos >= self.buf.len() {
                    return result;
                }
                let bit = (self.buf[self.byte_pos] >> (7 - self.bit_pos)) & 1;
                result |= (bit as u64) << (n - 1 - i);
                self.bit_pos += 1;
                if self.bit_pos == 8 {
                    self.byte_pos += 1;
                    self.bit_pos = 0;
                }
            }
            result
        }
        fn bytes_consumed(&self) -> usize {
            self.byte_pos + usize::from(self.bit_pos > 0)
        }
    }

    let mut bs = HdrBits {
        buf: data,
        byte_pos: 0,
        bit_pos: 0,
    };

    // Header: original size, compressed size, 256-entry delta-coded weight table
    let orig_bits = bs.read(6) as u8 + 1;
    let orig_size = bs.read(orig_bits);
    if orig_size > MAX_DECODE_OUTPUT as u64 {
        return None;
    }
    let orig_size = orig_size as usize;
    let press_bits = bs.read(6) as u8 + 1;
    let _ = bs.read(press_bits); // press_size not needed for decode

    let mut weight = [0u16; 256];
    for i in 0..256usize {
        let bn = (bs.read(3) as u8 + 1) * 2;
        let minus = bs.read(1) != 0;
        let val = bs.read(bn) as u16;
        weight[i] = if i == 0 {
            val
        } else if minus {
            weight[i - 1].wrapping_sub(val)
        } else {
            weight[i - 1].wrapping_add(val)
        };
    }

    let head_size = bs.bytes_consumed();

    // --- Build Huffman tree (511 nodes: 0..255 leaves, 256..510 internal) ---
    const NODES: usize = 511;
    let mut w = [0u64; NODES];
    let mut parent = [-1i32; NODES];
    let mut child = [[-1i32; 2]; NODES];

    for i in 0..256usize {
        w[i] = weight[i] as u64;
    }

    let mut data_num = 256usize;
    let mut node_num = 256usize;

    while data_num > 1 {
        let mut min1 = -1i32;
        let mut min2 = -1i32;
        let mut found = 0usize;
        let mut ni = 0usize;
        while found < data_num && ni < NODES {
            if parent[ni] != -1 {
                ni += 1;
                continue;
            }
            found += 1;
            if min1 == -1 || w[min1 as usize] > w[ni] {
                min2 = min1;
                min1 = ni as i32;
            } else if min2 == -1 || w[min2 as usize] > w[ni] {
                min2 = ni as i32;
            }
            ni += 1;
        }
        if min1 == -1 || min2 == -1 {
            break;
        }
        w[node_num] = w[min1 as usize] + w[min2 as usize];
        parent[node_num] = -1;
        child[node_num][0] = min1;
        child[node_num][1] = min2;
        parent[min1 as usize] = node_num as i32;
        parent[min2 as usize] = node_num as i32;
        node_num += 1;
        data_num -= 1;
    }

    let root = (node_num - 1) as i32; // 510 for 256 symbols

    // --- Decode body using LSB-first bit stream ---
    let press_data = data.get(head_size..).unwrap_or(&[]);
    let mut out = vec![0u8; orig_size];

    let mut ps = 0usize; // byte position in press_data
    let mut pc = 0u32; // bits consumed in current byte
    let mut pb = press_data.first().copied().unwrap_or(0) as u32;

    for slot in out.iter_mut() {
        let mut ni = root;
        while ni > 255 {
            if pc == 8 {
                ps += 1;
                pb = press_data.get(ps).copied().unwrap_or(0) as u32;
                pc = 0;
            }
            let bit = pb & 1;
            pb >>= 1;
            pc += 1;
            let next = child[ni as usize][bit as usize];
            if next < 0 {
                return None;
            }
            ni = next;
        }
        *slot = ni as u8;
    }

    Some(out)
}

// ---------------------------------------------------------------------------
// DXA v8 — LZ decoder (port of DxLib DXArchive.cpp::Decode)
// ---------------------------------------------------------------------------

const MIN_COMPRESS: usize = 4;

/// Decode DxLib LZ-compressed data.
///
/// 9-byte header: destsize(u32) | total_srcsize(u32, includes the 9 header bytes) | keycode(u8).
/// Returns the decompressed bytes, or `None` if the data is malformed.
fn lz_decode(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 9 {
        return None;
    }

    let dest_size = u32::from_le_bytes(data[0..4].try_into().ok()?) as usize;
    let src_size = (u32::from_le_bytes(data[4..8].try_into().ok()?) as usize).checked_sub(9)?;
    let keycode = data[8] as u32;

    if dest_size > MAX_DECODE_OUTPUT {
        return None;
    }
    let mut out = vec![0u8; dest_size];
    let mut dp = 0usize;
    let mut sp = 9usize;
    let mut remaining = src_size;

    while remaining > 0 && dp < dest_size {
        if sp >= data.len() {
            break;
        }
        let byte = data[sp] as u32;

        if byte != keycode {
            out[dp] = data[sp];
            dp += 1;
            sp += 1;
            remaining -= 1;
            continue;
        }

        if sp + 1 >= data.len() || remaining < 2 {
            break;
        }
        let next = data[sp + 1] as u32;
        if next == keycode {
            out[dp] = keycode as u8;
            dp += 1;
            sp += 2;
            remaining -= 2;
            continue;
        }

        let mut code = next;
        if code > keycode {
            code -= 1;
        }
        sp += 2;
        remaining -= 2;

        let mut conbo = (code >> 3) as usize;
        if code & 0x04 != 0 {
            if sp >= data.len() || remaining == 0 {
                break;
            }
            conbo |= (data[sp] as usize) << 5;
            sp += 1;
            remaining -= 1;
        }
        conbo += MIN_COMPRESS;

        let index_size = (code & 0x03) as usize;
        let index = match index_size {
            0 => {
                if sp >= data.len() || remaining == 0 {
                    break;
                }
                let v = data[sp] as usize;
                sp += 1;
                remaining -= 1;
                v
            }
            1 => {
                if sp + 1 >= data.len() || remaining < 2 {
                    break;
                }
                let v = u16::from_le_bytes([data[sp], data[sp + 1]]) as usize;
                sp += 2;
                remaining -= 2;
                v
            }
            2 => {
                if sp + 2 >= data.len() || remaining < 3 {
                    break;
                }
                let v = (data[sp] as usize)
                    | ((data[sp + 1] as usize) << 8)
                    | ((data[sp + 2] as usize) << 16);
                sp += 3;
                remaining -= 3;
                v
            }
            _ => break,
        };
        let index = index + 1; // encoder stored (real_index - 1)

        if index > dp || dp + conbo > dest_size {
            break;
        }

        if index < conbo {
            // Overlapping copy — doubles the pattern each pass (RLE-like)
            let mut num = index;
            while conbo > num {
                for k in 0..num {
                    out[dp + k] = out[dp - num + k];
                }
                dp += num;
                conbo -= num;
                num += num;
            }
            for k in 0..conbo {
                out[dp + k] = out[dp - num + k];
            }
            dp += conbo;
        } else {
            // Non-overlapping back-reference
            out.copy_within(dp - index..dp - index + conbo, dp);
            dp += conbo;
        }
    }

    Some(out)
}

// ---------------------------------------------------------------------------
// DXA v8 — 7-byte key scheme (DxLib KeyCreate / KeyConv)
// ---------------------------------------------------------------------------

/// Standard CRC32 (polynomial 0xEDB88320).
fn crc32(data: &[u8]) -> u32 {
    static TABLE: std::sync::OnceLock<[u32; 256]> = std::sync::OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut t = [0u32; 256];
        for (i, slot) in t.iter_mut().enumerate() {
            let mut crc = i as u32;
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
            *slot = crc;
        }
        t
    });
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc = (crc >> 8) ^ table[((crc ^ b as u32) & 0xFF) as usize];
    }
    !crc
}

/// DxLib `KeyCreate`: derive a 7-byte XOR key from `source`.
///
/// Splits into even-indexed and odd-indexed bytes, CRC32 of each,
/// then concatenates: `CRC32_even[0..4] | CRC32_odd[0..3]`.
pub(crate) fn key_create_7(source: &[u8]) -> [u8; 7] {
    let even: Vec<u8> = source.iter().copied().step_by(2).collect();
    let odd: Vec<u8> = source.iter().copied().skip(1).step_by(2).collect();
    let c0 = crc32(&even).to_le_bytes();
    let c1 = crc32(&odd).to_le_bytes();
    [c0[0], c0[1], c0[2], c0[3], c1[0], c1[1], c1[2]]
}

/// XOR `data` in place with a 7-byte `key`, starting at `key_pos = offset % 7`.
pub(crate) fn key_conv7(data: &mut [u8], offset: u64, key: &[u8; 7]) {
    let mut pos = (offset % 7) as usize;
    for byte in data.iter_mut() {
        *byte ^= key[pos];
        pos += 1;
        if pos == 7 {
            pos = 0;
        }
    }
}

/// Known Wolf RPG v8 key strings (DXA v8 uses 7-byte CRC32-derived keys), with the
/// crypt version (high 16 bits of `DARC_HEAD::Flags`) each one belongs to.
///
/// Format constants from the public mode table of UberWolf (`UberWolfLib/WolfDec.cpp`,
/// `DEFAULT_CRYPT_MODES`, MIT, Sinflower); the table's trailing 0x00 terminator is
/// not part of the key string. "v3.14" is verified on a real Wolf 3.658 game
/// (crypt version 314) on 2026-10-26; "v3.00" is not verified on a real game yet.
pub(crate) const WOLF_V8_KEY_STRINGS: &[(&str, u16, &[u8])] = &[
    ("v2.225", 0, b"WLFRPrO!p(;s5((8P@((UFWlu$#5(="),
    (
        "v3.00",
        300,
        &[
            0x0F, 0x53, 0xE1, 0x3E, 0x8E, 0xB5, 0x41, 0x91, 0x52, 0x16, 0x55, 0xAE, 0x34, 0xC9,
            0x8F, 0x79, 0x59, 0x2F, 0x59, 0x6B, 0x95, 0x19, 0x9B, 0x1B, 0x35, 0x9A, 0x2F, 0xDE,
            0xC9, 0x7C, 0x12, 0x96, 0xC3, 0x14, 0xB5, 0x0F, 0x53, 0xE1, 0x3E, 0x8E,
        ],
    ),
    (
        "v3.14",
        314,
        &[
            0x31, 0xF9, 0x01, 0x36, 0xA3, 0xE3, 0x8D, 0x3C, 0x7B, 0xC3, 0x7D, 0x25, 0xAD, 0x63,
            0x28, 0x19, 0x1B, 0xF7, 0x8E, 0x6C, 0xC4, 0xE5, 0xE2, 0x76, 0x82, 0xEA, 0x4F, 0xED,
            0x61, 0xDA, 0xE0, 0x44, 0x5B, 0xB6, 0x46, 0x3B, 0x06, 0xD5, 0xCE, 0xB6, 0x78, 0x58,
            0xD0, 0x7C, 0x82,
        ],
    ),
];

/// First crypt version of the WOLF RPG Editor 3.31+ "new crypt" (encrypted header
/// addresses + AES-CTR data), not read natively (`DXArchive.cpp`: `cryptVersion >= 331`).
const NEW_CRYPT_MIN: u16 = 331;
/// Crypt versions of the ChaCha20 (WolfX) modes, and first Pro crypt version.
const CHACHA_CRYPT_VERSIONS: [u16; 2] = [0x64, 0xC8];
const PRO_CRYPT_MIN: u16 = 1000;

/// Crypt version of a DXA v8 archive (high 16 bits of the plaintext `Flags`), read
/// without decrypting anything: also available for 3.31+ archives this reader refuses.
/// `None` for any other archive version.
pub fn v8_crypt_version(data: &[u8]) -> Option<u16> {
    if read_signature(data).ok()? != 8 || data.len() < 48 {
        return None;
    }
    let flags = u32::from_le_bytes(data[44..48].try_into().ok()?);
    Some((flags >> 16) as u16)
}

/// Refuse the v8 crypt versions this reader does not implement, with the error
/// that tells the user what to do.
fn check_v8_crypt_version(crypt_version: u16) -> Result<(), DecryptorError> {
    if CHACHA_CRYPT_VERSIONS.contains(&crypt_version) || crypt_version >= PRO_CRYPT_MIN {
        return Err(DecryptorError::PossibleWolfX);
    }
    if crypt_version >= NEW_CRYPT_MIN {
        return Err(DecryptorError::UnsupportedWolfCrypt(crypt_version));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// DXA v8 — DARC_DIRECTORY helpers for per-file key derivation
// ---------------------------------------------------------------------------

/// One entry in the DXA v8 DARC_DIRECTORY table (32 bytes each).
#[derive(Debug, Clone)]
struct DarcDir {
    dir_address: u64,        // offset of this dir's own DARC_FILEHEAD in file table
    parent_dir_address: u64, // offset in dir table of parent (u64::MAX = root)
    file_head_num: u64,
    file_head_address: u64, // offset of first child entry in file table
}

fn parse_darc_dirs(toc: &[u8], dt_offset: usize) -> Vec<DarcDir> {
    let dt = if dt_offset < toc.len() {
        &toc[dt_offset..]
    } else {
        return vec![];
    };
    let n = dt.len() / 32;
    let mut dirs = Vec::with_capacity(n);
    for i in 0..n {
        let b = i * 32;
        if b + 32 > dt.len() {
            break;
        }
        let u64_at = |off: usize| u64::from_le_bytes(dt[b + off..b + off + 8].try_into().unwrap());
        dirs.push(DarcDir {
            dir_address: u64_at(0),
            parent_dir_address: u64_at(8),
            file_head_num: u64_at(16),
            file_head_address: u64_at(24),
        });
    }
    dirs
}

/// Find which `DarcDir` owns the file-table entry at `entry_ft_offset` (relative to FT start).
fn find_parent_dir(dirs: &[DarcDir], entry_ft_offset: u64, entry_sz: u64) -> usize {
    for (i, dir) in dirs.iter().enumerate() {
        // Fields come from the attacker-controlled TOC: a forged range that
        // overflows u64 cannot own any entry — skip it instead of panicking.
        let Some(end) = dir
            .file_head_num
            .checked_mul(entry_sz)
            .and_then(|span| dir.file_head_address.checked_add(span))
        else {
            continue;
        };
        if dir.file_head_num > 0
            && entry_ft_offset >= dir.file_head_address
            && entry_ft_offset < end
        {
            return i;
        }
    }
    0
}

/// Read the uppercase null-terminated name at `toc[name_address + 4..]`.
///
/// Name table format (DxLib `AddFileNameData`):
/// `[0:2]=PackNum, [2:4]=Parity, [4:4+PackNum*4]=UPPERCASE_name, [4+PackNum*4:]=original`
fn read_uppercase_name(toc: &[u8], name_address: usize) -> &[u8] {
    let start = name_address.saturating_add(4);
    if start >= toc.len() {
        return b"";
    }
    let s = &toc[start..];
    let end = s.iter().position(|&b| b == 0).unwrap_or(s.len());
    &s[..end]
}

/// Read the original (mixed-case) null-terminated name from the v6/v8 name table.
fn read_original_name(toc: &[u8], name_address: usize) -> &[u8] {
    if name_address.saturating_add(4) > toc.len() {
        return b"";
    }
    let pack_num = u16::from_le_bytes([toc[name_address], toc[name_address + 1]]) as usize;
    let orig_start = name_address.saturating_add(4).saturating_add(pack_num * 4);
    if orig_start >= toc.len() {
        return b"";
    }
    let s = &toc[orig_start..];
    let end = s.iter().position(|&b| b == 0).unwrap_or(s.len());
    &s[..end]
}

/// Build the per-file key string for DXA v8 (`CreateKeyFileString`).
///
/// Result: `global_key_str` + UPPERCASE_filename + UPPERCASE_dir1 + ...
/// (non-root ancestors, closest first — root's name is never appended).
fn build_per_file_key_str(
    global_key_str: &[u8],
    toc: &[u8],
    name_addr: usize,
    parent_dir_idx: usize,
    dirs: &[DarcDir],
    ft_offset: usize,
) -> Vec<u8> {
    let mut ks = global_key_str.to_vec();
    ks.extend_from_slice(read_uppercase_name(toc, name_addr));

    if dirs.is_empty() {
        return ks;
    }
    let mut dir = &dirs[parent_dir_idx];
    while dir.parent_dir_address != u64::MAX {
        let dir_ft_off = ft_offset.saturating_add(dir.dir_address as usize);
        if dir_ft_off.saturating_add(8) <= toc.len() {
            let dir_name_addr =
                u64::from_le_bytes(toc[dir_ft_off..dir_ft_off + 8].try_into().unwrap()) as usize;
            ks.extend_from_slice(read_uppercase_name(toc, dir_name_addr));
        }
        let parent_idx = (dir.parent_dir_address / 32) as usize;
        if parent_idx >= dirs.len() {
            break;
        }
        dir = &dirs[parent_idx];
    }
    ks
}

/// Build the full archive-relative path of a v6/v8 file entry by walking the
/// directory parent chain (root → file), e.g. `/BasicData/DataBase.dat`.
fn build_file_path(
    toc: &[u8],
    file_name: &str,
    parent_dir_idx: usize,
    dirs: &[DarcDir],
    ft_offset: usize,
    code_page: u32,
) -> String {
    let mut components = vec![file_name.to_string()];

    if !dirs.is_empty() {
        let mut dir = &dirs[parent_dir_idx];
        // Mirror build_per_file_key_str: a dir whose parent_dir_address ==
        // MAX is the archive root — its (placeholder) name is never included.
        while dir.parent_dir_address != u64::MAX {
            let dir_ft_off = ft_offset + dir.dir_address as usize;
            if dir_ft_off + 8 <= toc.len() {
                let dir_name_addr =
                    u64::from_le_bytes(toc[dir_ft_off..dir_ft_off + 8].try_into().unwrap())
                        as usize;
                let dir_name = decode_name(read_original_name(toc, dir_name_addr), code_page);
                components.push(dir_name);
            }
            let parent_idx = (dir.parent_dir_address / 32) as usize;
            if parent_idx >= dirs.len() {
                break;
            }
            dir = &dirs[parent_idx];
        }
    }

    components.reverse();
    format!("/{}", components.join("/"))
}

// ---------------------------------------------------------------------------
// DXA v8 — TOC decompression + key discovery
// ---------------------------------------------------------------------------

fn decompress_v8_toc(
    data: &[u8],
    toc_offset: usize,
    head_size: usize,
    has_key: bool,
    no_head_press: bool,
    key: &[u8; 7],
) -> Result<Vec<u8>, DecryptorError> {
    if no_head_press {
        let mut raw = data[toc_offset..].to_vec();
        if has_key {
            key_conv7(&mut raw, 0, key);
        }
        if raw.len() < head_size {
            return Err(DecryptorError::HeaderTooShort);
        }
        return Ok(raw);
    }
    let mut huff_buf = data[toc_offset..].to_vec();
    if has_key {
        key_conv7(&mut huff_buf, 0, key);
    }
    let lz_buf = huffman_decode(&huff_buf).ok_or(DecryptorError::HeaderTooShort)?;
    let toc_raw = lz_decode(&lz_buf).ok_or(DecryptorError::HeaderTooShort)?;
    if toc_raw.len() < head_size {
        return Err(DecryptorError::HeaderTooShort);
    }
    Ok(toc_raw)
}

/// Try each known v8 key string — the one registered for `crypt_version` first —
/// and return `(global_key, key_string)` for the first that successfully
/// decompresses the TOC to at least `head_size` bytes.
///
/// With an uncompressed TOC (`no_head_press`, e.g. an archive rebuilt by Hoshi2Star)
/// every key "succeeds", so the crypt-version match is what selects the right one.
fn find_v8_key(
    data: &[u8],
    toc_offset: usize,
    head_size: usize,
    has_key: bool,
    no_head_press: bool,
    crypt_version: u16,
) -> Result<([u8; 7], &'static [u8]), DecryptorError> {
    if !has_key {
        return Ok(([0u8; 7], b""));
    }
    let preferred = WOLF_V8_KEY_STRINGS
        .iter()
        .filter(|&&(_, version, _)| version == crypt_version);
    let others = WOLF_V8_KEY_STRINGS
        .iter()
        .filter(|&&(_, version, _)| version != crypt_version);
    for &(_, _, key_str) in preferred.chain(others) {
        let global_key = key_create_7(key_str);
        if decompress_v8_toc(
            data,
            toc_offset,
            head_size,
            has_key,
            no_head_press,
            &global_key,
        )
        .is_ok()
        {
            return Ok((global_key, key_str));
        }
    }
    Err(DecryptorError::PossibleWolfX)
}

// ---------------------------------------------------------------------------
// DXA v8 — per-file helpers
// ---------------------------------------------------------------------------

/// Extract a file whose data is Huffman-encoded (no LZ).
///
/// For files ≤ `huff_kb * 2` bytes, the entire file is Huffman-encoded.
/// For larger files, the first and last `huff_kb` bytes are Huffman-encoded and
/// the middle section follows immediately on disk (encrypted with a shifted key).
#[allow(clippy::too_many_arguments)]
fn extract_v8_huffman_only(
    archive: &[u8],
    file_start: usize,
    unpacked: usize,
    huff_sz: usize,
    huff_kb: usize,
    key: &[u8; 7],
    has_key: bool,
    key_offset: u64,
) -> Result<Vec<u8>, DecryptorError> {
    let huff_end = file_start
        .checked_add(huff_sz)
        .filter(|&end| end <= archive.len())
        .ok_or(DecryptorError::HeaderTooShort)?;
    let mut huff_buf = archive[file_start..huff_end].to_vec();
    if has_key {
        key_conv7(&mut huff_buf, key_offset, key);
    }
    let decoded = huffman_decode(&huff_buf).ok_or(DecryptorError::UnsupportedCompression)?;

    // Small file (≤ 2*huff_kb) or HuffmanEncodeKB == 0xFF: entire file was encoded
    if huff_kb == usize::MAX || unpacked <= huff_kb * 2 {
        if decoded.len() < unpacked {
            return Err(DecryptorError::HeaderTooShort);
        }
        return Ok(decoded[..unpacked].to_vec());
    }

    // Large file: decoded = [front_huff_kb | back_huff_kb], middle is raw on disk
    if decoded.len() < huff_kb * 2 {
        return Err(DecryptorError::HeaderTooShort);
    }
    let middle_sz = unpacked - huff_kb * 2;
    let mid_end = huff_end
        .checked_add(middle_sz)
        .filter(|&end| end <= archive.len())
        .ok_or(DecryptorError::HeaderTooShort)?;
    let mut mid_buf = archive[huff_end..mid_end].to_vec();
    if has_key {
        let mid_key_offset = key_offset
            .checked_add(huff_sz as u64)
            .ok_or(DecryptorError::HeaderTooShort)?;
        key_conv7(&mut mid_buf, mid_key_offset, key);
    }

    let mut out = Vec::with_capacity(unpacked);
    out.extend_from_slice(&decoded[..huff_kb]);
    out.extend_from_slice(&mid_buf);
    out.extend_from_slice(&decoded[huff_kb..]);
    if out.len() != unpacked {
        return Err(DecryptorError::HeaderTooShort);
    }
    Ok(out)
}

/// Reconstruct the LZ stream for a file whose LZ data is partially Huffman-encoded.
///
/// The first and last `huff_kb` bytes of the LZ stream are Huffman-encoded;
/// the middle section follows immediately on disk.
#[allow(clippy::too_many_arguments)]
fn assemble_v8_lz_stream(
    archive: &[u8],
    file_start: usize,
    press_sz: usize,
    huff_sz: usize,
    huff_kb: usize,
    key: &[u8; 7],
    has_key: bool,
    key_offset: u64,
) -> Result<Vec<u8>, DecryptorError> {
    let huff_end = file_start
        .checked_add(huff_sz)
        .filter(|&end| end <= archive.len())
        .ok_or(DecryptorError::HeaderTooShort)?;
    let mut huff_buf = archive[file_start..huff_end].to_vec();
    if has_key {
        key_conv7(&mut huff_buf, key_offset, key);
    }
    let decoded = huffman_decode(&huff_buf).ok_or(DecryptorError::UnsupportedCompression)?;

    // Small LZ stream (≤ 2*huff_kb) or HuffmanEncodeKB == 0xFF: fully encoded
    if huff_kb == usize::MAX || press_sz <= huff_kb * 2 {
        if decoded.len() < press_sz {
            return Err(DecryptorError::HeaderTooShort);
        }
        return Ok(decoded[..press_sz].to_vec());
    }

    // Large LZ stream: decoded = [front_huff_kb | back_huff_kb], middle on disk
    if decoded.len() < huff_kb * 2 {
        return Err(DecryptorError::HeaderTooShort);
    }
    let middle_sz = press_sz - huff_kb * 2;
    let mid_end = huff_end
        .checked_add(middle_sz)
        .filter(|&end| end <= archive.len())
        .ok_or(DecryptorError::HeaderTooShort)?;
    let mut mid_buf = archive[huff_end..mid_end].to_vec();
    if has_key {
        let mid_key_offset = key_offset
            .checked_add(huff_sz as u64)
            .ok_or(DecryptorError::HeaderTooShort)?;
        key_conv7(&mut mid_buf, mid_key_offset, key);
    }

    let mut lz_stream = Vec::with_capacity(press_sz);
    lz_stream.extend_from_slice(&decoded[..huff_kb]);
    lz_stream.extend_from_slice(&mid_buf);
    lz_stream.extend_from_slice(&decoded[huff_kb..]);
    if lz_stream.len() != press_sz {
        return Err(DecryptorError::HeaderTooShort);
    }
    Ok(lz_stream)
}

// ---------------------------------------------------------------------------
// DXA v8 — full extraction pipeline
// ---------------------------------------------------------------------------

/// Extract all files from a DXA v8 archive.
///
/// DXA v8 differences from v6:
/// - DARC_HEAD is PLAINTEXT (no XOR); 7-byte global key is CRC32-derived from a KeyString
/// - `HeadSize` = UNCOMPRESSED TOC size; TOC on disk = XOR(key,pos=0) + Huffman + LZ
/// - `DARC_FILEHEAD` is 72 bytes (adds `HuffPressDataSize` at offset 0x40)
/// - Per-file key = KeyCreate(global_KeyString + UPPERCASE_filename + UPPERCASE_parent_dirs)
fn extract_all_v8(
    data: &[u8],
    wanted: &dyn Fn(&str) -> bool,
) -> Result<WolfArchive, DecryptorError> {
    if data.len() < 64 {
        return Err(DecryptorError::HeaderTooShort);
    }

    let u32_le = |off: usize| u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
    let u64_le = |off: usize| u64::from_le_bytes(data[off..off + 8].try_into().unwrap());

    let head_size = u32_le(4) as usize; // uncompressed TOC size
    let base_offset = u64_le(8) as usize; // data section start (= 64 for this format)
    let toc_offset = u64_le(16) as usize; // absolute TOC start in archive file
    let file_table = u64_le(24) as usize; // relative to TOC start
    let dir_table = u64_le(32) as usize; // relative to TOC start
    let code_page = u32_le(40);
    let flags = u32_le(44);
    // Checked first: from 3.31 on, the address fields above are encrypted and
    // would otherwise surface as a misleading "header too short".
    let crypt_version = (flags >> 16) as u16;
    check_v8_crypt_version(crypt_version)?;
    let huff_kb_raw = data[48] as usize;
    let huff_kb = if huff_kb_raw == 0xFF {
        usize::MAX
    } else {
        huff_kb_raw * 1024
    };

    let has_key = flags & 0x0000_0001 == 0;
    let no_head_press = flags & 0x0000_0002 != 0;

    if toc_offset > data.len() {
        return Err(DecryptorError::HeaderTooShort);
    }

    // Derive global 7-byte key from known key strings (or null key if unencrypted)
    let (global_key, key_str) = find_v8_key(
        data,
        toc_offset,
        head_size,
        has_key,
        no_head_press,
        crypt_version,
    )?;

    // Decompress TOC: XOR(global_key, pos=0) → Huffman → LZ → raw TOC bytes
    let toc_data = decompress_v8_toc(
        data,
        toc_offset,
        head_size,
        has_key,
        no_head_press,
        &global_key,
    )?;

    // Parse DARC_DIRECTORY table (needed for per-file key derivation)
    let dirs = parse_darc_dirs(&toc_data, dir_table);

    // Parse v8 file entries (0x48 bytes each)
    const ENTRY_SZ: usize = 0x48;
    let ft = file_table;
    let dt = dir_table;
    if dt <= ft || dt > toc_data.len() {
        return Ok(WolfArchive {
            version: 8,
            code_page: Some(code_page),
            files: vec![],
        });
    }

    let n_entries = (dt - ft) / ENTRY_SZ;
    let mut files = Vec::with_capacity(n_entries);

    for i in 0..n_entries {
        let base = ft + i * ENTRY_SZ;
        if base + ENTRY_SZ > toc_data.len() {
            break;
        }
        let u64_at = |off: usize| -> u64 {
            u64::from_le_bytes(toc_data[base + off..base + off + 8].try_into().unwrap())
        };
        let i64_at = |off: usize| -> i64 {
            i64::from_le_bytes(toc_data[base + off..base + off + 8].try_into().unwrap())
        };

        let attributes = (u64_at(0x08) & 0xFFFF_FFFF) as u32;
        if attributes & 0x10 != 0 {
            continue; // directory entry — skip
        }

        let name_offset = u64_at(0x00);
        let data_offset = i64_at(0x28) as u64;
        let unpacked_size = i64_at(0x30) as u64;
        let packed_size = i64_at(0x38);
        let huff_press_data_size = u64_at(0x40);
        let entry_ft_offset = (i * ENTRY_SZ) as u64;
        let parent_idx = find_parent_dir(&dirs, entry_ft_offset, ENTRY_SZ as u64);
        let raw_name = read_original_name(&toc_data, name_offset as usize);
        let name = decode_name(raw_name, code_page);
        let path = build_file_path(&toc_data, &name, parent_idx, &dirs, ft, code_page);
        if !wanted(&path) {
            continue;
        }

        // Per-file key: KeyCreate(global_key_str + UPPERCASE_filename + UPPERCASE_dirs)
        let file_key = if has_key {
            let per_key_str = build_per_file_key_str(
                key_str,
                &toc_data,
                name_offset as usize,
                parent_idx,
                &dirs,
                ft,
            );
            key_create_7(&per_key_str)
        } else {
            [0u8; 7]
        };

        // All entry fields are attacker-controlled u64s read from the TOC:
        // offset math must be checked, casts must not truncate.
        let file_start = (base_offset as u64)
            .checked_add(data_offset)
            .and_then(|v| usize::try_from(v).ok())
            .ok_or(DecryptorError::HeaderTooShort)?;
        let unp = usize::try_from(unpacked_size).map_err(|_| DecryptorError::HeaderTooShort)?;
        let huff_sz = huff_press_data_size;
        let press_sz = packed_size;
        let key_off = unpacked_size; // Wolf RPG: key position = DataSize (unpacked_size)

        let file_data: Vec<u8> = match (huff_sz == u64::MAX, press_sz == -1) {
            // Case 4: raw XOR only
            (true, true) => {
                let end = file_start
                    .checked_add(unp)
                    .filter(|&end| end <= data.len())
                    .ok_or(DecryptorError::HeaderTooShort)?;
                let mut buf = data[file_start..end].to_vec();
                if has_key {
                    key_conv7(&mut buf, key_off, &file_key);
                }
                buf
            }
            // Case 3: Huffman-encoded, no LZ
            (false, true) => {
                let hz = usize::try_from(huff_sz).map_err(|_| DecryptorError::HeaderTooShort)?;
                extract_v8_huffman_only(
                    data, file_start, unp, hz, huff_kb, &file_key, has_key, key_off,
                )?
            }
            // Case 2: LZ only, no Huffman
            (true, false) => {
                let pz = usize::try_from(press_sz).map_err(|_| DecryptorError::HeaderTooShort)?;
                let end = file_start
                    .checked_add(pz)
                    .filter(|&end| end <= data.len())
                    .ok_or(DecryptorError::HeaderTooShort)?;
                let mut lz_buf = data[file_start..end].to_vec();
                if has_key {
                    key_conv7(&mut lz_buf, key_off, &file_key);
                }
                lz_decode(&lz_buf).ok_or(DecryptorError::UnsupportedCompression)?
            }
            // Case 1: Huffman + LZ
            (false, false) => {
                let pz = usize::try_from(press_sz).map_err(|_| DecryptorError::HeaderTooShort)?;
                let hz = usize::try_from(huff_sz).map_err(|_| DecryptorError::HeaderTooShort)?;
                let lz_stream = assemble_v8_lz_stream(
                    data, file_start, pz, hz, huff_kb, &file_key, has_key, key_off,
                )?;
                lz_decode(&lz_stream).ok_or(DecryptorError::UnsupportedCompression)?
            }
        };

        files.push(WolfFile {
            name,
            path,
            data: file_data,
            unpacked_size,
        });
    }

    Ok(WolfArchive {
        version: 8,
        code_page: Some(code_page),
        files,
    })
}

/// Synthetic DXA archives in the real DxLib layout, shared by the decryptor and
/// detector tests.
#[cfg(test)]
pub(crate) mod test_archives {
    use super::*;

    /// One file of a synthetic DxLib archive (`dir`: optional single sub-directory).
    pub(crate) struct DxFile<'a> {
        pub dir: Option<&'a str>,
        pub name: &'a str,
        pub content: &'a [u8],
        pub compress: bool,
    }

    /// A synthetic archive and where its TOC and file entries sit.
    pub(crate) struct BuiltArchive {
        pub bytes: Vec<u8>,
        /// Absolute offset of the TOC in `bytes`.
        pub toc_offset: usize,
        /// Offset of each `DxFile` entry within the TOC, in input order.
        pub entry_offsets: Vec<usize>,
    }

    /// DxLib name entry (`AddFileNameData`): `[u16 PackNum][u16 Parity][UPPER][original]`,
    /// both names NUL-padded to `PackNum * 4` bytes — the layout real archives use.
    fn push_dx_name(table: &mut Vec<u8>, name: &str) -> u64 {
        let offset = table.len() as u64;
        let bytes = name.as_bytes();
        let pack = bytes.len() / 4 + 1;
        let parity = bytes
            .iter()
            .fold(0u16, |acc, &b| acc.wrapping_add(u16::from(b)));
        table.extend_from_slice(&(pack as u16).to_le_bytes());
        table.extend_from_slice(&parity.to_le_bytes());
        let mut upper = bytes.to_ascii_uppercase();
        upper.resize(pack * 4, 0);
        table.extend_from_slice(&upper);
        let mut original = bytes.to_vec();
        original.resize(pack * 4, 0);
        table.extend_from_slice(&original);
        offset
    }

    /// DxLib LZ stream made of literals only (valid `lz_decode` input).
    fn lz_literals(content: &[u8]) -> Vec<u8> {
        let keycode = 0xFFu8;
        let mut body = Vec::new();
        for &b in content {
            body.push(b);
            if b == keycode {
                body.push(keycode);
            }
        }
        let mut out = Vec::new();
        out.extend_from_slice(&(content.len() as u32).to_le_bytes());
        out.extend_from_slice(&((body.len() + 9) as u32).to_le_bytes());
        out.push(keycode);
        out.extend_from_slice(&body);
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn write_dx_entry(
        ft: &mut [u8],
        at: usize,
        entry_size: usize,
        name: u64,
        attributes: u64,
        data: u64,
        unpacked: u64,
        packed: i64,
    ) {
        ft[at..at + 8].copy_from_slice(&name.to_le_bytes());
        ft[at + 0x08..at + 0x10].copy_from_slice(&attributes.to_le_bytes());
        ft[at + 0x28..at + 0x30].copy_from_slice(&data.to_le_bytes());
        ft[at + 0x30..at + 0x38].copy_from_slice(&unpacked.to_le_bytes());
        ft[at + 0x38..at + 0x40].copy_from_slice(&packed.to_le_bytes());
        if entry_size == 0x48 {
            // v8 HuffPressDataSize: u64::MAX = no Huffman
            ft[at + 0x40..at + 0x48].copy_from_slice(&u64::MAX.to_le_bytes());
        }
    }

    /// Plaintext TOC (name table | file table | directory table) for `files`, with a
    /// root directory and at most one sub-directory. `data_entries[i]` =
    /// `(data_offset, unpacked, packed)` of file `i`.
    /// Returns `(toc, file_table, dir_table, entry_offsets_in_toc)`.
    fn build_dx_toc(
        files: &[DxFile],
        data_entries: &[(u64, u64, i64)],
        entry_size: usize,
    ) -> (Vec<u8>, u64, u64, Vec<usize>) {
        let subdir = files.iter().find_map(|f| f.dir);
        let root_files: Vec<usize> = (0..files.len())
            .filter(|&i| files[i].dir.is_none())
            .collect();
        let sub_files: Vec<usize> = (0..files.len())
            .filter(|&i| files[i].dir.is_some())
            .collect();

        let mut names = Vec::new();
        let root_name = push_dx_name(&mut names, "");
        let sub_name = subdir.map(|d| push_dx_name(&mut names, d));
        let file_names: Vec<u64> = files
            .iter()
            .map(|f| push_dx_name(&mut names, f.name))
            .collect();

        // File table: root head, root children (sub-dir head first), sub-dir children.
        let has_sub = usize::from(subdir.is_some());
        let root_children = root_files.len() + has_sub;
        let sub_head = entry_size;
        let first_root_file = entry_size * (1 + has_sub);
        let first_sub_file = entry_size * (1 + root_children);
        let mut ft = vec![0u8; entry_size * (1 + root_children + sub_files.len())];
        write_dx_entry(&mut ft, 0, entry_size, root_name, 0x10, 0, 0, -1);
        if let Some(name) = sub_name {
            write_dx_entry(&mut ft, sub_head, entry_size, name, 0x10, 32, 0, -1);
        }
        let mut entry_offsets = vec![0usize; files.len()];
        let placed = root_files
            .iter()
            .enumerate()
            .map(|(k, &i)| (i, first_root_file + k * entry_size))
            .chain(
                sub_files
                    .iter()
                    .enumerate()
                    .map(|(k, &i)| (i, first_sub_file + k * entry_size)),
            );
        for (i, at) in placed {
            let (data, unpacked, packed) = data_entries[i];
            write_dx_entry(
                &mut ft,
                at,
                entry_size,
                file_names[i],
                0x20,
                data,
                unpacked,
                packed,
            );
            entry_offsets[i] = names.len() + at;
        }

        let mut dt = Vec::new();
        for v in [0u64, u64::MAX, root_children as u64, entry_size as u64] {
            dt.extend_from_slice(&v.to_le_bytes());
        }
        if subdir.is_some() {
            for v in [
                sub_head as u64,
                0,
                sub_files.len() as u64,
                first_sub_file as u64,
            ] {
                dt.extend_from_slice(&v.to_le_bytes());
            }
        }

        let file_table = names.len() as u64;
        let dir_table = file_table + ft.len() as u64;
        let mut toc = names;
        toc.extend_from_slice(&ft);
        toc.extend_from_slice(&dt);
        (toc, file_table, dir_table, entry_offsets)
    }

    /// DXA v6 archive laid out like a real Wolf 2.2x `Data.wolf`: 0x30-byte header
    /// (encrypted from offset 0 when `encrypt_signature`, else with a plaintext `DX`),
    /// file data (XOR at offset = unpacked size; literal LZ when `compress`), TOC.
    pub(crate) fn make_v6_dx_archive(
        key: &[u8; 12],
        files: &[DxFile],
        code_page: u32,
        encrypt_signature: bool,
    ) -> BuiltArchive {
        const HEAD: usize = 0x30;
        let mut data = Vec::new();
        let mut entries = Vec::new();
        for f in files {
            let mut stored = if f.compress {
                lz_literals(f.content)
            } else {
                f.content.to_vec()
            };
            let packed = if f.compress { stored.len() as i64 } else { -1 };
            key_conv(&mut stored, f.content.len() as u64, key);
            entries.push((data.len() as u64, f.content.len() as u64, packed));
            data.extend_from_slice(&stored);
        }
        let (mut toc, file_table, dir_table, entry_offsets) = build_dx_toc(files, &entries, 0x40);
        let toc_offset = HEAD + data.len();

        let mut head = Vec::with_capacity(HEAD);
        head.extend_from_slice(b"DX");
        head.extend_from_slice(&6u16.to_le_bytes());
        head.extend_from_slice(&(toc.len() as u32).to_le_bytes());
        head.extend_from_slice(&(HEAD as u64).to_le_bytes());
        head.extend_from_slice(&(toc_offset as u64).to_le_bytes());
        head.extend_from_slice(&file_table.to_le_bytes());
        head.extend_from_slice(&dir_table.to_le_bytes());
        head.extend_from_slice(&code_page.to_le_bytes());
        head.extend_from_slice(&[0u8; 4]);
        if encrypt_signature {
            key_conv(&mut head, 0, key);
        } else {
            key_conv(&mut head[4..], 4, key);
        }
        key_conv(&mut toc, 0, key);

        let mut bytes = head;
        bytes.extend_from_slice(&data);
        bytes.extend_from_slice(&toc);
        BuiltArchive {
            bytes,
            toc_offset,
            entry_offsets,
        }
    }

    /// Single-file v6 archive at the root, plaintext signature.
    pub(crate) fn make_v6_archive(
        key: &[u8; 12],
        file_name: &str,
        content: &[u8],
        code_page: u32,
    ) -> Vec<u8> {
        let files = [DxFile {
            dir: None,
            name: file_name,
            content,
            compress: false,
        }];
        make_v6_dx_archive(key, &files, code_page, false).bytes
    }

    /// DXA v8 archive (Wolf 3.0x/3.1x layout): plaintext 0x40-byte header with
    /// `crypt_version` in the high 16 bits of `Flags`, uncompressed TOC
    /// (`DXA_FLAG_NO_HEAD_PRESS`) encrypted with the global key, raw files encrypted
    /// with their per-file key (key string + UPPER name + UPPER directory).
    pub(crate) fn make_v8_dx_archive(
        key_str: &[u8],
        crypt_version: u16,
        files: &[DxFile],
    ) -> Vec<u8> {
        const HEAD: usize = 0x40;
        let mut data = Vec::new();
        let mut entries = Vec::new();
        for f in files {
            let mut per_file = key_str.to_vec();
            per_file.extend_from_slice(f.name.to_ascii_uppercase().as_bytes());
            if let Some(dir) = f.dir {
                per_file.extend_from_slice(dir.to_ascii_uppercase().as_bytes());
            }
            let mut stored = f.content.to_vec();
            key_conv7(
                &mut stored,
                f.content.len() as u64,
                &key_create_7(&per_file),
            );
            entries.push((data.len() as u64, f.content.len() as u64, -1i64));
            data.extend_from_slice(&stored);
        }
        let (mut toc, file_table, dir_table, _) = build_dx_toc(files, &entries, 0x48);
        let toc_len = toc.len();
        key_conv7(&mut toc, 0, &key_create_7(key_str));
        let toc_offset = HEAD + data.len();

        let mut head = Vec::with_capacity(HEAD);
        head.extend_from_slice(b"DX");
        head.extend_from_slice(&8u16.to_le_bytes());
        head.extend_from_slice(&(toc_len as u32).to_le_bytes());
        head.extend_from_slice(&(HEAD as u64).to_le_bytes());
        head.extend_from_slice(&(toc_offset as u64).to_le_bytes());
        head.extend_from_slice(&file_table.to_le_bytes());
        head.extend_from_slice(&dir_table.to_le_bytes());
        head.extend_from_slice(&65001u32.to_le_bytes());
        head.extend_from_slice(&((u32::from(crypt_version) << 16) | 0x2).to_le_bytes());
        head.resize(HEAD, 0);

        let mut bytes = head;
        bytes.extend_from_slice(&data);
        bytes.extend_from_slice(&toc);
        bytes
    }

    /// Decrypt the TOC of a [`make_v6_dx_archive`] result in place, apply `patch`,
    /// then re-encrypt (`key_conv` is a symmetric XOR).
    pub(crate) fn patch_v6_toc(
        built: &mut BuiltArchive,
        key: &[u8; 12],
        patch: impl FnOnce(&mut [u8]),
    ) {
        let toc = &mut built.bytes[built.toc_offset..];
        key_conv(toc, 0, key);
        patch(toc);
        key_conv(toc, 0, key);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::test_archives::*;
    use super::*;

    // --- DXA v8: crc32 / key_create_7 / key_conv7 ---

    #[test]
    fn test_key_create_7_wolf_v2225() {
        // Python reference: key_create_7(b"WLFRPrO!p(;s5((8P@((UFWlu$#5(=")
        //   even = b"WFRrp;5(8@(FluS#="  → CRC32 = 0xA76CF056
        //   odd  = b"LPO!(s(P(UW$5("    → CRC32 = 0x0ECFF087
        // key = [0x56, 0xF0, 0x6C, 0xA7, 0x87, 0xF0, 0x0E]
        let key = key_create_7(b"WLFRPrO!p(;s5((8P@((UFWlu$#5(=");
        // Verify round-trip: XOR apply then undo restores original
        let original = b"SomeTestData123".to_vec();
        let mut buf = original.clone();
        key_conv7(&mut buf, 0, &key);
        assert_ne!(buf, original);
        key_conv7(&mut buf, 0, &key);
        assert_eq!(buf, original);
    }

    #[test]
    fn test_key_conv7_symmetric() {
        let key = [0x56u8, 0xF0, 0x6C, 0xA7, 0x87, 0xF0, 0x0E];
        let original = b"Hello, Wolf v8!".to_vec();
        let mut data = original.clone();
        key_conv7(&mut data, 0, &key);
        assert_ne!(data, original);
        key_conv7(&mut data, 0, &key);
        assert_eq!(data, original);
    }

    #[test]
    fn test_key_conv7_offset_wraps() {
        // offset=7 → key_pos = 7 % 7 = 0 → same as offset=0
        let key = [0x56u8, 0xF0, 0x6C, 0xA7, 0x87, 0xF0, 0x0E];
        let mut a = [0x00u8];
        let mut b = [0x00u8];
        key_conv7(&mut a, 0, &key);
        key_conv7(&mut b, 7, &key);
        assert_eq!(a[0], b[0]);
    }

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

    // --- Step 5: read_header_v6 + CodePage ---

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
        entries[e..e + 0x04].copy_from_slice(&10u32.to_le_bytes());
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
        entries[e..e + 0x08].copy_from_slice(&10i64.to_le_bytes());
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

    fn v8_key_string(name: &str) -> &'static [u8] {
        WOLF_V8_KEY_STRINGS
            .iter()
            .find(|(n, _, _)| *n == name)
            .map(|&(_, _, k)| k)
            .unwrap()
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
    fn test_extract_all_v6_encrypted_signature_guessed_key() {
        // Wolf 2.23 `Data.wolf`: the whole header is XOR-encrypted from offset 0
        // (signature included) with a key absent from WOLF_KEYS.
        let key = [
            0xC7u8, 0x05, 0xCA, 0x7D, 0x8D, 0xE3, 0xDE, 0xF1, 0xD9, 0x0C, 0x85, 0xF4,
        ];
        assert!(WOLF_KEYS.iter().all(|&(_, k)| k != key));
        let files = [DxFile {
            dir: None,
            name: "Game.dat",
            content: b"encrypted-signature archive",
            compress: false,
        }];
        let built = make_v6_dx_archive(&key, &files, 932, true);
        assert!(matches!(
            read_signature(&built.bytes),
            Err(DecryptorError::InvalidSignature)
        ));

        let result = extract_all(&built.bytes).unwrap();
        assert_eq!(result.version, 6);
        assert_eq!(result.code_page, Some(932));
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.files[0].name, "Game.dat");
        assert_eq!(
            result.files[0].data.as_slice(),
            b"encrypted-signature archive"
        );
    }

    #[test]
    fn test_extract_all_v6_lz_entries_and_directory_paths() {
        let key = WOLF_KEYS
            .iter()
            .find(|(n, _)| *n == "v2.20")
            .map(|(_, k)| *k)
            .unwrap();
        let map = b"WOLFM map payload \xFF with the LZ keycode inside \xFF\xFF";
        let files = [
            DxFile {
                dir: Some("MapData"),
                name: "Map001.mps",
                content: map,
                compress: true,
            },
            DxFile {
                dir: None,
                name: "Game.dat",
                content: b"raw root file",
                compress: false,
            },
        ];
        let built = make_v6_dx_archive(&key, &files, 932, true);

        let result = extract_all(&built.bytes).unwrap();
        let by_path = |path: &str| {
            result
                .files
                .iter()
                .find(|f| f.path == path)
                .unwrap_or_else(|| panic!("{path} not found"))
        };
        let mps = by_path("/MapData/Map001.mps");
        assert_eq!(mps.name, "Map001.mps");
        assert_eq!(mps.data.as_slice(), map.as_slice());
        assert_eq!(by_path("/Game.dat").data.as_slice(), b"raw root file");
    }

    #[test]
    fn test_extract_all_v8_crypt_314_uses_v314_key_and_paths() {
        let files = [
            DxFile {
                dir: Some("MapData"),
                name: "TitleMap.mps",
                content: b"v3 map bytes",
                compress: false,
            },
            DxFile {
                dir: None,
                name: "Game.dat",
                content: b"v3 game bytes",
                compress: false,
            },
        ];
        let archive = make_v8_dx_archive(v8_key_string("v3.14"), 314, &files);

        let result = extract_all(&archive).unwrap();
        assert_eq!(result.version, 8);
        let map = result
            .files
            .iter()
            .find(|f| f.path == "/MapData/TitleMap.mps")
            .unwrap();
        assert_eq!(map.data.as_slice(), b"v3 map bytes");
        let game = result.files.iter().find(|f| f.path == "/Game.dat").unwrap();
        assert_eq!(game.data.as_slice(), b"v3 game bytes");
    }

    #[test]
    fn test_find_v8_key_prefers_the_crypt_version_key() {
        // Uncompressed TOC: every key "decompresses", so the crypt version decides.
        let archive = make_v8_dx_archive(v8_key_string("v3.14"), 314, &[]);
        let toc_offset = u64::from_le_bytes(archive[16..24].try_into().unwrap()) as usize;
        let (_, key_str) = find_v8_key(&archive, toc_offset, 0, true, true, 314).unwrap();
        assert_eq!(key_str, v8_key_string("v3.14"));
        let (_, key_str) = find_v8_key(&archive, toc_offset, 0, true, true, 0).unwrap();
        assert_eq!(key_str, v8_key_string("v2.225"));
    }

    #[test]
    fn test_v8_new_crypt_and_wolfx_versions_are_refused() {
        assert!(check_v8_crypt_version(0).is_ok());
        assert!(check_v8_crypt_version(300).is_ok());
        assert!(check_v8_crypt_version(314).is_ok());
        assert!(matches!(
            check_v8_crypt_version(331),
            Err(DecryptorError::UnsupportedWolfCrypt(331))
        ));
        for wolfx in [0x64, 0xC8, 1000, 1020] {
            assert!(matches!(
                check_v8_crypt_version(wolfx),
                Err(DecryptorError::PossibleWolfX)
            ));
        }

        // Wolf 3.666 header: crypt version 350, encrypted address fields.
        let mut header = vec![0u8; 64];
        header[0..4].copy_from_slice(&[b'D', b'X', 8, 0]);
        header[8..16].copy_from_slice(&0x10FE_13E7_2871_166Eu64.to_le_bytes());
        header[44..48].copy_from_slice(&(350u32 << 16).to_le_bytes());
        assert!(matches!(
            extract_all(&header),
            Err(DecryptorError::UnsupportedWolfCrypt(350))
        ));
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

    #[test]
    #[ignore]
    fn diag_real_data_wolf_honoka_v8() {
        let path = "/home/blackat/project/Hoshi2Star/docs/games/月咲流ホノカver1.03/Data.wolf";
        let data = std::fs::read(path).expect("cannot read Data.wolf");

        println!("file_size={}", data.len());
        match extract_all(&data) {
            Ok(archive) => {
                println!(
                    "✅ OK  version={}  code_page={:?}  files={}",
                    archive.version,
                    archive.code_page,
                    archive.files.len()
                );
                for f in archive.files.iter().take(30) {
                    println!("  {:<50}  {} bytes", f.name, f.data.len());
                }
            }
            Err(e) => println!("❌ FAILED: {e}"),
        }
    }

    #[test]
    #[ignore]
    fn diag_real_data_wolf_honoka() {
        let path = "/home/blackat/project/Hoshi2Star/docs/games/月咲流ホノカver1.03/Data.wolf";
        let data = std::fs::read(path).expect("cannot read Data.wolf");

        println!("file_size = {}", data.len());
        println!("version = {}", data[2]);

        // Try every known key + guess
        let version = read_signature(&data).unwrap();
        for &(name, key) in WOLF_KEYS {
            let valid = is_valid_key(&data, version, &key);
            if valid {
                println!("WOLF_KEYS[{name}] passes is_valid_key");
                // Decode header with this key
                if let Ok((_, base, idx, ft, dt, cp)) = read_header(&data, &key) {
                    println!("  base={base:#x} index_offset={idx} ft={ft} dt={dt} cp={cp:?}");
                }
            }
        }
        if let Some(key) = guess_key_v6(&data) {
            println!("guess_key_v6 = {:02x?}", key);
            if let Ok((_, base, idx, ft, dt, cp)) = read_header(&data, &key) {
                println!(
                    "  base={base:#x} index_offset={idx} file_table={ft} dir_table={dt} cp={cp:?}"
                );
                if let Ok(h) = read_header_v6(&data, &key) {
                    let is = h.index_size as u64;
                    println!("  index_size={is}  toc_end={}", idx as usize + is as usize);
                }
            }
        } else {
            println!("guess_key_v6 = None");
        }

        match extract_all(&data) {
            Ok(archive) => {
                println!(
                    "✅ OK  version={}  code_page={:?}  files={}",
                    archive.version,
                    archive.code_page,
                    archive.files.len()
                );
                for f in &archive.files {
                    println!("  {:<50}  {} bytes", f.name, f.data.len());
                }
            }
            Err(e) => println!("❌ FAILED: {e}"),
        }
    }

    // --- Phase 2 (audit 2026-07-01): malformed input → Err, never panic ---

    /// Decrypt the TOC of a `make_v5_archive` result in place, apply `patch`,
    /// then re-encrypt (`key_conv` is a symmetric XOR).
    fn patch_v5_toc(
        archive: &mut [u8],
        key: &[u8; 12],
        content_len: usize,
        patch: impl FnOnce(&mut [u8]),
    ) {
        let index_offset = 0x18 + content_len;
        let toc = &mut archive[index_offset..];
        key_conv(toc, index_offset as u64 % 12, key);
        patch(toc);
        key_conv(toc, index_offset as u64 % 12, key);
    }

    #[test]
    fn test_huffman_decode_huge_orig_size_returns_none() {
        // All-ones header claims orig_bits = 64 and orig_size ≈ u64::MAX:
        // must refuse instead of panicking with "capacity overflow".
        assert!(huffman_decode(&[0xFF; 16]).is_none());
    }

    #[test]
    fn test_lz_decode_huge_dest_size_returns_none() {
        // destsize = u32::MAX with a 1-byte body: must refuse the 4 GiB allocation.
        let mut data = Vec::new();
        data.extend_from_slice(&u32::MAX.to_le_bytes()); // dest_size
        data.extend_from_slice(&10u32.to_le_bytes()); // total_srcsize (9 header + 1)
        data.push(0xAA); // keycode
        data.push(0x42); // body
        assert!(lz_decode(&data).is_none());
    }

    #[test]
    fn test_extract_all_v5_name_offset_out_of_bounds_is_err() {
        let key = WOLF_KEYS
            .iter()
            .find(|(n, _)| *n == "v2.20")
            .map(|(_, k)| *k)
            .unwrap();
        let content = b"0123456789";
        let mut archive = make_v5_archive(&key, "game.dat", content);
        // Name table "game.dat\0" = 9 bytes → entry starts at 9;
        // entry[0x00..0x04] = name_offset → point far beyond the TOC.
        patch_v5_toc(&mut archive, &key, content.len(), |toc| {
            toc[9..13].copy_from_slice(&0xFFFFu32.to_le_bytes());
        });
        assert!(extract_all(&archive).is_err());
    }

    #[test]
    fn test_extract_all_v6_huge_data_offset_is_err() {
        let key = WOLF_KEYS
            .iter()
            .find(|(n, _)| *n == "v2.20")
            .map(|(_, k)| *k)
            .unwrap();
        let files = [DxFile {
            dir: None,
            name: "data.bin",
            content: b"0123456789",
            compress: false,
        }];
        let mut built = make_v6_dx_archive(&key, &files, 932, false);
        // entry[0x28..0x30] = data_offset → u64::MAX (base_offset + data_offset wraps).
        let at = built.entry_offsets[0] + 0x28;
        patch_v6_toc(&mut built, &key, |toc| {
            toc[at..at + 8].copy_from_slice(&(-1i64).to_le_bytes());
        });
        assert!(extract_all(&built.bytes).is_err());
    }

    #[test]
    fn test_extract_v8_huffman_only_short_decoded_is_err() {
        // [0u8; 16] huffman-decodes to an EMPTY buffer (orig_size = 0), while
        // unpacked = 2049 > 2*huff_kb selects the "large file" branch that used
        // to slice decoded[..huff_kb] unchecked.
        let archive = vec![0u8; 4096];
        let r = extract_v8_huffman_only(&archive, 0, 2049, 16, 1024, &[0u8; 7], false, 0);
        assert!(r.is_err());
    }

    #[test]
    fn test_assemble_v8_lz_stream_short_decoded_is_err() {
        let archive = vec![0u8; 4096];
        let r = assemble_v8_lz_stream(&archive, 0, 2049, 16, 1024, &[0u8; 7], false, 0);
        assert!(r.is_err());
    }

    #[test]
    fn test_extract_v8_huffman_only_offset_overflow_is_err() {
        // file_start near usize::MAX: file_start + huff_sz must not wrap.
        let archive = vec![0u8; 64];
        let r = extract_v8_huffman_only(
            &archive,
            usize::MAX - 8,
            2049,
            16,
            1024,
            &[0u8; 7],
            false,
            0,
        );
        assert!(r.is_err());
    }
}
