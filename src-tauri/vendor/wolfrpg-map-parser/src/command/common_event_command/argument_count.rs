#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct ArgumentCount {
    number_arguments: u8,
    string_arguments: u8
}

impl ArgumentCount {
    pub fn new(argument_count: u8) -> Self {
        Self {
            number_arguments: argument_count & 0x0f,
            string_arguments: argument_count >> 4,
        }
    }

    pub fn number_arguments(&self) -> u8 {
        self.number_arguments
    }

    pub fn number_arguments_mut(&mut self) -> &mut u8 {
        &mut self.number_arguments
    }

    pub fn string_arguments(&self) -> u8 {
        self.string_arguments
    }

    pub fn string_arguments_mut(&mut self) -> &mut u8 {
        &mut self.string_arguments
    }
}