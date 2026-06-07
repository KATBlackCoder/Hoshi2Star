#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::command::effect_command::scroll_screen::scroll_operation::ScrollOperation;
use crate::command::effect_command::scroll_screen::scroll_speed::ScrollSpeed;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Options {
    scroll_operation: ScrollOperation,
    scroll_speed: ScrollSpeed,
    wait_until_done: bool,
    pixel_units: bool
}

impl Options {
    pub fn new(options: u32) -> Self {
        Self {
            scroll_operation: ScrollOperation::new((options & 0x0f) as u8),
            scroll_speed: ScrollSpeed::new(((options & 0xf0) >> 4) as u8),
            wait_until_done: (options >> 8) & 0b00000001 != 0,
            pixel_units:     (options >> 8) & 0b00000010 != 0,
        }
    }

    pub fn scroll_operation(&self) -> &ScrollOperation {
        &self.scroll_operation
    }
    
    pub fn scroll_operation_mut(&mut self) -> &mut ScrollOperation {
        &mut self.scroll_operation
    }

    pub fn scroll_speed(&self) -> &ScrollSpeed {
        &self.scroll_speed
    }
    
    pub fn scroll_speed_mut(&mut self) -> &mut ScrollSpeed {
        &mut self.scroll_speed
    }

    pub fn wait_until_done(&self) -> bool {
        self.wait_until_done
    }
    
    pub fn wait_until_done_mut(&mut self) -> &mut bool {
        &mut self.wait_until_done
    }

    pub fn pixel_units(&self) -> bool {
        self.pixel_units
    }
    
    pub fn pixel_units_mut(&mut self) -> &mut bool {
        &mut self.pixel_units
    }
}