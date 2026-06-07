use crate::byte_utils::as_u32_le;
use crate::command::picture_command::colors::Colors;
use crate::command::picture_command::show::color_values_fields::ColorValuesFields;
use crate::command::picture_command::show::parsable_fields::ParsableFields;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct ZoomFields {
    color_values_state: ColorValuesFields,
    zoom_height: u32
}

impl ZoomFields {
    pub fn colors(&self) -> &Colors {
        self.color_values_state.colors()
    }

    pub fn colors_mut(&mut self) -> &mut Colors {
        self.color_values_state.colors_mut()
    }

    pub fn delay(&self) -> u32 {
        self.color_values_state.delay()
    }

    pub fn delay_mut(&mut self) -> &mut u32 {
        self.color_values_state.delay_mut()
    }

    pub fn range_count(&self) -> u32 {
        self.color_values_state.range_count()
    }

    pub fn range_count_mut(&mut self) -> &mut u32 {
        self.color_values_state.range_count_mut()
    }

    pub fn color_values(&self) -> &[u32; 3] {
        self.color_values_state.color_values()
    }

    pub fn color_values_mut(&mut self) -> &mut [u32; 3] {
        self.color_values_state.color_values_mut()
    }

    pub fn zoom_height(&self) -> u32 {
        self.zoom_height
    }

    pub fn zoom_height_mut(&mut self) -> &mut u32 {
        &mut self.zoom_height
    }
}

impl ParsableFields<ZoomFields> for ZoomFields {
    fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let (bytes_read, color_values_state): (usize, ColorValuesFields)
            = ColorValuesFields::parse(bytes);
        offset += bytes_read;

        let zoom_height: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        (offset, Self {
            color_values_state,
            zoom_height
        })
    }
}