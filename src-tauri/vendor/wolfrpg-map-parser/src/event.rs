use crate::byte_utils::{as_u32_be, as_u32_le, parse_string};
use crate::page::Page;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

const EVENT_SIGNATURE: u32 = 0x6f393000;

/// An event on a specific map.
///
/// An event is any NPC or item that can interact with the player or can be interacted with.
/// This struct contains detailed information about the position of the event and one or more pages
/// containing extra details on how to render the event, plus the scripts related to this event.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
#[allow(unused)]
pub struct Event {
    id: u32,
    name: String,
    position_x: u32,
    position_y: u32,
    unknown1: u32,
    pages: Vec<Page>
}

impl Event {
    /// Parse raw bytes into a single [`Event`] struct.
    ///
    /// Use of this method is highly discouraged unless you know exactly what you are doing.
    /// Prefer using [`Map::parse`] and then extract what you want from the structure tree.
    ///
    /// # Panics
    /// This function will panic if the given bytes do not represent a valid event structure.
    ///
    /// This might be caused by unaligned bytes, corrupt files, incompatible format updates and
    /// library bugs.
    /// If you are confident you are doing everything right, feel free to report an issue on [GitHub].
    ///
    /// [`Map::parse`]: crate::map::Map::parse
    /// [GitHub]: https://github.com/G1org1owo/wolfrpg-map-parser/issues
    pub fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let signature: u32 = as_u32_be(&bytes[offset..offset + 4]);
        offset += 4;

        if signature != EVENT_SIGNATURE {
            panic!("Invalid event signature: {:02x}.", signature);
        }

        offset += 1; // padding

        let id: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let (bytes_read, name): (usize, String) = parse_string(&bytes[offset..]);
        offset += bytes_read;

        let position_x: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let position_y: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let page_count: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let unknown1: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let mut pages: Vec<Page> = vec![];
        for _ in 0..page_count {
            let (bytes_read, page): (usize, Page) = Page::parse(&bytes[offset..]);
            offset += bytes_read;
            pages.push(page);
        }

        let event_end: u8 = bytes[offset];
        offset += 1;

        if event_end != 0x70 {
            panic!("Expected event end but found {:02x}.", event_end);
        }

        (offset, Self {
            id,
            name,
            position_x,
            position_y,
            unknown1,
            pages,
        })
    }

    /// Parse raw bytes into an [`Event`] collection.
    ///
    /// Use of this method is highly discouraged unless you know exactly what you are doing.
    /// Prefer using [`Map::parse`] and then extract what you want from the structure tree.
    ///
    /// # Panics
    /// This function will panic if the given bytes do not represent a valid event list structure.
    ///
    /// This might be caused by unaligned bytes, corrupt files, incompatible format updates and
    /// library bugs.
    /// If you are confident you are doing everything right, feel free to report an issue on [GitHub].
    ///
    /// [`Map::parse`]: crate::map::Map::parse
    /// [GitHub]: https://github.com/G1org1owo/wolfrpg-map-parser/issues
    pub fn parse_multiple(bytes: &[u8], count: u32) -> (usize, Vec<Self>) {
        let mut offset: usize = 0;
        let mut events: Vec<Event> = Vec::new();

        for _ in 0..count {
            let (bytes_read, event): (usize, Self) = Self::parse(&bytes[offset..]);

            offset += bytes_read;
            events.push(event);
        }

        (offset, events)
    }

    /// The unique identifier of this event.
    pub fn id(&self) -> u32 {
        self.id
    }

    /// The name of this event.
    ///
    /// This is only useful to recognize different events from a programming standpoint and is not
    /// shown in game whatsoever.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Mutable reference accessor for [`Event::name`] .
    pub fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }

    /// The x coordinate of this event, in tiles.
    pub fn position_x(&self) -> u32 {
        self.position_x
    }

    /// Mutable reference accessor for [`Event::position_x`].
    pub fn position_x_mut(&mut self) -> &mut u32 {
        &mut self.position_x
    }

    /// The y coordinate of this event, in tiles.
    pub fn position_y(&self) -> u32 {
        self.position_y
    }

    /// Mutable reference accessor for [`Event::position_y`].
    pub fn position_y_mut(&mut self) -> &mut u32 {
        &mut self.position_y
    }

    /// A collection of pages representing the different states this event can be in.
    ///
    /// Each event can have up to ten pages describing its behaviour. The page that is actually run
    /// is the one with the highest index that meets the requirements of its [`Page::event_trigger`]
    /// and [`Page::conditions`] fields.
    pub fn pages(&self) -> &Vec<Page> {
        &self.pages
    }

    /// Mutable reference accessor for [`Event::pages`].
    pub fn pages_mut(&mut self) -> &mut Vec<Page> {
        &mut self.pages
    }
}