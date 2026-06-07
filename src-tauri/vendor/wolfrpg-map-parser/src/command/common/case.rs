use crate::byte_utils::{as_u32_be, as_u32_le};
use crate::command::common::case_type::CaseType;
use crate::command::Command;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Case {
    case_type: CaseType,
    case_id: u32,
    commands: Vec<Command>,
}

impl Case {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, u32, Self) {
        let mut offset: usize = 0;

        let case_type: u32 = as_u32_be(&bytes[offset..offset+4]);
        let case_type: CaseType = CaseType::new(case_type);
        offset += 4;

        offset += 1; // Padding

        let case_id: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        offset += 3; // Unknown, most probably padding

        let mut command_count: u32 = 1; // Case counts as command
        let (bytes_read, commands_read, commands): (usize, u32, Vec<Command>)
            = Command::parse_multiple(&bytes[offset..]);
        offset += bytes_read;
        command_count += commands_read;

        (offset, command_count, Self {
            case_type,
            case_id,
            commands,
        })
    }

    pub(crate) fn parse_multiple(bytes: &[u8], case_count: usize) -> (usize, u32, Vec<Case>) {
        let mut cases: Vec<Case> = Vec::with_capacity(case_count);
        let mut offset: usize = 0;
        let mut commands: u32 = 0;

        for _ in 0..case_count {
            let (bytes_read, commands_read, case): (usize, u32, Case)
                = Self::parse(&bytes[offset..]);
            cases.push(case);
            offset += bytes_read;
            commands += commands_read;
        }

        (offset, commands, cases)
    }

    pub fn case_type(&self) -> &CaseType {
        &self.case_type
    }

    pub fn case_type_mut(&mut self) -> &mut CaseType {
        &mut self.case_type
    }

    pub fn case_id(&self) -> u32 {
        self.case_id
    }

    pub fn case_id_mut(&mut self) -> &mut u32 {
        &mut self.case_id
    }

    pub fn commands(&self) -> &Vec<Command> {
        &self.commands
    }

    pub fn commands_mut(&mut self) -> &mut Vec<Command> {
        &mut self.commands
    }
}