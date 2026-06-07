#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum SoundType {
    DBEntry     = 0x00,
    Variable    = 0x01,
    Filename    = 0x02,
    Unknown
}

impl SoundType {
    pub const fn new(sound_type: u8) -> Self {
        match sound_type {
            0x00 => SoundType::DBEntry,
            0x01 => SoundType::Variable,
            0x02 => SoundType::Filename,
            _ => SoundType::Unknown
        }
    }
}