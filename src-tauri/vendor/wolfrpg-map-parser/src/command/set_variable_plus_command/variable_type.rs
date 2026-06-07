#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum VariableType {
    Character       = 0b00000001,
    Position        = 0b00000010,
    Other           = 0b00000011,
    PictureNumber   = 0b00000100,
    Unknown
}

impl VariableType {
    pub const fn new(command_type: u8) -> Self {
        match command_type {
            0b00000001 => VariableType::Character,
            0b00000010 => VariableType::Position,
            0b00000011 => VariableType::Other,
            0b00000100 => VariableType::PictureNumber,
            _ => VariableType::Unknown
        }
    }
}