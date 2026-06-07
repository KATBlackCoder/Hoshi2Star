use crate::byte_utils::as_u32_le;
use crate::command::db_management_command::state::State;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

type DBStrings = (Option<String>, Option<String>, Option<String>);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Base {
    value: u32,
}

impl Base {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self, DBStrings) {
        let mut offset: usize = 0;

        let value: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        offset += 1; // padding

        // I could not manually create a command with string count != 4, but I found a map 
        // that has a command with string count 0, so we'll handle that case.
        let string_count: u8 = bytes[offset]; 
        offset += 1;

        if string_count > 0 {
            offset += 5; // In this variant, value should always be a number, so we skip this string
        }

        let (bytes_read, db_strings): (usize, DBStrings)
            = State::parse_strings(string_count, &bytes[offset..]);
        offset += bytes_read;
        
        let state: Self = Self {
            value
        };

        (offset, state, db_strings)
    }

    pub fn value(&self) -> u32 {
        self.value
    }

    pub fn value_mut(&mut self) -> &mut u32 {
        &mut self.value
    }
}