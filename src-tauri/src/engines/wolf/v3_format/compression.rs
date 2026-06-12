//! LZ4 wrapper for WOLF RPG Editor v3.x `.mps`/`CommonEvent.dat` files.
//!
//! On-disk layout: a 25-byte header (`magic[20]` + `version: u32` +
//! `unknown2: u8`), followed by `dec_size: u32`, `enc_size: u32`, then an
//! `enc_size`-byte raw LZ4 block that decompresses to `dec_size` bytes. The
//! decompressed payload, prefixed by the 25-byte header, is what
//! [`super::map::MapV3::parse`] expects.

use super::V3FormatError;

/// 16-byte fixed magic prefix shared by all v3.x map/database files:
/// 10 zero bytes followed by `"WOLFM\0"`.
const MAGIC_PREFIX: [u8; 16] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x57, 0x4F, 0x4C, 0x46, 0x4D, 0x00,
];

/// Offset of the `version: u32` field, also the size of the LZ4-compressed
/// header prefix that is written/read uncompressed.
const HEADER_SIZE: usize = 25;

/// Returns `true` if `bytes` looks like an LZ4-wrapped v3.x map/database
/// file: the fixed 16-byte magic prefix, a UTF-8 flag byte (`0x00` or
/// `0x55`), three zero bytes, and a `version >= 0x65` byte that selects the
/// LZ4-compressed v3.x layout.
pub(crate) fn is_lz4_v3(bytes: &[u8]) -> bool {
    bytes.len() >= 33
        && bytes[0..16] == MAGIC_PREFIX
        && matches!(bytes[16], 0x00 | 0x55)
        && bytes[17..20] == [0x00, 0x00, 0x00]
        && bytes[20] >= 0x65
}

/// Decompresses an LZ4-wrapped v3.x file into `header[0..25] ++ payload`,
/// where `payload` is the LZ4-decompressed body (starting with `unknown3`).
/// The returned buffer is what [`super::map::MapV3::parse`] expects.
pub(crate) fn decompress_v3(bytes: &[u8]) -> Result<Vec<u8>, V3FormatError> {
    decompress_block(bytes, HEADER_SIZE)
}

/// Recompresses a `header[0..25] ++ payload` buffer (as produced by
/// [`decompress_v3`] / [`super::map::MapV3::dump`]) back into the on-disk
/// LZ4-wrapped layout.
pub(crate) fn recompress_v3(decompressed: &[u8]) -> Result<Vec<u8>, V3FormatError> {
    recompress_block(decompressed, HEADER_SIZE)
}

/// Decompresses a `header[0..header_size] + dec_size:u32 + enc_size:u32 +
/// block` buffer into `header[0..header_size] ++ payload`, where `payload`
/// is the LZ4-decompressed body. Shared by [`decompress_v3`] (`.mps`,
/// `header_size == 25`) and [`super::common_events::decompress`]
/// (`CommonEvent.dat`, `header_size == 11`).
pub(crate) fn decompress_block(bytes: &[u8], header_size: usize) -> Result<Vec<u8>, V3FormatError> {
    let block_start = header_size + 8;
    if bytes.len() < block_start {
        return Err(V3FormatError::UnexpectedEof {
            offset: 0,
            needed: block_start,
            available: bytes.len(),
        });
    }

    let dec_size =
        u32::from_le_bytes(bytes[header_size..header_size + 4].try_into().unwrap()) as usize;
    let enc_size =
        u32::from_le_bytes(bytes[header_size + 4..header_size + 8].try_into().unwrap()) as usize;
    let block_end = block_start
        .checked_add(enc_size)
        .ok_or(V3FormatError::Lz4Block {
            message: "enc_size overflow".to_owned(),
        })?;

    if bytes.len() < block_end {
        return Err(V3FormatError::UnexpectedEof {
            offset: block_start,
            needed: enc_size,
            available: bytes.len() - block_start.min(bytes.len()),
        });
    }

    let payload =
        lz4_flex::block::decompress(&bytes[block_start..block_end], dec_size).map_err(|e| {
            V3FormatError::Lz4Block {
                message: e.to_string(),
            }
        })?;

    let mut out = Vec::with_capacity(header_size + payload.len());
    out.extend_from_slice(&bytes[0..header_size]);
    out.extend_from_slice(&payload);
    Ok(out)
}

/// Recompresses a `header[0..header_size] ++ payload` buffer back into the
/// on-disk `header + dec_size:u32 + enc_size:u32 + block` layout — the
/// inverse of [`decompress_block`].
pub(crate) fn recompress_block(
    decompressed: &[u8],
    header_size: usize,
) -> Result<Vec<u8>, V3FormatError> {
    if decompressed.len() < header_size {
        return Err(V3FormatError::UnexpectedEof {
            offset: 0,
            needed: header_size,
            available: decompressed.len(),
        });
    }

    let header = &decompressed[..header_size];
    let payload = &decompressed[header_size..];
    let block = lz4_flex::block::compress(payload);

    let mut out = Vec::with_capacity(header_size + 8 + block.len());
    out.extend_from_slice(header);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(&(block.len() as u32).to_le_bytes());
    out.extend_from_slice(&block);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_header(version: u32, is_utf8: bool) -> [u8; HEADER_SIZE] {
        let mut header = [0u8; HEADER_SIZE];
        header[0..16].copy_from_slice(&MAGIC_PREFIX);
        header[16] = if is_utf8 { 0x55 } else { 0x00 };
        header[20..24].copy_from_slice(&version.to_le_bytes());
        header[24] = 0;
        header
    }

    #[test]
    fn test_is_lz4_v3_detects_valid_header() {
        let header = synthetic_header(0x67, true);
        let mut bytes = header.to_vec();
        bytes.extend_from_slice(&[0u8; 13]); // pad to >= 33 bytes
        assert!(is_lz4_v3(&bytes));
    }

    #[test]
    fn test_is_lz4_v3_rejects_short_buffer() {
        assert!(!is_lz4_v3(&[0u8; 10]));
    }

    #[test]
    fn test_is_lz4_v3_rejects_pre_v3_version() {
        let header = synthetic_header(0x10, false);
        let mut bytes = header.to_vec();
        bytes.extend_from_slice(&[0u8; 13]);
        assert!(!is_lz4_v3(&bytes));
    }

    #[test]
    fn test_decompress_recompress_round_trip() {
        let header = synthetic_header(0x67, true);
        let payload = b"hello wolf v3 payload, compress me please".to_vec();

        let compressed = recompress_v3(&[header.to_vec(), payload.clone()].concat()).unwrap();
        assert!(is_lz4_v3(&compressed));

        let decompressed = decompress_v3(&compressed).unwrap();
        assert_eq!(&decompressed[..HEADER_SIZE], &header);
        assert_eq!(&decompressed[HEADER_SIZE..], payload.as_slice());
    }
}
