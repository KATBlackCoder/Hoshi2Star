//! `.mps` map structures for WOLF RPG Editor v3.x.
//!
//! Port of WolfTL's `Map::load`/`dump` (`Map.hpp:496-604`), `Event::Init`/
//! `Dump` (`Map.hpp:315-367`) and `Page::Init`/`Dump` (`Map.hpp:44-125`).
//!
//! [`MapV3::parse`] takes the buffer produced by
//! [`crate::engines::wolf::v3_format::compression::decompress_v3`] (`header[0..25] ++ payload`) and
//! [`MapV3::dump`] produces the inverse, ready for
//! [`super::compression::recompress_v3`].

use super::coder::{ByteReader, ByteWriter};
use super::command::{Command, RouteCommand};
use super::V3FormatError;

/// `Page::m_conditions` size: `1 + 4 + 4*4 + 4*4` opaque bytes.
const CONDITIONS_LEN: usize = 1 + 4 + 4 * 4 + 4 * 4;
/// `Page::m_movement` size: 4 opaque bytes.
const MOVEMENT_LEN: usize = 4;
/// `Page::Dump`'s trailing terminator.
const PAGE_TERMINATOR: u8 = 0x7A;

/// `Event::MAGIC_NUMBER1` (`Map.hpp:449`).
const EVENT_MAGIC1: [u8; 4] = [0x39, 0x30, 0x00, 0x00];
/// `Event::MAGIC_NUMBER2` (`Map.hpp:450`).
const EVENT_MAGIC2: [u8; 4] = [0x00, 0x00, 0x00, 0x00];
/// Byte preceding each `Page::Init` call inside `Event::Init`'s page loop.
const PAGE_INDICATOR: u8 = 0x79;
/// `Event::Init`'s page-loop terminator.
const EVENT_TERMINATOR: u8 = 0x70;

/// Byte preceding each `Event::Init` call inside `Map::load`'s event loop.
const EVENT_INDICATOR: u8 = 0x6F;
/// `Map::load`'s event-loop terminator.
const MAP_TERMINATOR: u8 = 0x66;

/// `Map::load`'s "version >= this means v3.5+" threshold (`Command::s_v35`).
const VERSION_V35: u32 = 0x67;

/// "No tile data" marker for UTF-8 maps: `ReadInt() == -1` (`Map.hpp:520-525`).
const NO_TILES_MARKER: u32 = 0xFFFF_FFFF;

/// One event page (`Page::Init`/`Dump`, `Map.hpp:44-125`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PageV3 {
    pub unknown1: u32,
    pub graphic_name: String,
    pub graphic_direction: u8,
    pub graphic_frame: u8,
    pub graphic_opacity: u8,
    pub graphic_render_mode: u8,
    /// Opaque, preserved byte-for-byte (`CONDITIONS_LEN` bytes).
    pub conditions: Vec<u8>,
    /// Opaque, preserved byte-for-byte (`MOVEMENT_LEN` bytes).
    pub movement: Vec<u8>,
    pub flags: u8,
    pub route_flags: u8,
    pub route: Vec<RouteCommand>,
    pub commands: Vec<Command>,
    pub features: u32,
    pub shadow_graphic_num: u8,
    pub collision_width: u8,
    pub collision_height: u8,
    /// Only present when `features > 3`.
    pub page_transfer: Option<u8>,
}

