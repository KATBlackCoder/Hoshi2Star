#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Settings for a specific tile in a given tileset.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Tile {
    tag_number: u8,
    down_not_passable: bool,
    left_not_passable: bool,
    right_not_passable: bool,
    up_not_passable: bool,
    always_above_character: bool,
    bottom_half_translucent: bool,
    conceal_character_behind: bool,
    match_passable_under: bool
}

impl Tile {
    pub fn new(tag_number: u8, options: u32) -> Self {
        Self {
            tag_number,
            down_not_passable: options & 0b00000001 != 0,
            left_not_passable: options & 0b00000010 != 0,
            right_not_passable: options & 0b00000100 != 0,
            up_not_passable: options & 0b00001000 != 0,
            always_above_character: options & 0b00010000 != 0,

            bottom_half_translucent: options & 0b01000000 != 0,

            conceal_character_behind: (options >> 8) & 0b00000001 != 0,
            match_passable_under: (options >> 8) & 0b00000010 != 0
        }
    }

    /// A value that can be assigned to tiles and retrieved via the [`SetVariablePlus`] command.
    ///
    /// [`SetVariablePlus`]: crate::command::set_variable_plus_command::SetVariablePlusCommand
    pub fn tag_number(&self) -> u8 {
        self.tag_number
    }

    /// Mutable reference accessor for [`Tile::tag_number`].
    pub fn tag_number_mut(&mut self) -> &mut u8 {
        &mut self.tag_number
    }

    /// Whether the tile can be passed from its downside.
    pub fn down_not_passable(&self) -> bool {
        self.down_not_passable
    }

    /// Mutable reference accessor for [`Tile::down_not_passable`].
    pub fn down_not_passable_mut(&mut self) -> &mut bool {
        &mut self.down_not_passable
    }

    /// Whether the tile can be passed from its left side.
    pub fn left_not_passable(&self) -> bool {
        self.left_not_passable
    }

    /// Mutable reference accessor for [`Tile::left_not_passable`].
    pub fn left_not_passable_mut(&mut self) -> &mut bool {
        &mut self.left_not_passable
    }

    /// Whether the tile can be passed from its right side.
    pub fn right_not_passable(&self) -> bool {
        self.right_not_passable
    }

    /// Mutable reference accessor for [`Tile::right_not_passable`].
    pub fn right_not_passable_mut(&mut self) -> &mut bool {
        &mut self.right_not_passable
    }

    /// Whether the tile can be passed from its upside.
    pub fn up_not_passable(&self) -> bool {
        self.up_not_passable
    }

    /// Mutable reference accessor for [`Tile::up_not_passable`].
    pub fn up_not_passable_mut(&mut self) -> &mut bool {
        &mut self.up_not_passable
    }

    /// If true, this tile will always be displayed above a character.
    pub fn always_above_character(&self) -> bool {
        self.always_above_character
    }

    /// Mutable reference accessor for [`Tile::always_above_character`].
    pub fn always_above_character_mut(&mut self) -> &mut bool {
        &mut self.always_above_character
    }

    /// If true, the bottom half of this tile will be translucent.
    pub fn bottom_half_translucent(&self) -> bool {
        self.bottom_half_translucent
    }

    /// Mutable reference accessor for [`Tile::bottom_half_translucent`].
    pub fn bottom_half_translucent_mut(&mut self) -> &mut bool {
        &mut self.bottom_half_translucent
    }

    /// If true, this tile will be displayed above characters with a smaller or equal y position.
    pub fn conceal_character_behind(&self) -> bool {
        self.conceal_character_behind
    }

    /// Mutable reference accessor for [`Tile::conceal_character_behind`].
    pub fn conceal_character_behind_mut(&mut self) -> &mut bool {
        &mut self.conceal_character_behind
    }

    /// If true, whether this tile is passable or not depends on the tile on the lower layer.
    pub fn match_passable_under(&self) -> bool {
        self.match_passable_under
    }

    /// Mutable reference accessor for [`Tile::match_passable_under`].
    pub fn match_passable_under_mut(&mut self) -> &mut bool {
        &mut self.match_passable_under
    }
}