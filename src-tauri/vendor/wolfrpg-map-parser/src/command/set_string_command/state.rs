#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::set_string_command::base::Base;
use crate::command::set_string_command::dynamic::Dynamic;
use crate::command::set_string_command::input::Input;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum State {
    Base(Base),
    Dynamic(Dynamic),
    Input(Input),
}

impl State {
    pub(crate) fn parse_base(bytes: &[u8]) -> (usize, Self) {
        let (bytes_read, command): (usize, Base) = Base::parse(bytes);

        (bytes_read, Self::Base(command))
    }

    pub(crate) fn parse_dynamic(bytes: &[u8]) -> (usize, Self) {
        let (bytes_read, command): (usize, Dynamic) = Dynamic::parse(bytes);

        (bytes_read, Self::Dynamic(command))
    }

    pub(crate) fn parse_input(bytes: &[u8]) -> (usize, Self) {
        let (bytes_read, command): (usize, Input) = Input::parse(bytes);

        (bytes_read, Self::Input(command))
    }
}