#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::input_key_command::automatic_input::basic::Basic;
use crate::command::input_key_command::automatic_input::input_type::InputType;
use crate::command::input_key_command::automatic_input::keyboard::Keyboard;
use crate::command::input_key_command::automatic_input::mouse::Mouse;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum State {
    Basic(Basic),
    Keyboard(Keyboard),
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
                let (bytes_read, mouse): (usize, Mouse) = Mouse::parse(bytes);

                (bytes_read, Self::Mouse(mouse))
            }

            _ => unreachable!()
        }
    }

    pub(crate) fn parse_keyboard(bytes: &[u8], _: &InputType) -> (usize, Self) {
        let (bytes_read, keyboard): (usize, Keyboard) = Keyboard::parse(bytes);

        (bytes_read, Self::Keyboard(keyboard))
    }
}