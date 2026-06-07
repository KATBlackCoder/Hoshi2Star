#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum Colors {
    Same        = 0x01,
    Different   = 0x02,
    Unknown
}

impl Colors {
    pub const fn new(colors: u8) -> Self {
        match colors {
            0x01 => Colors::Same,
            0x02 => Colors::Different,
            _ => Colors::Unknown
        }
    }
}