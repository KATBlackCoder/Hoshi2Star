#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::string_condition_command::compare_operator::CompareOperator;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Operator {
    value_is_variable: bool,
    operator: CompareOperator
}

impl Operator {
    pub fn new(operator: u8) -> Self {
        Self {
            value_is_variable: operator & 0b00000001 != 0,
            operator: CompareOperator::new(operator >> 4)
        }
    }

    pub fn value_is_variable(&self) -> bool {
        self.value_is_variable
    }

    pub fn value_is_variable_mut(&mut self) -> &mut bool {
        &mut self.value_is_variable
    }

    pub fn operator(&self) -> &CompareOperator {
        &self.operator
    }

    pub fn operator_mut(&mut self) -> &mut CompareOperator {
        &mut self.operator
    }
}

