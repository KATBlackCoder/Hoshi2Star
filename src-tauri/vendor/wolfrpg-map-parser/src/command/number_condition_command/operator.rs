#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::common::compare_operator::CompareOperator;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Operator {
    operator: CompareOperator,
    not_variable: bool,
}

impl Operator {
    pub fn new(operator: u8) -> Self {
        Self {
            operator: CompareOperator::new(operator & 0x0f),
            not_variable: operator & 0b00010000 != 0,
        }
    }

    pub fn operator(&self) -> &CompareOperator {
        &self.operator
    }

    pub fn operator_mut(&mut self) -> &mut CompareOperator {
        &mut self.operator
    }

    pub fn not_variable(&self) -> bool {
        self.not_variable
    }

    pub fn not_variable_mut(&mut self) -> &mut bool {
        &mut self.not_variable
    }
}
