//! In-house parser/writer for WOLF RPG Editor v3.x (>= Ver 3.00, `version >= 0x67`)
//! binary formats (`.mps` maps, `CommonEvent.dat`).
//!
//! Based on the flat command-frame model used by WolfTL
//! (github.com/Sinflower/WolfTL, MIT): every command is read generically as
//! `[argsCount][cid][args...][indent][stringArgs...][terminator][v35Unknown?]`,
//! in a single non-recursive loop. This differs from `wolfrpg_map_parser`'s
//! recursive container model, which does not compose with the v3.5+
//! `v35Unknown` trailer (see `docs/plans/f5-01-wolf-v3-mps-parser.md`).
//!
//! Scope: v3.x (Inko) only. v2.x (Honoka) continues to use `wolfrpg_map_parser`.

pub(crate) mod coder;
pub(crate) mod command;
pub(crate) mod common_events;
pub(crate) mod compression;
pub(crate) mod map;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum V3FormatError {
    #[error(
        "unexpected end of data at offset {offset}: needed {needed} bytes, {available} available"
    )]
    UnexpectedEof {
        offset: usize,
        needed: usize,
        available: usize,
    },
    #[error("zero-length string at offset {offset}")]
    ZeroLengthString { offset: usize },
    #[error("encoding error at offset {offset}: {message}")]
    InvalidEncoding { offset: usize, message: String },
    #[error("invalid command args-count byte {found:#04x} at offset {offset} (expected >= 1)")]
    InvalidArgsCount { offset: usize, found: u8 },
    #[error("invalid command terminator {found:#04x} at offset {offset} (expected 0x00 or 0x01)")]
    InvalidTerminator { offset: usize, found: u8 },
    #[error("invalid RouteCommand magic at offset {offset}: expected {expected:02x?}, found {found:02x?}")]
    InvalidRouteMagic {
        offset: usize,
        expected: [u8; 2],
        found: [u8; 2],
    },
    #[error("LZ4 block error: {message}")]
    Lz4Block { message: String },
    #[error("invalid magic at offset {offset}: expected {expected:02x?}, found {found:02x?} ({context})")]
    InvalidMagic {
        offset: usize,
        expected: Vec<u8>,
        found: Vec<u8>,
        context: &'static str,
    },
    #[error(
        "unexpected byte {found:#04x} at offset {offset} (expected {expected:#04x}, {context})"
    )]
    UnexpectedByte {
        offset: usize,
        expected: u8,
        found: u8,
        context: &'static str,
    },
    #[error("count mismatch: expected {expected}, got {actual} ({context})")]
    CountMismatch {
        expected: u32,
        actual: u32,
        context: &'static str,
    },
    #[error("trailing data after map terminator: {remaining} bytes remain at offset {offset}")]
    TrailingData { offset: usize, remaining: usize },
}
