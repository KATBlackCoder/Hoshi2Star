#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum BlendType {
    Normal      = 0x00,
    Add         = 0x01,
    Multiply    = 0x02,
    Subtract    = 0x03,
    Unknown
}

impl BlendType {
    pub const fn new(blend: u8) -> Self {
        match blend {
            0x00 => BlendType::Normal,
            0x01 => BlendType::Add,
            0x02 => BlendType::Multiply,
            0x03 => BlendType::Subtract,
            _ => BlendType::Unknown
        }
    }
}