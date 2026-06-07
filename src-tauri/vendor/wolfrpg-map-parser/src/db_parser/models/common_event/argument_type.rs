#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum ArgumentType {
    Base      = 0x00,
    DBRef     = 0x01,
    CreateOpt = 0x02,
    Unknown
}

impl ArgumentType {
    pub const fn new(argument_type: u8) -> Self {
        match argument_type {
            0x00 => Self::Base,
            0x01 => Self::DBRef,
            0x02 => Self::CreateOpt,
            _ => Self::Unknown
        }
    }
}