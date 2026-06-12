//! Generic flat command frame for WOLF RPG Editor v3.x event command lists.
//!
//! Port of WolfTL's `Command::Init`/`Dump` (`Command.hpp:706-770`) and
//! `RouteCommand::Init`/`Dump` (`RouteCommand.hpp:37-57`). Every command —
//! including loop/branch/choice "container" commands — is read with the same
//! generic frame; nesting is expressed via the `indent` field, not via
//! recursion. See `docs/plans/f5-01-wolf-v3-mps-parser.md` for the format
//! rationale.

use super::coder::{ByteReader, ByteWriter};
use super::V3FormatError;

/// `CommandType::Message` — `ShowMessage`, text in `string_args[0]`.
pub(crate) const CID_MESSAGE: u32 = 101;
/// `CommandType::Choices` — `ShowChoice`, choice texts in `string_args`.
pub(crate) const CID_CHOICES: u32 = 102;

const TERMINATOR_NORMAL: u8 = 0x00;
const TERMINATOR_MOVE: u8 = 0x01;
const ROUTE_COMMAND_MAGIC: [u8; 2] = [0x01, 0x00];

/// One step of a movement route (`CommandType::Move`'s embedded route, or a
/// page's "not running" route).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RouteCommand {
    pub id: u8,
    pub args: Vec<u32>,
}

impl RouteCommand {
    pub(crate) fn parse(reader: &mut ByteReader) -> Result<Self, V3FormatError> {
        let id = reader.read_u8()?;
        let arg_count = reader.read_u8()? as usize;
        let mut args = Vec::with_capacity(arg_count);
        for _ in 0..arg_count {
            args.push(reader.read_u32_le()?);
        }

        let magic_offset = reader.position();
        let m0 = reader.read_u8()?;
        let m1 = reader.read_u8()?;
        if [m0, m1] != ROUTE_COMMAND_MAGIC {
            return Err(V3FormatError::InvalidRouteMagic {
                offset: magic_offset,
                expected: ROUTE_COMMAND_MAGIC,
                found: [m0, m1],
            });
        }

        Ok(Self { id, args })
    }

    pub(crate) fn dump(&self, writer: &mut ByteWriter) {
        writer.write_u8(self.id);
        writer.write_u8(self.args.len() as u8);
        for &arg in &self.args {
            writer.write_u32_le(arg);
        }
        writer.write_bytes(&ROUTE_COMMAND_MAGIC);
    }
}

/// Extra data embedded after a `CommandType::Move` command (terminator
/// `0x01`): 5 unknown bytes, a flags byte, then an embedded movement route.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MoveData {
    pub unknown: [u8; 5],
    pub flags: u8,
    pub route: Vec<RouteCommand>,
}

/// One event command, in WolfTL's generic flat layout.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Command {
    pub cid: u32,
    pub args: Vec<u32>,
    pub indent: u8,
    pub string_args: Vec<String>,
    /// `Some` only for `CommandType::Move` (terminator `0x01`).
    pub move_data: Option<MoveData>,
    /// Opaque v3.5+ trailer (`Command::s_v35`), preserved byte-for-byte.
    /// Empty when `v35` is `false`.
    pub v35_unknown: Vec<u8>,
}

