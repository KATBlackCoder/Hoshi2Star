#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use crate::byte_utils::parse_string;

/// Miscellaneous information on a WolfRPG Editor game
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq)]
pub struct GameData {
    tile_size: u8,
    character_directions: u8,
    character_movements: u8,
    game_name: String,
    fonts: [String; 4]
}

impl GameData {
    pub fn parse(bytes: &[u8]) -> GameData {
        let mut offset: usize = 0;

        let tile_size: u8 = bytes[offset];
        offset += 1;

        let character_directions: u8 = bytes[offset];
        offset += 1;

        let character_movements: u8 = bytes[offset];
        offset += 1;

        offset += 23; // Unknown

        let (bytes_read, game_name): (usize, String) = parse_string(&bytes[offset..]);
        offset += bytes_read;

        // Unknown
        let (bytes_read, _): (usize, String) = parse_string(&bytes[offset..]);
        offset += bytes_read;

        // Unknown
        let (bytes_read, _): (usize, String) = parse_string(&bytes[offset..]);
        offset += bytes_read;

        let mut fonts: Vec<String> = Vec::with_capacity(4);
        for _ in 0..4 {
            let (bytes_read, font): (usize, String) = parse_string(&bytes[offset..]);
            offset += bytes_read;
            fonts.push(font);
        }

        let fonts: [String; 4] = fonts.try_into().unwrap();

        GameData {
            tile_size,
            character_directions,
            character_movements,
            game_name,
            fonts
        }
    }
    pub fn tile_size(&self) -> u8 {
        self.tile_size
    }

    pub fn tile_size_mut(&mut self) -> &mut u8 {
        &mut self.tile_size
    }

    pub fn character_directions(&self) -> u8 {
        self.character_directions
    }

    pub fn character_directions_mut(&mut self) -> &mut u8 {
        &mut self.character_directions
    }

    pub fn character_movements(&self) -> u8 {
        self.character_movements
    }

    pub fn character_movements_mut(&mut self) -> &mut u8 {
        &mut self.character_movements
    }

    pub fn game_name(&self) -> &str {
        &self.game_name
    }

    pub fn game_name_mut(&mut self) -> &mut String {
        &mut self.game_name
    }

    pub fn fonts(&self) -> &[String; 4] {
        &self.fonts
    }

    pub fn fonts_mut(&mut self) -> &mut [String; 4] {
        &mut self.fonts
    }
}