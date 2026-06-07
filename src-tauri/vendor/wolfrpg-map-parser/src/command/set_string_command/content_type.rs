#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum ContentType {
    StringLiteral   = 0b00000000,
    StringVariable  = 0b00000001,
    Variable        = 0b00000010,
    UserInput       = 0b00000011,
    Unknown
}

impl ContentType {
    pub const fn new(content_type: u8) -> Self {
        match content_type {
            0b00000000 => ContentType::StringLiteral,
            0b00000001 => ContentType::StringVariable,
            0b00000010 => ContentType::Variable,
            0b00000011 => ContentType::UserInput,
            _ => ContentType::Unknown
        }
    }
}