#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::as_u32_le;
use crate::command::chip_management_command::options::Options;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct MapChipSettings {
    chip: u32,
    options: Options
}

impl MapChipSettings {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let chip: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let options: u32 = as_u32_le(&bytes[offset..offset + 4]);
        let options: Options = Options::new(options);
        offset += 4;

        offset += 3; // Offset

        (offset, Self {
            chip,
            options
        })
    }

    pub fn chip(&self) -> u32 {
        self.chip
    }
    
    pub fn chip_mut(&mut self) -> &mut Options {
        &mut self.options
    }

    pub fn options(&self) -> &Options {
        &self.options
    }
    
    pub fn options_mut(&mut self) -> &mut Options {
        &mut self.options
    }
}