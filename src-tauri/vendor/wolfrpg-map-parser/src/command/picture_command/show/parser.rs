use crate::byte_utils::as_u32_le;
use crate::command::picture_command::show::parsable_fields::ParsableFields;

pub(crate) fn parse_fields<T: ParsableFields<T>>(bytes: &[u8])
                -> (usize, (u32, u32, Option<u32>, T)) {
    let mut offset: usize = 0;

    let position_x: u32 = as_u32_le(&bytes[offset..offset+4]);
    offset += 4;

    let position_y: u32 = as_u32_le(&bytes[offset..offset+4]);
    offset += 4;

    offset += 4; // zoom
    offset += 4; // angle

    let filename_variable: u32 = as_u32_le(&bytes[offset..offset+4]);
    offset += 4;

    offset += 3; // Padding

    let (bytes_read, fields): (usize, T)
        = T::parse(&bytes[offset..]);
    offset += bytes_read;

    offset += 1; // Padding

    (offset, (
        position_x,
        position_y,
        Some(filename_variable),
        fields
    ))
}