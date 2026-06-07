use crate::byte_utils::as_u32_le;
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
#[allow(unused)]
pub struct Base {
    unknown1: u32,
}

impl Base {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset = 0;

        let unknown1: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        offset += 1; // command end signature

        (offset, Self {
            unknown1,
        })
    }
}