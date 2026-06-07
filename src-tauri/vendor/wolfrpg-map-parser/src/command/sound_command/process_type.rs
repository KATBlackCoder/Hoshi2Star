#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum ProcessType {
    Playback    = 0x00,
    Preload     = 0x01,
    FreeMemory  = 0x03,
    Unknown
}

impl ProcessType {
    pub const fn new(process_type: u8) -> Self {
        match process_type {
            0x00 => ProcessType::Playback,
            0x01 => ProcessType::Preload,
            0x03 => ProcessType::FreeMemory,
            _ => ProcessType::Unknown
        }
    }
}