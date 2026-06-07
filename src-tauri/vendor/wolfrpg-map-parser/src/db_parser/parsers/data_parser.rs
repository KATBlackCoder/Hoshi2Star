use crate::byte_utils::as_u32_le;
use crate::db_parser::table::Table;
use std::fs;
use std::io::Result;
use std::path::Path;
use crate::db_parser::DATA_MAGIC;

/// Parse a .dat file containing information on a WolfRPG Editor internal database.
///
/// Returns the data inside a database table. For the DB schema, use [`project_parser::parse`].
/// If you have already read the bytes, consider using [`parse_bytes`].
/// 
/// # Panics
/// This function will panic if the given file does not represent a valid database data structure.
/// 
/// [`project_parser::parse`]: crate::project_parser::parse
pub fn parse(data: &Path) -> Result<Vec<Table>> {
    match fs::read(data) {
        Ok(contents) => {
            Ok(parse_bytes(&contents))
        }
        Err(e) => {
            Err(e)
        }
    }
}

/// Parse bytes containing information on a WolfRPG Editor internal database.
///
/// Returns the data inside a database table. For the DB schema, use [`project_parser::parse_bytes`].
/// If you need to read the file to call this function, consider using [`parse`].
/// 
/// # Panics
/// This function will panic if the given bytes do not represent a valid database data structure.
/// 
/// [`project_parser::parse_bytes`]: crate::project_parser::parse_bytes
#[allow(unused_assignments)]
pub fn parse_bytes(bytes: &[u8]) -> Vec<Table> {
    let mut offset: usize = 0;

    let header: &[u8] = &bytes[0..11];
    offset += 11;

    if &header[..10] != DATA_MAGIC {
        panic!("Invalid data header.");
    }
    
    let type_count: usize = as_u32_le(&bytes[offset..]) as usize;
    offset += 4;
    
    let mut tables: Vec<Table> = Vec::with_capacity(type_count);
    
    for i in 0..type_count {
        let (bytes_read, table): (usize, Table) = Table::parse(&bytes[offset..], i);
        offset += bytes_read;
        tables.push(table);
    }
    
    offset += 1; // Should be 0xc1
    
    tables
}