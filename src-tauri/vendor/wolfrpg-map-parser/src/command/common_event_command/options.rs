#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Options {
    is_arg1_string: bool,
    is_arg2_string: bool,
    is_arg3_string: bool,
    is_arg4_string: bool,
    has_return_value: bool,
}

impl Options {
    pub fn new(options: [u8; 3]) -> Self {
        Self {
            is_arg1_string:   options[0] & 0b00010000 != 0,
            is_arg2_string:   options[0] & 0b00100000 != 0,
            is_arg3_string:   options[0] & 0b01000000 != 0,
            is_arg4_string:   options[0] & 0b10000000 != 0,
            has_return_value: options[2] & 0b00000001 != 0,
        }
    }

    pub fn is_arg_string(&self, arg: u8) -> bool {
        match arg {
            1 => self.is_arg1_string,
            2 => self.is_arg2_string,
            3 => self.is_arg3_string,
            4 => self.is_arg4_string,
            _ => panic!("Invalid argument: arg must be an integer between 1 and 4, {arg} provided")
        }
    }

    pub fn set_arg_string(&mut self, arg: u8, value: bool) {
        match arg {
            1 => self.is_arg1_string = value,
            2 => self.is_arg2_string = value,
            3 => self.is_arg3_string = value,
            4 => self.is_arg4_string = value,
            _ => panic!("Invalid argument: arg must be an integer between 1 and 4, {arg} provided")
        }
    }

    pub fn has_return_value(&self) -> bool {
        self.has_return_value
    }

    pub fn has_return_value_mut(&mut self) -> &mut bool {
        &mut self.has_return_value
    }

    pub fn string_argument_count(&self) -> u8 {
        self.is_arg1_string as u8
        + self.is_arg2_string as u8
        + self.is_arg3_string as u8
        + self.is_arg4_string as u8
    }
}