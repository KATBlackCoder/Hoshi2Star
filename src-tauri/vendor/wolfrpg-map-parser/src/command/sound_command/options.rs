#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::sound_command::operation::Operation;
use crate::command::sound_command::process_type::ProcessType;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Options {
    process_type: ProcessType,
    operation: Operation
}

impl Options {
    pub fn new(options: u8) -> Self {
        Self {
            process_type: ProcessType::new(options & 0x0f),
            operation: Operation::new(options >> 4)
        }
    }

    pub fn process_type(&self) -> &ProcessType {
        &self.process_type
    }
    
    pub fn process_type_mut(&mut self) -> &mut ProcessType {
        &mut self.process_type
    }

    pub fn operation(&self) -> &Operation {
        &self.operation
    }
    
    pub fn operation_mut(&mut self) -> &mut Operation {
        &mut self.operation
    }
}