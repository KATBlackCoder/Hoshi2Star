#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum InputType {
    Basic   = 0x00,
    Device  = 0x10,
    Unknown
}

impl InputType {
    pub const fn new(input_type: u8) -> Self {
        match input_type {
            0x00 => InputType::Basic,
            0x10 => InputType::Device,
            _ => InputType::Unknown
        }
    }
}