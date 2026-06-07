#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::as_u32_le;
use crate::command::input_key_command::input_toggle::device_inputs::DeviceInputs;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Device {
    inputs: DeviceInputs,
    enable: bool,
    key_code: Option<u32>
}

impl Device {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let inputs: u8 = bytes[offset];
        let inputs: DeviceInputs = DeviceInputs::new(inputs);
        offset += 1;

        let enable: bool = bytes[offset] == 0;
        offset += 1;

        offset += 1; // Padding

        offset += 1; // input_type

        let key_code: Option<u32> = match inputs {
            DeviceInputs::KeyboardKey => {
                let key_code: u32 = as_u32_le(&bytes[offset..offset + 4]);
                offset += 4;

                Some(key_code)
            }

            _ => None,
        };

        (offset, Self {
            inputs,
            enable,
            key_code
        })
    }

    pub fn inputs(&self) -> &DeviceInputs {
        &self.inputs
    }

    pub fn inputs_mut(&mut self) -> &mut DeviceInputs {
        &mut self.inputs
    }

    pub fn enable(&self) -> bool {
        self.enable
    }

    pub fn enable_mut(&mut self) -> &mut bool {
        &mut self.enable
    }

    pub fn key_code(&self) -> Option<u32> {
        self.key_code
    }

    pub fn key_code_mut(&mut self) -> &mut Option<u32> {
        &mut self.key_code
    }
}