#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::as_u32_le;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Base {
    process_time: u32
}

impl Base {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let process_time: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        (offset, Self {
            process_time
        })
    }

    pub fn process_time(&self) -> u32 {
        self.process_time
    }

    pub fn process_time_mut(&mut self) -> &mut u32 {
        &mut self.process_time
    }
}