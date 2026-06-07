use crate::byte_utils::{as_u32_le, as_u32_vec, parse_string};
use crate::db_parser::models::tile::Tile;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Detailed information on a tileset.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Tileset {
    index: usize,
    name: String,
    base_tileset: String,
    auto_tiles: [String; 15],
    tiles: Vec<Tile>
}

impl Tileset {
    pub(crate) fn parse(bytes: &[u8], index: usize) -> (usize, Self) {
        let mut offset: usize = 0;

        let (bytes_read, name): (usize, String) = parse_string(&bytes[offset..]);
        offset += bytes_read;
        
        let (bytes_read, base_tileset): (usize, String) = parse_string(&bytes[offset..]);
        offset += bytes_read;
        
        let mut auto_tiles: Vec<String> = Vec::with_capacity(15);
        
        for _ in 0..15 {
            let (bytes_read, string): (usize, String) = parse_string(&bytes[offset..]);
            offset += bytes_read;
            auto_tiles.push(string);
        }
        
        let auto_tiles: [String; 15] = auto_tiles.try_into().unwrap();
        
        offset += 1; // Padding
        
        let tags_len: usize = as_u32_le(&bytes[offset..]) as usize;
        offset += 4;
        
        let tag_numbers: Vec<u8> = bytes[offset..][..tags_len].to_vec();
        offset += tags_len;
        
        offset += 1; // Padding
        
        // Should be equal to tags_len
        let directions_len: usize = as_u32_le(&bytes[offset..]) as usize; 
        offset += 4;
        
        let directions: Vec<u32> = as_u32_vec(&bytes[offset..][..4 * directions_len]);
        offset += 4 * directions_len;
        
        let tiles: Vec<Tile> = tag_numbers.iter()
            .zip(directions.iter())
            .map(|(tag, options)| Tile::new(*tag, *options))
            .collect();
        
        (offset, Self {
            index,
            name,
            base_tileset,
            auto_tiles,
            tiles
        })
    }

    /// The index of this tileset in the list.
    pub fn index(&self) -> usize {
        self.index
    }

    /// The name given to this tileset.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Mutable reference accessor for [`Tileset::name`].
    pub fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }

    /// The file from which most of the tiles are taken.
    pub fn base_tileset(&self) -> &str {
        &self.base_tileset
    }

    /// Mutable reference accessor for [`Tileset::base_tileset`].
    pub fn base_tileset_mut(&mut self) -> &mut String {
        &mut self.base_tileset
    }

    /// A list of files from which the automatic tiles are taken.
    /// 
    /// Automatic tiles display differently based on the tiles they are next to, such as
    /// water rendering the edge of a pond if it's close to grass.
    pub fn auto_tiles(&self) -> &[String; 15] {
        &self.auto_tiles
    }

    /// Mutable reference accessor for [`Tileset::auto_tiles`].
    pub fn auto_tiles_mut(&mut self) -> &mut [String; 15] {
        &mut self.auto_tiles
    }

    /// Specific settings for each tile of the tileset.
    pub fn tiles(&self) -> &Vec<Tile> {
        &self.tiles
    }

    /// Mutable reference accessor for [`Tileset::tiles`].
    pub fn tiles_mut(&mut self) -> &mut Vec<Tile> {
        &mut self.tiles
    }
}