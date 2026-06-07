#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::as_u32_le;
use crate::command::picture_command::erase::delay::Delay;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
#[allow(unused)]
pub struct Range {
    delay_fields: Delay,
    unknown1: u32,
    range_count: u32
}

impl Range {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let (bytes_read, delay_fields): (usize, Delay)
            = Delay::parse(&bytes[offset..]);
        offset += bytes_read;

        let unknown1: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let range_count: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        (offset, Self {
            delay_fields,
            unknown1,
            range_count
        })
    }

    pub fn process_time(&self) -> u32 {
        self.delay_fields.process_time()
    }

    pub fn process_time_mut(&mut self) -> &mut u32 {
        self.delay_fields.process_time_mut()
    }

    pub fn delay(&self) -> u32 {
        self.delay_fields.delay()
    }

    pub fn delay_mut(&mut self) -> &mut u32 {
        self.delay_fields.delay_mut()
    }

    pub fn range_count(&self) -> u32 {
        self.range_count
    }

    pub fn range_count_mut(&mut self) -> &mut u32 {
        &mut self.range_count
    }
}