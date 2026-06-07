#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use move_type::MoveType;
use crate::byte_utils::as_u16_le;
use crate::common::r#move::state::State;

pub mod move_type;
pub mod state;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Move {
    move_type: MoveType,
    state: State
}

impl Move {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let move_type: u16 = as_u16_le(&bytes[offset..offset + 2]);
        let move_type: MoveType = MoveType::new(move_type);
        offset += 2;

        let (bytes_read, state): (usize, State) = State::parse(&bytes[offset..], &move_type);
        offset += bytes_read;

        offset += 2; // Move end signature

        (offset, Move {
            move_type,
            state
        })
    }

    pub(crate) fn parse_multiple(bytes: &[u8], move_count: u32) -> (usize, Vec<Move>) {
        let mut offset: usize = 0;
        let mut moves: Vec<Move> = Vec::with_capacity(move_count as usize);

        for _ in 0..move_count {
            let (bytes_read, mov): (usize, Move) = Move::parse(&bytes[offset..]);
            offset += bytes_read;
            moves.push(mov);
        }

        (offset, moves)
    }

    pub fn move_type(&self) -> &MoveType {
        &self.move_type
    }
    
    pub fn move_type_mut(&mut self) -> &mut MoveType {
        &mut self.move_type
    }

    pub fn state(&self) -> &State {
        &self.state
    }
    
    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }
}