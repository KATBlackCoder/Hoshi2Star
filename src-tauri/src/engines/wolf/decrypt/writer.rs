//! Wolf RPG DXA archive writer (plan RW, RW-3).
//!
//! A Wolf game reads its `.wolf` archive before any loose file in `Data/`
//! (verified in game on Wolf 2.23 and 3.658), so a translation is delivered as
//! a rebuilt archive. Both layouts follow the prototypes validated in game
//! (`dxa6.py`, `dxa8.py`, plan RW decision D4):
//!
//! - **v6**: the data area is rewritten in its original physical order. An
//!   unchanged file keeps its encrypted bytes and its original padding when that
//!   padding is the 4-byte alignment; a replaced file is stored uncompressed.
//!   The TOC keeps its layout with updated addresses and sizes, and the header
//!   is re-encrypted in its original mode (signature encrypted or in clear).
//! - **v8**: the archive is copied up to its TOC, replaced files are appended
//!   uncompressed (8-byte alignment), and the TOC is written uncompressed with
//!   `DXA_FLAG_NO_HEAD_PRESS`.
//!
//! Replaced files are XOR-encrypted at key position = their size, like every
//! Wolf archive file.

use super::legacy_xor::{
    key_conv, key_conv7, read_layout, DecryptorError, DxaCrypt, DxaEntry, DxaLayout,
};
use std::collections::BTreeMap;
use std::io::Write;

