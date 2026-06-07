#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::common::case::Case;
use crate::command::common::CASES_END_SIGNATURE;
use crate::command::number_condition_command::condition::Condition;

pub mod condition;
pub mod operator;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct NumberConditionCommand {
    else_case: bool,
    conditions: Vec<Condition>,
    cases: Vec<Case>,
}

impl NumberConditionCommand {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, u32, Self) {
        let mut offset: usize = 0;

        let (case_count, else_case): (u8, bool) = Self::parse_case_count(bytes[offset]);
        offset += 1;

        offset += 3; // padding

        let (bytes_read, conditions): (usize, Vec<Condition>)
            = Self::parse_conditions(&bytes[offset..], case_count as usize);

        offset += bytes_read;

        offset += 3; // Command end

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

    fn parse_conditions(bytes: &[u8], condition_count: usize) -> (usize, Vec<Condition>) {
        let mut offset: usize = 0;
        let mut conditions: Vec<Condition> = Vec::with_capacity(condition_count);

        for _ in 0..condition_count {
            let (bytes_read, condition): (usize, Condition)
                = Condition::parse(&bytes[offset..]);
            conditions.push(condition);
            offset += bytes_read;
        }

        (offset, conditions)
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