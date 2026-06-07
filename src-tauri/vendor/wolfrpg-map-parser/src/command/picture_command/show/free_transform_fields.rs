use crate::byte_utils::as_u32_le;
use crate::command::picture_command::colors::Colors;
use crate::command::picture_command::show::parsable_fields::ParsableFields;
use crate::command::picture_command::show::zoom_fields::ZoomFields;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct FreeTransformFields {
    zoom_state: ZoomFields,
    top_right_x: u32,
    top_right_y: u32,
    bottom_left_x: u32,
    bottom_left_y: u32,
    bottom_right_x: u32,
    bottom_right_y: u32
}

impl FreeTransformFields {
    pub fn colors(&self) -> &Colors {
        self.zoom_state.colors()
    }

    pub fn colors_mut(&mut self) -> &mut Colors {
        self.zoom_state.colors_mut()
    }

    pub fn delay(&self) -> u32 {
        self.zoom_state.delay()
    }

    pub fn delay_mut(&mut self) -> &mut u32 {
        self.zoom_state.delay_mut()
    }

    pub fn range_count(&self) -> u32 {
        self.zoom_state.range_count()
    }

    pub fn range_count_mut(&mut self) -> &mut u32 {
        self.zoom_state.range_count_mut()
    }

    pub fn color_values(&self) -> &[u32; 3] {
        self.zoom_state.color_values()
    }

    pub fn color_values_mut(&mut self) -> &mut [u32; 3] {
        self.zoom_state.color_values_mut()
    }

    pub fn zoom_height(&self) -> u32 {
        self.zoom_state.zoom_height()
    }

    pub fn zoom_height_mut(&mut self) -> &mut u32 {
        self.zoom_state.zoom_height_mut()
    }

    pub fn top_right_x(&self) -> u32 {
        self.top_right_x
    }

    pub fn top_right_x_mut(&mut self) -> &mut u32 {
        &mut self.top_right_x
    }

    pub fn top_right_y(&self) -> u32 {
        self.top_right_y
    }

    pub fn top_right_y_mut(&mut self) -> &mut u32 {
        &mut self.top_right_y
    }

    pub fn bottom_left_x(&self) -> u32 {
        self.bottom_left_x
    }

    pub fn bottom_left_x_mut(&mut self) -> &mut u32 {
        &mut self.bottom_left_x
    }

    pub fn bottom_left_y(&self) -> u32 {
        self.bottom_left_y
    }

    pub fn bottom_left_y_mut(&mut self) -> &mut u32 {
        &mut self.bottom_left_y
    }

    pub fn bottom_right_x(&self) -> u32 {
        self.bottom_right_x
    }

    pub fn bottom_right_x_mut(&mut self) -> &mut u32 {
        &mut self.bottom_right_x
    }

    pub fn bottom_right_y(&self) -> u32 {
        self.bottom_right_y
    }

    pub fn bottom_right_y_mut(&mut self) -> &mut u32 {
        &mut self.bottom_right_y
    }
}

impl ParsableFields<FreeTransformFields> for FreeTransformFields {
    fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let (bytes_read, zoom_state): (usize, ZoomFields) = ZoomFields::parse(bytes);
        offset += bytes_read;

        let top_right_x: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let top_right_y: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let bottom_left_x: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let bottom_left_y: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let bottom_right_x: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let bottom_right_y: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        (offset, Self {
            zoom_state,
            top_right_x,
            top_right_y,
            bottom_left_x,
            bottom_left_y,
            bottom_right_x,
            bottom_right_y
        })
    }
}