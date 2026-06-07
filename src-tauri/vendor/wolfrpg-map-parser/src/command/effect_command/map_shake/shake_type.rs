#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum ShakeType {
    Vertical    = 0x00,
    Horizontal  = 0x01,
    Stop        = 0x02,
    Unknown
}

impl ShakeType {
    pub const fn new(shake_type: u8) -> Self {
        match shake_type {
            0x00 => ShakeType::Vertical,
            0x01 => ShakeType::Horizontal,
            0x02 => ShakeType::Stop,
            _ => ShakeType::Unknown
        }
    }
}