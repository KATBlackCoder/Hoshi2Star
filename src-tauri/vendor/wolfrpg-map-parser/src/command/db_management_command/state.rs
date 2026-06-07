#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::parse_string;
use crate::command::db_management_command::base::Base;
use crate::command::db_management_command::csv::CSV;
use crate::command::db_management_command::string;

type DBStrings = (Option<String>, Option<String>, Option<String>);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum State {
    Base(Base),
    String(string::String),
    CSV(CSV)
}

impl State {
    pub(crate) fn parse_base(bytes: &[u8]) -> (usize, Self, DBStrings) {
        let (bytes_read, command, strings): (usize, Base, DBStrings) = Base::parse(bytes);

        (bytes_read, Self::Base(command), strings)
    }

    pub(crate) fn parse_string(bytes: &[u8]) -> (usize, Self, DBStrings) {
        let (bytes_read, command, strings): (usize, string::String, DBStrings) 
            = string::String::parse(bytes);

        (bytes_read, Self::String(command), strings)
    }

    pub(crate) fn parse_csv(bytes: &[u8]) -> (usize, Self, DBStrings) {
        let (bytes_read, command, strings): (usize, CSV, DBStrings) = CSV::parse(bytes);

        (bytes_read, Self::CSV(command), strings)
    }
    
    pub(crate) fn parse_strings(string_count: u8, bytes: &[u8]) -> (usize, DBStrings) {
        let mut offset: usize = 0;

        let db_type_string: Option<String> = if string_count > 1 {
            let (bytes_read, db_type_string): (usize, String) = parse_string(&bytes[offset..]);
            offset += bytes_read;
            Some(db_type_string)
        } else {
            None
        };
        
        let data_string: Option<String> = if string_count > 2 {
            let (bytes_read, data_string): (usize, String) = parse_string(&bytes[offset..]);
            offset += bytes_read;
            Some(data_string)
        } else {
            None
        };
        
        let field_string: Option<String> = if string_count > 3 {
            let (bytes_read, field_string): (usize, String) = parse_string(&bytes[offset..]);
            offset += bytes_read;
            Some(field_string)
        } else {
            None
        };

        (offset, (db_type_string, data_string, field_string))
    }
}