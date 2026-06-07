use crate::command::input_key_command::input_key::mouse_options::MouseOptions;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Mouse {
    options: MouseOptions
}

impl Mouse {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let options: u8 = bytes[offset];
        let options: MouseOptions = MouseOptions::new(options);
        offset += 1;

        offset += 1; // input_type

        offset += 2; // Padding

        (offset, Self {
            options,
        })
    }

    pub fn options(&self) -> &MouseOptions {
        &self.options
    }

    pub fn options_mut(&mut self) -> &mut MouseOptions {
        &mut self.options
    }
}