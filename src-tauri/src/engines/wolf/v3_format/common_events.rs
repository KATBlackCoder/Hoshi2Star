//! `CommonEvent.dat` structures for WOLF RPG Editor v3.x (v3.5+, Inko).
//!
//! Port of WolfTL's `CommonEvents::load`/`dump` and `CommonEvent::init`/`Dump`
//! (`CommonEvents.hpp:203-462`). Reuses [`super::command::Command`] for each
//! event's flat command list, exactly like [`super::map::PageV3`].
//!
//! On-disk layout: `leadingByte(0x00) + magic[9] + version: u8`, where
//! `magic[5]` is the UTF-8 flag (`0x55`) / Shift-JIS flag (`0x00`), followed
//! — for `version` `0x93`/`0xCC` (v3.5, the only case Inko uses) — by
//! `decSize: u32 + encSize: u32 + <LZ4 block>`. [`is_lz4_v3`] detects this
//! header; [`decompress`]/[`recompress`] wrap [`super::compression`]'s
//! generic block (de)compression with this format's 11-byte header.
//! [`CommonEventsV3::parse`]/[`dump`](CommonEventsV3::dump) operate on the
//! resulting `header[0..11] ++ payload` buffer.

use super::coder::{ByteReader, ByteWriter};
use super::command::Command;
use super::V3FormatError;

/// Leading byte written by `FileCoder`'s constructor for non-`Project`/
/// non-`Map` file types (`FileCoder.hpp:136-137`).
const LEADING_BYTE: u8 = 0x00;
/// `CommonEvents::MAGIC_NUMBER` (`CommonEvents.hpp:461`); index 5 is the
/// UTF-8 flag (`0x55`) / Shift-JIS flag (`0x00`).
const MAGIC: [u8; 9] = [0x57, 0x00, 0x00, 0x4F, 0x4C, 0x00, 0x46, 0x43, 0x00];
/// Index of the UTF-8 flag within [`MAGIC`].
const UTF8_MAGIC_INDEX: usize = 5;
/// `leadingByte(1) + magic(9) + version(1)`.
const HEADER_SIZE: usize = 11;
/// `m_version` values that select v3.5 mode (`Command::s_v35` + LZ4 `Unpack`).
const V35_VERSIONS: [u8; 2] = [0x93, 0xCC];

/// `CommonEvent::init`'s header indicator (`CommonEvents.hpp:206`).
const EVENT_INDICATOR: u8 = 0x8E;
/// `CommonEvent::init`'s post-commands data indicator (`CommonEvents.hpp:230`).
const DATA_INDICATOR: u8 = 0x8F;
/// `CommonEvent::init`'s `unknown9` indicator, also the "no `unknown10`"
/// terminator (`CommonEvents.hpp:262`/`270`).
const UNKNOWN9_INDICATOR: u8 = 0x91;
/// `CommonEvent::init`'s `unknown10` block indicator/terminator
/// (`CommonEvents.hpp:268`/`280`).
const UNKNOWN10_INDICATOR: u8 = 0x92;
/// `CommonEvent::m_unknown2` size (`CommonEvents.hpp:212`).
const UNKNOWN2_LEN: usize = 7;
/// `CommonEvent::m_unknown7` size (`CommonEvents.hpp:257`).
const UNKNOWN7_LEN: usize = 0x1D;
/// `CommonEvent::m_unknown8` is a fixed-size array, not size-prefixed
/// (`CommonEvents.hpp:302`).
const UNKNOWN8_LEN: usize = 100;
/// `CommonEvents::load`'s terminator must be `>= 0x89` (`CommonEvents.hpp:402`).
const TERMINATOR_MIN: u8 = 0x89;

