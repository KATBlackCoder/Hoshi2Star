#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::as_u32_le;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Wait {
    frame_count: u32
}

impl Wait {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let frame_count: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        offset += 3; // Command end signature

        (offset, Self {
            frame_count
        })
    }

    pub fn frame_count(&self) -> u32 {
        self.frame_count
    }
    
    pub fn frame_count_mut(&mut self) -> &mut u32 {
        &mut self.frame_count
    }
}