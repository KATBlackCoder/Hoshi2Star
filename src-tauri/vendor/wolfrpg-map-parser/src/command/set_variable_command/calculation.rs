#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum Calculation {
    Plus        = 0x00,
    Minus       = 0x01,
    Times       = 0x02,
    Divides     = 0x03,
    Remainder   = 0x04,
    BitwiseAnd  = 0x05,
    Random      = 0x06,
    /// Used only for angle calculations, means the left and right side of the calculation
    /// should not be used to calculate the complexive right side before assignment
    Nothing     = 0x0F,
    Unknown
}

impl Calculation {
    pub const fn from_u8(calculation: u8) -> Self {
        match calculation {
            0x00 => Calculation::Plus,
            0x01 => Calculation::Minus,
            0x02 => Calculation::Times,
            0x03 => Calculation::Divides,
            0x04 => Calculation::Remainder,
            0x05 => Calculation::BitwiseAnd,
            0x06 => Calculation::Random,
            0x0F => Calculation::Nothing,
            _ => Calculation::Unknown
        }
    }
}
