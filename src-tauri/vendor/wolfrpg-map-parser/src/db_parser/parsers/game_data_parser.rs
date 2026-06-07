use crate::db_parser::game_data::GameData;
use crate::db_parser::DATA_MAGIC;
use std::fs;
use std::io::Result;
use std::path::Path;

/// Parse a .dat file containing information on a WolfRPG Editor game.
///
/// Returns miscellaneous data regarding the game.
/// If you have already read the bytes, consider using [`parse_bytes`].
///
/// # Panics
/// This function will panic if the given file does not represent a valid game data structure.
pub fn parse(data: &Path) -> Result<GameData> {
    match fs::read(data) {
        Ok(contents) => {
            Ok(parse_bytes(&contents))
        }
        Err(e) => {
            Err(e)
        }
    }
}

/// Parse bytes containing information on a WolfRPG Editor game.
///
/// Returns miscellaneous data regarding the game.
/// If you need to read the file to call this function, consider using [`parse`].
///
/// # Panics
/// This function will panic if the given bytes do not represent a valid game data structure.
pub fn parse_bytes(bytes: &[u8]) -> GameData {
    let mut offset: usize = 0;

    let header: &[u8] = &bytes[0..11];
    offset += 11;

    if &header[..10] != DATA_MAGIC {
        panic!("Invalid data header.");
    }

    offset += 3; // Padding

    GameData::parse(&bytes[offset..])
}