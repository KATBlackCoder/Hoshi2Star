use crate::byte_utils::as_u32_le;
use crate::command::set_variable_plus_command::character_field::CharacterField;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Character {
    character: u32,
    field: CharacterField,
}

impl Character {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        offset += 2; // Unused in this variant

        let character: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let field: u32 = as_u32_le(&bytes[offset..offset + 4]);
        let field: CharacterField = CharacterField::new(field);
        offset += 4;

        (offset, Self {
            character,
            field,
        })
    }

    pub fn character(&self) -> u32 {
        self.character
    }

    pub fn character_mut(&mut self) -> &mut u32 {
        &mut self.character
    }

    pub fn field(&self) -> &CharacterField {
        &self.field
    }

    pub fn field_mut(&mut self) -> &mut CharacterField {
        &mut self.field
    }
}