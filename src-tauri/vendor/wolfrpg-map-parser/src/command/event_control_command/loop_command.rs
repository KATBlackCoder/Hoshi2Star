#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::Command;

use crate::command::common::LOOP_END_SIGNATURE;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Loop {
    commands: Vec<Command>,
}

impl Loop {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, u32, Self) {
        let mut offset: usize = 0;
        offset += 3; // Command end signature

        let (bytes_read, mut commands_read, commands): (usize, u32, Vec<Command>)
            = Command::parse_multiple(&bytes[offset..]);
        offset += bytes_read;

        let loop_end_signature: &[u8] = &bytes[offset..offset+8];
        offset += 8;
        commands_read += 1;

        if loop_end_signature[..4] != LOOP_END_SIGNATURE[..4] {
            panic!("Invalid loop end.");
        }

        (offset, commands_read, Self {
            commands
        })
    }

    pub fn commands(&self) -> &Vec<Command> {
        &self.commands
    }

    pub fn commands_mut(&mut self) -> &mut Vec<Command> {
        &mut self.commands
    }
}