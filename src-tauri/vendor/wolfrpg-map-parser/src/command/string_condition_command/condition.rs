#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::common::u32_or_string::U32OrString;
use crate::command::string_condition_command::operator::Operator;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Condition {
    variable: u32,
    operator: Operator,
    value: U32OrString
}

impl Condition {
    pub fn new(variable: u32, operator: Operator, value: U32OrString) -> Condition {
        Condition {
            variable,
            operator,
            value
        }
    }

    pub fn variable(&self) -> u32 {
        self.variable
    }

    pub fn variable_mut(&mut self) -> &mut u32 {
        &mut self.variable
    }

    pub fn operator(&self) -> &Operator {
        &self.operator
    }

    pub fn operator_mut(&mut self) -> &mut Operator {
        &mut self.operator
    }

    pub fn value(&self) -> &U32OrString {
        &self.value
    }

    pub fn value_mut(&mut self) -> &mut U32OrString {
        &mut self.value
    }
}