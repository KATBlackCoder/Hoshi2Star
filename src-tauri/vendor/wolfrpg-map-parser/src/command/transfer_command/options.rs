#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::transfer_command::transition::Transition;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Options {
    precise_coordinates: bool,
    transition: Transition,
}

impl Options {
    pub fn new(options: u32) -> Self {
        Self {
            precise_coordinates: options & 0b00000001 != 0,
            transition: Transition::new(((options >> 4) & 0x0f) as u8),
        }
    }

    pub fn precise_coordinates(&self) -> bool {
        self.precise_coordinates
    }
    
    pub fn precise_coordinates_mut(&mut self) -> &mut bool {
        &mut self.precise_coordinates
    }

    pub fn transition(&self) -> &Transition {
        &self.transition
    }
    
    pub fn transition_mut(&mut self) -> &mut Transition {
        &mut self.transition
    }
}