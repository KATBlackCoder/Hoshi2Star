#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::save_load_command::parser::parse_variable_fields;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct LoadVariable {
    target_variable: u32,
    save_number: u32,
    source_variable: u32,
    source_is_pointer: bool
}

impl LoadVariable {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let (mut offset, (target_variable, save_number, source_variable, source_is_pointer))
            : (usize, (u32, u32, u32, bool)) = parse_variable_fields(bytes);

        offset += 3; // Command end signature

        (offset, Self {
            target_variable,
            save_number,
            source_variable,
            source_is_pointer
        })
    }

    pub fn target_variable(&self) -> u32 {
        self.target_variable
    }
    
    pub fn target_variable_mut(&mut self) -> &mut u32 {
        &mut self.target_variable
    }

    pub fn save_number(&self) -> u32 {
        self.save_number
    }
    
    pub fn save_number_mut(&mut self) -> &mut u32 {
        &mut self.save_number
    }

    pub fn source_variable(&self) -> u32 {
        self.source_variable
    }
    
    pub fn source_variable_mut(&mut self) -> &mut u32 {
        &mut self.source_variable
    }

    pub fn source_is_pointer(&self) -> bool {
        self.source_is_pointer
    }
    
    pub fn source_is_pointer_mut(&mut self) -> &mut bool {
        &mut self.source_is_pointer
    }
}