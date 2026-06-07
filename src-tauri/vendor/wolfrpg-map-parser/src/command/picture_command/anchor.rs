#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum Anchor {
    TopLeft     = 0x00,
    Center      = 0x01,
    BottomLeft  = 0x02,
    TopRight    = 0x03,
    BottomRight = 0x04,
    Unknown
}

impl Anchor {
    pub const fn new(anchor: u8) -> Self {
        match anchor {
            0x00 => Anchor::TopLeft,
            0x01 => Anchor::Center,
            0x02 => Anchor::BottomLeft,
            0x03 => Anchor::TopRight,
            0x04 => Anchor::BottomRight,
            _ => Anchor::Unknown
        }
    }
}