/// Returns `true` if `bytes` looks like a v3.5 `CommonEvent.dat`: the
/// 1-byte leading `0x00`, the 9-byte `MAGIC` (with index 5 being `0x00` or
/// `0x55`), and a `version` byte of `0x93` or `0xCC`.
pub(crate) fn is_lz4_v3(bytes: &[u8]) -> bool {
    bytes.len() >= HEADER_SIZE + 8
        && bytes[0] == LEADING_BYTE
        && bytes[1] == MAGIC[0]
        && bytes[2] == MAGIC[1]
        && bytes[3] == MAGIC[2]
        && bytes[4] == MAGIC[3]
        && bytes[5] == MAGIC[4]
        && matches!(bytes[1 + UTF8_MAGIC_INDEX], 0x00 | 0x55)
        && bytes[7] == MAGIC[6]
        && bytes[8] == MAGIC[7]
        && bytes[9] == MAGIC[8]
        && V35_VERSIONS.contains(&bytes[10])
}

/// Decompresses a v3.5 `CommonEvent.dat` into `header[0..11] ++ payload`,
/// where `payload` starts with `eventCnt: u32`. The returned buffer is what
/// [`CommonEventsV3::parse`] expects.
pub(crate) fn decompress(bytes: &[u8]) -> Result<Vec<u8>, V3FormatError> {
    super::compression::decompress_block(bytes, HEADER_SIZE)
}

/// Recompresses a `header[0..11] ++ payload` buffer (as produced by
/// [`decompress`] / [`CommonEventsV3::dump`]) back into the on-disk layout.
// CommonEvent.dat injection is not yet wired (extraction-only, matching v2.x
// parity); reserved for a future injector.
#[allow(dead_code)]
pub(crate) fn recompress(decompressed: &[u8]) -> Result<Vec<u8>, V3FormatError> {
    super::compression::recompress_block(decompressed, HEADER_SIZE)
}

/// One common event (`CommonEvent::init`/`Dump`, `CommonEvents.hpp:203-308`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CommonEventV3 {
    pub int_id: u32,
    pub unknown1: u32,
    pub unknown2: [u8; UNKNOWN2_LEN],
    pub name: String,
    pub commands: Vec<Command>,
    pub unknown11: String,
    pub description: String,
    pub unknown3: Vec<String>,
    pub unknown4: Vec<u8>,
    pub unknown5: Vec<Vec<String>>,
    pub unknown6: Vec<Vec<u32>>,
    /// Opaque, preserved byte-for-byte (`UNKNOWN7_LEN` bytes).
    pub unknown7: Vec<u8>,
    /// Always exactly `UNKNOWN8_LEN` strings (fixed-size array, not
    /// size-prefixed).
    pub unknown8: Vec<String>,
    pub unknown9: String,
    /// `Some((unknown10, unknown12))` if the optional `0x92` block is present.
    pub unknown10: Option<(String, u32)>,
}

