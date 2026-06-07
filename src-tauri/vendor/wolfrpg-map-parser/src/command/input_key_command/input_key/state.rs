#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::input_key_command::input_key::basic::Basic;
use crate::command::input_key_command::input_key::input_type::InputType;
use crate::command::input_key_command::input_key::keyboard_or_pad::KeyboardOrPad;
use crate::command::input_key_command::input_key::mouse::Mouse;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum State {
    Basic(Basic),
    KeyboardOrPad(KeyboardOrPad),
    Mouse(Mouse)
}

impl State {
    pub(crate) fn parse_base(bytes: &[u8], input_type: &InputType) -> (usize, Self) {
        match *input_type {
            InputType::Basic => {
                let (bytes_read, state): (usize, Basic) = Basic::parse(bytes);

                (bytes_read, Self::Basic(state))
            }

            InputType::Mouse => {
                let (bytes_read, state): (usize , Mouse) = Mouse::parse(bytes);

                (bytes_read, Self::Mouse(state))
            }

            _ => unreachable!()
        }
    }

    pub(crate) fn parse_keyboard_or_pad(bytes: &[u8], _ : &InputType) -> (usize, Self) {
        let (bytes_read, state): (usize, KeyboardOrPad) = KeyboardOrPad::parse(bytes);

        (bytes_read, Self::KeyboardOrPad(state))
    }
}

