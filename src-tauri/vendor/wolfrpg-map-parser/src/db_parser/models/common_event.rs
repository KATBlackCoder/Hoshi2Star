pub mod argument;
pub mod argument_type;
pub mod run_condition;
pub mod condition_type;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use crate::byte_utils::{as_u32_le, as_u32_vec, parse_string, parse_string_vec};
use crate::command::Command;
use crate::db_parser::common_event::run_condition::RunCondition;
use crate::db_parser::models::common_event::argument::Argument;

const EVENT_SIGNATURE: u8 = 0x8e;
const END_SIGNATURE: u8 = 0x92;

/// A common event, representing a common series of command that can be invoked from any event
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct CommonEvent {
    id: u32,
    run_condition: RunCondition,
    event_name: String,
    commands: Vec<Command>,
    note: String,
    arguments: Vec<Argument>,
    color: u32,
    var_names: Vec<String>,
    return_name: String,
    return_variable: u32
}

impl CommonEvent {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let signature: u8 = bytes[offset];
        offset += 1;

        if signature != EVENT_SIGNATURE {
            panic!("Invalid common event signature: {:02x}.", signature);
        }

        let id: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let condition_settings: u8 = bytes[offset];
        offset += 1;
        
        let condition_variable: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let condition_value: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;
        
        let run_condition: RunCondition = RunCondition::new(
            condition_settings,
            condition_variable,
            condition_value
        );

        let number_arguments_count: u8 = bytes[offset];
        offset += 1;

        let string_arguments_count: u8 = bytes[offset];
        offset += 1;

        let (bytes_read, event_name): (usize, String) = parse_string(&bytes[offset..]);
        offset += bytes_read;

        let command_count: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let (bytes_read, commands_read, commands): (usize, u32, Vec<Command>)
            = Command::parse_multiple(&bytes[offset..]);
        offset += bytes_read;

        if commands_read != command_count {
            panic!("Expected {} commands but only found {}.", command_count, commands_read);
        }

        // Unknown field
        let (bytes_read, _): (usize, String) = parse_string(&bytes[offset..]);
        offset += bytes_read;

        let (bytes_read, note): (usize, String) = parse_string(&bytes[offset..]);
        offset += bytes_read;

        offset += 1; // Unknown, probably padding

        let argument_names_count: usize = as_u32_le(&bytes[offset..offset+4]) as usize;
        offset += 4;

        let (bytes_read, argument_names): (usize, Vec<String>)
            = parse_string_vec(&bytes[offset..], argument_names_count);
        offset += bytes_read;

        let argument_types_count: usize = as_u32_le(&bytes[offset..offset+4]) as usize;
        offset += 4;

        let argument_types: &[u8] = &bytes[offset..][..argument_types_count];
        offset += argument_types_count;

        let (bytes_read, db_options): (usize, Vec<Vec<String>>)
            = Self::parse_db_options(&bytes[offset..]);
        offset += bytes_read;

        let (bytes_read, db_references): (usize, Vec<Vec<u32>>)
            = Self::parse_db_references(&bytes[offset..]);
        offset += bytes_read;

        let len: usize = as_u32_le(&bytes[offset..offset+4]) as usize;
        offset += 4;

        let argument_values: Vec<u32> = as_u32_vec(&bytes[offset..][..len*4]);
        offset += len * 4;

        offset += 1; // Unknown, probably padding

        let color: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let (bytes_read, var_names): (usize, Vec<String>)
            = parse_string_vec(&bytes[offset..], 100);
        offset += bytes_read;

        // Unknown values, probably padding
        offset += 1;
        offset += 4 + as_u32_le(&bytes[offset..offset+4]) as usize;
        offset += 1;

        let (bytes_read, return_name): (usize, String)
            = parse_string(&bytes[offset..]);
        offset += bytes_read;

        let return_variable: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let end_signature: u8 = bytes[offset];
        offset += 1;

        if end_signature != END_SIGNATURE {
            panic!("Invalid common event end signature: {:02x}.", end_signature);
        }

