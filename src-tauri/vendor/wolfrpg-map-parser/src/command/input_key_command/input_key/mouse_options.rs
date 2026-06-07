#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::input_key_command::input_key::mouse_target::MouseTarget;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct MouseOptions {
    target: MouseTarget,
    left_click: bool,
    right_click: bool,
    middle_click: bool,
    wait_for_input: bool
}

impl MouseOptions {
    pub fn new(options: u8) -> Self {
        Self {
            target: MouseTarget::new(options & 0x0f),
            left_click:     options & 0b00010000 != 0,
            right_click:    options & 0b00100000 != 0,
            middle_click:   options & 0b01000000 != 0,
            wait_for_input: options & 0b10000000 != 0
        }
    }

    pub fn target(&self) -> &MouseTarget {
        &self.target
    }

    pub fn target_mut(&mut self) -> &mut MouseTarget {
        &mut self.target
    }

    pub fn left_click(&self) -> bool {
        self.left_click
    }

    pub fn left_click_mut(&mut self) -> &mut bool {
        &mut self.left_click
    }

    pub fn right_click(&self) -> bool {
        self.right_click
    }

    pub fn right_click_mut(&mut self) -> &mut bool {
        &mut self.right_click
    }

    pub fn middle_click(&self) -> bool {
        self.middle_click
    }

    pub fn middle_click_mut(&mut self) -> &mut bool {
        &mut self.middle_click
    }

    pub fn wait_for_input(&self) -> bool {
        self.wait_for_input
    }

    pub fn wait_for_input_mut(&mut self) -> &mut bool {
        &mut self.wait_for_input
    }
}