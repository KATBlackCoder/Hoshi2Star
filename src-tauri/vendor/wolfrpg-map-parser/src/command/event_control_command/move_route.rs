pub mod options;

#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use crate::byte_utils::as_u32_le;
use crate::common::r#move::Move;
use crate::command::event_control_command::move_route::options::Options;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
#[allow(unused)]
pub struct MoveRoute {
    target: u32,
    unknown1: u32,
    unknown2: u32,
    options: Options,
    move_count: u32,
    moves: Vec<Move>
}

impl MoveRoute {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset: usize = 0;

        let target: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let unknown1: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let unknown2: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let options: u8 = bytes[offset];
        let options: Options = Options::new(options);
        offset += 1;

        let move_count: u32 = as_u32_le(&bytes[offset..offset+4]);
        offset += 4;

        let (bytes_read, moves): (usize, Vec<Move>)
            = Move::parse_multiple(&bytes[offset..], move_count);
        offset += bytes_read;

        (offset, Self {
            target,
            unknown1,
            unknown2,
            options,
            move_count,
            moves
        })
    }

    pub fn target(&self) -> u32 {
        self.target
    }
    
    pub fn target_mut(&mut self) -> &mut u32 {
        &mut self.target
    }

    pub fn options(&self) -> &Options {
        &self.options
    }
    
    pub fn options_mut(&mut self) -> &mut Options {
        &mut self.options
    }

    pub fn move_count(&self) -> u32 {
        self.move_count
    }
    
    pub fn move_count_mut(&mut self) -> &mut u32 {
        &mut self.move_count
    }

    pub fn moves(&self) -> &Vec<Move> {
        &self.moves
    }
    
    pub fn moves_mut(&mut self) -> &mut Vec<Move> {
        &mut self.moves
    }
}