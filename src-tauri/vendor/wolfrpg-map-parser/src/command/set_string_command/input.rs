use crate::byte_utils::as_u32_le;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Input {
    max_length: u32
}

impl Input {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let max_length: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        offset += 3; // Command end signature

        (offset, Self {
            max_length
        })
    }

    pub fn max_length(&self) -> u32 {
        self.max_length
    }

    pub fn max_length_mut(&mut self) -> &mut u32 {
        &mut self.max_length
    }
}