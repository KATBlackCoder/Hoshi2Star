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
#[allow(dead_code)] // instantiated in Step 7 (parse_index)
#[derive(Debug, Clone)]
pub(crate) struct DxFileEntry {
    pub name_offset: u64,
    pub attributes: u32,
    pub data_offset: u64,
    pub unpacked_size: u64,
    pub packed_size: i64, // -1 = uncompressed
}

#[allow(dead_code)] // used in Step 7/8 (parse_index, extract_all)
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
#[allow(dead_code)] // called by read_header_v5/v6 (Step 4/5) and extract_all (Step 8)
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

#[allow(dead_code)] // consumed by read_header (Step 5)
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
#[allow(dead_code)] // called by read_header (Step 5)
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
}
