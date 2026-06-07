pub mod operator;
pub mod compare_operator;
pub mod condition;

use crate::byte_utils::{as_u32_vec, parse_string_vec};
use crate::command::common::case::Case;
use crate::common::u32_or_string::U32OrString;
use crate::command::common::CASES_END_SIGNATURE;
use crate::command::string_condition_command::condition::Condition;
use crate::command::string_condition_command::operator::Operator;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct StringConditionCommand {
    else_case: bool,
    conditions: Vec<Condition>,
    cases: Vec<Case>
}

impl StringConditionCommand {
    pub(crate) fn parse(bytes: &[u8], signature: u32) -> (usize, u32, Self) {
        let mut offset: usize = 0;

        let (case_count, else_case): (u8,bool) = Self::parse_case_count(bytes[offset]);
        offset += 1;

        offset += 3; // Padding

        let variables: Vec<u32> = as_u32_vec(&bytes[offset..offset + (4 * case_count) as usize]);
        offset += 4 * case_count as usize;

        let value_count:usize = Self::value_count(signature, case_count as u32);
        let values: Vec<u32> = as_u32_vec(&bytes[offset..offset + (4 * value_count)]);
        offset += 4 * value_count;

        offset += 1; // Padding;

        let condition_count: usize = bytes[offset] as usize;
        offset += 1;

        let (bytes_read, conditions): (usize, Vec<String>)
            = parse_string_vec(&bytes[offset..], condition_count);
        offset += bytes_read;

        offset += 1; // Conditions end

        let conditions: Vec<Condition> = Self::make_conditions(variables, values, conditions);

        let case_count: usize = case_count as usize + else_case as usize;
        let (bytes_read, mut commands_read, cases): (usize, u32, Vec<Case>)
            = Case::parse_multiple(&bytes[offset..], case_count);
        offset += bytes_read;

        let cases_end: &[u8] = &bytes[offset..offset+8];
        offset += 8;
        commands_read += 1;

        if &cases_end[..4] != CASES_END_SIGNATURE {
            panic!("Invalid cases end.");
        }

        (offset, commands_read, Self {
            else_case,
            conditions,
            cases
        })
    }

    fn parse_case_count(cases: u8) -> (u8, bool) {
        (cases & 0x0f, cases & 0b00010000 != 0)
    }

    fn value_count(signature: u32, case_count: u32) -> usize {
        ((signature >> 24) - 2 - case_count) as usize
    }

    fn make_conditions(variables: Vec<u32>, values: Vec<u32>,
                       conditions: Vec<String>) -> Vec<Condition> {
        let mut ret_conditions: Vec<Condition> = Vec::with_capacity(variables.len());

        for i in 0..variables.len() {
            let operator: u8 = (variables[i] >> 24) as u8;
            let operator: Operator = Operator::new(operator);
            let variable: u32 = variables[i] & 0x00ffffff;

            let value: U32OrString = if operator.value_is_variable() {
                U32OrString::U32(values[i])
            } else {
              U32OrString::String(conditions[i].clone())
            };

            ret_conditions.push(Condition::new(variable, operator, value));
        }

        ret_conditions
    }

    pub fn else_case(&self) -> bool {
        self.else_case
    }
    
    pub fn else_case_mut(&mut self) -> &mut bool {
        &mut self.else_case
    }

    pub fn conditions(&self) -> &Vec<Condition> {
        &self.conditions
    }
    
    pub fn conditions_mut(&mut self) -> &mut Vec<Condition> {
        &mut self.conditions
    }

    pub fn cases(&self) -> &Vec<Case> {
        &self.cases
    }
    
    pub fn cases_mut(&mut self) -> &mut Vec<Case> {
        &mut self.cases
    }
}