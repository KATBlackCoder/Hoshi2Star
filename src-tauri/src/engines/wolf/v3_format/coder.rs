//! Generic byte-level reader/writer for WOLF RPG Editor v3.x binary formats.
//!
//! All integers are little-endian `u32`/`u8`. Strings are length-prefixed
//! (`u32` byte count, including a trailing `0x00` terminator) and encoded as
//! either UTF-8 or Shift-JIS depending on the file's global encoding flag
//! (`is_utf8`, derived from the `.mps`/`.dat` header byte 16).

use super::V3FormatError;
use encoding_rs::SHIFT_JIS;

/// Reads primitives sequentially from a byte slice, tracking position for
/// error reporting.
pub(crate) struct ByteReader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> ByteReader<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    pub(crate) fn position(&self) -> usize {
        self.pos
    }

    pub(crate) fn remaining(&self) -> usize {
        self.bytes.len() - self.pos
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], V3FormatError> {
        if self.remaining() < n {
            return Err(V3FormatError::UnexpectedEof {
                offset: self.pos,
                needed: n,
                available: self.remaining(),
            });
        }
        let slice = &self.bytes[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    pub(crate) fn read_u8(&mut self) -> Result<u8, V3FormatError> {
        Ok(self.take(1)?[0])
    }

    pub(crate) fn read_u32_le(&mut self) -> Result<u32, V3FormatError> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
    }

    pub(crate) fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], V3FormatError> {
        self.take(n)
    }

    /// Reads a length-prefixed string: `u32` byte count (including a
    /// trailing `0x00` terminator), then that many bytes, decoded as UTF-8 or
    /// Shift-JIS depending on `is_utf8`.
    pub(crate) fn read_string(&mut self, is_utf8: bool) -> Result<String, V3FormatError> {
        let offset = self.pos;
        let size = self.read_u32_le()? as usize;
        if size == 0 {
            return Err(V3FormatError::ZeroLengthString { offset });
        }
        let data = self.read_bytes(size)?;
        // Both encodings null-terminate the on-disk bytes; strip it before decoding.
        let body = if data.last() == Some(&0x00) {
            &data[..data.len() - 1]
        } else {
            data
        };

        if is_utf8 {
            std::str::from_utf8(body).map(str::to_owned).map_err(|e| {
                V3FormatError::InvalidEncoding {
                    offset,
                    message: format!("invalid UTF-8: {e}"),
                }
            })
        } else {
            let (decoded, _, had_errors) = SHIFT_JIS.decode(body);
            if had_errors {
                Err(V3FormatError::InvalidEncoding {
                    offset,
                    message: "invalid Shift-JIS".to_owned(),
                })
            } else {
                Ok(decoded.into_owned())
            }
        }
    }
}

/// Accumulates bytes written sequentially, mirroring `ByteReader`.
pub(crate) struct ByteWriter {
    buf: Vec<u8>,
}

impl ByteWriter {
    pub(crate) fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub(crate) fn write_u8(&mut self, value: u8) {
        self.buf.push(value);
    }

    pub(crate) fn write_u32_le(&mut self, value: u32) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    pub(crate) fn write_bytes(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// Writes a length-prefixed string: encodes `value` as UTF-8 or
    /// Shift-JIS (per `is_utf8`), appends a `0x00` terminator, then writes the
    /// `u32` byte count followed by the encoded bytes — the inverse of
    /// [`ByteReader::read_string`].
    pub(crate) fn write_string(&mut self, value: &str, is_utf8: bool) -> Result<(), V3FormatError> {
        let mut encoded = if is_utf8 {
            value.as_bytes().to_vec()
        } else {
            let (encoded, _, had_errors) = SHIFT_JIS.encode(value);
            if had_errors {
                return Err(V3FormatError::InvalidEncoding {
                    offset: self.buf.len(),
                    message: format!("'{value}' contains characters not in Shift-JIS"),
                });
            }
            encoded.into_owned()
        };
        encoded.push(0x00);

        self.write_u32_le(encoded.len() as u32);
        self.write_bytes(&encoded);
        Ok(())
    }

    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_u8_u32_le() {
        let mut r = ByteReader::new(&[0x01, 0x02, 0x00, 0x00, 0x00]);
        assert_eq!(r.read_u8().unwrap(), 0x01);
        assert_eq!(r.read_u32_le().unwrap(), 0x02);
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_read_unexpected_eof() {
        let mut r = ByteReader::new(&[0x01]);
        let err = r.read_u32_le().unwrap_err();
        assert_eq!(
            err,
            V3FormatError::UnexpectedEof {
                offset: 0,
                needed: 4,
                available: 1,
            }
        );
    }

    #[test]
    fn test_string_round_trip_utf8() {
        let mut w = ByteWriter::new();
        w.write_string("こんにちは", true).unwrap();
        let bytes = w.into_bytes();

        let mut r = ByteReader::new(&bytes);
        let s = r.read_string(true).unwrap();
        assert_eq!(s, "こんにちは");
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_string_round_trip_shift_jis() {
        let mut w = ByteWriter::new();
        w.write_string("テスト", false).unwrap();
        let bytes = w.into_bytes();

        let mut r = ByteReader::new(&bytes);
        let s = r.read_string(false).unwrap();
        assert_eq!(s, "テスト");
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_empty_string_round_trip() {
        let mut w = ByteWriter::new();
        w.write_string("", true).unwrap();
        let bytes = w.into_bytes();
        // size = 1 (just the 0x00 terminator)
        assert_eq!(bytes, vec![0x01, 0x00, 0x00, 0x00, 0x00]);

        let mut r = ByteReader::new(&bytes);
        assert_eq!(r.read_string(true).unwrap(), "");
    }

    #[test]
    fn test_zero_length_string_errors() {
        let mut r = ByteReader::new(&[0x00, 0x00, 0x00, 0x00]);
        let err = r.read_string(true).unwrap_err();
        assert_eq!(err, V3FormatError::ZeroLengthString { offset: 0 });
    }

    #[test]
    fn test_shift_jis_encode_error_for_unsupported_chars() {
        let mut w = ByteWriter::new();
        // é (U+00E9) is not representable in Shift-JIS.
        let err = w.write_string("café", false).unwrap_err();
        assert!(matches!(err, V3FormatError::InvalidEncoding { .. }));
    }
}
