pub mod state;
pub mod delay_reset;
pub mod base;
pub mod delay;
pub mod range;

#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::as_u32_le;
use crate::command::picture_command::display_operation::DisplayOperation;
use crate::command::picture_command::erase::state::State;
use crate::command::picture_command::options::Options;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Erase {
    options: Options,
    picture: u32,
    state: State,
}

impl Erase {
    fn parse(bytes: &[u8], parse_state: fn(&[u8], &Options) -> (usize, State)) -> (usize, Self) {
        let mut offset: usize = 0;

        let options: u32 = as_u32_le(&bytes[offset..offset + 4]);
        let options: Options = Options::new(options);
        offset += 4;

        let picture: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let (bytes_read, state): (usize, State) = parse_state(&bytes[offset..], &options);
        offset += bytes_read;

        offset += 3; // Command end signature

        (offset, Self {
            options,
            picture,
            state
        })
    }

    pub(crate) fn parse_delay_reset(bytes: &[u8]) -> (usize, Self) {
        let parse_state = |bytes: &[u8], _: &Options| -> (usize, State) {
            State::parse_delay_reset(bytes, false)
        };

        Self::parse(bytes, parse_state)
    }

    pub(crate) fn parse_base(bytes: &[u8]) -> (usize, Self) {
        let parse_state = |bytes: &[u8], options: &Options| -> (usize, State) {
            match *options.display_operation() {
                DisplayOperation::DelayReset => State::parse_delay_reset(bytes, true),
                DisplayOperation::Erase => State::parse_base(bytes),
                _ => unreachable!()
            }
        };

        Self::parse(bytes, parse_state)
    }

    pub(crate) fn parse_delay(bytes: &[u8]) -> (usize, Self) {
        Self::parse(bytes, |bytes: &[u8], _| State::parse_delay(bytes))
    }

    pub(crate) fn parse_range(bytes: &[u8]) -> (usize, Self) {
        Self::parse(bytes, |bytes, _| State::parse_range(bytes))
    }

    pub fn options(&self) -> &Options {
        &self.options
    }

    pub fn options_mut(&mut self) -> &mut Options {
        &mut self.options
    }

    pub fn picture(&self) -> u32 {
        self.picture
    }

    pub fn picture_mut(&mut self) -> &mut u32 {
        &mut self.picture
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }
}