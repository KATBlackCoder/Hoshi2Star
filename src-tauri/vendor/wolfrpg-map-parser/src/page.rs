pub mod blend_type;
pub mod event_trigger;
pub mod condition;
pub mod move_route;
pub mod options;

use crate::byte_utils::{as_u32_le, parse_string};
use crate::command::Command;
use crate::common::r#move::Move;
use crate::page::blend_type::BlendType;
use crate::page::condition::Condition;
use crate::page::event_trigger::EventTrigger;
use crate::page::move_route::MoveRoute;
use crate::page::options::Options;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

const PAGE_SIGNATURE: &[u8] = b"\x79\xff\xff\xff\xff";

/// An event page containing most of the event state.
///
/// Each event page describes the behaviour of the event, its appearance and the scripts that it
/// runs as a collection of [`Command`].
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
#[allow(unused)]
pub struct Page {
    icon: String,
    icon_row: u8,
    icon_column: u8,
    icon_opacity: u8,
    icon_blend: BlendType,
    event_trigger: EventTrigger,
    conditions: [Condition; 4],
    animation_speed: u8,
    move_speed: u8,
    move_frequency: u8,
    move_route: MoveRoute,
    options: Options,
    unknown1: u8,
    moves: Vec<Move>,
    commands: Vec<Command>,
    unknown2: u32,
    shadow_graphic: u8,
    range_extension_x: u8,
    range_extension_y: u8,
}

impl Page {
    /// Parse raw bytes into a single [`Page`] struct.
    ///
    /// Use of this method is highly discouraged unless you know exactly what you are doing.
    /// Prefer using [`Map::parse`] and then extract what you want from the structure tree.
    ///
    /// # Panics
    /// This function will panic if the given bytes do not represent a valid page structure.
    ///
    /// This might be caused by unaligned bytes, corrupt files, incompatible format updates and
    /// library bugs.
    /// If you are confident you are doing everything right, feel free to report an issue on [GitHub].
    ///
    /// [`Map::parse`]: crate::map::Map::parse
    /// [GitHub]: https://github.com/G1org1owo/wolfrpg-map-parser/issues
    pub fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let signature: &[u8] = &bytes[offset..offset+5];
        offset += 5;

        if signature != PAGE_SIGNATURE {
            panic!("Invalid page signature");
        }

        let (bytes_read, icon): (usize, String) = parse_string(&bytes[offset..]);
        offset += bytes_read;

        let icon_row: u8 = (bytes[offset] >> 1) - 1;
        let icon_column: u8 = bytes[offset+1];
        let icon_opacity: u8 = bytes[offset+2];
        let icon_blend: BlendType = BlendType::new(bytes[offset+3]);
        let event_trigger: EventTrigger = EventTrigger::new(bytes[offset+4]);
        offset += 5;

        let condition1: Condition = Self::parse_condition(&bytes[offset..], 0);
        let condition2: Condition = Self::parse_condition(&bytes[offset..], 1);
        let condition3: Condition = Self::parse_condition(&bytes[offset..], 2);
        let condition4: Condition = Self::parse_condition(&bytes[offset..], 3);
        offset += 36;

        let animation_speed: u8 = bytes[offset];
        let move_speed: u8 = bytes[offset+1];
        let move_frequency: u8 = bytes[offset+2];
        let move_route: MoveRoute = MoveRoute::new(bytes[offset+3]);
        let options: Options = Options::new(bytes[offset+4]);
        let unknown1: u8 = bytes[offset+5];
        offset += 6;

        let move_count: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let (bytes_read, moves): (usize, Vec<Move>) 
            = Move::parse_multiple(&bytes[offset..], move_count);
        offset += bytes_read;
        
        let command_count: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let (bytes_read, commands): (usize, Vec<Command>)
            = Self::parse_commands(&bytes[offset..], command_count);
        offset += bytes_read;

        let unknown2: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let shadow_graphic: u8 = bytes[offset];
        let range_extension_x: u8 = bytes[offset+1];
        let range_extension_y: u8 = bytes[offset+2];
        offset += 3;

        let page_end: u8 = bytes[offset];
        offset += 1;

        if page_end != 0x7a {
            panic!("Expected page end but found {:02x}", page_end);
        }

