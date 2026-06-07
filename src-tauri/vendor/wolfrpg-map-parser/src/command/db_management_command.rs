use crate::byte_utils::as_u32_le;
use crate::command::db_management_command::assignment::Assignment;
use crate::command::db_management_command::options::Options;
use crate::command::db_management_command::state::State;
use crate::common::u32_or_string::U32OrString;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub mod base;
pub mod options;
pub mod db_operation_type;
pub mod db_type;
pub mod assignment;
pub mod assignment_operator;
pub mod string;
pub mod csv;
pub mod state;

type DBStrings = (Option<String>, Option<String>, Option<String>);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct DBManagementCommand {
    db_type: U32OrString, // name for table?
    data: U32OrString,    // name for tuple?
    field: U32OrString,
    assignment: Assignment,
    options: Options,
    state: State
}

impl DBManagementCommand {
    fn parse(bytes: &[u8], parse_state: fn(&[u8]) -> (usize, State, DBStrings)) -> (usize, Self) {
        let mut offset: usize = 0;

        let db_type: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let data: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let field: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let assignment: u8 = bytes[offset];
        let assignment: Assignment = Assignment::new(assignment);
        offset += 1;

        let options: u8 = bytes[offset];
        let options: Options = Options::new(options);
        offset += 1;

        offset += 2; // padding

        let (bytes_read, state, db_strings) = parse_state(&bytes[offset..]);
        offset += bytes_read;

        let db_type: U32OrString = Self::get_u32_or_string(db_type, db_strings.0);
        let data: U32OrString = Self::get_u32_or_string(data, db_strings.1);
        let field: U32OrString = Self::get_u32_or_string(field, db_strings.2);

        offset += 1; // Command end signature

        (offset, Self {
            db_type,
            data,
            field,
            assignment,
            options,
            state
        })
    }

    pub(crate) fn parse_base(bytes: &[u8]) -> (usize, Self) {
        Self::parse(bytes, State::parse_base)
    }

    pub(crate) fn parse_string(bytes: &[u8]) -> (usize, Self) {
        Self::parse(bytes, State::parse_string)
    }

    pub(crate) fn parse_csv(bytes: &[u8]) -> (usize, Self) {
        Self::parse(bytes, State::parse_csv)
    }

    fn get_u32_or_string(value: u32, string: Option<String>) -> U32OrString {
        match string {
            Some(s) if !s.is_empty() => U32OrString::String(s),
            _ => U32OrString::U32(value),
        }
    }

    pub fn db_type(&self) -> &U32OrString {
        &self.db_type
    }
    
    pub fn db_type_mut(&mut self) -> &mut U32OrString {
        &mut self.db_type
    }

    pub fn data(&self) -> &U32OrString {
        &self.data
    }
    
    pub fn data_mut(&mut self) -> &mut U32OrString {
        &mut self.data
    }

    pub fn field(&self) -> &U32OrString {
        &self.field
    }
    
    pub fn field_mut(&mut self) -> &mut U32OrString {
        &mut self.field
    }

    pub fn assignment(&self) -> &Assignment {
        &self.assignment
    }
    
    pub fn assignment_mut(&mut self) -> &mut Assignment {
        &mut self.assignment
    }

    pub fn options(&self) -> &Options {
        &self.options
    }
    
    pub fn options_mut(&mut self) -> &mut Options {
        &mut self.options
    }

    pub fn state(&self) -> &State {
        &self.state
    }
    
    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }
}