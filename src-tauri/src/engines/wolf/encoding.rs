// Wolf RPG — Shift-JIS ↔ UTF-8 conversion layer.
// Functions become used in Step 3 (extractor) and Step 4 (mps parser).
#![allow(dead_code)]
//
// Wolf RPG v1/v2 files use Shift-JIS (code page 932).
// Wolf RPG v3+ files use UTF-8.
// This module centralises encoding logic to prevent drift between extractor and injector.

use encoding_rs::SHIFT_JIS;

/// Decode Shift-JIS bytes to a UTF-8 `String`.
pub(crate) fn decode_shiftjis(bytes: &[u8]) -> Result<String, String> {
    let (decoded, _, had_errors) = SHIFT_JIS.decode(bytes);
    if had_errors {
        Err(format!("Shift-JIS decode error on {} bytes", bytes.len()))
    } else {
        Ok(decoded.into_owned())
    }
}

/// Encode a UTF-8 `str` to Shift-JIS bytes.
///
/// Returns `Err` if any character cannot be represented in Shift-JIS (e.g. emoji,
/// Latin letters with diacritics beyond the limited SJIS range). Used by F4-04
/// injector as a guard — Wolf RPG v2 will crash if UTF-8 bytes are written back.
pub(crate) fn encode_shiftjis(text: &str) -> Result<Vec<u8>, String> {
    let (encoded, _, had_errors) = SHIFT_JIS.encode(text);
    if had_errors {
        Err(format!(
            "Shift-JIS encode error: '{}' contains characters not in Shift-JIS",
            text
        ))
    } else {
        Ok(encoded.into_owned())
    }
}

/// Select encoding based on Wolf RPG version.
///
/// - v3+ → UTF-8 (native)
/// - v1/v2 → Shift-JIS decode via `encoding_rs`
pub(crate) fn decode_wolf_text(
    bytes: &[u8],
    version: &crate::engines::detector::WolfVersion,
) -> Result<String, String> {
    if version.is_utf8() {
        String::from_utf8(bytes.to_vec()).map_err(|e| format!("UTF-8 decode error: {e}"))
    } else {
        decode_shiftjis(bytes)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_shiftjis_ascii() {
        let result = decode_shiftjis(b"Hello").unwrap();
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_decode_shiftjis_japanese() {
        // "テスト" in Shift-JIS: テ=0x83 0x65, ス=0x83 0x58, ト=0x83 0x67
        let sjis_bytes: &[u8] = &[0x83, 0x65, 0x83, 0x58, 0x83, 0x67];
        let result = decode_shiftjis(sjis_bytes).unwrap();
        assert_eq!(result, "テスト");
    }

    #[test]
    fn test_encode_shiftjis_ascii() {
        let result = encode_shiftjis("Hello").unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn test_encode_shiftjis_accented() {
        // "café" — é (U+00E9) is not representable in Shift-JIS
        let result = encode_shiftjis("café");
        assert!(
            result.is_err(),
            "should fail for characters outside Shift-JIS"
        );
    }
}