impl PageV3 {
    pub(crate) fn parse(
        reader: &mut ByteReader,
        is_utf8: bool,
        v35: bool,
    ) -> Result<Self, V3FormatError> {
        let unknown1 = reader.read_u32_le()?;
        let graphic_name = reader.read_string(is_utf8)?;
        let graphic_direction = reader.read_u8()?;
        let graphic_frame = reader.read_u8()?;
        let graphic_opacity = reader.read_u8()?;
        let graphic_render_mode = reader.read_u8()?;

        let conditions = reader.read_bytes(CONDITIONS_LEN)?.to_vec();
        let movement = reader.read_bytes(MOVEMENT_LEN)?.to_vec();

        let flags = reader.read_u8()?;
        let route_flags = reader.read_u8()?;

        let route_count = reader.read_u32_le()? as usize;
        let mut route = Vec::with_capacity(route_count);
        for _ in 0..route_count {
            route.push(RouteCommand::parse(reader)?);
        }

        let command_count = reader.read_u32_le()? as usize;
        let mut commands = Vec::with_capacity(command_count);
        for _ in 0..command_count {
            commands.push(Command::parse(reader, is_utf8, v35)?);
        }

        let features = reader.read_u32_le()?;
        let shadow_graphic_num = reader.read_u8()?;
        let collision_width = reader.read_u8()?;
        let collision_height = reader.read_u8()?;

        let page_transfer = if features > 3 {
            Some(reader.read_u8()?)
        } else {
            None
        };

        let terminator_offset = reader.position();
        let terminator = reader.read_u8()?;
        if terminator != PAGE_TERMINATOR {
            return Err(V3FormatError::UnexpectedByte {
                offset: terminator_offset,
                expected: PAGE_TERMINATOR,
                found: terminator,
                context: "page terminator",
            });
        }

        Ok(Self {
            unknown1,
            graphic_name,
            graphic_direction,
            graphic_frame,
            graphic_opacity,
            graphic_render_mode,
            conditions,
            movement,
            flags,
            route_flags,
            route,
            commands,
            features,
            shadow_graphic_num,
            collision_width,
            collision_height,
            page_transfer,
        })
    }

    pub(crate) fn dump(
        &self,
        writer: &mut ByteWriter,
        is_utf8: bool,
        v35: bool,
    ) -> Result<(), V3FormatError> {
        writer.write_u32_le(self.unknown1);
        writer.write_string(&self.graphic_name, is_utf8)?;
        writer.write_u8(self.graphic_direction);
        writer.write_u8(self.graphic_frame);
        writer.write_u8(self.graphic_opacity);
        writer.write_u8(self.graphic_render_mode);

        writer.write_bytes(&self.conditions);
        writer.write_bytes(&self.movement);

        writer.write_u8(self.flags);
        writer.write_u8(self.route_flags);

        writer.write_u32_le(self.route.len() as u32);
        for rc in &self.route {
            rc.dump(writer);
        }

        writer.write_u32_le(self.commands.len() as u32);
        for cmd in &self.commands {
            cmd.dump(writer, is_utf8, v35)?;
        }

        writer.write_u32_le(self.features);
        writer.write_u8(self.shadow_graphic_num);
        writer.write_u8(self.collision_width);
        writer.write_u8(self.collision_height);

        if let Some(page_transfer) = self.page_transfer {
            writer.write_u8(page_transfer);
        }

        writer.write_u8(PAGE_TERMINATOR);
        Ok(())
    }
}

/// One map event (`Event::Init`/`Dump`, `Map.hpp:315-367`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EventV3 {
    pub id: u32,
    pub name: String,
    pub x: u32,
    pub y: u32,
    pub pages: Vec<PageV3>,
}

impl EventV3 {
    pub(crate) fn parse(
        reader: &mut ByteReader,
        is_utf8: bool,
        v35: bool,
    ) -> Result<Self, V3FormatError> {
        let magic1_offset = reader.position();
        let magic1 = reader.read_bytes(4)?;
        if magic1 != EVENT_MAGIC1 {
            return Err(V3FormatError::InvalidMagic {
                offset: magic1_offset,
                expected: EVENT_MAGIC1.to_vec(),
                found: magic1.to_vec(),
                context: "event magic 1",
            });
        }

        let id = reader.read_u32_le()?;
        let name = reader.read_string(is_utf8)?;
        let x = reader.read_u32_le()?;
        let y = reader.read_u32_le()?;
        let page_count = reader.read_u32_le()?;

        let magic2_offset = reader.position();
        let magic2 = reader.read_bytes(4)?;
        if magic2 != EVENT_MAGIC2 {
            return Err(V3FormatError::InvalidMagic {
                offset: magic2_offset,
                expected: EVENT_MAGIC2.to_vec(),
                found: magic2.to_vec(),
                context: "event magic 2",
            });
        }

        let mut pages = Vec::new();
        loop {
            let indicator_offset = reader.position();
            let indicator = reader.read_u8()?;
            if indicator == PAGE_INDICATOR {
                pages.push(PageV3::parse(reader, is_utf8, v35)?);
            } else if indicator == EVENT_TERMINATOR {
                break;
            } else {
                return Err(V3FormatError::UnexpectedByte {
                    offset: indicator_offset,
                    expected: EVENT_TERMINATOR,
                    found: indicator,
                    context: "event page indicator/terminator",
                });
            }
        }

        if pages.len() as u32 != page_count {
            return Err(V3FormatError::CountMismatch {
                expected: page_count,
                actual: pages.len() as u32,
                context: "event pages",
            });
        }

        Ok(Self {
            id,
            name,
            x,
            y,
            pages,
        })
    }

