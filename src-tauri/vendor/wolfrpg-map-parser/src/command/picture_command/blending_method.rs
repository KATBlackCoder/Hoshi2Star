#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum BlendingMethod {
    Normal      = 0x00,
    Add         = 0x01,
    Subtract    = 0x02,
    Multiply    = 0x03,
    Same        = 0x0f,
    Unknown
}

impl BlendingMethod {
    pub const fn new(blending_method: u8) -> Self {
        match blending_method {
            0x00 => BlendingMethod::Normal,
            0x01 => BlendingMethod::Add,
            0x02 => BlendingMethod::Subtract,
            0x03 => BlendingMethod::Multiply,
            0x0f => BlendingMethod::Same,
            _ => BlendingMethod::Unknown
        }
    }
}