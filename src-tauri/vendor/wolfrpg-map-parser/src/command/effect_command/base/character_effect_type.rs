#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum CharacterEffectType {
    Flash           = 0x00,
    Shake           = 0x01,
    SwitchFlicker   = 0x02,
    SwitchAutoFlash = 0x03,
    Unknown
}

impl CharacterEffectType {
    pub const fn new(effect_type: u8) -> Self {
        match effect_type {
            0x00 => CharacterEffectType::Flash,
            0x01 => CharacterEffectType::Shake,
            0x02 => CharacterEffectType::SwitchFlicker,
            0x03 => CharacterEffectType::SwitchAutoFlash,
            _ => CharacterEffectType::Unknown
        }
    }
}