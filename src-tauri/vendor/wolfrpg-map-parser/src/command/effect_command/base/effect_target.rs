#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum EffectTarget {
    Picture     = 0x00,
    Character   = 0x01,
    Map         = 0x02,
    Unknown
}

impl EffectTarget {
    pub const fn new(target: u8) -> Self {
        match target {
            0x00 => EffectTarget::Picture,
            0x01 => EffectTarget::Character,
            0x02 => EffectTarget::Map,
            _ => EffectTarget::Unknown
        }
    }
}