#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::as_u32_le;
use crate::common::r#move::move_type::MoveType;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub enum State {
    ApproachEvent {
        event: u32,
    },
    Jump {
        right: i32,
        down: i32,
    },
    ApproachPosition {
        x: u32,
        y: u32,
    },
    SetVariable {
        variable: u32,
        value: u32,
    },
    AddToVariable {
        variable: u32,
        value: u32,
    },
    SetMoveSpeed {
        speed: u32,
    },
    SetAnimationSpeed {
        speed: u32,
    },
    SetMoveFrequency {
        frequency: u32,
    },
    SetGraphic {
        graphic: u32,
    },
    SetOpacity {
        opacity: u32,
    },
    SetHeight {
        height: u32,
    },
    PlaySound {
        sound: u32,
    },
    WaitFrames {
        frame_count: u32,
    },
    None
}

impl State {
    pub(crate) fn parse(bytes: &[u8], move_type: &MoveType) -> (usize, Self) {
        match *move_type {
            MoveType::ApproachEvent => {
                Self::parse_approach_event(bytes)
            },
            MoveType::Jump => {
                Self::parse_jump(bytes)
            },
            MoveType::ApproachPosition => {
                Self::parse_approach_position(bytes)
            },
            MoveType::SetVariable => {
                Self::parse_set_variable(bytes)
            },
            MoveType::AddToVariable => {
                Self::parse_add_to_variable(bytes)
            },
            MoveType::SetMoveSpeed => {
                Self::parse_set_move_speed(bytes)
            },
            MoveType::SetAnimationSpeed => {
                Self::parse_set_animation_speed(bytes)
            },
            MoveType::SetMoveFrequency => {
                Self::parse_move_frequency(bytes)
            },
            MoveType::SetGraphic => {
                Self::parse_set_graphic(bytes)
            },
            MoveType::SetOpacity => {
                Self::parse_set_opacity(bytes)
            },
            MoveType::SetHeight => {
                Self::parse_set_height(bytes)
            },
            MoveType::PlaySound => {
                Self::parse_play_sound(bytes)
            },
            MoveType::WaitFrames => {
                Self::parse_wait_frames(bytes)
            },
            _ => (0, Self::None),
        }
    }

    fn parse_approach_event(bytes: &[u8]) -> (usize, State) {
        let mut offset: usize = 0;

        let event: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        (offset, Self::ApproachEvent {
            event,
        })
    }

    fn parse_jump(bytes: &[u8]) -> (usize, State) {
        let mut offset: usize = 0;

        let right: i32 = as_u32_le(&bytes[offset..offset + 4]) as i32;
        offset += 4;

        let down: i32 = as_u32_le(&bytes[offset..offset + 4]) as i32;
        offset += 4;

        (offset, Self::Jump {
            right,
            down,
        })
    }

    fn parse_approach_position(bytes: &[u8]) -> (usize, State) {
        let mut offset: usize = 0;

        let x: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let y: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        (offset, Self::ApproachPosition {
            x,
            y,
        })
    }

    fn parse_set_variable(bytes: &[u8]) -> (usize, State) {
        let mut offset: usize = 0;

        let variable: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let value: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        (offset, Self::SetVariable {
            variable,
            value,
        })
    }

    fn parse_add_to_variable(bytes: &[u8]) -> (usize, State) {
        let mut offset: usize = 0;

        let variable: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let value: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        (offset, Self::AddToVariable {
            variable,
            value,
        })
    }

    fn parse_set_move_speed(bytes: &[u8]) -> (usize, State) {
        let mut offset: usize = 0;

        let speed: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        (offset, Self::SetMoveSpeed {
            speed,
        })
    }

    fn parse_set_animation_speed(bytes: &[u8]) -> (usize, State) {
        let mut offset: usize = 0;

        let speed: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        (offset, Self::SetAnimationSpeed {
            speed,
        })
    }

    fn parse_move_frequency(bytes: &[u8]) -> (usize, State) {
        let mut offset: usize = 0;

        let frequency: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        (offset, Self::SetMoveFrequency {
            frequency,
        })
    }

    fn parse_set_graphic(bytes: &[u8]) -> (usize, State) {
        let mut offset: usize = 0;

        let graphic: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        (offset, Self::SetGraphic {
            graphic,
        })
    }

    fn parse_set_opacity(bytes: &[u8]) -> (usize, State) {
        let mut offset: usize = 0;

        let opacity: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        (offset, Self::SetOpacity {
            opacity,
        })
    }

    fn parse_set_height(bytes: &[u8]) -> (usize, State) {
        let mut offset: usize = 0;

        let height: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        (offset, Self::SetHeight {
            height,
        })
    }

    fn parse_play_sound(bytes: &[u8]) -> (usize, State) {
        let mut offset: usize = 0;

        let sound: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        (offset, Self::PlaySound {
            sound,
        })
    }

    fn parse_wait_frames(bytes: &[u8]) -> (usize, State) {
        let mut offset: usize = 0;

        let frame_count: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        (offset, Self::WaitFrames {
            frame_count,
        })
    }
}