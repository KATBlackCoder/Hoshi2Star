use crate::byte_utils::{as_u16_le, as_u32_le};
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
#[allow(unused)]
pub struct Range {
    range_start: u16,
    range_end: u16,
    unknown1: u32,
}

impl Range {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset = 0;

        let range_start: u16 = as_u16_le(&bytes[offset..offset + 2]);
        offset += 2;

        let range_end: u16 = as_u16_le(&bytes[offset..offset + 2]);
        offset += 2;

        let unknown1: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        offset += 1; // command end signature

        (offset, Self {
            range_start,
            range_end,
            unknown1,
        })
    }

    pub fn range_start(&self) -> u16 {
        self.range_start
    }

    pub fn range_start_mut(&mut self) -> &mut u16 {
        &mut self.range_start
    }

    pub fn range_end(&self) -> u16 {
        self.range_end
    }

    pub fn range_end_mut(&mut self) -> &mut u16 {
        &mut self.range_end
    }
}