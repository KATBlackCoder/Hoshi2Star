#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum Target {
    CurrentMapId            = 0x00000000,
    PlayingBGM              = 0x00000001,
    PlayingBGS              = 0x00000002,
    BGMPosition             = 0x00000003,
    BGSPosition             = 0x00000004,
    CurrentBGMLength        = 0x00000005,
    CurrentBGSLength        = 0x00000006,
    MouseLeftClick          = 0x00000007,
    MouseRightClick         = 0x00000008,
    MouseMiddleClick        = 0x00000009,
    MouseWheelDelta         = 0x0000000a,
    MouseXDelta             = 0x0000000b,
    MouseYDelta             = 0x0000000c,
    EventIDAtMousePosition  = 0x0000000d,
    CallerId                = 0x0000000e,
    MapWidth                = 0x0000000f,
    MapHeight               = 0x00000010,
    ThisCommonId            = 0x00000011,
    ActiveEventId           = 0x00000012,
    ActiveEventLine         = 0x00000013,
    MouseX                  = 0x00000014,
    MouseY                  = 0x00000015,
    Unknown
}

impl Target {
    pub const fn new(target: u32) -> Self {
        match target {
            0x00000000 => Target::CurrentMapId,
            0x00000001 => Target::PlayingBGM,
            0x00000002 => Target::PlayingBGS,
            0x00000003 => Target::BGMPosition,
            0x00000004 => Target::BGSPosition,
            0x00000005 => Target::CurrentBGMLength,
            0x00000006 => Target::CurrentBGSLength,
            0x00000007 => Target::MouseLeftClick,
            0x00000008 => Target::MouseRightClick,
            0x00000009 => Target::MouseMiddleClick,
            0x0000000a => Target::MouseWheelDelta,
            0x0000000b => Target::MouseXDelta,
            0x0000000c => Target::MouseYDelta,
            0x0000000d => Target::EventIDAtMousePosition,
            0x0000000e => Target::CallerId,
            0x0000000f => Target::MapWidth,
            0x00000010 => Target::MapHeight,
            0x00000011 => Target::ThisCommonId,
            0x00000012 => Target::ActiveEventId,
            0x00000013 => Target::ActiveEventLine,
            0x00000014 => Target::MouseX,
            0x00000015 => Target::MouseY,
            _ => Target::Unknown
        }
    }
}