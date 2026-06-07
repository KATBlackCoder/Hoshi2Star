#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct KeyOptions {
    wait_for_input: bool
}

impl KeyOptions {
    pub fn new(options: u8) -> Self {
        Self {
            wait_for_input: options & 0b10000000 != 0
        }
    }

    pub fn wait_for_input(&self) -> bool {
        self.wait_for_input
    }

    pub fn wait_for_input_mut(&mut self) -> &mut bool {
        &mut self.wait_for_input
    }
}