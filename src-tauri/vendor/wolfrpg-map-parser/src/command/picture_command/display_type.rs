#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum DisplayType {
    Base              = 0x00,
    StringVar         = 0x10,
    StringAsPicture   = 0x20,
    Window            = 0x30,
    WindowByStringVar = 0x40,
    Unknown
}

impl DisplayType {
    pub const fn new(display_type: u8) -> Self {
        match display_type & 0xf0 {
            0x00 => DisplayType::Base,
            0x10 => DisplayType::StringVar,
            0x20 => DisplayType::StringAsPicture,
            0x30 => DisplayType::Window,
            0x40 => DisplayType::WindowByStringVar,
            _ => DisplayType::Unknown
        }
    }
}