impl CommonEventV3 {
    pub(crate) fn parse(
        reader: &mut ByteReader,
        is_utf8: bool,
        v35: bool,
    ) -> Result<Self, V3FormatError> {
        let indicator_offset = reader.position();
        let indicator = reader.read_u8()?;
        if indicator != EVENT_INDICATOR {
            return Err(V3FormatError::UnexpectedByte {
                offset: indicator_offset,
                expected: EVENT_INDICATOR,
                found: indicator,
                context: "common event header indicator",
            });
        }

        let int_id = reader.read_u32_le()?;
        let unknown1 = reader.read_u32_le()?;
        let mut unknown2 = [0u8; UNKNOWN2_LEN];
        unknown2.copy_from_slice(reader.read_bytes(UNKNOWN2_LEN)?);
        let name = reader.read_string(is_utf8)?;

        let command_count = reader.read_u32_le()? as usize;
        let mut commands = Vec::with_capacity(command_count);
        for _ in 0..command_count {
            commands.push(Command::parse(reader, is_utf8, v35)?);
        }

        let unknown11 = reader.read_string(is_utf8)?;
        let description = reader.read_string(is_utf8)?;

        let indicator_offset = reader.position();
        let indicator = reader.read_u8()?;
        if indicator != DATA_INDICATOR {
            return Err(V3FormatError::UnexpectedByte {
                offset: indicator_offset,
                expected: DATA_INDICATOR,
                found: indicator,
                context: "common event data indicator",
            });
        }

        let unknown3_count = reader.read_u32_le()? as usize;
        let mut unknown3 = Vec::with_capacity(unknown3_count);
        for _ in 0..unknown3_count {
            unknown3.push(reader.read_string(is_utf8)?);
        }

        let unknown4_count = reader.read_u32_le()? as usize;
        let mut unknown4 = Vec::with_capacity(unknown4_count);
        for _ in 0..unknown4_count {
            unknown4.push(reader.read_u8()?);
        }

        let unknown5_count = reader.read_u32_le()? as usize;
        let mut unknown5 = Vec::with_capacity(unknown5_count);
        for _ in 0..unknown5_count {
            let n = reader.read_u32_le()? as usize;
            let mut strs = Vec::with_capacity(n);
            for _ in 0..n {
                strs.push(reader.read_string(is_utf8)?);
            }
            unknown5.push(strs);
        }

        let unknown6_count = reader.read_u32_le()? as usize;
        let mut unknown6 = Vec::with_capacity(unknown6_count);
        for _ in 0..unknown6_count {
            let n = reader.read_u32_le()? as usize;
            let mut uints = Vec::with_capacity(n);
            for _ in 0..n {
                uints.push(reader.read_u32_le()?);
            }
            unknown6.push(uints);
        }

        let unknown7 = reader.read_bytes(UNKNOWN7_LEN)?.to_vec();

        let mut unknown8 = Vec::with_capacity(UNKNOWN8_LEN);
        for _ in 0..UNKNOWN8_LEN {
            unknown8.push(reader.read_string(is_utf8)?);
        }

        let indicator_offset = reader.position();
        let indicator = reader.read_u8()?;
        if indicator != UNKNOWN9_INDICATOR {
            return Err(V3FormatError::UnexpectedByte {
                offset: indicator_offset,
                expected: UNKNOWN9_INDICATOR,
                found: indicator,
                context: "common event unknown9 indicator",
            });
        }
        let unknown9 = reader.read_string(is_utf8)?;

        let indicator_offset = reader.position();
        let indicator = reader.read_u8()?;
        let unknown10 = if indicator == UNKNOWN10_INDICATOR {
            let unknown10 = reader.read_string(is_utf8)?;
            let unknown12 = reader.read_u32_le()?;

            let trailer_offset = reader.position();
            let trailer = reader.read_u8()?;
            if trailer != UNKNOWN10_INDICATOR {
                return Err(V3FormatError::UnexpectedByte {
                    offset: trailer_offset,
                    expected: UNKNOWN10_INDICATOR,
                    found: trailer,
                    context: "common event unknown10 trailer",
                });
            }

            Some((unknown10, unknown12))
        } else if indicator == UNKNOWN9_INDICATOR {
            None
        } else {
            return Err(V3FormatError::UnexpectedByte {
                offset: indicator_offset,
                expected: UNKNOWN10_INDICATOR,
                found: indicator,
                context: "common event unknown9/unknown10 indicator (expected 0x91 or 0x92)",
            });
        };

        Ok(Self {
            int_id,
            unknown1,
            unknown2,
            name,
            commands,
            unknown11,
            description,
            unknown3,
            unknown4,
            unknown5,
            unknown6,
            unknown7,
            unknown8,
            unknown9,
            unknown10,
        })
    }

