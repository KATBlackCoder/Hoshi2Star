#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::db_management_command::db_type::DBType;
use crate::command::db_management_command::db_operation_type::DBOperationType;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Options {
    db_type: DBType,
    db_operation_type: DBOperationType
}

impl Options {
    pub fn new(options: u8) -> Self {
        Self {
            db_type: DBType::new(options & 0x0f),
            db_operation_type: DBOperationType::new(options >> 4),
        }
    }

    pub fn db_type(&self) -> &DBType {
        &self.db_type
    }

    pub fn db_type_mut(&mut self) -> &mut DBType {
        &mut self.db_type
    }

    pub fn db_operation_type(&self) -> &DBOperationType {
        &self.db_operation_type
    }

    pub fn db_operation_type_mut(&mut self) -> &mut DBOperationType {
        &mut self.db_operation_type
    }
}