    pub(crate) fn dump(
        &self,
        writer: &mut ByteWriter,
        is_utf8: bool,
        v35: bool,
    ) -> Result<(), V3FormatError> {
        writer.write_bytes(&EVENT_MAGIC1);
        writer.write_u32_le(self.id);
        writer.write_string(&self.name, is_utf8)?;
        writer.write_u32_le(self.x);
        writer.write_u32_le(self.y);
        writer.write_u32_le(self.pages.len() as u32);
        writer.write_bytes(&EVENT_MAGIC2);

        for page in &self.pages {
            writer.write_u8(PAGE_INDICATOR);
            page.dump(writer, is_utf8, v35)?;
        }

        writer.write_u8(EVENT_TERMINATOR);
        Ok(())
    }
}

/// A `.mps` map (`Map::load`/`dump`, `Map.hpp:496-604`).
///
/// [`MapV3::parse`]/[`MapV3::dump`] operate on the full
/// `header[0..25] ++ payload` buffer produced by
/// [`crate::engines::wolf::v3_format::compression::decompress_v3`] — `magic`, `version` and
/// `unknown2` are the on-disk uncompressed header; everything else is the
/// LZ4-compressed payload.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MapV3 {
    /// 20-byte file magic; `magic[16]` is the UTF-8 flag (`0x55`) / Shift-JIS
    /// flag (`0x00`).
    pub magic: [u8; 20],
    pub version: u32,
    pub unknown2: u8,
    pub unknown3: String,
    pub tileset_id: u32,
    pub width: u32,
    pub height: u32,
    /// Only meaningful when `version >= 0x67` (v3.5+); `0` otherwise.
    pub unknown4: u32,
    /// Defaults to `3` for `version < 0x67`.
    pub layer_cnt: u32,
    /// `None` only for UTF-8 maps with no tile data (`ReadInt() == -1`).
    pub tiles: Option<Vec<u8>>,
    pub events: Vec<EventV3>,
}

impl MapV3 {
    pub(crate) fn is_utf8(&self) -> bool {
        self.magic[16] == 0x55
    }

    pub(crate) fn is_v35(&self) -> bool {
        self.version >= VERSION_V35
    }

