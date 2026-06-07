#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum ConditionType {
    CallOnly       = 0x00,
    AutoStart      = 0x01,
    Parallel       = 0x02,
    ParallelAlways = 0x03,
    Unknown,
}

impl ConditionType {
    pub const fn new(condition_type: u8) -> Self {
        match condition_type {
            0x00 => ConditionType::CallOnly,
            0x01 => ConditionType::AutoStart,
            0x02 => ConditionType::Parallel,
            0x03 => ConditionType::ParallelAlways,
            _ => ConditionType::Unknown
        }
    }
}