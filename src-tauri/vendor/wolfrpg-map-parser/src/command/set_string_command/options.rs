#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::set_string_command::content_type::ContentType;
use crate::command::set_string_command::variable_type::VariableType;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Options {
    content_type: ContentType,
    variable_type: VariableType,
}

impl Options {
    pub fn new(options: u8) -> Self {
        Self {
            content_type: ContentType::new(options & 0x0f),
            variable_type: VariableType::new(options >> 4 ),
        }
    }

    pub fn content_type(&self) -> &ContentType {
        &self.content_type
    }

    pub fn content_type_mut(&mut self) -> &mut ContentType {
        &mut self.content_type
    }

    pub fn variable_type(&self) -> &VariableType {
        &self.variable_type
    }

    pub fn variable_type_mut(&mut self) -> &mut VariableType {
        &mut self.variable_type
    }
}