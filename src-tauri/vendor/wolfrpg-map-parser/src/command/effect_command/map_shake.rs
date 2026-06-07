#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::as_u32_le;
use shake_type::ShakeType;

pub mod shake_type;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct MapShake {
    power: u8,
    speed: u8,
    shake_type: ShakeType,
    duration: u32,
}

impl MapShake {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let options: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let power: u8 = (options & 0x0f) as u8;
        let speed: u8 = ((options & 0xf0) >> 4) as u8;

        let shake_type: u8 = ((options >> 8) & 0xff) as u8;
        let shake_type: ShakeType = ShakeType::new(shake_type);

        let duration: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        offset += 3; // Command end signature

        (offset, Self {
            power,
            speed,
            shake_type,
            duration
        })
    }

    pub fn power(&self) -> u8 {
        self.power
    }
    
    pub fn power_mut(&mut self) -> &mut u8 {
        &mut self.power
    }

    pub fn speed(&self) -> u8 {
        self.speed
    }
    
    pub fn speed_mut(&mut self) -> &mut u8 {
        &mut self.speed
    }

    pub fn shake_type(&self) -> &ShakeType {
        &self.shake_type
    }
    
    pub fn shake_type_mut(&mut self) -> &mut ShakeType {
        &mut self.shake_type
    }

    pub fn duration(&self) -> u32 {
        self.duration
    }
    
    pub fn duration_mut(&mut self) -> &mut u32 {
        &mut self.duration
    }
}