use crate::byte_utils::as_u32_le;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Dynamic {
    source: u32
}

impl Dynamic {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let source: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        offset += 3; // Command end signature

        (offset, Self {
            source
        })
    }

    pub fn source(&self) -> u32 {
        self.source
    }

    pub fn source_mut(&mut self) -> &mut u32 {
        &mut self.source
    }
}