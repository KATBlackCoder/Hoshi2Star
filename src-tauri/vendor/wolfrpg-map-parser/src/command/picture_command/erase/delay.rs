#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::as_u32_le;
use crate::command::picture_command::erase::base::Base;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Delay {
    base_fields: Base,
    delay: u32,
}

impl Delay {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let (bytes_read, base_fields): (usize, Base)
            = Base::parse(&bytes[offset..]);
        offset += bytes_read;

        let delay: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        (offset, Self {
            base_fields,
            delay
        })
    }

    pub fn process_time(&self) -> u32 {
        self.base_fields.process_time()
    }

    pub fn process_time_mut(&mut self) -> &mut u32 {
        self.base_fields.process_time_mut()
    }

    pub fn delay(&self) -> u32 {
        self.delay
    }

    pub fn delay_mut(&mut self) -> &mut u32 {
        &mut self.delay
    }
}