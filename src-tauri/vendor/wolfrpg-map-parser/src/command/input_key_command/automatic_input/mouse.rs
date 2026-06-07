#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::as_u32_le;
use crate::command::input_key_command::automatic_input::mouse_options::MouseOptions;
use crate::command::input_key_command::automatic_input::mouse_type::MouseType;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
#[allow(unused)]
pub struct Mouse {
    options: MouseOptions,
    position_x: Option<u32>,
    position_y: Option<u32>,
    wheel_delta: Option<u32>,
    unknown1: Option<u32>
}

impl Mouse {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let options: u8 = bytes[offset];
        let options: MouseOptions = MouseOptions::new(options);
        offset += 1;

        offset += 2; // Padding

        offset += 1; // input_type

        let (mut position_x, mut position_y, mut wheel_delta, mut unknown1)
            : (Option<u32>, Option<u32>, Option<u32>, Option<u32>) = (None, None, None, None);

        match *options.mouse_type() {
            MouseType::Position => {
                position_x = Some(as_u32_le(&bytes[offset..offset + 4]));
                offset += 4;
                position_y = Some(as_u32_le(&bytes[offset..offset + 4]));
                offset += 4;
            }

            MouseType::Wheel => {
                wheel_delta = Some(as_u32_le(&bytes[offset..offset + 4]));
                offset += 4;
                unknown1 = Some(as_u32_le(&bytes[offset..offset + 4]));
                offset += 4;
            }

            _ => ()
        };

        (offset, Self {
            options,
            position_x,
            position_y,
            wheel_delta,
            unknown1
        })
    }

    pub fn options(&self) -> &MouseOptions {
        &self.options
    }

    pub fn options_mut(&mut self) -> &mut MouseOptions {
        &mut self.options
    }

    pub fn position_x(&self) -> Option<u32> {
        self.position_x
    }

    pub fn position_x_mut(&mut self) -> &mut Option<u32> {
        &mut self.position_x
    }

    pub fn position_y(&self) -> Option<u32> {
        self.position_y
    }

    pub fn position_y_mut(&mut self) -> &mut Option<u32> {
        &mut self.position_y
    }

    pub fn wheel_delta(&self) -> Option<u32> {
        self.wheel_delta
    }

    pub fn wheel_delta_mut(&mut self) -> &mut Option<u32> {
        &mut self.wheel_delta
    }
}