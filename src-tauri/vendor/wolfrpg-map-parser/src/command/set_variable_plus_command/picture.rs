use crate::byte_utils::as_u32_le;
use crate::command::set_variable_plus_command::picture_field::PictureField;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Picture {
    picture_number: u32,
    field: PictureField
}

impl Picture {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        offset += 2; // Unused in this variant

        let picture_number: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let field: u32 = as_u32_le(&bytes[offset..offset + 4]);
        let field: PictureField = PictureField::new(field);
        offset += 4;

        (offset, Picture {
            picture_number,
            field
        })
    }

    pub fn picture_number(&self) -> u32 {
        self.picture_number
    }

    pub fn picture_number_mut(&mut self) -> &mut u32 {
        &mut self.picture_number
    }

    pub fn field(&self) -> &PictureField {
        &self.field
    }

    pub fn field_mut(&mut self) -> &mut PictureField {
        &mut self.field
    }
}