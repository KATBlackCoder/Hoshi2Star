#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::sound_command::filename::Filename;
use crate::command::sound_command::options::Options;
use crate::command::sound_command::sound_type::SoundType;
use crate::command::sound_command::variable::Variable;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum State {
    Filename(Filename),
    Variable(Variable),
    FreeAll
}

impl State {
    pub(crate) fn parse_filename(bytes: &[u8], options: &Options, _: &SoundType) -> (usize, Self) {
        let (bytes_read, state): (usize, Filename) = Filename::parse(bytes, options);

        (bytes_read, Self::Filename(state))
    }

    pub(crate) fn parse_variable(bytes: &[u8], options: &Options, _: &SoundType) -> (usize, Self) {
        let (bytes_read, state): (usize, Variable) = Variable::parse(bytes, options);

        (bytes_read, Self::Variable(state))
    }

    pub(crate) fn parse_free_all(_: &[u8], _: &Options, sound_type: &SoundType) -> (usize, State) {
        match *sound_type {
            SoundType::Variable => (10, State::FreeAll),
            _ => (2, State::FreeAll),
        }
    }
}