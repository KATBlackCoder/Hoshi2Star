#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum Target {
    SavedPosition,   // 0xefd8ffff,
    Hero,            // 0xffffffff,
    Variable(u32)
}

impl Target {
    pub fn new(target: u32) -> Self {
        match target {
            0xefd8ffff => Self::SavedPosition,
            0xffffffff => Self::Hero,
            _ => Self::Variable(target)
        }
    }
}