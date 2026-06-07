#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::input_key_command::automatic_input::basic_options::BasicOptions;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Basic {
    options: BasicOptions
}

impl Basic {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let options: u8 = bytes[offset];
        let options: BasicOptions = BasicOptions::new(options);
        offset += 1;

        offset += 2; // Padding

        offset += 1; // input_type

        (offset, Self {
            options
        })
    }

    pub fn options(&self) -> &BasicOptions {
        &self.options
    }

    pub fn options_mut(&mut self) -> &mut BasicOptions {
        &mut self.options
    }
}