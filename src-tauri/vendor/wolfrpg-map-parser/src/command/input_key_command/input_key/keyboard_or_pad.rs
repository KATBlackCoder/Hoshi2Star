use crate::byte_utils::as_u32_le;
use crate::command::input_key_command::input_key::key_options::KeyOptions;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct KeyboardOrPad {
    options: KeyOptions,
    key_code: u32
}

impl KeyboardOrPad {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let options: u8 = bytes[offset];
        let options: KeyOptions = KeyOptions::new(options);
        offset += 1;

        offset += 1; // input_type

        offset += 2; // Padding

        let key_code: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        (offset, Self {
            options,
            key_code
        })
    }

    pub fn options(&self) -> &KeyOptions {
        &self.options
    }

    pub fn options_mut(&mut self) -> &mut KeyOptions {
        &mut self.options
    }

    pub fn key_code(&self) -> u32 {
        self.key_code
    }

    pub fn key_code_mut(&mut self) -> &mut u32 {
        &mut self.key_code
    }
}