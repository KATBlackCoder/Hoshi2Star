#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct ExtraCases {
    left_key: bool,
    right_key: bool,
    force_exit: bool
}

impl ExtraCases {
    pub fn new(extra_cases: u8) -> Self {
        Self {
            left_key:   extra_cases & 0b00000001 != 0,
            right_key:  extra_cases & 0b00000010 != 0,
            force_exit: extra_cases & 0b00000100 != 0
        }
    }

    pub fn count(&self) -> usize {
        self.left_key as usize
        + self.right_key as usize
        + self.force_exit as usize
    }

    pub fn left_key(&self) -> bool {
        self.left_key
    }

    pub fn left_key_mut(&mut self) -> &mut bool {
        &mut self.left_key
    }

    pub fn right_key(&self) -> bool {
        self.right_key
    }

    pub fn right_key_mut(&mut self) -> &mut bool {
        &mut self.right_key
    }

    pub fn force_exit(&self) -> bool {
        self.force_exit
    }

    pub fn force_exit_mut(&mut self) -> &mut bool {
        &mut self.force_exit
    }
}