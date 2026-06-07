use crate::byte_utils::{as_u32_le, parse_string};
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::db_management_command::state::State;

type DBStrings = (Option<String>, Option<String>, Option<String>);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct CSV {
    entry_count: u32,
    filename: String
}

impl CSV {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self, DBStrings) {
        let mut offset: usize = 0;

        let entry_count: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        offset += 1; // padding

        // I could not manually create a command with string count != 4, but I found a map 
        // that has a command with string count 0, so we'll handle that case.
        let string_count: u8 = bytes[offset];
        offset += 1;

        let (bytes_read, filename): (usize, String) = parse_string(&bytes[offset..]);
        offset += bytes_read;

        let (bytes_read, db_strings): (usize, DBStrings)
            = State::parse_strings(string_count, &bytes[offset..]);
        offset += bytes_read;

        let state: Self = Self {
            entry_count,
            filename
        };

        (offset, state, db_strings)
    }

    pub fn entry_count(&self) -> u32 {
        self.entry_count
    }

    pub fn entry_count_mut(&mut self) -> &mut u32 {
        &mut self.entry_count
    }

    pub fn filename(&self) -> &str {
        &self.filename
    }

    pub fn filename_mut(&mut self) -> &mut String {
        &mut self.filename
    }
}