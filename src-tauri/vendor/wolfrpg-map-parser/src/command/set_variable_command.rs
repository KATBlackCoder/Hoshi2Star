#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use state::State;
use crate::byte_utils::as_u32_le;
use crate::command::set_variable_command::operators::Operators;
use crate::command::set_variable_command::options::Options;
pub mod base;
pub mod assignment;
pub mod calculation;
pub mod options;
pub mod operators;
pub mod range;
pub mod state;
pub mod db;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct SetVariableCommand {
    variable: u32,
    left_side: u32,
    right_side: u32,
    options: Options,
    operators: Operators,
    state: State
}

impl SetVariableCommand {
    fn parse(bytes: &[u8], parse_state: fn(&[u8]) -> (usize, State)) -> (usize, Self) {
        let mut offset: usize = 0;

        let variable: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let left_side: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let right_side: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let options: u8 = bytes[offset];
        let options: Options = Options::new(options);
        offset += 1;

        let operators: u8 = bytes[offset];
        let operators: Operators = Operators::new(operators);
        offset += 1;

        let (bytes_read, state): (usize, State) = parse_state(&bytes[offset..]);

        offset += bytes_read;

        (offset, Self {
            variable,
            left_side,
            right_side,
            options,
            operators,
            state
        })
    }

    pub(crate) fn parse_base(bytes: &[u8]) -> (usize, Self) {
        Self::parse(bytes, State::parse_base)
    }

    pub(crate) fn parse_range(bytes: &[u8]) -> (usize, Self) {
        Self::parse(bytes, State::parse_range)
    }

    pub(crate) fn parse_db(bytes: &[u8]) -> (usize, Self) {
        Self::parse(bytes, State::parse_db)
    }

    pub fn variable(&self) -> u32 {
        self.variable
    }

    pub fn variable_mut(&mut self) -> &mut u32 {
        &mut self.variable
    }

    pub fn left_side(&self) -> u32 {
        self.left_side
    }

    pub fn left_side_mut(&mut self) -> &mut u32 {
        &mut self.left_side
    }

    pub fn right_side(&self) -> u32 {
        self.right_side
    }

    pub fn right_side_mut(&mut self) -> &mut u32 {
        &mut self.right_side
    }

    pub fn options(&self) -> &Options {
        &self.options
    }

    pub fn options_mut(&mut self) -> &mut Options {
        &mut self.options
    }

    pub fn operators(&self) -> &Operators {
        &self.operators
    }

    pub fn operators_mut(&mut self) -> &mut Operators {
        &mut self.operators
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }
}