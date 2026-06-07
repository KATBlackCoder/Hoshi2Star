#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use state::State;
use crate::byte_utils::as_u32_le;
use crate::command::set_variable_plus_command::assignment::Assignment;
use crate::command::set_variable_plus_command::options::Options;
use crate::command::set_variable_plus_command::variable_type::VariableType;

pub mod state;
pub mod character;
pub mod options;
pub mod variable_type;
pub mod assignment_operator;
pub mod assignment;
pub mod position;
pub mod picture;
pub mod picture_field;
pub mod other;
pub mod target;
pub mod character_field;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct SetVariablePlusCommand {
    variable: u32,
    options: Options,
    assignment: Assignment,
    state: State
}

impl SetVariablePlusCommand {
    fn parse(bytes: &[u8], parse_state: fn(&[u8]) -> (usize, State)) -> (usize, Self) {
        let mut offset: usize = 0;

        let variable: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let options: u8 = bytes[offset];
        let options:Options = Options::new(options);
        offset += 1;

        let assignment: u8 = bytes[offset];
        let assignment: Assignment = Assignment::new(assignment);
        offset += 1;

        let (bytes_read, state): (usize, State) = parse_state(&bytes[offset..]);
        offset += bytes_read;

        offset += 3; // Command end signature

        (offset, Self {
            variable,
            options,
            assignment,
            state
        })
    }

    pub(crate) fn parse_base(bytes: &[u8]) -> (usize, Self) {
        match Assignment::new(bytes[5]).variable_type() {
            VariableType::Character => Self::parse(bytes, State::parse_character),
            VariableType::Position => Self::parse(bytes, State::parse_position),
            VariableType::PictureNumber => Self::parse(bytes, State::parse_picture),
            _ => panic!("Invalid variable type: {:x}", bytes[5] & 0x0f)
        }
    }

    pub(crate) fn parse_other(bytes: &[u8]) -> (usize, Self) {
        Self::parse(bytes, State::parse_other)
    }

    pub fn variable(&self) -> u32 {
        self.variable
    }

    pub fn variable_mut(&mut self) -> &mut u32 {
        &mut self.variable
    }

    pub fn options(&self) -> &Options {
        &self.options
    }

    pub fn options_mut(&mut self) -> &mut Options {
        &mut self.options
    }

    pub fn assignment(&self) -> &Assignment {
        &self.assignment
    }

    pub fn assignment_mut(&mut self) -> &mut Assignment {
        &mut self.assignment
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }
}