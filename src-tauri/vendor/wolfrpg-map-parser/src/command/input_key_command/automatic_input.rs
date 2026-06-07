#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::input_key_command::automatic_input::input_type::InputType;
use crate::command::input_key_command::automatic_input::state::State;

pub mod basic;
pub mod state;
pub mod basic_options;
pub mod input_type;
pub mod keyboard;
pub mod mouse;
pub mod mouse_options;
pub mod mouse_type;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct AutomaticInput {
    input_type: InputType,
    state: State,
}

impl AutomaticInput {
    fn parse(bytes: &[u8], parse_state: fn(&[u8], &InputType) -> (usize, State)) -> (usize, Self) {
        let mut offset: usize = 0;

        let input_type: u8 = bytes[offset + 3];
        let input_type: InputType = InputType::new(input_type);

        let (bytes_read, state): (usize, State) = parse_state(&bytes[offset..], &input_type);
        offset += bytes_read;

        offset += 3; // Command end signature

        (offset, Self {
            input_type,
            state
        })
    }

    pub(crate) fn parse_base(bytes: &[u8]) -> (usize, Self) {
        Self::parse(bytes, State::parse_base)
    }

    pub(crate) fn parse_keyboard(bytes: &[u8]) -> (usize, Self) {
        Self::parse(bytes, State::parse_keyboard)
    }

    pub fn input_type(&self) -> &InputType {
        &self.input_type
    }

    pub fn input_type_mut(&mut self) -> &mut InputType {
        &mut self.input_type
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }
}