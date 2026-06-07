#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::as_u32_le;
use crate::command::number_condition_command::operator::Operator;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Condition {
    variable: u32,
    value: u32,
    operator: Operator
}

impl Condition {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let variable: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let value: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let operator: Operator = Operator::new(bytes[offset]);
        offset += 1;

        offset += 3; // Padding

        (offset, Self {
            variable,
            value,
            operator
        })
    }

    pub fn variable(&self) -> u32 {
        self.variable
    }

    pub fn variable_mut(&mut self) -> &mut u32 {
        &mut self.variable
    }

    pub fn value(&self) -> u32 {
        self.value
    }

    pub fn value_mut(&mut self) -> &mut u32 {
        &mut self.value
    }

    pub fn operator(&self) -> &Operator {
        &self.operator
    }

    pub fn operator_mut(&mut self) -> &mut Operator {
        &mut self.operator
    }
}