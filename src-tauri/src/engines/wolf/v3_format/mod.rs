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
//! Scope: v3.x LZ4 files and files stored in clear — v2.x Shift-JIS maps
//! (`0x64`) and `CommonEvent.dat` (e.g. `0x8F`), v3.x `0x64` UTF-8 maps — share
//! the same flat model (plan RW, RW-4). [`compression::unpack_map`] /
//! [`common_events::unpack`] pick the container from the version and
//! [`compression::pack_map`] / [`common_events::pack`] write it back the same way.

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
    #[error("tile data size overflows: width={width} height={height} layers={layer_cnt}")]
    TileSizeOverflow {
        width: u32,
        height: u32,
        layer_cnt: u32,
    },
}

/// Real-file check (plan RW, decision D5): `H2S_WOLF_GAMES` names a folder whose
/// sub-folders each hold a game's `MapData/` and `BasicData/` (directly or under
/// `Data/`, e.g. extracted from its archive). Game files never go in Git. Run:
/// `H2S_WOLF_GAMES=… cargo test -j 2 -- --ignored wolf_real`.
#[cfg(test)]
mod real_files {
    use super::{common_events, compression, map};
    use std::path::{Path, PathBuf};

    #[test]
    #[ignore = "needs H2S_WOLF_GAMES (real game files)"]
    fn wolf_real_round_trip() {
        let root = std::env::var_os("H2S_WOLF_GAMES").expect("set H2S_WOLF_GAMES");
        let mut checked = 0usize;
        let mut failures = Vec::new();
        for game in std::fs::read_dir(&root).expect("H2S_WOLF_GAMES").flatten() {
            for path in wolf_files(&game.path()) {
                let bytes = std::fs::read(&path).expect("read Wolf file");
                let is_map = path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("mps"));
                let result = if is_map {
                    check_map(&bytes)
                } else {
                    check_common_events(&bytes)
                };
                match result {
                    Ok(()) => checked += 1,
                    Err(e) => failures.push(format!("{}: {e}", path.display())),
                }
            }
        }
        assert!(
            failures.is_empty(),
            "{} failure(s):\n{}",
            failures.len(),
            failures.join("\n")
        );
        assert!(
            checked > 0,
            "no Wolf map or CommonEvent.dat under H2S_WOLF_GAMES"
        );
        eprintln!("{checked} Wolf files read and written back identically");
    }

    fn wolf_files(game: &Path) -> Vec<PathBuf> {
        let data = if game.join("Data").is_dir() {
            game.join("Data")
        } else {
            game.to_path_buf()
        };
        let mut files: Vec<PathBuf> = std::fs::read_dir(data.join("MapData"))
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("mps"))
            })
            .collect();
        files.sort();
        let common_events = data.join("BasicData").join("CommonEvent.dat");
        if common_events.is_file() {
            files.push(common_events);
        }
        files
    }

    /// `dump(parse(x)) == x` on the unpacked bytes; a file stored in clear is
    /// written back byte for byte, an LZ4 one must read back to the same bytes.
    fn check_map(bytes: &[u8]) -> Result<(), String> {
        let plain = compression::unpack_map(bytes).map_err(|e| e.to_string())?;
        let dumped = map::MapV3::parse(&plain)
            .and_then(|m| m.dump())
            .map_err(|e| e.to_string())?;
        if dumped != plain {
            return Err("dump(parse(x)) differs from x".to_owned());
        }
        let packed = compression::pack_map(dumped).map_err(|e| e.to_string())?;
        if compression::is_plain_map(bytes) {
            (packed == bytes)
                .then_some(())
                .ok_or_else(|| "map in clear not written back identically".to_owned())
        } else {
            let reread = compression::unpack_map(&packed).map_err(|e| e.to_string())?;
            (reread == plain)
                .then_some(())
                .ok_or_else(|| "recompressed map does not read back".to_owned())
        }
    }

    fn check_common_events(bytes: &[u8]) -> Result<(), String> {
        let plain = common_events::unpack(bytes).map_err(|e| e.to_string())?;
        let dumped = common_events::CommonEventsV3::parse(&plain)
            .and_then(|ce| ce.dump())
            .map_err(|e| e.to_string())?;
        if dumped != plain {
            return Err("dump(parse(x)) differs from x".to_owned());
        }
        let packed = common_events::pack(dumped).map_err(|e| e.to_string())?;
        if common_events::is_plain(bytes) {
            (packed == bytes)
                .then_some(())
                .ok_or_else(|| "CommonEvent.dat in clear not written back identically".to_owned())
        } else {
            let reread = common_events::unpack(&packed).map_err(|e| e.to_string())?;
            (reread == plain)
                .then_some(())
                .ok_or_else(|| "recompressed CommonEvent.dat does not read back".to_owned())
        }
    }
}
