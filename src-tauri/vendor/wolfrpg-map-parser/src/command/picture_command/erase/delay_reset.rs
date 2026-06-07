#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::as_u32_le;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct DelayReset {
    range_count: Option<u32>
}

impl DelayReset {
    pub(crate) fn parse(bytes: &[u8], range: bool) -> (usize, Self) {
        let mut offset: usize = 0;

        let range_count: Option<u32> = if range {
            let range_count: u32 = as_u32_le(&bytes[offset..offset+4]);
            offset += 4;

            Some(range_count)
        } else {
            None
        };

        (offset, Self {
            range_count
        })
    }

    pub fn range_count(&self) -> Option<u32> {
        self.range_count
    }

    pub fn range_count_mut(&mut self) -> &mut Option<u32> {
        &mut self.range_count
    }
}