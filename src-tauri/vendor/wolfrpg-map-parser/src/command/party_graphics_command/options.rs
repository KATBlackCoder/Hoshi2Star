#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::party_graphics_command::operation::Operation;
use crate::command::party_graphics_command::special_operation::SpecialOperation;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Options {
    operation: Operation,
    special_operation: SpecialOperation,
    graphics_is_variable: bool
}

impl Options {
    pub fn new(options: u32) -> Self {
        Self {
            operation: Operation::new((options & 0x0f) as u8),
            special_operation: SpecialOperation::new(((options >> 4) & 0x0f) as u8),
            graphics_is_variable: (options >> 8) & 0b00000001 != 0
        }
    }

    pub fn operation(&self) -> &Operation {
        &self.operation
    }

    pub fn operation_mut(&mut self) -> &mut Operation {
        &mut self.operation
    }

    pub fn special_operation(&self) -> &SpecialOperation {
        &self.special_operation
    }

    pub fn special_operation_mut(&mut self) -> &mut SpecialOperation {
        &mut self.special_operation
    }

    pub fn graphics_is_variable(&self) -> bool {
        self.graphics_is_variable
    }

    pub fn graphics_is_variable_mut(&mut self) -> &mut bool {
        &mut self.graphics_is_variable
    }
}