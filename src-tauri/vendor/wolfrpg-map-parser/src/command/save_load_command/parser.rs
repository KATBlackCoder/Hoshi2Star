use crate::byte_utils::as_u32_le;

pub(crate) fn parse_variable_fields(bytes: &[u8]) -> (usize, (u32, u32, u32, bool)) {
    let mut offset: usize = 0;

    let variable1: u32 = as_u32_le(&bytes[offset..offset + 4]);
    offset += 4;

    let save_number: u32 = as_u32_le(&bytes[offset..offset + 4]);
    offset += 4;

    let variable2: u32 = as_u32_le(&bytes[offset..offset + 4]);
    offset += 4;

    let is_pointer: bool = as_u32_le(&bytes[offset..offset+4]) != 0;
    offset += 4;

    (offset, (
        variable1,
        save_number,
        variable2,
        is_pointer,
    ))
}