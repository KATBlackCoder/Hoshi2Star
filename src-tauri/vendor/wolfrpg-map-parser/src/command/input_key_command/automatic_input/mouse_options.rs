#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::input_key_command::automatic_input::mouse_type::MouseType;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct MouseOptions {
    left_click: bool,
    right_click: bool,
    middle_click: bool,
    mouse_type: MouseType
}

impl MouseOptions {
    pub fn new(options: u8) -> Self {
        Self {
            left_click:     options & 0b00000001 != 0,
            right_click:    options & 0b00000010 != 0,
            middle_click:   options & 0b00000100 != 0,
            mouse_type: MouseType::new(options >> 3)
        }
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

    pub fn mouse_type(&self) -> &MouseType {
        &self.mouse_type
    }

    pub fn mouse_type_mut(&mut self) -> &mut MouseType {
        &mut self.mouse_type
    }
}