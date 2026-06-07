#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum Operation {
    Remove          = 0x00,
    Insert          = 0x01,
    Replace         = 0x02,
    RemoveGraphic   = 0x03,
    Special         = 0x04,
    Unknown
}

impl Operation {
    pub const fn new(operation: u8) -> Self {
        match operation {
            0x00 => Operation::Remove,
            0x01 => Operation::Insert,
            0x02 => Operation::Replace,
            0x03 => Operation::RemoveGraphic,
            0x04 => Operation::Special,
            _ => Operation::Unknown
        }
    }
}