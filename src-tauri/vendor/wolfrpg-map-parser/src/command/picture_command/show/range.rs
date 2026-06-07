use crate::command::picture_command::colors::Colors;
use crate::command::picture_command::show::parser::parse_fields;
use crate::command::picture_command::show::range_fields::RangeFields;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Range {
    position_x: u32,
    position_y: u32,
    fields: RangeFields
}

impl Range {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Option<u32>, Self) {
        let (offset, (position_x, position_y, filename_variable, fields))
            : (usize, (u32, u32, Option<u32>, RangeFields))
            = parse_fields(bytes);

        (offset, filename_variable, Self {
            position_x,
            position_y,
            fields
        })
    }

    pub fn position_x(&self) -> u32 {
        self.position_x
    }

    pub fn position_x_mut(&mut self) -> &mut u32 {
        &mut self.position_x
    }

    pub fn position_y(&self) -> u32 {
        self.position_y
    }

    pub fn position_y_mut(&mut self) -> &mut u32 {
        &mut self.position_y
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
}