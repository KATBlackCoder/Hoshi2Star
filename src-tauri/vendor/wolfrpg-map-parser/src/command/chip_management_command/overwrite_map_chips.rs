#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::as_u32_le;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct OverwriteMapChips{
    layer: u32,
    position_x: u32,
    position_y: u32,
    width: u32,
    height: u32,
    chip: u32
}

impl OverwriteMapChips{
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let layer: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let position_x: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let position_y: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let width: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let height: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let chip: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        offset += 3; // Command end signature

        (offset, Self {
            layer,
            position_x,
            position_y,
            width,
            height,
            chip
        })
    }

    pub fn layer(&self) -> u32 {
        self.layer
    }
    
    pub fn layer_mut(&mut self) -> &mut u32 {
        &mut self.layer
    }

    pub fn position_x(&self) -> u32 {
        self.position_x
    }
    
    pub fn position_x_mut(&mut self) -> &mut u32 {
        &mut self.position_x
    }

    pub fn position_y(&self) -> u32 {
        self.position_y
    }
    
    pub fn position_y_mut(&mut self) -> &mut u32 {
        &mut self.position_y
    }

    pub fn width(&self) -> u32 {
        self.width
    }
    
    pub fn width_mut(&mut self) -> &mut u32 {
        &mut self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
    
    pub fn height_mut(&mut self) -> &mut u32 {
        &mut self.height
    }

    pub fn chip(&self) -> u32 {
        self.chip
    }
    
    pub fn chip_mut(&mut self) -> &mut u32 {
        &mut self.chip
    }
}