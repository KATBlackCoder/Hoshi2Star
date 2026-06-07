#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum EnabledInputs {
    AllEnabled  = 0x00,
    NoMovement  = 0x01,
    AllDisabled = 0x02,
    Unknown
}

impl EnabledInputs {
    pub const fn new(input: u8) -> Self {
        match input {
            0x00 => EnabledInputs::AllEnabled,
            0x01 => EnabledInputs::NoMovement,
            0x02 => EnabledInputs::AllDisabled,
            _ => EnabledInputs::Unknown
        }
    }
}