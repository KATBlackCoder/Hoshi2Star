#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::picture_command::erase::base::Base;
use crate::command::picture_command::erase::delay::Delay;
use crate::command::picture_command::erase::delay_reset::DelayReset;
use crate::command::picture_command::erase::range::Range;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum State {
    DelayReset(DelayReset),
    Base(Base),
    Delay(Delay),
    Range(Range)
}

impl State {
    pub(crate) fn parse_delay_reset(bytes: &[u8], range: bool) -> (usize, Self) {
        let (bytes_read, state): (usize, DelayReset) = DelayReset::parse(bytes, range);

        (bytes_read, Self::DelayReset(state))
    }

    pub(crate) fn parse_base(bytes: &[u8]) -> (usize, Self) {
        let (bytes_read, base): (usize, Base) = Base::parse(bytes);

        (bytes_read, Self::Base(base))
    }

    pub(crate) fn parse_delay(bytes: &[u8]) -> (usize, Self) {
        let (bytes_read, delay): (usize, Delay) = Delay::parse(bytes);

        (bytes_read, Self::Delay(delay))
    }

    pub(crate) fn parse_range(bytes: &[u8]) -> (usize, Self) {
        let (bytes_read, range): (usize, Range) = Range::parse(bytes);

        (bytes_read, Self::Range(range))
    }
}