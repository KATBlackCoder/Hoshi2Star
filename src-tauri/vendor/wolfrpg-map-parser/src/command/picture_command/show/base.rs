use crate::byte_utils::as_u32_le;
use crate::command::picture_command::display_type::DisplayType;
use crate::command::picture_command::options::Options;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Base {
    position_x: u32,
    position_y: u32
}

impl Base {
    pub(crate) fn parse(bytes: &[u8], options: &Options) -> (usize, Option<u32>, Self) {
        let mut offset: usize = 0;

        let position_x: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let position_y: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        offset += 4; // zoom
        offset += 4; // angle

        let (bytes_read, filename_variable): (usize, Option<u32>)
            = Self::parse_filename_variable(&bytes[offset..], options);
        offset += bytes_read;

        offset += 1; // Padding

        (offset, filename_variable, Base{
            position_x,
            position_y,
        })
    }

    fn parse_filename_variable(bytes: &[u8], options: &Options) -> (usize, Option<u32>) {
        let mut offset: usize = 0;
        let filename_variable: Option<u32> = match *options.display_type() {
            DisplayType::StringVar | DisplayType::WindowByStringVar => {
                let filename_variable: u32 = as_u32_le(&bytes[offset..offset+4]);
                offset += 4;

                Some(filename_variable)
            }

            _ => None
        };

        (offset, filename_variable)
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
}