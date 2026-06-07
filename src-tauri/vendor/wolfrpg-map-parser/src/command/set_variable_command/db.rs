use crate::byte_utils::{as_u16_le, as_u32_le};
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
#[allow(unused)]
pub struct DB {
    unknown1: u16,
    db_type: u32,
    db_data: u32,
    db_field: u32,
    unknown2: u16,
}

impl DB {
    pub(crate) fn parse(bytes: &[u8]) -> (usize, Self) {
        let mut offset = 0;

        let unknown1: u16 = as_u16_le(&bytes[offset..offset + 2]);
        offset += 2;

        let db_type: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let db_data: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let db_field: u32 = as_u32_le(&bytes[offset..offset + 4]);
        offset += 4;

        let unknown2: u16 = as_u16_le(&bytes[offset..offset + 2]);
        offset += 2;

        offset += 1; // command end signature

        (offset, Self {
            unknown1,
            db_type,
            db_data,
            db_field,
            unknown2,
        })
    }

    pub fn db_type(&self) -> u32 {
        self.db_type
    }

    pub fn db_type_mut(&mut self) -> &mut u32 {
        &mut self.db_type
    }

    pub fn db_data(&self) -> u32 {
        self.db_data
    }

    pub fn db_data_mut(&mut self) -> &mut u32 {
        &mut self.db_data
    }

    pub fn db_field(&self) -> u32 {
        self.db_field
    }

    pub fn db_field_mut(&mut self) -> &mut u32 {
        &mut self.db_field
    }
}