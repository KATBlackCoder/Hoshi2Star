#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::as_u32_le;
use crate::command::sound_command::operation::Operation;
use crate::command::sound_command::options::Options;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Variable {
    delay_playback: Option<u32>,
    fade_time: Option<u32>,
    variable: u32,
    start_time: u32,
}

impl Variable {
    pub(crate) fn parse(bytes: &[u8], options: &Options) -> (usize, Self) {
        let mut offset: usize = 0;

        let value: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let (delay_playback, fade_time): (Option<u32>, Option<u32>) = match *options.operation() {
            Operation::SetSE => (Some(value), None),
            _ => (None, Some(value))
        };

        let variable: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let start_time: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        offset += 2; // Padding

        (offset, Self {
            delay_playback,
            fade_time,
            variable,
            start_time
        })
    }

    pub fn delay_playback(&self) -> Option<u32> {
        self.delay_playback
    }
    
    pub fn delay_playback_mut(&mut self) -> &mut Option<u32> {
        &mut self.delay_playback
    }

    pub fn fade_time(&self) -> Option<u32> {
        self.fade_time
    }
    
    pub fn fade_time_mut(&mut self) -> &mut Option<u32> {
        &mut self.fade_time
    }

    pub fn variable(&self) -> u32 {
        self.variable
    }
    
    pub fn variable_mut(&mut self) -> &mut u32 {
        &mut self.variable
    }

    pub fn start_time(&self) -> u32 {
        self.start_time
    }
    
    pub fn start_time_mut(&mut self) -> &mut u32 {
        &mut self.start_time
    }
}