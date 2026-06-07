#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum MouseTarget {
    Click       = 0x00,
    XPos        = 0x01,
    YPos        = 0x02,
    WheelDelta  = 0x03,
    Unknown
}

impl MouseTarget {
    pub const fn new(option: u8) -> Self {
        match option {
            0x00 => MouseTarget::Click,
            0x01 => MouseTarget::XPos,
            0x02 => MouseTarget::YPos,
            0x03 => MouseTarget::WheelDelta,
            _ => MouseTarget::Unknown
        }
    }
}