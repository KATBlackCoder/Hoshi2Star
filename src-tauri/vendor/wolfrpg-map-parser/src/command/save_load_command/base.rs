#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::as_u32_le;
use crate::command::save_load_command::operation::Operation;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Base {
    operation: Operation,
    save_number: u32
}

impl Base {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let operation: u32 = as_u32_le(&bytes[offset..offset + 4]);
        let operation: Operation = Operation::new(operation);
        offset += 4;

        let save_number: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        offset += 3; // Command end signature

        (offset, Self {
            operation,
            save_number
        })
    }

    pub fn operation(&self) -> &Operation {
        &self.operation
    }
    
    pub fn operation_mut(&mut self) -> &mut Operation {
        &mut self.operation
    }

    pub fn save_number(&self) -> u32 {
        self.save_number
    }
    
    pub fn save_number_mut(&mut self) -> &mut u32 {
        &mut self.save_number
    }
}