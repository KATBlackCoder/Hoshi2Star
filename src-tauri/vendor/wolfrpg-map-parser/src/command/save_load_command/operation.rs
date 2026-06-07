#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum Operation {
    Save = 0x00000000,
    Load = 0x00000001,
    Unknown
}

impl Operation {
    pub const fn new(operation: u32) -> Self {
        match operation {
            0x00000000 => Operation::Save,
            0x00000001 => Operation::Load,
            _ => Operation::Unknown
        }
    }
}