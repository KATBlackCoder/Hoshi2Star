use crate::command::picture_command::colors::Colors;
use crate::command::picture_command::show::free_transform_fields::FreeTransformFields;
use crate::command::picture_command::show::parser::parse_fields;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct FreeTransform {
    top_left_x: u32,
    top_left_y: u32,
    fields: FreeTransformFields
}

impl FreeTransform {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Option<u32>, Self) {
        let (offset, (top_left_x, top_left_y, filename_variable, fields))
            : (usize, (u32, u32, Option<u32>, FreeTransformFields))
            = parse_fields(bytes);

        (offset, filename_variable, Self {
            top_left_x,
            top_left_y,
            fields
        })
    }

    pub fn top_left_x(&self) -> u32 {
        self.top_left_x
    }

    pub fn top_left_x_mut(&mut self) -> &mut u32 {
        &mut self.top_left_x
    }

    pub fn top_left_y(&self) -> u32 {
        self.top_left_y
    }

    pub fn top_left_y_mut(&mut self) -> &mut u32 {
        &mut self.top_left_y
    }

    pub fn colors(&self) -> &Colors {
        self.fields.colors()
    }

    pub fn colors_mut(&mut self) -> &mut Colors {
        self.fields.colors_mut()
    }

    pub fn delay(&self) -> u32 {
        self.fields.delay()
    }

    pub fn delay_mut(&mut self) -> &mut u32 {
        self.fields.delay_mut()
    }

    pub fn range_count(&self) -> u32 {
        self.fields.range_count()
    }

    pub fn range_count_mut(&mut self) -> &mut u32 {
        self.fields.range_count_mut()
    }

    pub fn color_values(&self) -> &[u32; 3] {
        self.fields.color_values()
    }

    pub fn color_values_mut(&mut self) -> &mut [u32; 3] {
        self.fields.color_values_mut()
    }

    pub fn zoom_height(&self) -> u32 {
        self.fields.zoom_height()
    }

    pub fn zoom_height_mut(&mut self) -> &mut u32 {
        self.fields.zoom_height_mut()
    }

    pub fn top_right_x(&self) -> u32 {
        self.fields.top_right_x()
    }

    pub fn top_right_x_mut(&mut self) -> &mut u32 {
        self.fields.top_right_x_mut()
    }

    pub fn top_right_y(&self) -> u32 {
        self.fields.top_right_y()
    }

    pub fn top_right_y_mut(&mut self) -> &mut u32 {
        self.fields.top_right_y_mut()
    }

    pub fn bottom_left_x(&self) -> u32 {
        self.fields.bottom_left_x()
    }

    pub fn bottom_left_x_mut(&mut self) -> &mut u32 {
        self.fields.bottom_left_x_mut()
    }

    pub fn bottom_left_y(&self) -> u32 {
        self.fields.bottom_left_y()
    }

    pub fn bottom_left_y_mut(&mut self) -> &mut u32 {
        self.fields.bottom_left_y_mut()
    }

    pub fn bottom_right_x(&self) -> u32 {
        self.fields.bottom_right_x()
    }

    pub fn bottom_right_x_mut(&mut self) -> &mut u32 {
        self.fields.bottom_right_x_mut()
    }

    pub fn bottom_right_y(&self) -> u32 {
        self.fields.bottom_right_y()
    }

    pub fn bottom_right_y_mut(&mut self) -> &mut u32 {
        self.fields.bottom_right_y_mut()
    }
}