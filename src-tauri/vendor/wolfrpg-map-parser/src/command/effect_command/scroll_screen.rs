#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::as_u32_le;
use crate::command::effect_command::scroll_screen::options::Options;

pub mod options;
pub mod scroll_operation;
pub mod scroll_speed;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct ScrollScreen {
    options: Options,
    x: u32,
    y: u32
}

impl ScrollScreen {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let options: u32 = as_u32_le(&bytes[offset..offset+4]);
        let options: Options = Options::new(options);
        offset += 4;

        let x: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let y: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        offset += 3; // Command end signature

        (offset, Self {
            options,
            x,
            y
        })
    }

    pub fn options(&self) -> &Options {
        &self.options
    }
    
    pub fn options_mut(&mut self) -> &mut Options {
        &mut self.options
    }

    pub fn x(&self) -> u32 {
        self.x
    }
    
    pub fn x_mut(&mut self) -> &mut u32 {
        &mut self.x
    }

    pub fn y(&self) -> u32 {
        self.y
    }
    
    pub fn y_mut(&mut self) -> &mut u32 {
        &mut self.y
    }
}