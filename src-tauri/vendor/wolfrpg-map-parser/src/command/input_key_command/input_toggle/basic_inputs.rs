#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct BasicInputs {
    ok: bool,
    cancel: bool,
    sub: bool,
    down: bool,
    left: bool,
    right: bool,
    up: bool
}

impl BasicInputs {
    pub fn new(inputs: u8) -> Self {
        Self {
            ok:     inputs & 0b00000001 != 0,
            cancel: inputs & 0b00000010 != 0,
            sub:    inputs & 0b00000100 != 0,
            down:   inputs & 0b00010000 != 0,
            left:   inputs & 0b00100000 != 0,
            right:  inputs & 0b01000000 != 0,
            up:     inputs & 0b10000000 != 0,
        }
    }

    pub fn ok(&self) -> bool {
        self.ok
    }

    pub fn ok_mut(&mut self) -> &mut bool {
        &mut self.ok
    }

    pub fn cancel(&self) -> bool {
        self.cancel
    }

    pub fn cancel_mut(&mut self) -> &mut bool {
        &mut self.cancel
    }

    pub fn sub(&self) -> bool {
        self.sub
    }

    pub fn sub_mut(&mut self) -> &mut bool {
        &mut self.sub
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