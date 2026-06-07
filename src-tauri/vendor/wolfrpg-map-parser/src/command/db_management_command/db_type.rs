#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum DBType {
    VarDB  = 0b00000000,
    SysDB  = 0b00000001,
    UserDB = 0b00000010,
    Unknown
}

impl DBType {
    pub const fn new(db: u8) -> Self {
        match db {
            0b00000000 => DBType::VarDB,
            0b00000001 => DBType::SysDB,
            0b00000010 => DBType::UserDB,
            _ => DBType::Unknown
        }
    }
}