    /// Reserved for a future injector (see [`recompress`]).
    #[allow(dead_code)]
    pub(crate) fn dump(
        &self,
        writer: &mut ByteWriter,
        is_utf8: bool,
        v35: bool,
    ) -> Result<(), V3FormatError> {
        writer.write_u8(EVENT_INDICATOR);
        writer.write_u32_le(self.int_id);
        writer.write_u32_le(self.unknown1);
        writer.write_bytes(&self.unknown2);
        writer.write_string(&self.name, is_utf8)?;

        writer.write_u32_le(self.commands.len() as u32);
        for cmd in &self.commands {
            cmd.dump(writer, is_utf8, v35)?;
        }

        writer.write_string(&self.unknown11, is_utf8)?;
        writer.write_string(&self.description, is_utf8)?;
        writer.write_u8(DATA_INDICATOR);

        writer.write_u32_le(self.unknown3.len() as u32);
        for s in &self.unknown3 {
            writer.write_string(s, is_utf8)?;
        }

        writer.write_u32_le(self.unknown4.len() as u32);
        for &byte in &self.unknown4 {
            writer.write_u8(byte);
        }

        writer.write_u32_le(self.unknown5.len() as u32);
        for strs in &self.unknown5 {
            writer.write_u32_le(strs.len() as u32);
            for s in strs {
                writer.write_string(s, is_utf8)?;
            }
        }

        writer.write_u32_le(self.unknown6.len() as u32);
        for uints in &self.unknown6 {
            writer.write_u32_le(uints.len() as u32);
            for &u in uints {
                writer.write_u32_le(u);
            }
        }

        writer.write_bytes(&self.unknown7);
        for s in &self.unknown8 {
            writer.write_string(s, is_utf8)?;
        }

        writer.write_u8(UNKNOWN9_INDICATOR);
        writer.write_string(&self.unknown9, is_utf8)?;

        match &self.unknown10 {
            Some((unknown10, unknown12)) => {
                writer.write_u8(UNKNOWN10_INDICATOR);
                writer.write_string(unknown10, is_utf8)?;
                writer.write_u32_le(*unknown12);
                writer.write_u8(UNKNOWN10_INDICATOR);
            }
            None => writer.write_u8(UNKNOWN9_INDICATOR),
        }

        Ok(())
    }
}

/// `CommonEvent.dat` (`CommonEvents::load`/`dump`, `CommonEvents.hpp:382-441`).
///
/// [`CommonEventsV3::parse`]/[`dump`](CommonEventsV3::dump) operate on the
/// full `header[0..11] ++ payload` buffer produced by [`decompress`] —
/// `magic` and `version` are the on-disk uncompressed header; everything
/// else is the LZ4-compressed payload.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CommonEventsV3 {
    /// 9-byte file magic; `magic[5]` is the UTF-8 flag (`0x55`) / Shift-JIS
    /// flag (`0x00`).
    pub magic: [u8; 9],
    /// `0x93` or `0xCC` for v3.5 (the only case [`is_lz4_v3`] accepts).
    pub version: u8,
    pub events: Vec<CommonEventV3>,
    /// Must be `>= TERMINATOR_MIN` (`0x89`).
    pub terminator: u8,
}

impl CommonEventsV3 {
    /// Reserved for a future injector (see [`recompress`]).
    #[allow(dead_code)]
    pub(crate) fn is_utf8(&self) -> bool {
        self.magic[UTF8_MAGIC_INDEX] == 0x55
    }

    /// Reserved for a future injector (see [`recompress`]).
    #[allow(dead_code)]
    pub(crate) fn is_v35(&self) -> bool {
        V35_VERSIONS.contains(&self.version)
    }

    /// Parses `header[0..11] ++ payload` (the output of [`decompress`]).
    pub(crate) fn parse(bytes: &[u8]) -> Result<Self, V3FormatError> {
        let mut reader = ByteReader::new(bytes);

        let leading_offset = reader.position();
        let leading = reader.read_u8()?;
        if leading != LEADING_BYTE {
            return Err(V3FormatError::UnexpectedByte {
                offset: leading_offset,
                expected: LEADING_BYTE,
                found: leading,
                context: "common events leading byte",
            });
        }

        let mut magic = [0u8; 9];
        magic.copy_from_slice(reader.read_bytes(9)?);
        let is_utf8 = magic[UTF8_MAGIC_INDEX] == 0x55;

        let version = reader.read_u8()?;
        let v35 = V35_VERSIONS.contains(&version);

        let event_count = reader.read_u32_le()?;
        let mut events = Vec::with_capacity(event_count as usize);
        for _ in 0..event_count {
            events.push(CommonEventV3::parse(&mut reader, is_utf8, v35)?);
        }

        let terminator_offset = reader.position();
        let terminator = reader.read_u8()?;
        if terminator < TERMINATOR_MIN {
            return Err(V3FormatError::UnexpectedByte {
                offset: terminator_offset,
                expected: TERMINATOR_MIN,
                found: terminator,
                context: "common events terminator (must be >= 0x89)",
            });
        }

        if reader.remaining() != 0 {
            return Err(V3FormatError::TrailingData {
                offset: reader.position(),
                remaining: reader.remaining(),
            });
        }

        Ok(Self {
            magic,
            version,
            events,
            terminator,
        })
    }

