use std::borrow::Cow;
use encoding_rs::{Encoding, SHIFT_JIS};

#[allow(clippy::identity_op)]
pub fn as_u32_le(bytes: &[u8]) -> u32 {
    ((bytes[0] as u32) <<  0) |
    ((bytes[1] as u32) <<  8) |
    ((bytes[2] as u32) << 16) |
    ((bytes[3] as u32) << 24)
}

#[allow(clippy::identity_op)]
pub fn as_u32_be(bytes: &[u8]) -> u32 {
    ((bytes[0] as u32) << 24) |
    ((bytes[1] as u32) << 16) |
    ((bytes[2] as u32) <<  8) |
    ((bytes[3] as u32) <<  0)
}

#[allow(clippy::identity_op)]
pub fn as_u16_le(bytes: &[u8]) -> u16 {
    ((bytes[0] as u16) << 0) |
    ((bytes[1] as u16) << 8)
}

#[allow(unused)]
#[allow(clippy::identity_op)]
pub fn as_u16_be(bytes: &[u8]) -> u16 {
    ((bytes[0] as u16) << 8) |
    ((bytes[1] as u16) << 0)
}

pub fn as_u32_vec(bytes: &[u8]) -> Vec<u32> {
    let mut vec: Vec<u32> = vec!();
    for i in 0 .. (bytes.len() / 4) {
        vec.push(as_u32_le(&bytes[i*4..][..4]));
    }

    vec
}

pub fn as_string(bytes: &[u8], offset: usize, string_length: usize) -> String {
    let string_bytes: &[u8] = &bytes[offset..offset + string_length - 1];

    let (cow, encoding, _): (Cow<str>, &Encoding, bool) = SHIFT_JIS.decode(string_bytes);

    if encoding != SHIFT_JIS {
        panic!("String is not SHIFT-JIS");
    }

    cow.to_string()
}

pub fn parse_string(bytes: &[u8]) -> (usize, String) {
    let mut offset: usize = 0;

    let length: usize = as_u32_le(&bytes[offset..offset+4]) as usize;
    offset += 4;

    let string: String = as_string(bytes, offset, length);
    offset += length;

    (offset, string)
}

pub fn parse_string_vec(bytes: &[u8], count: usize) -> (usize, Vec<String>) {
    let mut offset: usize = 0;
    let mut strings: Vec<String> = Vec::with_capacity(count);

    for _ in 0..count {
        let (bytes_read, choice): (usize, String) = parse_string(&bytes[offset..]);
        offset += bytes_read;
        strings.push(choice);
    }

    (offset, strings)
}

pub fn as_blob(bytes: &[u8], element_size: usize) -> (usize, Vec<u8>) {
    let mut offset: usize = 0;

    let count: usize = as_u32_le(&bytes[offset..offset+4]) as usize * element_size;
    offset += 4;

    let blob: Vec<u8> = bytes[offset..offset + count].to_vec();
    offset += count;

    (offset, blob)
}

macro_rules! parse_optional_string {
    ( $bytes:expr, $offset:expr, $is_string:expr ) => {
        if $is_string {
            let (bytes_read, string): (usize, String)
                = parse_string(&$bytes[$offset..]);
            $offset += bytes_read;

            Some(string)
        } else {
            None
        }
    };
}

pub(crate) use parse_optional_string;