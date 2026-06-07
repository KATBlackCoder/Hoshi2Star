#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Options {
    bind_result: bool,
    use_variable_as_reference: bool,
    precise_position: bool
}

impl Options {
    pub fn new(options: u8) -> Self {
        Options {
            bind_result:                options & 0b00000001 != 0,
            use_variable_as_reference:  options & 0b00010000 != 0,
            precise_position:           options & 0b00100000 != 0,
        }
    }

    pub fn bind_result(&self) -> bool {
        self.bind_result
    }

    pub fn bind_result_mut(&mut self) -> &mut bool {
        &mut self.bind_result
    }

    pub fn use_variable_as_reference(&self) -> bool {
        self.use_variable_as_reference
    }

    pub fn use_variable_as_reference_mut(&mut self) -> &mut bool {
        &mut self.use_variable_as_reference
    }

    pub fn precise_position(&self) -> bool {
        self.precise_position
    }

    pub fn precise_position_mut(&mut self) -> &mut bool {
        &mut self.precise_position
    }
}