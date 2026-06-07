#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::input_key_command::input_key::direction_keys::DirectionKeys;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct BasicOptions {
    direction_keys: DirectionKeys,
    input_ok: bool,
    input_cancel: bool,
    input_subkey: bool,
    wait_for_input: bool
}

impl BasicOptions {
    pub fn new(options: u8) -> Self {
        Self {
            direction_keys: DirectionKeys::new(options & 0x0f),
            input_ok:       options & 0b00010000 != 0,
            input_cancel:   options & 0b00100000 != 0,
            input_subkey:   options & 0b01000000 != 0,
            wait_for_input: options & 0b10000000 != 0,
        }
    }

    pub fn direction_keys(&self) -> &DirectionKeys {
        &self.direction_keys
    }

    pub fn direction_keys_mut(&mut self) -> &mut DirectionKeys {
        &mut self.direction_keys
    }

    pub fn input_ok(&self) -> bool {
        self.input_ok
    }

    pub fn input_ok_mut(&mut self) -> &mut bool {
        &mut self.input_ok
    }

    pub fn input_cancel(&self) -> bool {
        self.input_cancel
    }

    pub fn input_cancel_mut(&mut self) -> &mut bool {
        &mut self.input_cancel
    }

    pub fn input_subkey(&self) -> bool {
        self.input_subkey
    }

    pub fn input_subkey_mut(&mut self) -> &mut bool {
        &mut self.input_subkey
    }

    pub fn wait_for_input(&self) -> bool {
        self.wait_for_input
    }

    pub fn wait_for_input_mut(&mut self) -> &mut bool {
        &mut self.wait_for_input
    }
}