use crate::byte_utils::{as_u16_le, parse_string_vec};
use crate::command::common::case::Case;
use crate::command::common::CASES_END_SIGNATURE;
use crate::command::show_choice_command::options::Options;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

pub mod cancel_case;
pub mod extra_cases;
pub mod options;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct ShowChoiceCommand {
    options: Options,
    choices: Vec<String>,
    cases: Vec<Case>,
}

impl ShowChoiceCommand {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, u32, Self){
        let mut offset: usize = 0;

        let options: u16 = as_u16_le(&bytes[offset..offset + 2]);
        let options: Options = Options::new(options);
        offset += 2;

        offset += 3; // Unknown, most probably padding

        // Should be equal to options.selected_choices
        let choice_count: usize = bytes[offset] as usize;
        offset += 1;

        let (bytes_read, choices): (usize, Vec<String>)
            = parse_string_vec(&bytes[offset..], choice_count);
        offset += bytes_read;
        offset += 1; // Should be 0x00 to indicate end of choices

        let case_count: usize = options.case_count();
        let (bytes_read, mut commands_read, cases): (usize, u32, Vec<Case>)
            = Case::parse_multiple(&bytes[offset..], case_count);
        offset += bytes_read;

        let cases_end: &[u8] = &bytes[offset..offset+8];
        offset += 8;
        commands_read += 1; // Signature counts as command

        if &cases_end[..4] != CASES_END_SIGNATURE {
            panic!("Invalid cases end.");
        }

        (offset, commands_read, Self {
            options,
            choices,
            cases,
        })
    }

    pub fn options(&self) -> &Options {
        &self.options
    }

    pub fn options_mut(&mut self) -> &mut Options {
        &mut self.options
    }

    pub fn choices(&self) -> &Vec<String> {
        &self.choices
    }

    pub fn choices_mut(&mut self) -> &mut Vec<String> {
        &mut self.choices
    }

    pub fn cases(&self) -> &Vec<Case> {
        &self.cases
    }

    pub fn cases_mut(&mut self) -> &mut Vec<Case> {
        &mut self.cases
    }
}
