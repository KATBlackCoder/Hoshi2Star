#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum ScrollSpeed {
    OneEight    = 0x00,
    OneFourth   = 0x01,
    OneHalf     = 0x02,
    One         = 0x03,
    Two         = 0x04,
    Four        = 0x05,
    Eight       = 0x06,
    Sixteen     = 0x07,
    Instant     = 0x08,
    ThirtyTwo   = 0x09,
    SixtyFour   = 0x0a,
    Unknown
}

impl ScrollSpeed {
    pub const fn new(speed: u8) -> Self {
        match speed {
            0x00 => ScrollSpeed::OneEight,
            0x01 => ScrollSpeed::OneFourth,
            0x02 => ScrollSpeed::OneHalf,
            0x03 => ScrollSpeed::One,
            0x04 => ScrollSpeed::Two,
            0x05 => ScrollSpeed::Four,
            0x06 => ScrollSpeed::Eight,
            0x07 => ScrollSpeed::Sixteen,
            0x08 => ScrollSpeed::Instant,
            0x09 => ScrollSpeed::ThirtyTwo,
            0x0a => ScrollSpeed::SixtyFour,
            _ => ScrollSpeed::Unknown
        }
    }
}