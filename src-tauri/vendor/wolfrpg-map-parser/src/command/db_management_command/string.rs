use crate::byte_utils::parse_string;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use std::string::String as StdString;
use crate::command::db_management_command::state::State;

type DBStrings = (Option<StdString>, Option<StdString>, Option<StdString>);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct String {
    value: StdString,
}

impl String {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self, DBStrings) {
        let mut offset: usize = 0;

        offset += 1; // padding

        // I could not manually create a command with string count != 4, but I found a map 
        // that has a command with string count 0, so we'll handle that case.
        let string_count: u8 = bytes[offset];
        offset += 1;

        let (bytes_read, value): (usize, StdString) = parse_string(&bytes[offset..]);
        offset += bytes_read;

        let (bytes_read, db_strings): (usize, DBStrings)
            = State::parse_strings(string_count, &bytes[offset..]);
        offset += bytes_read;

        let state: Self = Self {
            value
        };

        (offset, state, db_strings)
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn value_mut(&mut self) -> &mut StdString {
        &mut self.value
    }
}