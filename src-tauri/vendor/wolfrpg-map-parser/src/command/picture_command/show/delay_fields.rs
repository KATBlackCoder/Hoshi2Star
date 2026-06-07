use crate::byte_utils::as_u32_le;
use crate::command::picture_command::colors::Colors;
use crate::command::picture_command::show::colors_fields::ColorsFields;
use crate::command::picture_command::show::parsable_fields::ParsableFields;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct DelayFields {
    colors_state: ColorsFields,
    delay: u32
}

impl DelayFields {
    pub fn colors(&self) -> &Colors {
        self.colors_state.colors()
    }

    pub fn colors_mut(&mut self) -> &mut Colors {
        self.colors_state.colors_mut()
    }

    pub fn delay(&self) -> u32 {
        self.delay
    }

    pub fn delay_mut(&mut self) -> &mut u32 {
        &mut self.delay
    }
}

impl ParsableFields<DelayFields> for DelayFields {
    fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let (bytes_read, colors_state): (usize, ColorsFields) = ColorsFields::parse(bytes);
        offset += bytes_read;

        let delay: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        (offset, Self {
            colors_state,
            delay
        })
    }
}