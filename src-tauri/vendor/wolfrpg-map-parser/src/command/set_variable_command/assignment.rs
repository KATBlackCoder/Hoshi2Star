#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum Assignment {
    Equals          = 0x00,
    PlusEquals      = 0x01,
    MinusEquals     = 0x02,
    TimesEquals     = 0x03,
    DivideEquals    = 0x04,
    RemainderEquals = 0x05,
    LowerBound      = 0x06,
    UpperBound      = 0x07,
    Absolute        = 0x08,
    Angle           = 0x09,
    Sin             = 0x0a,
    Cos             = 0x0b,
    Unknown
}

impl Assignment {
    pub const fn from_u8(assignment: u8) -> Self {
        match assignment {
            0x00 => Assignment::Equals,
            0x01 => Assignment::PlusEquals,
            0x02 => Assignment::MinusEquals,
            0x03 => Assignment::TimesEquals,
            0x04 => Assignment::DivideEquals,
            0x05 => Assignment::RemainderEquals,
            0x06 => Assignment::LowerBound,
            0x07 => Assignment::UpperBound,
            0x08 => Assignment::Absolute,
            0x09 => Assignment::Angle,
            0x0a => Assignment::Sin,
            0x0b => Assignment::Cos,
            _ => {Assignment::Unknown}
        }
    }
}