#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::set_string_command::string_operation::StringOperation;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Operation {
    operation: StringOperation,
    input_cancel: bool,
    input_replace: bool,
}

impl Operation {
    pub fn new(operation: u8) -> Self {
        Self {
            operation: StringOperation::new(operation & 0x0f),
            input_replace: operation & 0b00010000 != 0,
            input_cancel: operation  & 0b00100000 != 0,
        }
    }

    pub fn operation(&self) -> &StringOperation {
        &self.operation
    }

    pub fn operation_mut(&mut self) -> &mut StringOperation {
        &mut self.operation
    }

    pub fn input_cancel(&self) -> bool {
        self.input_cancel
    }

    pub fn input_cancel_mut(&mut self) -> &mut bool {
        &mut self.input_cancel
    }

    pub fn input_replace(&self) -> bool {
        self.input_replace
    }

    pub fn input_replace_mut(&mut self) -> &mut bool {
        &mut self.input_replace
    }
}