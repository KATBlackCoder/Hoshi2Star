#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum PictureField {
    PositionX               = 0x00000000,
    PositionY               = 0x00000001,
    Width                   = 0x00000002,
    Height                  = 0x00000003,
    PatternNumber           = 0x00000004,
    Opacity                 = 0x00000005,
    Angle                   = 0x00000006,
    CursorHover             = 0x00000008,
    PictureInUse            = 0x00000009,
    StringDoneDisplaying    = 0x0000000a,
    ZoomWidth               = 0x0000000b,
    ZoomHeight              = 0x0000000c,
    TopLeftX                = 0x0000000d,
    TopLeftY                = 0x0000000e,
    TopRightX               = 0x0000000f,
    TopRightY               = 0x00000010,
    BottomLeftX             = 0x00000011,
    BottomLeftY             = 0x00000012,
    BottomRightX            = 0x00000013,
    BottomRightY            = 0x00000014,
    Unknown
}

impl PictureField {
    pub const fn new(field: u32) -> Self {
        match field {
            0x00000000 => PictureField::PositionX,
            0x00000001 => PictureField::PositionY,
            0x00000002 => PictureField::Width,
            0x00000003 => PictureField::Height,
            0x00000004 => PictureField::PatternNumber,
            0x00000005 => PictureField::Opacity,
            0x00000006 => PictureField::Angle,
            0x00000008 => PictureField::CursorHover,
            0x00000009 => PictureField::PictureInUse,
            0x0000000a => PictureField::StringDoneDisplaying,
            0x0000000b => PictureField::ZoomWidth,
            0x0000000c => PictureField::ZoomHeight,
            0x0000000d => PictureField::TopLeftX,
            0x0000000e => PictureField::TopLeftY,
            0x0000000f => PictureField::TopRightX,
            0x00000010 => PictureField::TopRightY,
            0x00000011 => PictureField::BottomLeftX,
            0x00000012 => PictureField::BottomLeftY,
            0x00000013 => PictureField::BottomRightX,
            0x00000014 => PictureField::BottomRightY,
            _ => PictureField::Unknown,
        }
    }
}