use crate::byte_utils::as_u32_le;
use crate::command::picture_command::colors::Colors;
use crate::command::picture_command::show::parsable_fields::ParsableFields;
use crate::command::picture_command::show::range_fields::RangeFields;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct ColorValuesFields {
    range_state: RangeFields,
    color_values: [u32; 3]
}

impl ColorValuesFields {
    fn parse_color_values(bytes: &[u8]) -> (usize, [u32; 3]) {
        let mut offset: usize = 0;

        let color1: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let color2: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let color3: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        (offset, [
            color1,
            color2,
            color3
        ])
    }

    pub fn colors(&self) -> &Colors {
        self.range_state.colors()
    }

    pub fn colors_mut(&mut self) -> &mut Colors {
        self.range_state.colors_mut()
    }

    pub fn delay(&self) -> u32 {
        self.range_state.delay()
    }

    pub fn delay_mut(&mut self) -> &mut u32 {
        self.range_state.delay_mut()
    }

    pub fn range_count(&self) -> u32 {
        self.range_state.range_count()
    }

    pub fn range_count_mut(&mut self) -> &mut u32 {
        self.range_state.range_count_mut()
    }

    pub fn color_values(&self) -> &[u32; 3] {
        &self.color_values
    }

    pub fn color_values_mut(&mut self) -> &mut [u32; 3] {
        &mut self.color_values
    }
}

impl ParsableFields<ColorValuesFields> for ColorValuesFields {
    fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let (bytes_read, range_state): (usize, RangeFields) = RangeFields::parse(bytes);
        offset += bytes_read;

        let (bytes_read, color_values): (usize, [u32; 3])
            = Self::parse_color_values(&bytes[offset..]);
        offset += bytes_read;

        (offset, Self {
            range_state,
            color_values
        })
    }
}