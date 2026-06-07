#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum AssignmentOperator {
    Equals          = 0x00,
    PlusEquals      = 0x01,
    MinusEquals     = 0x02,
    TimesEquals     = 0x03,
    DivideEquals    = 0x04,
    RemainderEquals = 0x05,
    LowerBound      = 0x06,
    UpperBound      = 0x07,
    Absolute        = 0x08,
    Unknown
}

impl AssignmentOperator {
    pub const fn new(operator: u8) -> Self {
        match operator {
            0x00 => AssignmentOperator::Equals,
            0x01 => AssignmentOperator::PlusEquals,
            0x02 => AssignmentOperator::MinusEquals,
            0x03 => AssignmentOperator::TimesEquals,
            0x04 => AssignmentOperator::DivideEquals,
            0x05 => AssignmentOperator::RemainderEquals,
            0x06 => AssignmentOperator::LowerBound,
            0x07 => AssignmentOperator::UpperBound,
            0x08 => AssignmentOperator::Absolute,
            _ => AssignmentOperator::Unknown
        }
    }
}