#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use crate::common::compare_operator::CompareOperator;
use crate::db_parser::common_event::condition_type::ConditionType;

/// A run condition for a common event
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct RunCondition {
    operator: CompareOperator,
    condition_type: ConditionType,
    variable: u32,
    value: u32,
}

impl RunCondition {
    pub const fn new(
        settings: u8,
        variable: u32,
        value: u32
    ) -> Self {
        Self {
            operator: CompareOperator::new((settings >> 4) & 0x0f),
            condition_type: ConditionType::new(settings & 0x0f),
            variable,
            value
        }
    }

    /// The operator used in the run condition
    pub fn operator(&self) -> &CompareOperator {
        &self.operator
    }

    /// The type of the run condition
    pub fn condition_type(&self) -> &ConditionType {
        &self.condition_type
    }

    /// The ID of the variable used in the run condition
    pub fn variable(&self) -> u32 {
        self.variable
    }

    /// The value used in the run condition
    pub fn value(&self) -> u32 {
        self.value
    }
}