#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum InputType {
    Basic       = 0x00,
    KeyBoard    = 0x10,
    Mouse       = 0x20,
    Unknown
}

impl InputType {
    pub const fn new(options: u8) -> InputType {
        match options {
            0x00 => Self::Basic,
            0x10 => Self::KeyBoard,
            0x20 => Self::Mouse,
            _ => Self::Unknown
        }
    }
}