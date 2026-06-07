#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum CompareOperator {
    GreaterThan     = 0x00,
    GreaterOrEquals = 0x01,
    Equals          = 0x02,
    LessOrEquals    = 0x03,
    LessThan        = 0x04,
    NotEqual        = 0x05,
    BitwiseAnd      = 0x06,
    Unknown
}

impl CompareOperator {
    pub const fn new(operator: u8) -> Self {
        match operator {
            0x00 => CompareOperator::GreaterThan,
            0x01 => CompareOperator::GreaterOrEquals,
            0x02 => CompareOperator::Equals,
            0x03 => CompareOperator::LessOrEquals,
            0x04 => CompareOperator::LessThan,
            0x05 => CompareOperator::NotEqual,
            0x06 => CompareOperator::BitwiseAnd,
            _ => CompareOperator::Unknown
        }
    }
}