#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum DirectionKeys {
    No          = 0x00,
    Dir4        = 0x01,
    Dir8        = 0x02,
    Up          = 0x03,
    Down        = 0x04,
    Left        = 0x05,
    Right       = 0x06,
    UpDown      = 0x07,
    LeftRight   = 0x08,
    Unknown
}

impl DirectionKeys {
    pub const fn new(direction: u8) -> Self {
        match direction {
            0x00 => Self::No,
            0x01 => Self::Dir4,
            0x02 => Self::Dir8,
            0x03 => Self::Up,
            0x04 => Self::Down,
            0x05 => Self::Left,
            0x06 => Self::Right,
            0x07 => Self::UpDown,
            0x08 => Self::LeftRight,
            _ => Self::Unknown,
        }
    }
}