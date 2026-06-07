use crate::command::picture_command::colors::Colors as PictureColors;
use crate::command::picture_command::show::colors_fields::ColorsFields;
use crate::command::picture_command::show::parser::parse_fields;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Colors {
    position_x: u32,
    position_y: u32,
    fields: ColorsFields
}

impl Colors {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Option<u32>, Self) {
        let (offset, (position_x, position_y, filename_variable, fields))
            : (usize, (u32, u32, Option<u32>, ColorsFields))
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

    pub fn colors(&self) -> &PictureColors {
        self.fields.colors()
    }

    pub fn colors_mut(&mut self) -> &mut PictureColors {
        self.fields.colors_mut()
    }
}