impl Command {
    /// Parses one command frame.
    ///
    /// `is_utf8` selects the string encoding (UTF-8 vs Shift-JIS, from the
    /// file's global encoding flag). `v35` selects whether a trailing
    /// `v35Unknown` blob follows (`version >= 0x67`, set once per file).
    pub(crate) fn parse(
        reader: &mut ByteReader,
        is_utf8: bool,
        v35: bool,
    ) -> Result<Self, V3FormatError> {
        let args_count_offset = reader.position();
        let args_count_byte = reader.read_u8()?;
        let args_count = args_count_byte
            .checked_sub(1)
            .ok_or(V3FormatError::InvalidArgsCount {
                offset: args_count_offset,
                found: args_count_byte,
            })? as usize;

        let cid = reader.read_u32_le()?;

        let mut args = Vec::with_capacity(args_count);
        for _ in 0..args_count {
            args.push(reader.read_u32_le()?);
        }

        let indent = reader.read_u8()?;

        let str_count = reader.read_u8()? as usize;
        let mut string_args = Vec::with_capacity(str_count);
        for _ in 0..str_count {
            string_args.push(reader.read_string(is_utf8)?);
        }

        let terminator_offset = reader.position();
        let terminator = reader.read_u8()?;
        let move_data = match terminator {
            TERMINATOR_NORMAL => None,
            TERMINATOR_MOVE => {
                let mut unknown = [0u8; 5];
                for byte in unknown.iter_mut() {
                    *byte = reader.read_u8()?;
                }
                let flags = reader.read_u8()?;

                let route_count = reader.read_u32_le()? as usize;
                let mut route = Vec::with_capacity(route_count);
                for _ in 0..route_count {
                    route.push(RouteCommand::parse(reader)?);
                }

                Some(MoveData {
                    unknown,
                    flags,
                    route,
                })
            }
            found => {
                return Err(V3FormatError::InvalidTerminator {
                    offset: terminator_offset,
                    found,
                })
            }
        };

        let v35_unknown = if v35 {
            let size = reader.read_u8()? as usize;
            reader.read_bytes(size)?.to_vec()
        } else {
            Vec::new()
        };

        Ok(Self {
            cid,
            args,
            indent,
            string_args,
            move_data,
            v35_unknown,
        })
    }