    /// Parses `header[0..25] ++ payload` (the output of
    /// [`crate::engines::wolf::v3_format::compression::decompress_v3`]).
    pub(crate) fn parse(bytes: &[u8]) -> Result<Self, V3FormatError> {
        let mut reader = ByteReader::new(bytes);

        let magic_bytes = reader.read_bytes(20)?;
        let mut magic = [0u8; 20];
        magic.copy_from_slice(magic_bytes);
        let is_utf8 = magic[16] == 0x55;

        let version = reader.read_u32_le()?;
        let unknown2 = reader.read_u8()?;
        let v35 = version >= VERSION_V35;

        let unknown3 = reader.read_string(is_utf8)?;
        let tileset_id = reader.read_u32_le()?;
        let width = reader.read_u32_le()?;
        let height = reader.read_u32_le()?;
        let event_count = reader.read_u32_le()?;

        let (unknown4, layer_cnt) = if v35 {
            (reader.read_u32_le()?, reader.read_u32_le()?)
        } else {
            (0, 3)
        };

        let tiles = if is_utf8 {
            let marker_offset = reader.position();
            let marker = reader.read_u32_le()?;
            if marker == NO_TILES_MARKER {
                None
            } else {
                let tile_len = (width * height * layer_cnt * 4) as usize;
                let extra = tile_len
                    .checked_sub(4)
                    .ok_or(V3FormatError::UnexpectedEof {
                        offset: marker_offset,
                        needed: 4,
                        available: tile_len,
                    })?;
                let mut data = Vec::with_capacity(tile_len);
                data.extend_from_slice(&marker.to_le_bytes());
                data.extend_from_slice(reader.read_bytes(extra)?);
                Some(data)
            }
        } else {
            let tile_len = (width * height * layer_cnt * 4) as usize;
            Some(reader.read_bytes(tile_len)?.to_vec())
        };

        let mut events = Vec::new();
        loop {
            let indicator_offset = reader.position();
            let indicator = reader.read_u8()?;
            if indicator == EVENT_INDICATOR {
                events.push(EventV3::parse(&mut reader, is_utf8, v35)?);
            } else if indicator == MAP_TERMINATOR {
                break;
            } else {
                return Err(V3FormatError::UnexpectedByte {
                    offset: indicator_offset,
                    expected: MAP_TERMINATOR,
                    found: indicator,
                    context: "map event indicator/terminator",
                });
            }
        }

        if events.len() as u32 != event_count {
            return Err(V3FormatError::CountMismatch {
                expected: event_count,
                actual: events.len() as u32,
                context: "map events",
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
            unknown2,
            unknown3,
            tileset_id,
            width,
            height,
            unknown4,
            layer_cnt,
            tiles,
            events,
        })
    }

    /// Writes `header[0..25] ++ payload` — the inverse of [`MapV3::parse`],
    /// ready for [`super::compression::recompress_v3`].
    pub(crate) fn dump(&self) -> Result<Vec<u8>, V3FormatError> {
        let is_utf8 = self.is_utf8();
        let v35 = self.is_v35();

        let mut writer = ByteWriter::new();
        writer.write_bytes(&self.magic);
        writer.write_u32_le(self.version);
        writer.write_u8(self.unknown2);

        writer.write_string(&self.unknown3, is_utf8)?;
        writer.write_u32_le(self.tileset_id);
        writer.write_u32_le(self.width);
        writer.write_u32_le(self.height);
        writer.write_u32_le(self.events.len() as u32);

        if v35 {
            writer.write_u32_le(self.unknown4);
            writer.write_u32_le(self.layer_cnt);
        }

        match &self.tiles {
            Some(tiles) => writer.write_bytes(tiles),
            None => writer.write_u32_le(NO_TILES_MARKER),
        }

        for event in &self.events {
            writer.write_u8(EVENT_INDICATOR);
            event.dump(&mut writer, is_utf8, v35)?;
        }

        writer.write_u8(MAP_TERMINATOR);

        Ok(writer.into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_page() -> PageV3 {
        PageV3 {
            unknown1: 0,
            graphic_name: String::new(),
            graphic_direction: 0,
            graphic_frame: 0,
            graphic_opacity: 0,
            graphic_render_mode: 0,
            conditions: vec![0u8; CONDITIONS_LEN],
            movement: vec![0u8; MOVEMENT_LEN],
            flags: 0,
            route_flags: 0,
            route: Vec::new(),
            commands: Vec::new(),
            features: 0,
            shadow_graphic_num: 0,
            collision_width: 0,
            collision_height: 0,
            page_transfer: None,
        }
    }

    #[test]
    fn test_page_round_trip_with_message_command() {
        let mut page = empty_page();
        page.graphic_name = "chara1".to_owned();
        page.commands.push(Command {
            cid: super::super::command::CID_MESSAGE,
            args: Vec::new(),
            indent: 0,
            string_args: vec!["Hello".to_owned()],
            move_data: None,
            v35_unknown: Vec::new(),
        });

        let mut writer = ByteWriter::new();
        page.dump(&mut writer, true, false).unwrap();
        let bytes = writer.into_bytes();

        let mut reader = ByteReader::new(&bytes);
        let parsed = PageV3::parse(&mut reader, true, false).unwrap();
        assert_eq!(reader.remaining(), 0);
        assert_eq!(parsed, page);

        let mut writer2 = ByteWriter::new();
        parsed.dump(&mut writer2, true, false).unwrap();
        assert_eq!(writer2.into_bytes(), bytes);
    }

    #[test]
    fn test_page_with_features_gt_3_has_page_transfer() {
        let mut page = empty_page();
        page.features = 4;
        page.page_transfer = Some(7);

        let mut writer = ByteWriter::new();
        page.dump(&mut writer, false, true).unwrap();
        let bytes = writer.into_bytes();

        let mut reader = ByteReader::new(&bytes);
        let parsed = PageV3::parse(&mut reader, false, true).unwrap();
        assert_eq!(reader.remaining(), 0);
        assert_eq!(parsed.page_transfer, Some(7));
        assert_eq!(parsed, page);
    }

    #[test]
    fn test_page_invalid_terminator_errors() {
        let page = empty_page();
        let mut writer = ByteWriter::new();
        page.dump(&mut writer, true, false).unwrap();
        let mut bytes = writer.into_bytes();
        let last = bytes.len() - 1;
        bytes[last] = 0x00; // corrupt terminator

        let mut reader = ByteReader::new(&bytes);
        let err = PageV3::parse(&mut reader, true, false).unwrap_err();
        assert!(matches!(
            err,
            V3FormatError::UnexpectedByte {
                expected: PAGE_TERMINATOR,
                found: 0x00,
                ..
            }
        ));
    }

    #[test]
    fn test_event_round_trip_with_one_page() {
        let event = EventV3 {
            id: 1,
            name: "EV001".to_owned(),
            x: 3,
            y: 4,
            pages: vec![empty_page()],
        };

        let mut writer = ByteWriter::new();
        event.dump(&mut writer, true, false).unwrap();
        let bytes = writer.into_bytes();

        let mut reader = ByteReader::new(&bytes);
        let parsed = EventV3::parse(&mut reader, true, false).unwrap();
        assert_eq!(reader.remaining(), 0);
        assert_eq!(parsed, event);
    }

    #[test]
    fn test_event_invalid_magic1_errors() {
        let event = EventV3 {
            id: 1,
            name: String::new(),
            x: 0,
            y: 0,
            pages: Vec::new(),
        };
        let mut writer = ByteWriter::new();
        event.dump(&mut writer, true, false).unwrap();
        let mut bytes = writer.into_bytes();
        bytes[0] = 0xFF; // corrupt magic1

        let mut reader = ByteReader::new(&bytes);
        let err = EventV3::parse(&mut reader, true, false).unwrap_err();
        assert!(matches!(
            err,
            V3FormatError::InvalidMagic {
                context: "event magic 1",
                ..
            }
        ));
    }

    #[test]
    fn test_event_page_count_mismatch_errors() {
        // Manually build an event header claiming 1 page but with zero pages.
        let mut writer = ByteWriter::new();
        writer.write_bytes(&EVENT_MAGIC1);
        writer.write_u32_le(1); // id
        writer.write_string("", true).unwrap(); // name
        writer.write_u32_le(0); // x
        writer.write_u32_le(0); // y
        writer.write_u32_le(1); // pageCount = 1 (lie)
        writer.write_bytes(&EVENT_MAGIC2);
        writer.write_u8(EVENT_TERMINATOR); // no pages follow
        let bytes = writer.into_bytes();

        let mut reader = ByteReader::new(&bytes);
        let err = EventV3::parse(&mut reader, true, false).unwrap_err();
        assert!(matches!(
            err,
            V3FormatError::CountMismatch {
                expected: 1,
                actual: 0,
                context: "event pages"
            }
        ));
    }

    fn empty_map(version: u32, is_utf8: bool, width: u32, height: u32) -> MapV3 {
        let mut magic = [0u8; 20];
        magic[10..15].copy_from_slice(b"WOLFM");
        magic[16] = if is_utf8 { 0x55 } else { 0x00 };

        let layer_cnt = 3;
        let tile_len = (width * height * layer_cnt * 4) as usize;
        let tiles = if is_utf8 && tile_len == 0 {
            None
        } else {
            Some(vec![0u8; tile_len])
        };

        MapV3 {
            magic,
            version,
            unknown2: 0,
            unknown3: String::new(),
            tileset_id: 1,
            width,
            height,
            unknown4: if version >= VERSION_V35 { 0 } else { 0 },
            layer_cnt,
            tiles,
            events: Vec::new(),
        }
    }

    #[test]
    fn test_map_round_trip_pre_v35_shift_jis_no_events() {
        let map = empty_map(0x60, false, 2, 2);
        let bytes = map.dump().unwrap();

        let parsed = MapV3::parse(&bytes).unwrap();
        assert_eq!(parsed, map);

        assert_eq!(parsed.dump().unwrap(), bytes);
    }

    #[test]
    fn test_map_round_trip_v35_utf8_no_tiles_with_event() {
        let mut map = empty_map(0x67, true, 2, 2);
        map.tiles = None; // -1 marker
        map.events.push(EventV3 {
            id: 0,
            name: "EV001".to_owned(),
            x: 1,
            y: 1,
            pages: vec![empty_page()],
        });

        let bytes = map.dump().unwrap();
        let parsed = MapV3::parse(&bytes).unwrap();
        assert_eq!(parsed, map);
        assert_eq!(parsed.dump().unwrap(), bytes);
    }

    #[test]
    fn test_map_round_trip_v35_utf8_with_tiles() {
        let mut map = empty_map(0x67, true, 2, 2);
        // Non-(-1) tile data: first 4 bytes must not equal 0xFFFFFFFF.
        map.tiles = Some(vec![0u8; (2 * 2 * 3 * 4) as usize]);

        let bytes = map.dump().unwrap();
        let parsed = MapV3::parse(&bytes).unwrap();
        assert_eq!(parsed, map);
        assert_eq!(parsed.dump().unwrap(), bytes);
    }

    #[test]
    fn test_map_event_terminator_mismatch_errors() {
        let map = empty_map(0x60, false, 0, 0);
        let mut bytes = map.dump().unwrap();
        let last = bytes.len() - 1;
        bytes[last] = 0xAB; // corrupt map terminator

        let err = MapV3::parse(&bytes).unwrap_err();
        assert!(matches!(
            err,
            V3FormatError::UnexpectedByte {
                expected: MAP_TERMINATOR,
                found: 0xAB,
                ..
            }
        ));
    }

    /// Decision gate (plan `docs/plans/f5-01-wolf-v3-mps-parser.md`, Phase 3):
    /// `MapV3::parse(bytes).dump() == bytes` must hold byte-exact for every
    /// real Inko (v3.x LZ4) `.mps` map. If any map fails, this test must fail
    /// loudly — do not loosen the assertions to force a pass.
    #[test]
    fn test_real_inko_maps_v3_round_trip() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../test/Densyanai_Inko_ver2.0/Data/MapData");

        let maps = ["Map001.mps", "Map001_1.mps", "Map001_2.mps", "TitleMap.mps"];

        for name in maps {
            let path = dir.join(name);
            let raw = std::fs::read(&path)
                .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));

            assert!(
                crate::engines::wolf::v3_format::compression::is_lz4_v3(&raw),
                "{name}: expected LZ4-wrapped v3.x header"
            );

            let decompressed = crate::engines::wolf::v3_format::compression::decompress_v3(&raw)
                .unwrap_or_else(|e| panic!("{name}: decompress_v3 failed: {e}"));

            let map = MapV3::parse(&decompressed)
                .unwrap_or_else(|e| panic!("{name}: MapV3::parse failed: {e}"));

            let dumped = map
                .dump()
                .unwrap_or_else(|e| panic!("{name}: MapV3::dump failed: {e}"));

            assert_eq!(
                dumped, decompressed,
                "{name}: dump(parse(bytes)) != bytes (byte-exact round-trip failed)"
            );
        }
    }
}
