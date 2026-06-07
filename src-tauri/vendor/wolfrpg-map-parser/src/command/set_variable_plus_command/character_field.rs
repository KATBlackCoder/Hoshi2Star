#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum CharacterField {
    StandardX               = 0x00000000,
    StandardY               = 0x00000001,
    PreciseX                = 0x00000002,
    PreciseY                = 0x00000003,
    HeightOffGround         = 0x00000004,
    Direction               = 0x00000005,
    ScreenX                 = 0x00000006,
    ScreenY                 = 0x00000007,
    ShadowGraphicNumber     = 0x00000008,
    CurrentTileTag          = 0x00000009,
    EventId                 = 0x0000000a,
    OnScreen                = 0x0000000b,
    ActivePage              = 0x0000000c,
    RunCondition            = 0x0000000d,
    RangeExtendX            = 0x0000000e,
    RangeExtendY            = 0x0000000f,
    AnimationPattern        = 0x00000010,
    Moving                  = 0x00000011,
    Unknown
}

impl CharacterField {
    pub const fn new(field: u32) -> Self {
        match field {
            0x00000000 => CharacterField::StandardX,
            0x00000001 => CharacterField::StandardY,
            0x00000002 => CharacterField::PreciseX,
            0x00000003 => CharacterField::PreciseY,
            0x00000004 => CharacterField::HeightOffGround,
            0x00000005 => CharacterField::Direction,
            0x00000006 => CharacterField::ScreenX,
            0x00000007 => CharacterField::ScreenY,
            0x00000008 => CharacterField::ShadowGraphicNumber,
            0x00000009 => CharacterField::CurrentTileTag,
            0x0000000a => CharacterField::EventId,
            0x0000000b => CharacterField::OnScreen,
            0x0000000c => CharacterField::ActivePage,
            0x0000000d => CharacterField::RunCondition,
            0x0000000e => CharacterField::RangeExtendX,
            0x0000000f => CharacterField::RangeExtendY,
            0x00000010 => CharacterField::AnimationPattern,
            0x00000011 => CharacterField::Moving,
            _ => CharacterField::Unknown
        }
    }
}