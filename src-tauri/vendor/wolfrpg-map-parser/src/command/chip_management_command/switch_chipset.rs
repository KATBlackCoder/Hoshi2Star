#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::as_u32_le;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct SwitchChipset {
    chipset: u32
}

impl SwitchChipset {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let chipset: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        offset += 3; // Command end signature

        (offset, Self {
            chipset
        })
    }

    pub fn chipset(&self) -> u32 {
        self.chipset
    }
    
    pub fn chipset_mut(&mut self) -> &mut u32 {
        &mut self.chipset
    }
}