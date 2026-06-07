use crate::byte_utils::{as_u32_le, as_u32_vec, parse_string_vec};
use crate::common::u32_or_string::U32OrString;
use crate::command::common_event_command::argument_count::ArgumentCount;
use crate::command::common_event_command::options::Options;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use std::cmp::max;
use std::collections::VecDeque;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Event {
    target: u32,
    argument_count: ArgumentCount,
    options: Options,
    number_arguments: Vec<u32>,
    string_arguments: Vec<U32OrString>,
    return_variable: Option<u32>,
    event_name: Option<String>,
}

impl Event {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let target: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let argument_count: u8 = bytes[offset];
        let argument_count: ArgumentCount = ArgumentCount::new(argument_count);
        offset += 1;

        let options: [u8; 3] = bytes[offset..offset+3].try_into().unwrap();
        let options: Options = Options::new(options);
        offset += 3;

        let (bytes_read, number_arguments): (usize, Vec<u32>)
            = Self::parse_u32_vec(bytes, offset, argument_count.number_arguments() as usize);
        offset += bytes_read;

        let (bytes_read, string_arguments_variables): (usize, Vec<u32>)
            = Self::parse_u32_vec(bytes, offset, argument_count.string_arguments() as usize);
        offset += bytes_read;

        let return_variable: Option<u32> = if options.has_return_value() {
            let ret: u32 = as_u32_le(&bytes[offset..offset+4]);
            offset += 4;

            Some(ret)
        } else {
            None
        };

        offset += 1; // padding

        let string_count: u8 = bytes[offset];
        offset += 1;

        let (bytes_read, strings): (usize, Vec<String>)
            = parse_string_vec(&bytes[offset..], string_count as usize);
        offset += bytes_read;

        let (event_name, string_arguments): (Option<String>, Vec<U32OrString>)
            = Self::convert_strings(string_arguments_variables, strings,
                                    argument_count.string_arguments() as usize, &options);

        offset += 1;

        (offset, Self {
            target,
            argument_count,
            options,
            number_arguments,
            string_arguments,
            return_variable,
            event_name,
        })
    }

    fn parse_u32_vec(bytes: &[u8], offset: usize, count: usize) -> (usize, Vec<u32>) {
        (count*4, as_u32_vec(&bytes[offset..offset + count*4]))
    }

    fn convert_strings(variables: Vec<u32>, strings: Vec<String>, count: usize, options: &Options)
        -> (Option<String>, Vec<U32OrString>) {
        let mut count: usize = max(count, strings.len());
        let mut string_arguments: Vec<U32OrString> = Vec::with_capacity(count);
        let mut strings: VecDeque<String> = strings.into_iter().collect();

        let event_name: Option<String> = if options.string_argument_count() < count as u8 {
            count -= 1;
            strings.pop_front()
        } else {
            None
        };

        for i in 0..count.max(variables.len()) {
            let arg: U32OrString = if options.is_arg_string((i+1) as u8) {
                U32OrString::String(strings[i].clone())
            } else {
                U32OrString::U32(variables[i])
            };

            string_arguments.push(arg);
        }

        (event_name, string_arguments)
    }

    pub fn target(&self) -> u32 {
        self.target
    }

    pub fn target_mut(&mut self) -> &mut u32 {
        &mut self.target
    }

    pub fn argument_count(&self) -> &ArgumentCount {
        &self.argument_count
    }

    pub fn argument_count_mut(&mut self) -> &mut ArgumentCount {
        &mut self.argument_count
    }

    pub fn options(&self) -> &Options {
        &self.options
    }

    pub fn options_mut(&mut self) -> &mut Options {
        &mut self.options
    }

    pub fn number_arguments(&self) -> &Vec<u32> {
        &self.number_arguments
    }

    pub fn number_arguments_mut(&mut self) -> &mut Vec<u32> {
        &mut self.number_arguments
    }

    pub fn string_arguments(&self) -> &Vec<U32OrString> {
        &self.string_arguments
    }

    pub fn string_arguments_mut(&mut self) -> &mut Vec<U32OrString> {
        &mut self.string_arguments
    }

    pub fn return_variable(&self) -> Option<u32> {
        self.return_variable
    }

    pub fn return_variable_mut(&mut self) -> &mut Option<u32> {
        &mut self.return_variable
    }

    pub fn event_name(&self) -> &Option<String> {
        &self.event_name
    }

    pub fn event_name_mut(&mut self) -> &mut Option<String> {
        &mut self.event_name
    }
}