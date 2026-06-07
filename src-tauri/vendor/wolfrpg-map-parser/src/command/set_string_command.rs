pub mod base;
pub mod options;
pub mod content_type;
pub mod variable_type;
pub mod string_operation;
pub mod operation;
pub mod dynamic;
pub mod input;
pub mod state;

use crate::byte_utils::as_u32_le;
use crate::command::set_string_command::content_type::ContentType;
use crate::command::set_string_command::operation::Operation;
use crate::command::set_string_command::options::Options;
use crate::command::set_string_command::state::State;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct SetStringCommand {
    variable: u32,
    options: Options,
    operation: Operation,
    state: State
}

impl SetStringCommand {
    fn parse(bytes: &[u8], parse_state: fn(&[u8]) -> (usize, State)) -> (usize, Self) {
        let mut offset: usize = 0;

        let variable: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let options: u8 = bytes[offset];
        let options: Options = Options::new(options);
        offset += 1;

        let operation: u8 = bytes[offset];
        let operation: Operation = Operation::new(operation);
        offset += 1;

        offset += 2; // Unknown, most probably padding

        let (bytes_read, state): (usize, State) = parse_state(&bytes[offset..]);
        offset += bytes_read;

        (offset, Self {
            variable,
            options,
            operation,
            state
        })
    }

    pub(crate) fn parse_base(bytes: &[u8]) -> (usize, Self) {
        Self::parse(bytes, State::parse_base)
    }

    pub(crate) fn parse_dynamic(bytes: &[u8]) -> (usize, Self) {
        match Options::new(bytes[4]).content_type() {
            ContentType::UserInput => Self::parse(bytes, State::parse_input),
            _ => Self::parse(bytes, State::parse_dynamic),
        }
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

    pub fn operation(&self) -> &Operation {
        &self.operation
    }

    pub fn operation_mut(&mut self) -> &mut Operation {
        &mut self.operation
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }
}