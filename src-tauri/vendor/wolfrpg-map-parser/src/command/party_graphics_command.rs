pub mod options;
pub mod operation;
pub mod special_operation;

use crate::byte_utils::{as_u32_le, parse_string};
use crate::common::u32_or_string::U32OrString;
use crate::command::party_graphics_command::operation::Operation;
use crate::command::party_graphics_command::options::Options;
use crate::byte_utils::parse_optional_string;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct PartyGraphicsCommand {
    options: Options,
    member: Option<u32>,
    graphics: Option<U32OrString>,
}

impl PartyGraphicsCommand {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let options: u32 = as_u32_le(&bytes[offset..offset + 4]);
        let options: Options = Options::new(options);
        offset += 4;

        let member: Option<u32> = match *options.operation() {
            Operation::Remove | Operation::Insert | Operation::Replace => {
                let member: u32 = as_u32_le(&bytes[offset..offset + 4]);
                offset += 4;

                Some(member)
            }
            _ => None
        };

        let graphics_variable: Option<u32> = if options.graphics_is_variable() {
            let graphics_variable: u32 = as_u32_le(&bytes[offset..offset + 4]);
            offset += 4;

            Some(graphics_variable)
        } else {
            None
        };

        offset += 1; // Padding

        let is_graphics_string: bool = bytes[offset] != 0;
        offset += 1;

        let graphics_string: Option<String> 
            = parse_optional_string!(bytes, offset, is_graphics_string);

        let graphics: Option<U32OrString> = match (graphics_variable, graphics_string) {
            (Some(variable), None) => Some(U32OrString::U32(variable)),
            (None, Some(string)) => Some(U32OrString::String(string)),
            (None, None) => None,
            _ => unreachable!()
        };

        offset += 1; // Command end signature

        (offset, Self {
            options,
            member,
            graphics
        })
    }

    pub fn options(&self) -> &Options {
        &self.options
    }
    
    pub fn options_mut(&mut self) -> &mut Options {
        &mut self.options
    }

    pub fn member(&self) -> Option<u32> {
        self.member
    }
    
    pub fn member_mut(&mut self) -> &mut Option<u32> {
        &mut self.member
    }

    pub fn graphics(&self) -> &Option<U32OrString> {
        &self.graphics
    }
    
    pub fn graphics_mut(&mut self) -> &mut Option<U32OrString> {
        &mut self.graphics
    }
}