#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum MouseType {
    Click       = 0x00,
    Position    = 0x01,
    Wheel       = 0x02,
    Unknown
}

impl MouseType {
    pub const fn new(mouse_type: u8) -> Self {
        match mouse_type {
            0x00 => MouseType::Click,
            0x01 => MouseType::Position,
            0x02 => MouseType::Wheel,
            _ => MouseType::Unknown
        }
    }
}