use crate::command::picture_command::colors::Colors;
use crate::command::picture_command::show::parsable_fields::ParsableFields;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct ColorsFields {
    colors: Colors
}

impl ColorsFields {
    pub fn colors(&self) -> &Colors {
        &self.colors
    }

    pub fn colors_mut(&mut self) -> &mut Colors {
        &mut self.colors
    }
}

impl ParsableFields<ColorsFields> for ColorsFields {
    fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let colors: u8 = bytes[offset];
        let colors: Colors = Colors::new(colors);
        offset += 1;

        (offset, Self {
            colors
        })
    }
}