use std::fs;
use std::io::Result;
use std::path::Path;
use crate::byte_utils::as_u32_le;
use crate::db_parser::DATA_MAGIC;
use crate::db_parser::models::tileset::Tileset;

/// Parse a .dat file containing information on the available tilesets.
/// 
/// Returns detailed information regarding each tileset available to the editor.
/// If you already have read the bytes, consider using [`parse_bytes`].
///
/// # Panics
/// This function will panic if the given file does not represent a valid tileset structure.
pub fn parse(data: &Path) -> Result<Vec<Tileset>> {
    match fs::read(data) {
        Ok(contents) => {
            Ok(parse_bytes(&contents))
        }
        Err(e) => {
            Err(e)
        }
    }
}

/// Parse bytes containing information on the available tilesets.
/// 
/// Returns detailed information regarding each tileset available to the editor.
/// If you need to read the file to call this function, consider using [`parse`].
///
/// # Panics
/// This function will panic if the given bytes do not represent a valid tileset structure.
#[allow(unused_assignments)]
pub fn parse_bytes(bytes: &[u8]) -> Vec<Tileset> {
    let mut offset: usize = 0;
    
    let header: &[u8] = &bytes[0..11];
    offset += 11;

    if &header[..10] != DATA_MAGIC {
        panic!("Invalid tileset header.");
    }

    let tileset_count: usize = as_u32_le(&bytes[offset..]) as usize;
    offset += 4;

    let mut tilesets: Vec<Tileset> = Vec::with_capacity(tileset_count);

    for i in 0..tileset_count {
        let (bytes_read, table): (usize, Tileset) = Tileset::parse(&bytes[offset..], i);
        offset += bytes_read;
        tilesets.push(table);
    }
    
    offset += 1; // Should be 0xcf

    tilesets
}