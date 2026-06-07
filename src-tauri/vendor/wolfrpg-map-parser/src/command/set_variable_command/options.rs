#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Options {
    bind_result: bool,
    real_number_calculation: bool,
    left_not_variable: bool,
    right_not_variable: bool,
    use_variable_as_reference: bool,
    use_left_as_reference: bool,
    use_right_as_reference: bool,
}

impl Options {
    pub fn new(options: u8) -> Self {
        Self {
            bind_result:                options & 0b00000001 != 0,
            real_number_calculation:    options & 0b00000010 != 0,
            left_not_variable:          options & 0b00000100 != 0,
            right_not_variable:         options & 0b00001000 != 0,
            use_variable_as_reference:  options & 0b00010000 != 0,
            use_left_as_reference:      options & 0b00100000 != 0,
            use_right_as_reference:     options & 0b01000000 != 0,
        }
    }

    pub fn bind_result(&self) -> bool {
        self.bind_result
    }

    pub fn bind_result_mut(&mut self) -> &mut bool {
        &mut self.bind_result
    }

    pub fn real_number_calculation(&self) -> bool {
        self.real_number_calculation
    }

    pub fn real_number_calculation_mut(&mut self) -> &mut bool {
        &mut self.real_number_calculation
    }

    pub fn left_not_variable(&self) -> bool {
        self.left_not_variable
    }

    pub fn left_not_variable_mut(&mut self) -> &mut bool {
        &mut self.left_not_variable
    }

    pub fn right_not_variable(&self) -> bool {
        self.right_not_variable
    }

    pub fn right_not_variable_mut(&mut self) -> &mut bool {
        &mut self.right_not_variable
    }

    pub fn use_variable_as_reference(&self) -> bool {
        self.use_variable_as_reference
    }

    pub fn use_variable_as_reference_mut(&mut self) -> &mut bool {
        &mut self.use_variable_as_reference
    }

    pub fn use_left_as_reference(&self) -> bool {
        self.use_left_as_reference
    }

    pub fn use_left_as_reference_mut(&mut self) -> &mut bool {
        &mut self.use_left_as_reference
    }

    pub fn use_right_as_reference(&self) -> bool {
        self.use_right_as_reference
    }

    pub fn use_right_as_reference_mut(&mut self) -> &mut bool {
        &mut self.use_right_as_reference
    }
}