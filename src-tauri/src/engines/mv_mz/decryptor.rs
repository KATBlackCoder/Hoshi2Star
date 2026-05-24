//! RPG Maker MV/MZ — asset decryptor
//!
//! Decrypts `.rpgmvp` (images), `.rpgmvo` (audio), `.rpgmvm` (video) files.
//!
//! ## Format
//! ```text
//! [0..16)   16-byte RPGM signature (magic)
//! [16..32)  first 16 bytes of original file XOR'd with the encryption key
//! [32..)    rest of original file, unchanged
//! ```
//!
//! The encryption key is a 32-character hex string from `System.json > encryptionKey`.
//! It encodes 16 bytes (2 hex chars per byte).

/// Magic prefix shared by MV and MZ encrypted assets.
/// Bytes 0..4 are `RPGM` (0x52, 0x50, 0x47, 0x4D).
/// Byte 4 is `V` (MV) or `Z` (MZ).
const RPGM_MAGIC_PREFIX: &[u8] = &[0x52, 0x50, 0x47, 0x4D];

/// Full header length (bytes to skip before encrypted data begins).
const HEADER_LEN: usize = 16;

#[derive(Debug, thiserror::Error)]
pub enum DecryptorError {
    #[error("file is too short to be a valid encrypted RPG Maker asset")]
    FileTooShort,
    #[error("file does not start with the RPGMV/RPGMZ magic signature")]
    InvalidMagic,
    #[error("encryption key is invalid (expected 32 hex characters): {0}")]
    InvalidKey(String),
}

/// Decrypt an RPG Maker MV/MZ encrypted asset.
///
/// `data` is the raw bytes of the `.rpgmvp/.rpgmvo/.rpgmvm` file.
/// `key` is the 32-character hex string from `System.json > encryptionKey`.
///
/// Returns the decrypted file content (the original image/audio/video bytes).
pub fn decrypt_file(data: &[u8], key: &str) -> Result<Vec<u8>, DecryptorError> {
    let key_bytes = parse_hex_key(key)?;

    if data.len() < HEADER_LEN + key_bytes.len() {
        return Err(DecryptorError::FileTooShort);
    }

    // Validate RPGM prefix (first 4 bytes) and version byte (V or Z)
    if data.len() < 5 || &data[..4] != RPGM_MAGIC_PREFIX || (data[4] != b'V' && data[4] != b'Z') {
        return Err(DecryptorError::InvalidMagic);
    }

    // XOR bytes [HEADER_LEN..HEADER_LEN+16] with the key
    let mut result = Vec::with_capacity(data.len() - HEADER_LEN);
    for i in 0..key_bytes.len() {
        result.push(data[HEADER_LEN + i] ^ key_bytes[i]);
    }
    // Append the rest of the file unchanged
    result.extend_from_slice(&data[HEADER_LEN + key_bytes.len()..]);

    Ok(result)
}

/// Parse a 32-character hex string into 16 bytes.
fn parse_hex_key(key: &str) -> Result<[u8; 16], DecryptorError> {
    if key.len() != 32 {
        return Err(DecryptorError::InvalidKey(format!(
            "length {} != 32",
            key.len()
        )));
    }
    let mut bytes = [0u8; 16];
    for (i, chunk) in key.as_bytes().chunks(2).enumerate() {
        let hex_str = std::str::from_utf8(chunk)
            .map_err(|_| DecryptorError::InvalidKey("non-UTF8 characters".into()))?;
        bytes[i] = u8::from_str_radix(hex_str, 16)
            .map_err(|_| DecryptorError::InvalidKey(format!("invalid hex byte: {hex_str}")))?;
    }
    Ok(bytes)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a synthetic encrypted file from a "plaintext" payload and key.
    fn make_encrypted(payload: &[u8], key: &[u8; 16], is_mz: bool) -> Vec<u8> {
        let mut file = Vec::new();
        // 16-byte header
        file.extend_from_slice(RPGM_MAGIC_PREFIX);
        file.push(if is_mz { b'Z' } else { b'V' });
        file.extend_from_slice(&[
            0x00, 0x00, 0x00, 0x00, 0x03, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]);
        // XOR first 16 bytes of payload with key
        for i in 0..16 {
            file.push(payload[i] ^ key[i]);
        }
        // Rest unchanged
        file.extend_from_slice(&payload[16..]);
        file
    }

    fn key_to_hex(key: &[u8; 16]) -> String {
        key.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn test_decrypt_mv_round_trip() {
        let key: [u8; 16] = [
            0x9a, 0x58, 0xa3, 0xab, 0x5d, 0x37, 0xb8, 0xc6, 0xe0, 0xf7, 0xa9, 0xb2, 0xc4, 0xd5,
            0xe6, 0xf7,
        ];
        // 32 bytes of fake PNG data (at minimum HEADER_LEN + 16 = 32 bytes)
        let original: Vec<u8> = (0u8..32).collect();

        let encrypted = make_encrypted(&original, &key, false);
        let decrypted = decrypt_file(&encrypted, &key_to_hex(&key)).unwrap();

        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_decrypt_mz_round_trip() {
        let key: [u8; 16] = [0x01; 16];
        let original: Vec<u8> = (0u8..48).collect();

        let encrypted = make_encrypted(&original, &key, true);
        let decrypted = decrypt_file(&encrypted, &key_to_hex(&key)).unwrap();

        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_decrypt_error_invalid_magic() {
        let key = "00000000000000000000000000000000";
        // File that does NOT start with RPGM
        let bad_data = vec![0xFF; 64];
        let result = decrypt_file(&bad_data, key);
        assert!(matches!(result, Err(DecryptorError::InvalidMagic)));
    }

    #[test]
    fn test_decrypt_error_file_too_short() {
        let key = "9a58a3ab5d37b8c6e0f7a9b2c4d5e6f7";
        // Valid magic but too short (< 32 bytes)
        let mut data = vec![0u8; 20];
        data[..4].copy_from_slice(RPGM_MAGIC_PREFIX);
        data[4] = b'V';
        let result = decrypt_file(&data, key);
        assert!(matches!(result, Err(DecryptorError::FileTooShort)));
    }

    #[test]
    fn test_decrypt_error_invalid_key_length() {
        let key: [u8; 16] = [0x00; 16];
        let original: Vec<u8> = (0u8..32).collect();
        let encrypted = make_encrypted(&original, &key, false);

        let result = decrypt_file(&encrypted, "tooshort");
        assert!(matches!(result, Err(DecryptorError::InvalidKey(_))));
    }

    #[test]
    fn test_decrypt_error_invalid_key_hex() {
        let key: [u8; 16] = [0x00; 16];
        let original: Vec<u8> = (0u8..32).collect();
        let encrypted = make_encrypted(&original, &key, false);

        let result = decrypt_file(&encrypted, "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz");
        assert!(matches!(result, Err(DecryptorError::InvalidKey(_))));
    }

    #[test]
    fn test_parse_hex_key_all_zeros() {
        let result = parse_hex_key("00000000000000000000000000000000").unwrap();
        assert_eq!(result, [0u8; 16]);
    }

    #[test]
    fn test_parse_hex_key_all_ff() {
        let result = parse_hex_key("ffffffffffffffffffffffffffffffff").unwrap();
        assert_eq!(result, [0xFFu8; 16]);
    }
}
