#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct BasicOptions {
    input_ok: bool,
    input_cancel: bool,
    input_subkey: bool,
    down: bool,
    left: bool,
    right: bool,
    up: bool,
}

impl BasicOptions {
    pub fn new(options: u8) -> Self {
        Self {
            input_ok:       options & 0b00000001 != 0,
            input_cancel:   options & 0b00000010 != 0,
            input_subkey:   options & 0b00000100 != 0,
            down:           options & 0b00010000 != 0,
            left:           options & 0b00100000 != 0,
            right:          options & 0b01000000 != 0,
            up:             options & 0b10000000 != 0,
        }
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

    pub fn down(&self) -> bool {
        self.down
    }

    pub fn down_mut(&mut self) -> &mut bool {
        &mut self.down
    }

    pub fn left(&self) -> bool {
        self.left
    }

    pub fn left_mut(&mut self) -> &mut bool {
        &mut self.left
    }

    pub fn right(&self) -> bool {
        self.right
    }

    pub fn right_mut(&mut self) -> &mut bool {
        &mut self.right
    }

    pub fn up(&self) -> bool {
        self.up
    }

    pub fn up_mut(&mut self) -> &mut bool {
        &mut self.up
    }
}