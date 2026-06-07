pub mod options;
pub mod effect_type;
pub mod effect_target;
pub mod character_effect_type;
pub mod map_effect_type;
pub mod picture_effect_type;

#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::as_u32_le;
use crate::command::effect_command::base::options::Options;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Base {
    options: Options,
    duration: u32,
    target: u32,
    range: u32,
    value1: u32,
    value2: u32,
    value3: u32
}

impl Base {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let options: u32 = as_u32_le(&bytes[offset..offset + 4]);
        let options: Options = Options::new(options);
        offset += 4;

        let duration: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let target: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let range: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let value1: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let value2: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let value3: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        offset += 3; // Command end signature

        (offset, Self {
            options,
            duration,
            target,
            range,
            value1,
            value2,
            value3
        })
    }

    pub fn options(&self) -> &Options {
        &self.options
    }
    
    pub fn options_mut(&mut self) -> &mut Options {
        &mut self.options
    }

    pub fn duration(&self) -> u32 {
        self.duration
    }
    
    pub fn duration_mut(&mut self) -> &mut u32 {
        &mut self.duration
    }

    pub fn target(&self) -> u32 {
        self.target
    }
    
    pub fn target_mut(&mut self) -> &mut u32 {
        &mut self.target
    }

    pub fn range(&self) -> u32 {
        self.range
    }
    
    pub fn range_mut(&mut self) -> &mut u32 {
        &mut self.range
    }

    pub fn value1(&self) -> u32 {
        self.value1
    }
    
    pub fn value1_mut(&mut self) -> &mut u32 {
        &mut self.value1
    }

    pub fn value2(&self) -> u32 {
        self.value2
    }
    
    pub fn value2_mut(&mut self) -> &mut u32 {
        &mut self.value2
    }

    pub fn value3(&self) -> u32 {
        self.value3
    }
    
    pub fn value3_mut(&mut self) -> &mut u32 {
        &mut self.value3
    }
}