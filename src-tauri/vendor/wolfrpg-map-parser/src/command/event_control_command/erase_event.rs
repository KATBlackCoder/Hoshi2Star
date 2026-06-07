#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::as_u32_le;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct EraseEvent {
    event: u32,
    fade_frames: u32
}

impl EraseEvent {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let event: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let fade_frames: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        offset += 3; // Command end signature

        (offset, EraseEvent {
            event,
            fade_frames
        })
    }

    pub fn event(&self) -> u32 {
        self.event
    }
    
    pub fn event_mut(&mut self) -> &mut u32 {
        &mut self.event
    }

    pub fn fade_frames(&self) -> u32 {
        self.fade_frames
    }
    
    pub fn fade_frames_mut(&mut self) -> &mut u32 {
        &mut self.fade_frames
    }
}