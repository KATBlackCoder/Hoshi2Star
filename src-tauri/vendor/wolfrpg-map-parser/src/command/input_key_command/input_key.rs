#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::as_u32_le;
use crate::command::input_key_command::input_key::input_type::InputType;
use crate::command::input_key_command::input_key::state::State;

pub mod state;
pub mod basic;
pub mod basic_options;
pub mod direction_keys;
pub mod input_type;
pub mod keyboard_or_pad;
pub mod key_options;
pub mod mouse_target;
pub mod mouse_options;
pub mod mouse;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct InputKey {
    variable: u32,
    input_type: InputType,
    specific_key: bool,
    state: State
}

impl InputKey {
    fn parse(bytes: &[u8], parse_state: fn(&[u8], &InputType) -> (usize, State)) -> (usize, Self) {
        let mut offset: usize = 0;

        let variable: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let input_type: u8 = bytes[offset + 1];
        let (input_type, specific_key): (InputType, bool) = Self::parse_input(input_type);

        let (bytes_read, state): (usize, State) = parse_state(&bytes[offset..], &input_type);
        offset += bytes_read;

        offset += 3; // Command end signature

        (offset, Self {
            variable,
            input_type,
            specific_key,
            state
        })
    }

    fn parse_input(input: u8) -> (InputType, bool) {
        (InputType::new(input & 0x0f), input & 0b00010000 != 0)
    }

    pub(crate) fn parse_base(bytes: &[u8]) -> (usize, Self) {
        Self::parse(bytes, State::parse_base)
    }

    pub(crate) fn parse_keyboard_or_pad(bytes: &[u8]) -> (usize, Self) {
        Self::parse(bytes, State::parse_keyboard_or_pad)
    }

    pub fn variable(&self) -> u32 {
        self.variable
    }

    pub fn variable_mut(&mut self) -> &mut u32 {
        &mut self.variable
    }

    pub fn input_type(&self) -> &InputType {
        &self.input_type
    }

    pub fn input_type_mut(&mut self) -> &mut InputType {
        &mut self.input_type
    }

    pub fn specific_key(&self) -> bool {
        self.specific_key
    }

    pub fn specific_key_mut(&mut self) -> &mut bool {
        &mut self.specific_key
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }
}