#[derive(Debug, thiserror::Error)]
pub enum WriterError {
    #[error("archive read error: {0}")]
    Decryptor(#[from] DecryptorError),
    #[error("file not found in the archive: {0}")]
    UnknownPath(String),
    #[error("inconsistent archive layout: {0}")]
    Layout(&'static str),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// What [`rebuild_archive`] wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebuildReport {
    /// DXA version of the archive (6 or 8).
    pub version: u8,
    /// Number of files replaced.
    pub replaced: usize,
    /// Size of the rebuilt archive, in bytes.
    pub size: u64,
}

/// DXA v6 header size (`DARC_HEAD` + 4 reserved bytes).
const V6_HEAD_SIZE: usize = 0x30;
/// DXA v8 header size.
const V8_HEAD_SIZE: usize = 0x40;
/// `DARC_FILEHEAD` size in v6 / v8.
const V6_ENTRY_SIZE: usize = 0x40;
const V8_ENTRY_SIZE: usize = 0x48;
/// `DXA_FLAG_NO_HEAD_PRESS`: the TOC is stored without Huffman/LZ compression.
const DXA_FLAG_NO_HEAD_PRESS: u32 = 0x2;

/// Rebuild `archive` with the files of `replacements` (full archive path, as in
/// `WolfFile::path`, → plaintext bytes) replaced, and write it to `out`.
///
/// Every path must exist in the archive; nothing is written otherwise.
pub fn rebuild_archive(
    archive: &[u8],
    replacements: &BTreeMap<String, Vec<u8>>,
    out: &mut dyn Write,
) -> Result<RebuildReport, WriterError> {
    let layout = read_layout(archive)?;
    if let Some(path) = replacements
        .keys()
        .find(|path| !layout.entries.iter().any(|entry| &entry.path == *path))
    {
        return Err(WriterError::UnknownPath(path.clone()));
    }

    let size = match layout.crypt {
        DxaCrypt::Xor12 {
            key,
            signature_encrypted,
        } => rebuild_v6(
            archive,
            &layout,
            &key,
            signature_encrypted,
            replacements,
            out,
        )?,
        DxaCrypt::Xor7 { global_key } => {
            rebuild_v8(archive, &layout, global_key, replacements, out)?
        }
    };
    Ok(RebuildReport {
        version: layout.version,
        replaced: replacements.len(),
        size,
    })
}

fn to_usize(value: u64, what: &'static str) -> Result<usize, WriterError> {
    usize::try_from(value).map_err(|_| WriterError::Layout(what))
}

/// Offset in the plaintext TOC of the entry field at `field`, checked.
fn entry_field(
    layout: &DxaLayout,
    entry: &DxaEntry,
    entry_size: usize,
    field: usize,
) -> Result<usize, WriterError> {
    let at = layout
        .file_table
        .checked_add(entry.entry_offset)
        .and_then(|at| usize::try_from(at).ok())
        .filter(|&at| at + entry_size <= layout.toc.len())
        .ok_or(WriterError::Layout("file entry outside the TOC"))?;
    Ok(at + field)
}

fn put_u64(toc: &mut [u8], at: usize, value: u64) {
    toc[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

fn rebuild_v6(
    archive: &[u8],
    layout: &DxaLayout,
    key: &[u8; 12],
    signature_encrypted: bool,
    replacements: &BTreeMap<String, Vec<u8>>,
    out: &mut dyn Write,
) -> Result<u64, WriterError> {
    let base = to_usize(layout.base_offset, "data offset")?;
    let toc_offset = to_usize(layout.toc_offset, "TOC offset")?;
    let toc_end = toc_offset
        .checked_add(layout.toc.len())
        .filter(|&end| end <= archive.len())
        .ok_or(WriterError::Layout("TOC beyond the end of the archive"))?;
    if base < V6_HEAD_SIZE || toc_offset < base {
        return Err(WriterError::Layout("v6 header addresses"));
    }

    let mut toc = layout.toc.clone();
    let mut order: Vec<&DxaEntry> = layout.entries.iter().collect();
    order.sort_by_key(|entry| entry.data_offset);

    let mut data = Vec::with_capacity(toc_offset - base);
    for (i, entry) in order.iter().enumerate() {
        let cursor = data.len() as u64;
        let address = entry_field(layout, entry, V6_ENTRY_SIZE, 0x28)?;
        match replacements.get(&entry.path) {
            Some(new) => {
                let mut blob = new.clone();
                key_conv(&mut blob, new.len() as u64, key);
                put_u64(&mut toc, address, cursor);
                put_u64(&mut toc, address + 8, new.len() as u64);
                put_u64(&mut toc, address + 16, u64::MAX); // PressDataSize = -1
                data.extend_from_slice(&blob);
                data.resize(data.len().next_multiple_of(4), 0);
            }
            None => {
                let start = layout
                    .base_offset
                    .checked_add(entry.data_offset)
                    .and_then(|start| usize::try_from(start).ok())
                    .ok_or(WriterError::Layout("file data address"))?;
                let end = start
                    .checked_add(to_usize(entry.stored_size, "file size")?)
                    .filter(|&end| end <= toc_offset)
                    .ok_or(WriterError::Layout("file data beyond the data area"))?;
                put_u64(&mut toc, address, cursor);
                data.extend_from_slice(&archive[start..end]);
                // Original padding up to the next file (or the TOC), kept when it
                // is exactly the 4-byte alignment DxLib writes.
                let next = match order.get(i + 1) {
                    Some(next) => base.saturating_add(to_usize(next.data_offset, "file data")?),
                    None => toc_offset,
                };
                let needed = (end - start).next_multiple_of(4) - (end - start);
                match archive.get(end..next) {
                    Some(padding) if padding.len() == needed => data.extend_from_slice(padding),
                    _ => data.resize(data.len() + needed, 0),
                }
            }
        }
    }

    let new_toc_offset = (base + data.len()) as u64;
    let mut head = archive[..V6_HEAD_SIZE].to_vec();
    let from = if signature_encrypted { 0 } else { 4 };
    key_conv(&mut head[from..], from as u64, key);
    put_u64(&mut head, 0x10, new_toc_offset);
    key_conv(&mut head[from..], from as u64, key);
    key_conv(&mut toc, 0, key);

    out.write_all(&head)?;
    out.write_all(&archive[V6_HEAD_SIZE..base])?;
    out.write_all(&data)?;
    out.write_all(&toc)?;
    out.write_all(&archive[toc_end..])?;
    Ok((base + data.len() + toc.len() + (archive.len() - toc_end)) as u64)
}

fn rebuild_v8(
    archive: &[u8],
    layout: &DxaLayout,
    global_key: Option<[u8; 7]>,
    replacements: &BTreeMap<String, Vec<u8>>,
    out: &mut dyn Write,
) -> Result<u64, WriterError> {
    let base = to_usize(layout.base_offset, "data offset")?;
    let toc_offset = to_usize(layout.toc_offset, "TOC offset")?;
    if base < V8_HEAD_SIZE || toc_offset < base || toc_offset > archive.len() {
        return Err(WriterError::Layout("v8 header addresses"));
    }

    let mut toc = layout.toc.clone();
    let mut appended = Vec::new();
    let mut cursor = (toc_offset - base) as u64;
    for entry in &layout.entries {
        let Some(new) = replacements.get(&entry.path) else {
            continue;
        };
        let pad = cursor.next_multiple_of(8) - cursor;
        appended.resize(appended.len() + pad as usize, 0);
        cursor += pad;

        let mut blob = new.clone();
        if let Some(file_key) = entry.file_key {
            key_conv7(&mut blob, new.len() as u64, &file_key);
        }
        let address = entry_field(layout, entry, V8_ENTRY_SIZE, 0x28)?;
        put_u64(&mut toc, address, cursor);
        put_u64(&mut toc, address + 8, new.len() as u64);
        put_u64(&mut toc, address + 16, u64::MAX); // PressDataSize: no LZ
        put_u64(&mut toc, address + 24, u64::MAX); // HuffPressDataSize: no Huffman
        appended.extend_from_slice(&blob);
        cursor += new.len() as u64;
    }

    let mut head = archive[..V8_HEAD_SIZE].to_vec();
    put_u64(&mut head, 0x10, layout.base_offset + cursor);
    let flags = u32::from_le_bytes([head[0x2C], head[0x2D], head[0x2E], head[0x2F]]);
    head[0x2C..0x30].copy_from_slice(&(flags | DXA_FLAG_NO_HEAD_PRESS).to_le_bytes());
    if let Some(global_key) = global_key {
        key_conv7(&mut toc, 0, &global_key);
    }

    out.write_all(&head)?;
    out.write_all(&archive[V8_HEAD_SIZE..toc_offset])?;
    out.write_all(&appended)?;
    out.write_all(&toc)?;
    Ok((toc_offset + appended.len() + toc.len()) as u64)
}

#[cfg(test)]
mod tests {
    use super::super::legacy_xor::test_archives::{make_v6_dx_archive, make_v8_dx_archive, DxFile};
    use super::super::legacy_xor::{extract_all, WOLF_KEYS, WOLF_V8_KEY_STRINGS};
    use super::*;

    const MAP: &[u8] = b"WOLFM map payload \xFF with the LZ keycode \xFF\xFF inside";
    const GAME: &[u8] = b"raw root file";

    fn v6_files() -> [DxFile<'static>; 2] {
        [
            DxFile {
                dir: Some("MapData"),
                name: "Map001.mps",
                content: MAP,
                compress: true,
            },
            DxFile {
                dir: None,
                name: "Game.dat",
                content: GAME,
                compress: false,
            },
        ]
    }

    fn v8_files() -> [DxFile<'static>; 2] {
        [
            DxFile {
                dir: Some("MapData"),
                name: "TitleMap.mps",
                content: MAP,
                compress: false,
            },
            DxFile {
                dir: None,
                name: "Game.dat",
                content: GAME,
                compress: false,
            },
        ]
    }

    fn v2_20_key() -> [u8; 12] {
        WOLF_KEYS
            .iter()
            .find(|(name, _)| *name == "v2.20")
            .map(|(_, key)| *key)
            .unwrap()
    }

    fn v8_key_string(name: &str) -> &'static [u8] {
        WOLF_V8_KEY_STRINGS
            .iter()
            .find(|(n, _, _)| *n == name)
            .map(|&(_, _, key)| key)
            .unwrap()
    }

    fn rebuild(archive: &[u8], replacements: &[(&str, &[u8])]) -> (Vec<u8>, RebuildReport) {
        let replacements: BTreeMap<String, Vec<u8>> = replacements
            .iter()
            .map(|(path, bytes)| ((*path).to_owned(), bytes.to_vec()))
            .collect();
        let mut out = Vec::new();
        let report = rebuild_archive(archive, &replacements, &mut out).unwrap();
        assert_eq!(report.size, out.len() as u64);
        (out, report)
    }

    fn file<'a>(archive: &'a super::super::legacy_xor::WolfArchive, path: &str) -> &'a [u8] {
        &archive
            .files
            .iter()
            .find(|f| f.path == path)
            .unwrap_or_else(|| panic!("{path} not found"))
            .data
    }

    #[test]
    fn test_v6_rebuild_without_change_is_byte_identical() {
        for encrypt_signature in [true, false] {
            let archive =
                make_v6_dx_archive(&v2_20_key(), &v6_files(), 932, encrypt_signature).bytes;
            let (rebuilt, report) = rebuild(&archive, &[]);
            assert_eq!(report.version, 6);
            assert_eq!(rebuilt, archive, "encrypt_signature={encrypt_signature}");
        }
    }

    #[test]
    fn test_v6_replaced_file_reads_back_and_others_are_unchanged() {
        let translated: &[u8] = b"translated map, stored uncompressed";
        for encrypt_signature in [true, false] {
            let archive =
                make_v6_dx_archive(&v2_20_key(), &v6_files(), 932, encrypt_signature).bytes;
            let (rebuilt, report) = rebuild(&archive, &[("/MapData/Map001.mps", translated)]);
            assert_eq!(report.replaced, 1);
            assert_eq!(rebuilt[..2] == *b"DX", !encrypt_signature);

            let reread = extract_all(&rebuilt).unwrap();
            assert_eq!(reread.files.len(), 2);
            assert_eq!(file(&reread, "/MapData/Map001.mps"), translated);
            assert_eq!(file(&reread, "/Game.dat"), GAME);
        }
    }

    #[test]
    fn test_v8_rebuild_without_change_is_byte_identical() {
        let archive = make_v8_dx_archive(v8_key_string("v3.14"), 314, &v8_files());
        let (rebuilt, report) = rebuild(&archive, &[]);
        assert_eq!(report.version, 8);
        assert_eq!(rebuilt, archive);
    }

    #[test]
    fn test_v8_replaced_file_is_appended_and_reads_back() {
        let archive = make_v8_dx_archive(v8_key_string("v3.14"), 314, &v8_files());
        let translated: &[u8] = b"translated title map";
        let (rebuilt, _) = rebuild(&archive, &[("/MapData/TitleMap.mps", translated)]);

        // Unchanged data stays in place; the TOC moved after the appended file.
        let old_toc = u64::from_le_bytes(archive[16..24].try_into().unwrap()) as usize;
        let new_toc = u64::from_le_bytes(rebuilt[16..24].try_into().unwrap()) as usize;
        assert_eq!(rebuilt[64..old_toc], archive[64..old_toc]);
        assert!(new_toc >= old_toc + translated.len());
        let flags = u32::from_le_bytes(rebuilt[44..48].try_into().unwrap());
        assert_eq!(flags >> 16, 314);
        assert_ne!(flags & DXA_FLAG_NO_HEAD_PRESS, 0);

        let reread = extract_all(&rebuilt).unwrap();
        assert_eq!(file(&reread, "/MapData/TitleMap.mps"), translated);
        assert_eq!(file(&reread, "/Game.dat"), GAME);
    }

    #[test]
    fn test_unknown_path_is_refused_and_nothing_written() {
        let archive = make_v6_dx_archive(&v2_20_key(), &v6_files(), 932, true).bytes;
        let replacements = BTreeMap::from([("/MapData/Map999.mps".to_owned(), b"x".to_vec())]);
        let mut out = Vec::<u8>::new();
        assert!(matches!(
            rebuild_archive(&archive, &replacements, &mut out),
            Err(WriterError::UnknownPath(path)) if path == "/MapData/Map999.mps"
        ));
        assert!(out.is_empty());
    }

    #[test]
    fn test_new_crypt_archive_is_refused() {
        let mut header = vec![0u8; 64];
        header[0..4].copy_from_slice(&[b'D', b'X', 8, 0]);
        header[44..48].copy_from_slice(&(350u32 << 16).to_le_bytes());
        assert!(matches!(
            rebuild_archive(&header, &BTreeMap::new(), &mut Vec::<u8>::new()),
            Err(WriterError::Decryptor(
                DecryptorError::UnsupportedWolfCrypt(350)
            ))
        ));
    }

    /// Real-archive check (plan RW, decision D5): `H2S_WOLF_ARCHIVES` lists
    /// `.wolf` files separated by `:` (never in Git). Needs about three times
    /// the archive size in memory. Run:
    /// `H2S_WOLF_ARCHIVES=… cargo test -j 2 -- --ignored wolf_real`.
    #[test]
    #[ignore = "needs H2S_WOLF_ARCHIVES (real game archives)"]
    fn wolf_real_archive_rebuild() {
        let list = std::env::var("H2S_WOLF_ARCHIVES").expect("set H2S_WOLF_ARCHIVES");
        for path in list.split(':').filter(|path| !path.is_empty()) {
            let archive = std::fs::read(path).expect("read archive");
            let original = extract_all(&archive).expect("read original archive");

            let (rebuilt, report) = rebuild(&archive, &[]);
            if report.version == 6 {
                assert!(
                    rebuilt == archive,
                    "{path}: v6 rebuild without change differs"
                );
            }
            let reread = extract_all(&rebuilt).expect("read rebuilt archive");
            drop(rebuilt);
            assert_eq!(reread.files.len(), original.files.len(), "{path}");
            for f in &original.files {
                assert_eq!(
                    file(&reread, &f.path),
                    f.data.as_slice(),
                    "{path}: {}",
                    f.path
                );
            }
            drop(reread);

            let target = original
                .files
                .iter()
                .find(|f| f.path.to_ascii_lowercase().ends_with(".mps"))
                .unwrap_or(&original.files[0]);
            let mut changed = target.data.clone();
            changed.extend_from_slice(b"H2S");
            let (rebuilt, _) = rebuild(&archive, &[(target.path.as_str(), changed.as_slice())]);
            let reread = extract_all(&rebuilt).expect("read rebuilt archive");
            for f in &original.files {
                let expected = if f.path == target.path {
                    changed.as_slice()
                } else {
                    f.data.as_slice()
                };
                assert_eq!(file(&reread, &f.path), expected, "{path}: {}", f.path);
            }
            eprintln!("{path}: v{} archive rebuilt and read back", report.version);
        }
    }
}
