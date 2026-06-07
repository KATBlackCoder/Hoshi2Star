#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Options {
    repeat_actions: bool,
    skip_impossible_moves: bool,
    wait_until_done: bool,
}

impl Options {
    pub const fn new(options: u8) -> Self {
        Self {
            repeat_actions:         options & 0b00000001 != 0,
            skip_impossible_moves:  options & 0b00000010 != 0,
            wait_until_done:        options & 0b00000100 != 0,
        }
    }

    pub fn repeat_actions(&self) -> bool {
        self.repeat_actions
    }
    
    pub fn repeat_actions_mut(&mut self) -> &mut bool {
        &mut self.repeat_actions
    }

    pub fn skip_impossible_moves(&self) -> bool {
        self.skip_impossible_moves
    }
    
    pub fn skip_impossible_moves_mut(&mut self) -> &mut bool {
        &mut self.skip_impossible_moves
    }

    pub fn wait_until_done(&self) -> bool {
        self.wait_until_done
    }
    
    pub fn wait_until_done_mut(&mut self) -> &mut bool {
        &mut self.wait_until_done
    }
}