#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::{as_u32_vec, as_u32_le};
use crate::event::Event;

const MAP_SIGNATURE: &[u8]
    = b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x57\x4F\x4C\x46\x4D\x00\x00\x00\x00\x00";

/// A Wolf RPG Editor map (.mps) file.
///
/// Contains all information needed for rendering a level, from the single tiles to the events
/// set to happen.
///
/// # Examples
/// ```
/// use wolfrpg_map_parser::Map;
/// use std::fs;
///
/// match fs::read("filepath.mps") {
///     Ok(bytes) => {
///         let map: Map = Map::parse(&bytes);
///         // Data manipulation ...
///     }
///     Err(_) => {
///         // Error handling ...
///     }
/// }
/// ```
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Map {
    tileset: u32,
    width: u32,
    height: u32,
    layer1: Vec<u32>,
    layer2: Vec<u32>,
    layer3: Vec<u32>,
    events: Vec<Event>,
}

impl Map {
    /// Parse raw bytes into a [`Map`] struct.
    ///
    /// This is the main driver that should be used when loading a .mps file. Using other methods is
    /// highly discouraged, unless you know what you are doing and need that extra bit of speed.
    ///
    /// # Panics
    /// This function will panic if the given bytes do not represent a valid map structure.
    ///
    /// This might be caused by corrupt files, incompatible format updates and library bugs.
    /// In each of these cases, feel free to report an issue on [GitHub].
    ///
    /// [GitHub]: https://github.com/G1org1owo/wolfrpg-map-parser/issues
    ///
    /// # Examples
    /// ```
    /// use wolfrpg_map_parser::Map;
    /// use std::fs;
    ///
    /// match fs::read("filepath.mps") {
    ///     Ok(bytes) => {
    ///         let map: Map = Map::parse(&bytes);
    ///         // Data manipulation ...
    ///     }
    ///     Err(_) => {
    ///         // Error handling ...
    ///     }
    /// }
    /// ```
    pub fn parse(bytes: &[u8]) -> Self {
        let mut offset: usize = 0;

        let magic: &[u8] = &bytes[offset..offset+20];
        offset += 20;

        if magic != MAP_SIGNATURE {
            panic!("Invalid WOLF map signature.");
        }

        offset += 5; // Unknown data

        let skippable: usize = as_u32_le(&bytes[offset..offset+4]) as usize;
        offset += 4;
        offset += skippable;

        let tileset: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let width: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let height: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let event_count: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let layer_length: usize = (width * height * 4) as usize;
        let layer1: Vec<u32> = as_u32_vec(
            &bytes[offset..offset + layer_length],
        );
        offset += layer_length;

        let layer2: Vec<u32> = as_u32_vec(
            &bytes[offset..offset + layer_length]
        );
        offset += layer_length;

        let layer3: Vec<u32> = as_u32_vec(
            &bytes[offset..offset + layer_length]
        );
        offset += layer_length;

        let (bytes_read, events): (usize, Vec<Event>)
            = Event::parse_multiple(&bytes[offset..], event_count);
        offset += bytes_read;

        let map_end: u8 = bytes[offset];

        if map_end != 0x66 {
            panic!("Expected map end but found {:02x}.", map_end);
        }

        Self {
            tileset,
            width,
            height,
            layer1,
            layer2,
            layer3,
            events
        }
    }

    /// The ID of the set of pictures used for each tile.
    pub fn tileset(&self) -> u32 {
        self.tileset
    }

    /// Mutable reference accessor for [`Map::tileset`].
    pub fn tileset_mut(&mut self) -> &mut u32 {
        &mut self.tileset
    }

    /// The width of the map, in tiles.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Mutable reference accessor for [`Map::width`].
    pub fn width_mut(&mut self) -> &mut u32 {
        &mut self.width
    }

    /// The height of the map, in tiles.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Mutable reference accessor for [`Map::height`].
    pub fn height_mut(&mut self) -> &mut u32 {
        &mut self.height
    }

    /// The bottom-most layer of tiles.
    ///
    /// Each layer is painted above the lower ones.
    pub fn layer1(&self) -> &Vec<u32> {
        &self.layer1
    }

    /// Mutable reference accessor for [`Map::layer1`].
    pub fn layer1_mut(&mut self) -> &mut Vec<u32> {
        &mut self.layer1
    }

    /// The middle layer of tiles.
    ///
    /// Each layer is painted above the lower ones.
    pub fn layer2(&self) -> &Vec<u32> {
        &self.layer2
    }

    /// Mutable reference accessor for [`Map::layer2`].
    pub fn layer2_mut(&mut self) -> &mut Vec<u32> {
        &mut self.layer2
    }

    /// The top-most layer of tiles.
    ///
    /// Each layer is painted above the lower ones.
    pub fn layer3(&self) -> &Vec<u32> {
        &self.layer3
    }

    /// Mutable reference accessor for [`Map::layer3`].
    pub fn layer3_mut(&mut self) -> &mut Vec<u32> {
        &mut self.layer3
    }

    /// A collection of all the events set on this map.
    ///
    /// Events are painted above each other based on the following priority:
    /// 1. If [`Page::options::above_hero`] is `true`, then the event has the highest priority.
    /// 2. If [`Page::options::above_hero`] is `false` and [`Page::options::slip_through`] is `false`,
    ///     then the event has the second-highest priority.
    /// 3. if [`Page::options::above_hero`] is `false` and [`Page::options::slip_through`] is `true`,
    ///     then the event has the lowest-priority.
    ///
    /// When events have the same priority, they are displayed in order of [`Event::id`].
    ///
    /// [`Page::options::above_hero`]: (event::page::options::Options::above_hero)
    /// [`Page::options::slip_through`]: (event::page::options::Options::slip_through)
    pub fn events(&self) -> &Vec<Event> {
        &self.events
    }

    /// Mutable reference accessor for [`Map::events`].
    pub fn events_mut(&mut self) -> &mut Vec<Event> {
        &mut self.events
    }
}