    /// Writes this command frame — the inverse of [`Command::parse`].
    pub(crate) fn dump(
        &self,
        writer: &mut ByteWriter,
        is_utf8: bool,
        v35: bool,
    ) -> Result<(), V3FormatError> {
        writer.write_u8((self.args.len() + 1) as u8);
        writer.write_u32_le(self.cid);
        for &arg in &self.args {
            writer.write_u32_le(arg);
        }

        writer.write_u8(self.indent);
        writer.write_u8(self.string_args.len() as u8);
        for s in &self.string_args {
            writer.write_string(s, is_utf8)?;
        }

        match &self.move_data {
            None => writer.write_u8(TERMINATOR_NORMAL),
            Some(move_data) => {
                writer.write_u8(TERMINATOR_MOVE);
                writer.write_bytes(&move_data.unknown);
                writer.write_u8(move_data.flags);
                writer.write_u32_le(move_data.route.len() as u32);
                for rc in &move_data.route {
                    rc.dump(writer);
                }
            }
        }

        if v35 {
            writer.write_u8(self.v35_unknown.len() as u8);
            writer.write_bytes(&self.v35_unknown);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(bytes: &[u8], is_utf8: bool, v35: bool) -> Command {
        let mut reader = ByteReader::new(bytes);
        let command = Command::parse(&mut reader, is_utf8, v35).unwrap();
        assert_eq!(reader.remaining(), 0, "parser should consume all bytes");

        let mut writer = ByteWriter::new();
        command.dump(&mut writer, is_utf8, v35).unwrap();
        assert_eq!(writer.into_bytes(), bytes, "dump should reproduce input");

        command
    }

    #[test]
    fn test_message_no_v35() {
        let mut w = ByteWriter::new();
        w.write_u8(1); // argsCount byte = 0 args + 1
        w.write_u32_le(CID_MESSAGE);
        w.write_u8(0); // indent
        w.write_u8(1); // strCount
        w.write_string("Hello", true).unwrap();
        w.write_u8(0x00); // terminator
        let bytes = w.into_bytes();

        let command = round_trip(&bytes, true, false);
        assert_eq!(command.cid, CID_MESSAGE);
        assert_eq!(command.args, Vec::<u32>::new());
        assert_eq!(command.indent, 0);
        assert_eq!(command.string_args, vec!["Hello".to_owned()]);
        assert!(command.move_data.is_none());
        assert!(command.v35_unknown.is_empty());
    }

    #[test]
    fn test_message_with_v35_unknown() {
        let mut w = ByteWriter::new();
        w.write_u8(1);
        w.write_u32_le(CID_MESSAGE);
        w.write_u8(2); // indent
        w.write_u8(1);
        w.write_string("こんにちは", true).unwrap();
        w.write_u8(0x00);
        // v35Unknown: 3 opaque bytes
        w.write_u8(3);
        w.write_bytes(&[0xAA, 0xBB, 0xCC]);
        let bytes = w.into_bytes();

        let command = round_trip(&bytes, true, true);
        assert_eq!(command.cid, CID_MESSAGE);
        assert_eq!(command.indent, 2);
        assert_eq!(command.v35_unknown, vec![0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn test_choices_multiple_string_args() {
        let mut w = ByteWriter::new();
        w.write_u8(2); // argsCount byte = 1 arg + 1
        w.write_u32_le(CID_CHOICES);
        w.write_u32_le(3); // args[0] = choice count
        w.write_u8(0); // indent
        w.write_u8(3); // strCount
        w.write_string("はい", true).unwrap();
        w.write_string("いいえ", true).unwrap();
        w.write_string("キャンセル", true).unwrap();
        w.write_u8(0x00);
        w.write_u8(0); // v35Unknown size = 0
        let bytes = w.into_bytes();

        let command = round_trip(&bytes, true, true);
        assert_eq!(command.cid, CID_CHOICES);
        assert_eq!(command.args, vec![3]);
        assert_eq!(
            command.string_args,
            vec![
                "はい".to_owned(),
                "いいえ".to_owned(),
                "キャンセル".to_owned()
            ]
        );
        assert!(command.v35_unknown.is_empty());
    }

    #[test]
    fn test_move_command_with_route() {
        let mut w = ByteWriter::new();
        w.write_u8(1); // argsCount byte = 0 args + 1
        w.write_u32_le(201); // CommandType::Move
        w.write_u8(0); // indent
        w.write_u8(0); // strCount
        w.write_u8(0x01); // terminator = Move
        w.write_bytes(&[0x01, 0x02, 0x03, 0x04, 0x05]); // 5 unknown bytes
        w.write_u8(0xFF); // flags
        w.write_u32_le(2); // routeCount
                           // RouteCommand 1: id=0x10, no args
        w.write_u8(0x10);
        w.write_u8(0);
        w.write_bytes(&ROUTE_COMMAND_MAGIC);
        // RouteCommand 2: id=0x20, 1 arg
        w.write_u8(0x20);
        w.write_u8(1);
        w.write_u32_le(42);
        w.write_bytes(&ROUTE_COMMAND_MAGIC);
        let bytes = w.into_bytes();

        let command = round_trip(&bytes, true, false);
        assert_eq!(command.cid, 201);
        let move_data = command.move_data.unwrap();
        assert_eq!(move_data.unknown, [0x01, 0x02, 0x03, 0x04, 0x05]);
        assert_eq!(move_data.flags, 0xFF);
        assert_eq!(
            move_data.route,
            vec![
                RouteCommand {
                    id: 0x10,
                    args: vec![]
                },
                RouteCommand {
                    id: 0x20,
                    args: vec![42]
                },
            ]
        );
    }

    #[test]
    fn test_invalid_terminator_errors() {
        let mut w = ByteWriter::new();
        w.write_u8(1);
        w.write_u32_le(CID_MESSAGE);
        w.write_u8(0);
        w.write_u8(0);
        w.write_u8(0x42); // invalid terminator
        let bytes = w.into_bytes();

        let mut reader = ByteReader::new(&bytes);
        let err = Command::parse(&mut reader, true, false).unwrap_err();
        assert!(matches!(
            err,
            V3FormatError::InvalidTerminator { found: 0x42, .. }
        ));
    }

    #[test]
    fn test_invalid_route_magic_errors() {
        let mut w = ByteWriter::new();
        w.write_u8(0x10);
        w.write_u8(0);
        w.write_bytes(&[0xFF, 0xFF]); // wrong magic
        let bytes = w.into_bytes();

        let mut reader = ByteReader::new(&bytes);
        let err = RouteCommand::parse(&mut reader).unwrap_err();
        assert!(matches!(err, V3FormatError::InvalidRouteMagic { .. }));
    }
}
