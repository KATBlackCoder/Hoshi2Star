#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum DisplayOperation {
    LoadFile             = 0x00,
    Move                 = 0x01,
    Erase                = 0x02,
    DelayReset           = 0x03,
    Unknown
}

impl DisplayOperation {
    pub const fn new(display_type: u8) -> Self {
        match display_type & 0x0f {
            0x00 => DisplayOperation::LoadFile,
            0x01 => DisplayOperation::Move,
            0x02 => DisplayOperation::Erase,
            0x03 => DisplayOperation::DelayReset,
            _ => DisplayOperation::Unknown
        }
    }
}