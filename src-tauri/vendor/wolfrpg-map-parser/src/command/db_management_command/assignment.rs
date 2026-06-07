#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::db_management_command::assignment_operator::AssignmentOperator;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Assignment {
    use_variable_as_reference: bool,
    operator: AssignmentOperator
}

impl Assignment {
    pub fn new(assignment: u8) -> Self {
        Self {
            use_variable_as_reference: assignment & 0b00000001 != 0,
            operator: AssignmentOperator::new(assignment >> 4)
        }
    }

    pub fn use_variable_as_reference(&self) -> bool {
        self.use_variable_as_reference
    }

    pub fn use_variable_as_reference_mut(&mut self) -> &mut bool {
        &mut self.use_variable_as_reference
    }

    pub fn operator(&self) -> &AssignmentOperator {
        &self.operator
    }

    pub fn operator_mut(&mut self) -> &mut AssignmentOperator {
        &mut self.operator
    }
}