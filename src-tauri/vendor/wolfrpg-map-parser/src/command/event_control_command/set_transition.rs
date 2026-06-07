#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::{as_u16_le, as_u32_le};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct SetTransition {
    transition_number: u32,
    fade_frames: u16,
    wait_until_done: bool
}

impl SetTransition {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let transition_number: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let fade_frames: u16 = as_u16_le(&bytes[offset..offset+2]);
        offset += 2;

        let wait_until_done: bool = bytes[offset] != 0;
        offset += 1;

        offset += 4; // Command end signature

        (offset, Self {
            transition_number,
            fade_frames,
            wait_until_done
        })
    }

    pub fn transition_number(&self) -> u32 {
        self.transition_number
    }
    
    pub fn transition_number_mut(&mut self) -> &mut u32 {
        &mut self.transition_number
    }

    pub fn fade_frames(&self) -> u16 {
        self.fade_frames
    }
    
    pub fn fade_frames_mut(&mut self) -> &mut u16 {
        &mut self.fade_frames
    }

    pub fn wait_until_done(&self) -> bool {
        self.wait_until_done
    }
    
    pub fn wait_until_done_mut(&mut self) -> &mut bool {
        &mut self.wait_until_done
    }
}