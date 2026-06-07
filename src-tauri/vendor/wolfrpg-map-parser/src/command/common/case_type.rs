#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum CaseType {
    Case    = 0x02910100,
    Extra   = 0x02920100,
    Cancel  = 0x02a50100,
    Else    = 0x02a40100,
    Unknown
}

impl CaseType {
    pub const fn new(case_type: u32) -> Self {
        match case_type {
            0x02910100 => CaseType::Case,
            0x02920100 => CaseType::Extra,
            0x02a50100 => CaseType::Cancel,
            0x02a40100 => CaseType::Else,
            _ => CaseType::Unknown
        }
    }
}

