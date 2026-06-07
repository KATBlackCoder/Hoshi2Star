use crate::byte_utils::{as_u32_le, as_u32_vec, parse_string};
use crate::common::u32_or_string::U32OrString;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A database table for storing related data.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone)]
pub struct Table {
    index: usize,
    rows: Vec<Vec<U32OrString>>,
}

impl Table {
    pub(crate) fn parse(bytes: &[u8], index: usize) -> (usize, Self) {
        let mut offset: usize = 0;

        let _ = as_u32_le(&bytes[offset..]);
        offset += 4;

        let _db_type: usize = as_u32_le(&bytes[offset..]) as usize;
        offset += 4;

        let field_count: usize = as_u32_le(&bytes[offset..]) as usize;
        offset += 4;

        let fields: Vec<u32> = as_u32_vec(&bytes[offset..][..4 * field_count]);
        offset += 4 * field_count;

        let fields: Vec<(u32, u32)> = fields.iter().map(|f| {
            (f / 1000, f % 1000)
        }).collect();

        let int_count: usize = fields.iter().filter(|(t, _)| *t==1).count();
        let str_count: usize = fields.iter().filter(|(t, _)| *t==2).count();

        let item_count: usize = as_u32_le(&bytes[offset..]) as usize;
        offset += 4;

        let mut rows: Vec<Vec<U32OrString>> = Vec::with_capacity(item_count);

        for _ in 0..item_count {
            let mut row_int: Vec<u32> = vec![];
            let mut row_str = vec![];

            for _ in 0..int_count {
                let int: u32 = as_u32_le(&bytes[offset..]);
                offset += 4;

                row_int.push(int);
            }

            for _ in 0..str_count {
                let (bytes_read, str): (usize, String) = parse_string(&bytes[offset..]);
                offset += bytes_read;

                row_str.push(str);
            }

            let mut row: Vec<U32OrString> = Vec::with_capacity(field_count);

            for (t, i) in &fields {
                match t {
                    1 => row.push(U32OrString::U32(row_int[*i as usize])),
                    2 => row.push(U32OrString::String(row_str[*i as usize].clone())),
                    _ => unreachable!(),
                }
            }

            rows.push(row);
        }

        (offset, Table {
            index,
            rows
        })
    }

    /// The index of this table in the database.
    pub fn index(&self) -> usize {
        self.index
    }

    /// A list of tuples representing the database entries.
    pub fn rows(&self) -> &Vec<Vec<U32OrString>> {
        &self.rows
    }

    /// Mutable reference accessor for [`Table::rows`].
    pub fn rows_mut(&mut self) -> &mut Vec<Vec<U32OrString>> {
        &mut self.rows
    }
}