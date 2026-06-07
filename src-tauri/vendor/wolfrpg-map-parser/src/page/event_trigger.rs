#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum EventTrigger {
    ConfirmKey      = 0x00,
    AutoRun         = 0x01,
    ParallelProcess = 0x02,
    PlayerTouch     = 0x03,
    EventTouch      = 0x04,
    Unknown
}

impl EventTrigger {
    pub const fn new(trigger: u8) -> Self {
        match trigger {
            0x00 => EventTrigger::ConfirmKey,
            0x01 => EventTrigger::AutoRun,
            0x02 => EventTrigger::ParallelProcess,
            0x03 => EventTrigger::PlayerTouch,
            0x04 => EventTrigger::EventTouch,
            _ => EventTrigger::Unknown
        }
    }
}