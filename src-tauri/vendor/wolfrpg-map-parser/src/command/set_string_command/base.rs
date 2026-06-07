use crate::byte_utils::parse_string;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Base {
    string: String,
    replace: Option<String>
}

impl Base {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        offset += 1; // Unknown, most probably padding

        let string_count: u8 = bytes[offset];
        offset += 1;

        let (bytes_read, string): (usize, String) = parse_string(&bytes[offset..]);
        offset += bytes_read;

        let replace: Option<String> = if string_count == 2 {
            let (bytes_read, replace): (usize, String) = parse_string(&bytes[offset..]);
            offset += bytes_read;

            Some(replace)
        } else {
            None
        };

        offset += 1; // Command end signature

        (offset, Self {
            string,
            replace
        })
    }

    pub fn string(&self) -> &str {
        &self.string
    }

    pub fn string_mut(&mut self) -> &mut String {
        &mut self.string
    }

    pub fn replace(&self) -> &Option<String> {
        &self.replace
    }

    pub fn replace_mut(&mut self) -> &mut Option<String> {
        &mut self.replace
    }
}