        (offset, Self {
            icon,
            icon_row,
            icon_column,
            icon_opacity,
            icon_blend,
            event_trigger,
            conditions: [condition1, condition2, condition3, condition4],
            animation_speed,
            move_speed,
            move_frequency,
            move_route,
            options,
            unknown1,
            moves,
            commands,
            unknown2,
            shadow_graphic,
            range_extension_x,
            range_extension_y,
        })
    }

    fn parse_condition(bytes: &[u8], count: usize) -> Condition {
        Condition::new(
            bytes[count],
            as_u32_le(&bytes[4*(1+count)..4*(2+count)]),
            as_u32_le(&bytes[4*(5+count)..4*(6+count)]),
        )
    }

    fn parse_commands(bytes: &[u8], command_count: u32) -> (usize, Vec<Command>){
        let mut offset: usize = 0;
        let mut commands: Vec<Command> = Vec::new();

        let mut i: u32 = 0;
        while i<command_count {
            let (bytes_read, commands_read, command): (usize, u32, Command)
                = Command::parse(&bytes[offset..]);
            offset += bytes_read;
            commands.push(command);

            i += commands_read;
        }

        (offset, commands)
    }

    /// The filename of the icon sprite sheet used by this page.
    pub fn icon(&self) -> &str {
        &self.icon
    }

    /// Mutable reference accessor for [`Page::icon`].
    pub fn icon_mut(&mut self) -> &mut String {
        &mut self.icon
    }

    /// The row of the selected icon in the selected sprite sheet.
    pub fn icon_row(&self) -> u8 {
        self.icon_row
    }

    /// Mutable reference accessor for [`Page::icon_row`].
    pub fn icon_row_mut(&mut self) -> &mut u8 {
        &mut self.icon_row
    }

    /// The column of the selected icon in the selected sprite sheet.
    pub fn icon_column(&self) -> u8 {
        self.icon_column
    }

    /// Mutable reference accessor for [`Page::icon_column`].
    pub fn icon_column_mut(&mut self) -> &mut u8 {
        &mut self.icon_column
    }

    /// The opacity the icon should have when rendered.
    pub fn icon_opacity(&self) -> u8 {
        self.icon_opacity
    }

    /// Mutable reference accessor for [`Page::icon_opacity`].
    pub fn icon_opacity_mut(&mut self) -> &mut u8 {
        &mut self.icon_opacity
    }

    /// The blend method used for painting the icon onto the background.
    pub fn icon_blend(&self) -> &BlendType {
        &self.icon_blend
    }

    /// Mutable reference accessor for [`Page::icon_blend`].
    pub fn icon_blend_mut(&mut self) -> &mut BlendType {
        &mut self.icon_blend
    }

    /// The condition by which the event is triggered.
    pub fn event_trigger(&self) -> &EventTrigger {
        &self.event_trigger
    }

    /// Mutable reference accessor for [`Page::event_trigger`].
    pub fn event_trigger_mut(&mut self) -> &mut EventTrigger {
        &mut self.event_trigger
    }

    /// The conditions that must be mut for the script to run.
    pub fn conditions(&self) -> &[Condition; 4] {
        &self.conditions
    }

    /// Mutable reference accessor for [`Page::conditions`].
    pub fn conditions_mut(&mut self) -> &mut [Condition; 4] {
        &mut self.conditions
    }

    /// The speed at which the event's animation advance.
    pub fn animation_speed(&self) -> u8 {
        self.animation_speed
    }

    /// Mutable reference accessor for [`Page::animation_speed`].
    pub fn animation_speed_mut(&mut self) -> &mut u8 {
        &mut self.animation_speed
    }

    /// The speed at which the event moves.
    pub fn move_speed(&self) -> u8 {
        self.move_speed
    }

    /// Mutable reference accessor for [`Page::move_speed`].
    pub fn move_speed_mut(&mut self) -> &mut u8 {
        &mut self.move_speed
    }

    /// The frequency at which the event moves.
    pub fn move_frequency(&self) -> u8 {
        self.move_frequency
    }

    /// Mutable reference accessor for [`Page::move_frequency`].
    pub fn move_frequency_mut(&mut self) -> &mut u8 {
        &mut self.move_frequency
    }

    /// How (and whether) the event should move.
    pub fn move_route(&self) -> &MoveRoute {
        &self.move_route
    }

    /// Mutable reference accessor for [`Page::move_route`].
    pub fn move_route_mut(&mut self) -> &mut MoveRoute {
        &mut self.move_route
    }

    /// A set of options that influence the event rendering and behaviour.
    pub fn options(&self) -> &Options {
        &self.options
    }

    /// Mutable reference accessor for [`Page::options`].
    pub fn options_mut(&mut self) -> &mut Options {
        &mut self.options
    }

    /// The pattern the event follows when moving.
    ///
    /// This field is only valid if [`Page::move_route`] is `MoveRoute::Custom`, otherwise it's
    /// empty.
    pub fn moves(&self) -> &Vec<Move> {
        &self.moves
    }

    /// Mutable reference accessor for [`Page::moves`].
    pub fn moves_mut(&mut self) -> &mut Vec<Move> {
        &mut self.moves
    }

    /// The event script, in the form of a [`Command`] collection.
    pub fn commands(&self) -> &Vec<Command> {
        &self.commands
    }

    /// Mutable reference accessor for [`Page::commands`].
    pub fn commands_mut(&mut self) -> &mut Vec<Command> {
        &mut self.commands
    }

    /// The graphic to use for the shadow, if shadow graphics are enabled.
    pub fn shadow_graphic(&self) -> u8 {
        self.shadow_graphic
    }

    /// Mutable reference accessor for [`Page::shadow_graphic`].
    pub fn shadow_graphic_mut(&mut self) -> &mut u8 {
        &mut self.shadow_graphic
    }

    /// How much the trigger area is expanded horizontally.
    ///
    /// This field is only valid if [`Page::event_trigger`] is `EventTrigger::ConfirmKey`,
    /// `EventTrigger::EventTouch` or `EventTrigger::PlayerTouch`
    pub fn range_extension_x(&self) -> u8 {
        self.range_extension_x
    }

    /// Mutable reference accessor for [`Page::range_extension_x`].
    pub fn range_extension_x_mut(&mut self) -> &mut u8 {
        &mut self.range_extension_x
    }

    /// How much the trigger area is expanded horizontally.
    ///
    /// This field is only valid if [`Page::event_trigger`] is `EventTrigger::ConfirmKey`,
    /// `EventTrigger::EventTouch` or `EventTrigger::PlayerTouch`
    pub fn range_extension_y(&self) -> u8 {
        self.range_extension_y
    }

    /// Mutable reference accessor for [`Page::range_extension_y`].
    pub fn range_extension_y_mut(&mut self) -> &mut u8 {
        &mut self.range_extension_y
    }
}