        let arguments: Vec<Argument> = Self::pack_arguments(
            number_arguments_count as usize,
            string_arguments_count as usize,
            argument_names,
            argument_types,
            argument_values,
            db_options,
            db_references
        );

        (offset, Self {
            id,
            run_condition,
            event_name,
            commands,
            note,
            arguments,
            color,
            var_names,
            return_name,
            return_variable
        })
    }

    fn parse_db_options(bytes: &[u8]) -> (usize, Vec<Vec<String>>) {
        let mut offset: usize = 0;

        let db_option_count: usize = as_u32_le(&bytes[offset..offset+4]) as usize;
        offset += 4;

        let mut db_options: Vec<Vec<String>> = vec![];

        for _ in 0..db_option_count {
            let len: usize = as_u32_le(&bytes[offset..offset+4]) as usize;
            offset += 4;

            let (bytes_read, options): (usize, Vec<String>)
                = parse_string_vec(&bytes[offset..], len);
            offset += bytes_read;

            db_options.push(options);
        }

        (offset, db_options)
    }

    fn parse_db_references(bytes: &[u8]) -> (usize, Vec<Vec<u32>>) {
        let mut offset: usize = 0;

        let db_references_count: usize = as_u32_le(&bytes[offset..offset+4]) as usize;
        offset += 4;

        let mut db_references: Vec<Vec<u32>> = vec![];

        for _ in 0..db_references_count {
            let len: usize = as_u32_le(&bytes[offset..offset+4]) as usize;
            offset += 4;

            let references: Vec<u32>
                = as_u32_vec(&bytes[offset..][..len*4]);
            offset += len*4;

            db_references.push(references);
        }

        (offset, db_references)
    }

    fn pack_arguments(
        number_argument_count: usize,
        string_argument_count: usize,
        argument_names: Vec<String>,
        argument_types: &[u8],
        default_values: Vec<u32>,
        db_options: Vec<Vec<String>>,
        db_references: Vec<Vec<u32>>
    ) -> Vec<Argument> {
        let mut arguments: Vec<Argument> = vec![];

        for i in 0..number_argument_count {
            let argument_name: String = argument_names[i].clone();
            let argument_type: u8 = argument_types[i];
            let default_value: u32 = default_values[i];
            let db_options: Vec<String> = db_options[i].clone();
            let db_references: Vec<u32> = db_references[i].clone();

            arguments.push(Argument::new(
                argument_type,
                argument_name,
                Some(default_value),
                db_options,
                db_references
            ));
        }

        for i in 5..5+string_argument_count {
            let argument_name: String = argument_names[i].clone();
            let argument_type: u8 = argument_types[i];
            let db_options: Vec<String> = db_options[i].clone();
            let db_references: Vec<u32> = db_references[i].clone();

            arguments.push(Argument::new(
                argument_type,
                argument_name,
                None,
                db_options,
                db_references
            ));
        }

        arguments
    }

    /// The ID of this common event in the database
    pub fn id(&self) -> u32 {
        self.id
    }

    /// The condition under which this event will run
    pub fn run_condition(&self) -> &RunCondition {
        &self.run_condition
    }

    /// The name of this event, which can be used for invoking it
    pub fn event_name(&self) -> &str {
        &self.event_name
    }

    /// The event script, in the form of a [`Command`] collection.
    pub fn commands(&self) -> &Vec<Command> {
        &self.commands
    }

    /// A developer note for this event
    pub fn note(&self) -> &str {
        &self.note
    }

    /// A list of arguments that can be passed when invoking this event
    pub fn arguments(&self) -> &Vec<Argument> {
        &self.arguments
    }

    /// The color of this event, for display purposes
    pub fn color(&self) -> u32 {
        self.color
    }

    /// A list of names for each of the 100 local variables available in this event.
    pub fn var_names(&self) -> &Vec<String> {
        &self.var_names
    }

    /// The name of the variable in which the return value will be put
    pub fn return_name(&self) -> &str {
        &self.return_name
    }

    /// The address of the variable in which the return value will be put
    pub fn return_variable(&self) -> u32 {
        self.return_variable
    }
}