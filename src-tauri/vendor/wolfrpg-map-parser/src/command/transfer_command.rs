#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::as_u32_le;
use crate::command::transfer_command::options::Options;
use crate::command::transfer_command::target::Target;

pub mod target;
pub mod options;
pub mod transition;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct TransferCommand {
    target: Target,
    db_variable: Option<u32>,
    destination_x: u32,
    destination_y: u32,
    destination_map: Option<u32>,
    options: Options
}

impl TransferCommand {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let target: u32 = as_u32_le(&bytes[offset..offset+4]);
        let target: Target = Target::new(target);
        offset += 4;

        let db_variable: Option<u32> = match target {
            Target::SavedPosition => {
                let db_variable: u32 = as_u32_le(&bytes[offset..offset+4]);
                offset += 4;

                Some(db_variable)
            },
            _ => None
        };

        let destination_x: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let destination_y: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let destination_map: Option<u32> = match target {
            Target::SavedPosition => None,
            _ => {
                let destination_map: u32 = as_u32_le(&bytes[offset..offset+4]);
                offset += 4;

                Some(destination_map)
            }
        };

        let options: u32 = as_u32_le(&bytes[offset..offset+4]);
        let options: Options = Options::new(options);
        offset += 4;

        offset += 3; // Command end signature

        (offset, Self {
            target,
            db_variable,
            destination_x,
            destination_y,
            destination_map,
            options
        })
    }

    pub fn target(&self) -> &Target {
        &self.target
    }
    
    pub fn target_mut(&mut self) -> &mut Target {
        &mut self.target
    }

    pub fn db_variable(&self) -> Option<u32> {
        self.db_variable
    }
    
    pub fn db_variable_mut(&mut self) -> &mut Option<u32> {
        &mut self.db_variable
    }

    pub fn destination_x(&self) -> u32 {
        self.destination_x
    }
    
    pub fn destination_x_mut(&mut self) -> &mut u32 {
        &mut self.destination_x
    }

    pub fn destination_y(&self) -> u32 {
        self.destination_y
    }
    
    pub fn destination_y_mut(&mut self) -> &mut u32 {
        &mut self.destination_y
    }

    pub fn destination_map(&self) -> Option<u32> {
        self.destination_map
    }
    
    pub fn destination_map_mut(&mut self) -> &mut Option<u32> {
        &mut self.destination_map
    }

    pub fn options(&self) -> &Options {
        &self.options
    }
    
    pub fn options_mut(&mut self) -> &mut Options {
        &mut self.options
    }
}