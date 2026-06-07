#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum CompareOperator {
    Equals      = 0x00,
    NotEquals   = 0x01,
    Includes    = 0x02,
    StartsWith  = 0x03,
    Unknown
}

impl CompareOperator {
    pub const fn new(operator: u8) -> Self {
        match operator {
            0x00 => CompareOperator::Equals,
            0x01 => CompareOperator::NotEquals,
            0x02 => CompareOperator::Includes,
            0x03 => CompareOperator::StartsWith,
            _ => CompareOperator::Unknown
        }
    }
}