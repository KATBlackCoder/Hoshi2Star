#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::set_variable_plus_command::assignment_operator::AssignmentOperator;
use crate::command::set_variable_plus_command::variable_type::VariableType;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Assignment {
    operator: AssignmentOperator,
    variable_type: VariableType
}

impl Assignment {
    pub fn new(assignment: u8) -> Self {
        Self {
            operator: AssignmentOperator::new(assignment & 0x0f),
            variable_type: VariableType::new(assignment >> 4)
        }
    }

    pub fn operator(&self) -> &AssignmentOperator {
        &self.operator
    }

    pub fn operator_mut(&mut self) -> &mut AssignmentOperator {
        &mut self.operator
    }

    pub fn variable_type(&self) -> &VariableType {
        &self.variable_type
    }

    pub fn variable_type_mut(&mut self) -> &mut VariableType {
        &mut self.variable_type
    }
}