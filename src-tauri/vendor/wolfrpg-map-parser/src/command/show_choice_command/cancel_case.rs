#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum CancelCase {
    Choice(u8),
    Separate,
    No
}

impl CancelCase {
    pub fn new(cancel_case: u8) -> Self {
        match cancel_case {
            0 => CancelCase::Separate,
            1 => CancelCase::No,
            _ => CancelCase::Choice(cancel_case - 2)
        }
    }
}