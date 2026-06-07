use crate::byte_utils::parse_string;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct ShowTextCommand {
    text: String
}

impl ShowTextCommand {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;
        offset += 2; // Unknown, probably padding

        let (bytes_read, text): (usize, String) = parse_string(&bytes[offset..]);
        offset += bytes_read;

        offset += 1; // command end byte, should be 0x00

        (offset, Self {
            text
        })
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn text_mut(&mut self) -> &mut String {
        &mut self.text
    }
}