    /// Writes `header[0..11] ++ payload` — the inverse of
    /// [`CommonEventsV3::parse`], ready for [`recompress`].
    /// Reserved for a future injector (see [`recompress`]).
    #[allow(dead_code)]
    pub(crate) fn dump(&self) -> Result<Vec<u8>, V3FormatError> {
        let is_utf8 = self.is_utf8();
        let v35 = self.is_v35();

        let mut writer = ByteWriter::new();
        writer.write_u8(LEADING_BYTE);
        writer.write_bytes(&self.magic);
        writer.write_u8(self.version);

        writer.write_u32_le(self.events.len() as u32);
        for event in &self.events {
            event.dump(&mut writer, is_utf8, v35)?;
        }

        writer.write_u8(self.terminator);

        Ok(writer.into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_event() -> CommonEventV3 {
        CommonEventV3 {
            int_id: 1,
            unknown1: 0,
            unknown2: [0u8; UNKNOWN2_LEN],
            name: "Ev001".to_owned(),
            commands: Vec::new(),
            unknown11: String::new(),
            description: String::new(),
            unknown3: Vec::new(),
            unknown4: Vec::new(),
            unknown5: Vec::new(),
            unknown6: Vec::new(),
            unknown7: vec![0u8; UNKNOWN7_LEN],
            unknown8: vec![String::new(); UNKNOWN8_LEN],
            unknown9: String::new(),
            unknown10: None,
        }
    }

    #[test]
    fn test_event_round_trip_no_unknown10() {
        let event = empty_event();

        let mut writer = ByteWriter::new();
        event.dump(&mut writer, true, false).unwrap();
        let bytes = writer.into_bytes();

        let mut reader = ByteReader::new(&bytes);
        let parsed = CommonEventV3::parse(&mut reader, true, false).unwrap();
        assert_eq!(reader.remaining(), 0);
        assert_eq!(parsed, event);

        let mut writer2 = ByteWriter::new();
        parsed.dump(&mut writer2, true, false).unwrap();
        assert_eq!(writer2.into_bytes(), bytes);
    }

    #[test]
    fn test_event_round_trip_with_unknown10_and_command() {
        let mut event = empty_event();
        event.unknown10 = Some(("extra".to_owned(), 42));
        event.commands.push(Command {
            cid: super::super::command::CID_MESSAGE,
            args: Vec::new(),
            indent: 0,
            string_args: vec!["こんにちは".to_owned()],
            move_data: None,
            v35_unknown: vec![0xAB],
        });
        event.unknown3 = vec!["a".to_owned(), "b".to_owned()];
        event.unknown4 = vec![1, 2, 3];
        event.unknown5 = vec![vec!["x".to_owned(), "y".to_owned()], vec![]];
        event.unknown6 = vec![vec![1, 2], vec![3]];

        let mut writer = ByteWriter::new();
        event.dump(&mut writer, true, true).unwrap();
        let bytes = writer.into_bytes();

        let mut reader = ByteReader::new(&bytes);
        let parsed = CommonEventV3::parse(&mut reader, true, true).unwrap();
        assert_eq!(reader.remaining(), 0);
        assert_eq!(parsed, event);

        let mut writer2 = ByteWriter::new();
        parsed.dump(&mut writer2, true, true).unwrap();
        assert_eq!(writer2.into_bytes(), bytes);
    }

    #[test]
    fn test_event_invalid_header_indicator_errors() {
        let mut writer = ByteWriter::new();
        writer.write_u8(0x00); // wrong indicator
        let bytes = writer.into_bytes();

        let mut reader = ByteReader::new(&bytes);
        let err = CommonEventV3::parse(&mut reader, true, false).unwrap_err();
        assert!(matches!(
            err,
            V3FormatError::UnexpectedByte {
                expected: EVENT_INDICATOR,
                found: 0x00,
                ..
            }
        ));
    }

    fn empty_common_events(is_utf8: bool, version: u8) -> CommonEventsV3 {
        let mut magic = MAGIC;
        magic[UTF8_MAGIC_INDEX] = if is_utf8 { 0x55 } else { 0x00 };
        CommonEventsV3 {
            magic,
            version,
            events: Vec::new(),
            terminator: 0x89,
        }
    }

    #[test]
    fn test_common_events_round_trip_no_events() {
        let ce = empty_common_events(true, 0x93);
        let bytes = ce.dump().unwrap();

        let parsed = CommonEventsV3::parse(&bytes).unwrap();
        assert_eq!(parsed, ce);
        assert_eq!(parsed.dump().unwrap(), bytes);
    }

    #[test]
    fn test_common_events_round_trip_with_event() {
        let mut ce = empty_common_events(true, 0xCC);
        ce.events.push(empty_event());

        let bytes = ce.dump().unwrap();
        let parsed = CommonEventsV3::parse(&bytes).unwrap();
        assert_eq!(parsed, ce);
        assert_eq!(parsed.dump().unwrap(), bytes);
    }

    #[test]
    fn test_common_events_terminator_too_small_errors() {
        let ce = empty_common_events(true, 0x93);
        let mut bytes = ce.dump().unwrap();
        let last = bytes.len() - 1;
        bytes[last] = 0x10; // < TERMINATOR_MIN

        let err = CommonEventsV3::parse(&bytes).unwrap_err();
        assert!(matches!(
            err,
            V3FormatError::UnexpectedByte {
                expected: TERMINATOR_MIN,
                found: 0x10,
                ..
            }
        ));
    }

    #[test]
    fn test_is_lz4_v3_detects_real_inko_header() {
        // Header bytes from the real Inko CommonEvent.dat (verified via xxd):
        // 00 57 00 00 4f 4c 55 46 43 00 93 ...
        let mut bytes = vec![
            0x00, 0x57, 0x00, 0x00, 0x4F, 0x4C, 0x55, 0x46, 0x43, 0x00, 0x93,
        ];
        bytes.extend_from_slice(&[0u8; 8]);
        assert!(is_lz4_v3(&bytes));
    }

    #[test]
    fn test_is_lz4_v3_rejects_non_v35_version() {
        let mut bytes = vec![
            0x00, 0x57, 0x00, 0x00, 0x4F, 0x4C, 0x00, 0x46, 0x43, 0x00, 0x10,
        ];
        bytes.extend_from_slice(&[0u8; 8]);
        assert!(!is_lz4_v3(&bytes));
    }

    /// Decision check (plan `docs/plans/f5-01-wolf-v3-mps-parser.md`, Phase 7):
    /// `CommonEventsV3::parse(decompress(bytes)).dump() ==
    /// decompress(bytes)` must hold byte-exact for the real Inko
    /// `CommonEvent.dat`.
    #[test]
    fn test_real_inko_common_events_v3_round_trip() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../test/Densyanai_Inko_ver2.0/Data/BasicData/CommonEvent.dat");
        let raw = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));

        assert!(
            is_lz4_v3(&raw),
            "expected v3.5 LZ4-wrapped CommonEvent.dat header"
        );

        let decompressed = decompress(&raw).unwrap_or_else(|e| panic!("decompress failed: {e}"));

        let common_events = CommonEventsV3::parse(&decompressed)
            .unwrap_or_else(|e| panic!("CommonEventsV3::parse failed: {e}"));

        let dumped = common_events
            .dump()
            .unwrap_or_else(|e| panic!("CommonEventsV3::dump failed: {e}"));

        assert_eq!(
            dumped, decompressed,
            "dump(parse(bytes)) != bytes (byte-exact round-trip failed)"
        );

        eprintln!(
            "Inko CommonEvent.dat → {} events (version {:#04x})",
            common_events.events.len(),
            common_events.version
        );
    }
}
