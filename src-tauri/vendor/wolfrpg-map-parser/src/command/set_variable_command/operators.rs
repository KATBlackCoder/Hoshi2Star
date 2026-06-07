#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::set_variable_command::assignment::Assignment;
use crate::command::set_variable_command::calculation::Calculation;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Operators {
    assignment: Assignment,
    calculation: Calculation,
}

impl Operators {
    pub fn new(operators: u8) -> Self {
        Self {
            assignment: Assignment::from_u8(operators & 0x0f),
            calculation: Calculation::from_u8(operators >> 4),
        }
    }

    pub fn assignment(&self) -> &Assignment {
        &self.assignment
    }

    pub fn assignment_mut(&mut self) -> &mut Assignment {
        &mut self.assignment
    }

    pub fn calculation(&self) -> &Calculation {
        &self.calculation
    }

    pub fn calculation_mut(&mut self) -> &mut Calculation {
        &mut self.calculation
    }
}