use crate::byte_utils::as_u32_le;
use crate::db_parser::models::common_event::CommonEvent;
use std::fs;
use std::path::Path;
use crate::db_parser::COMMON_EVENTS_MAGIC;

/// Parse a .dat file containing information on a WolfRPG Editor common events database.
///
/// If you have already read the bytes, consider using [`parse_bytes`].
///
/// # Panics
/// This function will panic if the given file does not represent a valid common events data structure.
pub fn parse(data: &Path) -> std::io::Result<Vec<CommonEvent>> {
    match fs::read(data) {
        Ok(contents) => {
            Ok(parse_bytes(&contents))
        }
        Err(e) => {
            Err(e)
        }
    }
}

/// Parse bytes containing information on a WolfRPG Editor common events database.
///
/// If you need to read the file to call this function, consider using [`parse`].
///
/// # Panics
/// This function will panic if the given bytes do not represent a valid common events data structure.
pub fn parse_bytes(bytes: &[u8]) -> Vec<CommonEvent> {
    let mut offset: usize = 0;

    let signature: &[u8] = &bytes[offset..][..11];
    offset += 11;

    if signature != COMMON_EVENTS_MAGIC {
        panic!("Invalid common events header.");
    }

    let event_count: usize = as_u32_le(&bytes[offset..offset+4]) as usize;
    offset += 4;

    let mut events: Vec<CommonEvent> = vec![];
    for _ in 0..event_count {
        let (bytes_read, event): (usize, CommonEvent)
            = CommonEvent::parse(&bytes[offset..]);
        offset += bytes_read;

        events.push(event);
    };

    events
}