#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use crate::db_parser::common_event::argument_type::ArgumentType;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Argument {
    argument_type: ArgumentType,
    argument_name: String,
    default_value: Option<u32>,
    db_options: Vec<String>,
    db_references: Vec<u32>
}

impl Argument {
    pub fn new(
        argument_type: u8,
        argument_name: String,
        default_value: Option<u32>,
        db_options: Vec<String>,
        db_references: Vec<u32>
    ) -> Self {
        Self {
            argument_type: ArgumentType::new(argument_type),
            argument_name,
            default_value,
            db_options,
            db_references
        }
    }

    /// Whether this argument receives a number
    pub fn is_number(&self) -> bool {
        self.default_value.is_some()
    }

    /// Whether this argument receives a string
    pub fn is_string(&self) -> bool {
        self.default_value.is_none()
    }

    /// The type of argument that can be passed
    pub fn argument_type(&self) -> &ArgumentType {
        &self.argument_type
    }

    /// The name of the argument variable
    pub fn argument_name(&self) -> &str {
        &self.argument_name
    }

    /// The default value for this argument. This is only set for number arguments and is `None` otherwise.
    pub fn default_value(&self) -> Option<u32> {
        self.default_value
    }

    /// Extra options for `ArgumentType::DBRef` arguments
    pub fn db_options(&self) -> &Vec<String> {
        &self.db_options
    }

    /// The database the argument references
    pub fn db_references(&self) -> &Vec<u32> {
        &self.db_references
    }
}