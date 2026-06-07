#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::as_u16_le;
use crate::command::sound_command::options::Options;
use crate::command::sound_command::sound_type::SoundType;
use crate::command::sound_command::state::State;

pub mod options;
pub mod process_type;
pub mod operation;
pub mod sound_type;
pub mod state;
pub mod filename;
pub mod variable;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct SoundCommand {
    options: Options,
    systemdb_entry: u16,
    sound_type: SoundType,
    state: State
}

impl SoundCommand {
    fn parse(bytes: &[u8], parse_state: fn(&[u8], &Options, &SoundType) -> (usize, State))
        -> (usize, Self) {
        let mut offset: usize = 0;

        let options: u8 = bytes[offset];
        let options: Options = Options::new(options);
        offset += 1;

        let systemdb_entry: u16 = as_u16_le(&bytes[offset..offset + 2]);
        offset += 2;

        let sound_type: SoundType = SoundType::new(bytes[offset]);
        offset += 1;

        let (bytes_read, state): (usize, State)
            = parse_state(&bytes[offset..], &options, &sound_type);
        offset += bytes_read;

        offset += 1; // Command end signature

        (offset, Self {
            options,
            systemdb_entry,
            sound_type,
            state
        })
    }

    pub(crate) fn parse_filename(bytes: &[u8]) -> (usize, Self) {
        Self::parse(bytes, State::parse_filename)
    }

    pub(crate) fn parse_variable(bytes: &[u8]) -> (usize, Self) {
        Self::parse(bytes, State::parse_variable)
    }

    pub(crate) fn parse_free_all(bytes: &[u8]) -> (usize, Self) {
        Self::parse(bytes, State::parse_free_all)
    }

    pub fn options(&self) -> &Options {
        &self.options
    }
    
    pub fn options_mut(&mut self) -> &mut Options {
        &mut self.options
    }

    pub fn systemdb_entry(&self) -> u16 {
        self.systemdb_entry
    }
    
    pub fn systemdb_entry_mut(&mut self) -> &mut u16 {
        &mut self.systemdb_entry
    }

    pub fn sound_type(&self) -> &SoundType {
        &self.sound_type
    }
    
    pub fn sound_type_mut(&mut self) -> &mut SoundType {
        &mut self.sound_type
    }

    pub fn state(&self) -> &State {
        &self.state
    }
    
    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }
}