use crate::byte_utils::{as_u32_le, parse_string};
use crate::command::sound_command::operation::Operation;
use crate::command::sound_command::options::Options;
use crate::command::sound_command::variable::Variable;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Filename {
    variable_state: Variable,
    volume: u32,
    tempo: u32,
    loop_point: Option<u32>,
    sound_filename: String
}

impl Filename {
    pub(crate) fn parse(bytes: &[u8], options: &Options) -> (usize, Self) {
        let mut offset: usize = 0;

        let (bytes_read, variable_state): (usize, Variable) = Variable::parse(bytes, options);
        offset += bytes_read;

        let volume: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let tempo: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let loop_point: Option<u32> = match *options.operation() {
            Operation::SetSE => None,
            _ => {
                let loop_point: u32 = as_u32_le(&bytes[offset..offset + 4]);
                offset += 4;

                Some(loop_point)
            }
        };

        let (bytes_read, sound_filename): (usize, String) = parse_string(&bytes[offset..]);
        offset += bytes_read;

        (offset, Self {
            variable_state,
            volume,
            tempo,
            loop_point,
            sound_filename
        })
    }

    pub fn delay_playback(&self) -> Option<u32> {
        self.variable_state.delay_playback()
    }

    pub fn delay_playback_mut(&mut self) -> &mut Option<u32> {
        self.variable_state.delay_playback_mut()
    }

    pub fn fade_time(&self) -> Option<u32> {
        self.variable_state.fade_time()
    }

    pub fn fade_time_mut(&mut self) -> &mut Option<u32> {
        self.variable_state.fade_time_mut()
    }

    pub fn variable(&self) -> u32 {
        self.variable_state.variable()
    }

    pub fn variable_mut(&mut self) -> &mut u32 {
        self.variable_state.variable_mut()
    }

    pub fn start_time(&self) -> u32 {
        self.variable_state.start_time()
    }

    pub fn start_time_mut(&mut self) -> &mut u32 {
        self.variable_state.start_time_mut()
    }

    pub fn volume(&self) -> u32 {
        self.volume
    }
    
    pub fn volume_mut(&mut self) -> &mut u32 {
        &mut self.volume
    }

    pub fn tempo(&self) -> u32 {
        self.tempo
    }
    
    pub fn tempo_mut(&mut self) -> &mut u32 {
        &mut self.tempo
    }

    pub fn loop_point(&self) -> Option<u32> {
        self.loop_point
    }
    
    pub fn loop_point_mut(&mut self) -> &mut Option<u32> {
        &mut self.loop_point
    }

    pub fn sound_filename(&self) -> &str {
        &self.sound_filename
    }
    
    pub fn sound_filename_mut(&mut self) -> &mut String {
        &mut self.sound_filename
    }
}