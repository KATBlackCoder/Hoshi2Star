#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum PictureEffectType {
    Flash                       = 0x00,
    ColorCorrect                = 0x01,
    DrawPositionShift           = 0x02,
    Shake                       = 0x03,
    Zoom                        = 0x04,
    SwitchFlicker               = 0x05,
    SwitchAutoFlash             = 0x06,
    AutoEnlarge                 = 0x07,
    AutoPatternSwitchOne        = 0x08,
    AutoPatternSwitchLoop       = 0x09,
    AutoPatternSwitchRoundTrip  = 0x0a,
    Unknown
}

impl PictureEffectType {
    pub const fn new(effect_type: u8) -> Self {
        match effect_type {
            0x00 => PictureEffectType::Flash,
            0x01 => PictureEffectType::ColorCorrect,
            0x02 => PictureEffectType::DrawPositionShift,
            0x03 => PictureEffectType::Shake,
            0x04 => PictureEffectType::Zoom,
            0x05 => PictureEffectType::SwitchFlicker,
            0x06 => PictureEffectType::SwitchAutoFlash,
            0x07 => PictureEffectType::AutoEnlarge,
            0x08 => PictureEffectType::AutoPatternSwitchOne,
            0x09 => PictureEffectType::AutoPatternSwitchLoop,
            0x0a => PictureEffectType::AutoPatternSwitchRoundTrip,
            _ => PictureEffectType::Unknown
        }
    }
}