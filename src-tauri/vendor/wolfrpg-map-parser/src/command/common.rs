pub mod case;
pub mod case_type;

pub const CASES_END_SIGNATURE: &[u8] = b"\x01\xf3\x01\x00";
pub const LOOP_END_SIGNATURE: &[u8] = b"\x01\xf2\x01\x00\x00\x00\x00\x00";