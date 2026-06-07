#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum InputType {
    Basic       = 0x00,
    KeyBoard    = 0x01,
    Pad         = 0x02,
    Mouse       = 0x03,
    Unknown
}

impl InputType {
    pub const fn new(input_type: u8) -> Self {
        match input_type {
            0x00 => InputType::Basic,
            0x01 => InputType::KeyBoard,
            0x02 => InputType::Pad,
            0x03 => InputType::Mouse,
            _ => InputType::Unknown
        }
    }
}