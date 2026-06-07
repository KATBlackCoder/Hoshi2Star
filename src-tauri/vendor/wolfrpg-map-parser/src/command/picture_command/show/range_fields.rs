use crate::byte_utils::as_u32_le;
use crate::command::picture_command::colors::Colors;
use crate::command::picture_command::show::delay_fields::DelayFields;
use crate::command::picture_command::show::parsable_fields::ParsableFields;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct RangeFields {
    delay_state: DelayFields,
    range_count: u32
}

impl RangeFields {
    pub fn colors(&self) -> &Colors {
        self.delay_state.colors()
    }

    pub fn colors_mut(&mut self) -> &mut Colors {
        self.delay_state.colors_mut()
    }

    pub fn delay(&self) -> u32 {
        self.delay_state.delay()
    }

    pub fn delay_mut(&mut self) -> &mut u32 {
        self.delay_state.delay_mut()
    }

    pub fn range_count(&self) -> u32 {
        self.range_count
    }

    pub fn range_count_mut(&mut self) -> &mut u32 {
        &mut self.range_count
    }
}

impl ParsableFields<RangeFields> for RangeFields {
    fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let (bytes_read, delay_state): (usize, DelayFields) = DelayFields::parse(bytes);
        offset += bytes_read;

        let range_count: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        (offset, Self {
            delay_state,
            range_count
        })
    }
}