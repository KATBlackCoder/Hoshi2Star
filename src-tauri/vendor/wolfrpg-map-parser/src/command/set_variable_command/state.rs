use crate::command::set_variable_command::base::Base;
use crate::command::set_variable_command::db::DB;
use crate::command::set_variable_command::range::Range;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum State {
    Base(Base),
    Range(Range),
    DB(DB),
}

impl State {
    pub(crate) fn parse_base(bytes: &[u8]) -> (usize, Self) {
        let (bytes_read, command): (usize, Base) = Base::parse(bytes);

        (bytes_read, Self::Base(command))
    }

    pub(crate) fn parse_range(bytes: &[u8]) -> (usize, Self) {
        let (bytes_read, command): (usize, Range) = Range::parse(bytes);

        (bytes_read, Self::Range(command))
    }

    pub(crate) fn parse_db(bytes: &[u8]) -> (usize, Self) {
        let (bytes_read, command): (usize, DB) = DB::parse(bytes);

        (bytes_read, Self::DB(command))
    }
}