#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::as_u32_le;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct ChangeColor {
    red: u8,
    green: u8,
    blue: u8,
    flash: bool,
    duration: u32
}

impl ChangeColor {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let red: u8 = bytes[offset];
        offset += 1;

        let green: u8 = bytes[offset];
        offset += 1;

        let blue: u8 = bytes[offset];
        offset += 1;

        let flash: bool = bytes[offset] != 0;
        offset += 1;

        let duration: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        offset += 3; // Command end signature

        (offset, Self {
            red,
            green,
            blue,
            flash,
            duration
        })
    }

    pub fn red(&self) -> u8 {
        self.red
    }
    
    pub fn red_mut(&mut self) -> &mut u8 {
        &mut self.red
    }

    pub fn green(&self) -> u8 {
        self.green
    }
    
    pub fn green_mut(&mut self) -> &mut u8 {
        &mut self.green
    }

    pub fn blue(&self) -> u8 {
        self.blue
    }
    
    pub fn blue_mut(&mut self) -> &mut u8 {
        &mut self.blue
    }

    pub fn flash(&self) -> bool {
        self.flash
    }
    
    pub fn flash_mut(&mut self) -> &mut bool {
        &mut self.flash
    }

    pub fn duration(&self) -> u32 {
        self.duration
    }
    
    pub fn duration_mut(&mut self) -> &mut u32 {
        &mut self.duration
    }
}