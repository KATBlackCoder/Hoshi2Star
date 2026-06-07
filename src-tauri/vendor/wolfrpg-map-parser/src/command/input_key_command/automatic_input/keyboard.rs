#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::as_u32_le;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Keyboard {
    key_code: u32
}

impl Keyboard {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        offset += 3; // padding

        offset += 1; // input_type

        let key_code: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        (offset, Self {
            key_code
        })
    }
    pub fn key_code(&self) -> u32 {
        self.key_code
    }

    pub fn key_code_mut(&mut self) -> &mut u32 {
        &mut self.key_code
    }
}