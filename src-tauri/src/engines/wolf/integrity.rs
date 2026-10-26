//! Export integrity gate for Wolf RPG (plan RW, RW-6).
//!
//! Every map, `CommonEvent.dat` and database the export produces is re-read and
//! compared with the original bytes the injector read. The only differences
//! allowed are the planned translations (segment key → final text): with every
//! text blanked, both files must be identical; then each text must be the
//! planned one (planned key) or the original one (any other key). The check
//! does not reuse the injector's code paths, so an injector bug shows up here.
//!
//! No "source changed" check: Wolf has no pristine copy of the game files, so
//! once an export is unpacked over the game the original already holds the
//! previous translation. A game-update guard needs that copy (separate lot).

use std::collections::{BTreeMap, HashMap, HashSet};

use super::dat_parser::{self, DatFile};
use super::decrypt::legacy_xor::{extract_matching, read_layout, DxaEntry, DxaLayout};
use super::v3_format::command::{Command, CID_CHOICES, CID_MESSAGE};
use super::v3_format::common_events::{self, CommonEventsV3};
use super::v3_format::compression;
use super::v3_format::map::MapV3;
use crate::engines::mv_mz::integrity::IntegrityIssue;

/// Issues listed per file before the gate stops collecting (the export is
/// rejected either way; this only bounds the error message).
const MAX_ISSUES_PER_FILE: usize = 20;

/// Every text of a file, keyed like the extractor, in file order.
type Texts = Vec<(String, String)>;

/// Move the 101/102 texts of `command` into `texts` (leaving them blank).
fn take_command_texts(prefix: &str, cmd_idx: usize, command: &mut Command, texts: &mut Texts) {
    match command.cid {
        CID_MESSAGE => {
            if let Some(text) = command.string_args.first_mut() {
                texts.push((format!("{prefix}/{cmd_idx}"), std::mem::take(text)));
            }
        }
        CID_CHOICES => {
            for (choice_idx, choice) in command.string_args.iter_mut().enumerate() {
                texts.push((
                    format!("{prefix}/{cmd_idx}/choices/{choice_idx}"),
                    std::mem::take(choice),
                ));
            }
        }
        _ => {}
    }
}

/// `map` with its texts blanked, and those texts (keys
/// `MapData/{map_name}/events/{e}/pages/{p}/{c}[/choices/{i}]`).
fn split_map(mut map: MapV3, map_name: &str) -> (MapV3, Texts) {
    let mut texts = Vec::new();
    for (event_idx, event) in map.events.iter_mut().enumerate() {
        for (page_idx, page) in event.pages.iter_mut().enumerate() {
            let prefix = format!("MapData/{map_name}/events/{event_idx}/pages/{page_idx}");
            for (cmd_idx, command) in page.commands.iter_mut().enumerate() {
                take_command_texts(&prefix, cmd_idx, command, &mut texts);
            }
        }
    }
    (map, texts)
}

/// Same as [`split_map`] for `CommonEvent.dat` (keys
/// `CommonEvents/{event_name}/{e}/{c}[/choices/{i}]`).
fn split_common_events(mut common_events: CommonEventsV3) -> (CommonEventsV3, Texts) {
    let mut texts = Vec::new();
    for (event_idx, event) in common_events.events.iter_mut().enumerate() {
        let prefix = format!("CommonEvents/{}/{event_idx}", event.name);
        for (cmd_idx, command) in event.commands.iter_mut().enumerate() {
            take_command_texts(&prefix, cmd_idx, command, &mut texts);
        }
    }
    (common_events, texts)
}

/// Same for a database: keys `Database/{db_name}/{type}/{entry}/{field}`, the
/// first string field of that name (as the injector resolves it); a later field
/// with the same name gets a key no translation can carry.
fn split_dat(mut dat: DatFile, db_name: &str) -> (DatFile, Texts) {
    let mut texts = Vec::new();
    for (type_idx, dat_type) in dat.types.iter_mut().enumerate() {
        let mut seen = HashSet::new();
        let slot_names: Vec<Option<String>> = dat_type
            .fields
            .iter()
            .filter(|field| field.is_valid() && field.is_string())
            .map(|field| seen.insert(field.name.clone()).then(|| field.name.clone()))
            .collect();
        for (entry_idx, entry) in dat_type.entries.iter_mut().enumerate() {
            for (slot, value) in entry.string_values.iter_mut().enumerate() {
                let key = match slot_names.get(slot) {
                    Some(Some(name)) => format!("Database/{db_name}/{type_idx}/{entry_idx}/{name}"),
                    _ => format!("Database/{db_name}/{type_idx}/{entry_idx}/#{slot}"),
                };
                texts.push((key, std::mem::take(value)));
            }
        }
    }
    (dat, texts)
}

/// Compare the texts of two files with the same structure (hence the same keys
/// in the same order) against the planned translations.
fn compare_texts(
    file: &str,
    original: &Texts,
    output: &Texts,
    planned: &HashMap<String, String>,
) -> Vec<IntegrityIssue> {
    let mut issues = Vec::new();
    for ((key, before), (_, after)) in original.iter().zip(output) {
        let issue = match planned.get(key) {
            Some(text) if after != text => IntegrityIssue::PlannedValueMissing {
                file: file.to_string(),
                pointer: key.clone(),
            },
            None if after != before => IntegrityIssue::UnplannedChange {
                file: file.to_string(),
                pointer: key.clone(),
            },
            _ => continue,
        };
        issues.push(issue);
    }
    let known: HashSet<&str> = original.iter().map(|(key, _)| key.as_str()).collect();
    let mut absent: Vec<&String> = planned
        .keys()
        .filter(|key| !known.contains(key.as_str()))
        .collect();
    absent.sort();
    issues.extend(
        absent
            .into_iter()
            .map(|key| IntegrityIssue::PlannedValueMissing {
                file: file.to_string(),
                pointer: key.clone(),
            }),
    );
    issues.truncate(MAX_ISSUES_PER_FILE);
    issues
}

fn unparseable(file: &str, reason: impl ToString) -> Vec<IntegrityIssue> {
    vec![IntegrityIssue::Unparseable {
        file: file.to_string(),
        reason: reason.to_string(),
    }]
}

fn structure_changed(file: &str, reason: &str) -> Vec<IntegrityIssue> {
    vec![IntegrityIssue::StructureChanged {
        file: file.to_string(),
        pointer: String::new(),
        reason: reason.to_string(),
    }]
}

/// Verify that the `.mps` `output` differs from `original` exactly by `planned`.
pub fn verify_map_edit(
    file: &str,
    map_name: &str,
    original: &[u8],
    output: &[u8],
    planned: &HashMap<String, String>,
) -> Vec<IntegrityIssue> {
    let parse =
        |bytes: &[u8]| compression::unpack_map(bytes).and_then(|plain| MapV3::parse(&plain));
    let (before, after) = match (parse(original), parse(output)) {
        (Ok(before), Ok(after)) => (before, after),
        (Err(e), _) | (_, Err(e)) => return unparseable(file, e),
    };
    let (before, before_texts) = split_map(before, map_name);
    let (after, after_texts) = split_map(after, map_name);
    if before != after {
        return structure_changed(file, "map data outside the translated texts changed");
    }
    compare_texts(file, &before_texts, &after_texts, planned)
}

/// Verify that the `CommonEvent.dat` `output` differs from `original` exactly by
/// `planned`.
pub fn verify_common_events_edit(
    file: &str,
    original: &[u8],
    output: &[u8],
    planned: &HashMap<String, String>,
) -> Vec<IntegrityIssue> {
    let parse =
        |bytes: &[u8]| common_events::unpack(bytes).and_then(|plain| CommonEventsV3::parse(&plain));
    let (before, after) = match (parse(original), parse(output)) {
        (Ok(before), Ok(after)) => (before, after),
        (Err(e), _) | (_, Err(e)) => return unparseable(file, e),
    };
    let (before, before_texts) = split_common_events(before);
    let (after, after_texts) = split_common_events(after);
    if before != after {
        return structure_changed(
            file,
            "common event data outside the translated texts changed",
        );
    }
    compare_texts(file, &before_texts, &after_texts, planned)
}

/// Verify that the database `output` (read with its unchanged `.project`
/// schema) differs from `original` exactly by `planned`.
pub fn verify_dat_edit(
    file: &str,
    db_name: &str,
    project: &[u8],
    original: &[u8],
    output: &[u8],
    planned: &HashMap<String, String>,
) -> Vec<IntegrityIssue> {
    if original.get(..11) != output.get(..11) {
        return structure_changed(file, "database header changed");
    }
    let parse = |bytes: &[u8]| dat_parser::parse_database(project, bytes);
    let (before, after) = match (parse(original), parse(output)) {
        (Ok(before), Ok(after)) => (before, after),
        (Err(e), _) | (_, Err(e)) => return unparseable(file, e),
    };
    let (before, before_texts) = split_dat(before, db_name);
    let (after, after_texts) = split_dat(after, db_name);
    if before != after {
        return structure_changed(file, "database data outside the translated texts changed");
    }
    compare_texts(file, &before_texts, &after_texts, planned)
}

/// Stored (still encrypted) bytes of `entry` in a v6 archive.
fn stored_bytes<'a>(archive: &'a [u8], layout: &DxaLayout, entry: &DxaEntry) -> Option<&'a [u8]> {
    let start = usize::try_from(layout.base_offset.checked_add(entry.data_offset)?).ok()?;
    let end = start.checked_add(usize::try_from(entry.stored_size).ok()?)?;
    archive.get(start..end)
}

/// Data address, sizes and Huffman size of a v8 entry, as stored in the TOC.
fn entry_fields<'a>(layout: &'a DxaLayout, entry: &DxaEntry) -> Option<&'a [u8]> {
    let at = usize::try_from(layout.file_table.checked_add(entry.entry_offset)?).ok()?;
    layout.toc.get(at + 0x28..at + 0x48)
}

/// Verify a rebuilt archive (plan RW, RW-5) without decoding every file: the
/// same files in the same order; every file not in `replacements` keeps its
/// stored bytes (v6: same encrypted slice; v8: same entry fields, and the data
/// copied before the original TOC is unchanged); every replaced file reads back
/// as planned. The files inside (maps, databases) were checked before the
/// rebuild by the other `verify_*` functions.
pub fn verify_archive_rebuild(
    file: &str,
    original: &[u8],
    rebuilt: &[u8],
    replacements: &BTreeMap<String, Vec<u8>>,
) -> Vec<IntegrityIssue> {
    let (before, after) = match (read_layout(original), read_layout(rebuilt)) {
        (Ok(before), Ok(after)) => (before, after),
        (Err(e), _) | (_, Err(e)) => return unparseable(file, e),
    };
    let paths = |layout: &DxaLayout| -> Vec<String> {
        layout
            .entries
            .iter()
            .map(|entry| entry.path.clone())
            .collect()
    };
    if before.version != after.version || paths(&before) != paths(&after) {
        return structure_changed(file, "the archive's file list changed");
    }
    if before.version == 8 {
        // Everything before the original TOC is copied; only the header's TOC
        // address (0x10) and flags (0x2C) may differ.
        // Compared in place: the data area can weigh gigabytes.
        let toc = usize::try_from(before.toc_offset).unwrap_or(usize::MAX);
        let unchanged = original.get(..0x10) == rebuilt.get(..0x10)
            && original.get(0x18..0x2C) == rebuilt.get(0x18..0x2C)
            && original.get(0x30..toc).is_some()
            && original.get(0x30..toc) == rebuilt.get(0x30..toc);
        if !unchanged {
            return structure_changed(file, "data copied from the original archive changed");
        }
    }

    let mut issues = Vec::new();
    for (old, new) in before.entries.iter().zip(&after.entries) {
        if replacements.contains_key(&old.path) {
            continue;
        }
        let unchanged = if before.version == 8 {
            entry_fields(&before, old).is_some()
                && entry_fields(&before, old) == entry_fields(&after, new)
        } else {
            stored_bytes(original, &before, old).is_some()
                && stored_bytes(original, &before, old) == stored_bytes(rebuilt, &after, new)
        };
        if !unchanged {
            issues.push(IntegrityIssue::UnplannedChange {
                file: file.to_string(),
                pointer: old.path.clone(),
            });
        }
    }

    let replaced = match extract_matching(rebuilt, &|path| replacements.contains_key(path)) {
        Ok(archive) => archive.files,
        Err(e) => return unparseable(file, e),
    };
    for (path, expected) in replacements {
        let found = replaced.iter().find(|f| &f.path == path).map(|f| &f.data);
        if found != Some(expected) {
            issues.push(IntegrityIssue::PlannedValueMissing {
                file: file.to_string(),
                pointer: path.clone(),
            });
        }
    }
    issues.truncate(MAX_ISSUES_PER_FILE);
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::detector::WolfVersion;
    use crate::engines::wolf::dat_parser::{make_minimal_dat_pub, make_minimal_project_pub};
    use crate::engines::wolf::injector::{
        inject_common_events, inject_dat, inject_map, WolfTranslation,
    };

    const MESSAGE: &str = "こんにちは、旅の人";
    const CHOICES: [&str; 2] = ["はい、行きます", "いいえ、残ります"];
    const MESSAGE_KEY: &str = "MapData/Map001/events/0/pages/0/0";
    const CHOICE_KEY: &str = "MapData/Map001/events/0/pages/0/1/choices/1";

    fn v2() -> WolfVersion {
        WolfVersion { major: 2, minor: 0 }
    }

    fn map_bytes(version: u32, is_utf8: bool) -> Vec<u8> {
        let map = MapV3::synthetic_messages(version, is_utf8, &[MESSAGE], &CHOICES);
        compression::pack_map(map.dump().unwrap()).unwrap()
    }

    fn plan(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(key, text)| ((*key).to_string(), (*text).to_string()))
            .collect()
    }

    fn translations(planned: &HashMap<String, String>) -> Vec<WolfTranslation> {
        planned
            .iter()
            .map(|(key, text)| WolfTranslation {
                key: key.clone(),
                text: text.clone(),
            })
            .collect()
    }

    fn inject(original: &[u8], planned: &HashMap<String, String>) -> Vec<u8> {
        inject_map("Map001", original, &translations(planned), &v2())
            .unwrap()
            .0
    }

    #[test]
    fn test_injected_map_passes_in_every_container() {
        let planned = plan(&[
            (MESSAGE_KEY, "Hello, traveller"),
            (CHOICE_KEY, "No, I stay"),
        ]);
        for (version, is_utf8) in [(0x64, false), (0x64, true), (0x67, true)] {
            let original = map_bytes(version, is_utf8);
            let output = inject(&original, &planned);
            assert_eq!(
                verify_map_edit("Map001.mps", "Map001", &original, &output, &planned),
                Vec::new(),
                "version {version:#x}"
            );
        }
    }

    /// Re-export over an already translated game: the original is the previous
    /// export, and must still pass.
    #[test]
    fn test_injected_map_passes_on_an_already_translated_original() {
        let original = map_bytes(0x64, false);
        let first = inject(&original, &plan(&[(MESSAGE_KEY, "Hello")]));
        let planned = plan(&[(MESSAGE_KEY, "Hello again"), (CHOICE_KEY, "No")]);
        let second = inject(&first, &planned);
        assert!(verify_map_edit("Map001.mps", "Map001", &first, &second, &planned).is_empty());
        let same = plan(&[(MESSAGE_KEY, "Hello")]);
        let unchanged = inject(&first, &same);
        assert!(verify_map_edit("Map001.mps", "Map001", &first, &unchanged, &same).is_empty());
    }

    #[test]
    fn test_unplanned_and_missing_texts_are_reported() {
        let original = map_bytes(0x64, false);
        let output = inject(&original, &plan(&[(MESSAGE_KEY, "Hello")]));

        // The message changed but nothing planned it.
        assert_eq!(
            verify_map_edit("Map001.mps", "Map001", &original, &output, &HashMap::new()),
            vec![IntegrityIssue::UnplannedChange {
                file: "Map001.mps".into(),
                pointer: MESSAGE_KEY.into(),
            }]
        );
        // A planned translation is absent from the output, and a planned key
        // does not exist in the map.
        let planned = plan(&[
            (CHOICE_KEY, "No"),
            ("MapData/Map001/events/9/pages/0/0", "Lost"),
        ]);
        let issues = verify_map_edit("Map001.mps", "Map001", &original, &original, &planned);
        assert_eq!(issues.len(), 2, "{issues:?}");
        assert!(issues
            .iter()
            .all(|issue| matches!(issue, IntegrityIssue::PlannedValueMissing { .. })));
    }

    #[test]
    fn test_structure_change_and_unreadable_output_are_reported() {
        let original = map_bytes(0x64, false);
        let mut other = MapV3::synthetic_messages(0x64, false, &[MESSAGE], &CHOICES);
        other.tileset_id = 2;
        let changed = compression::pack_map(other.dump().unwrap()).unwrap();
        assert!(matches!(
            verify_map_edit("Map001.mps", "Map001", &original, &changed, &HashMap::new())
                .as_slice(),
            [IntegrityIssue::StructureChanged { .. }]
        ));
        assert!(matches!(
            verify_map_edit(
                "Map001.mps",
                "Map001",
                &original,
                b"garbage",
                &HashMap::new()
            )
            .as_slice(),
            [IntegrityIssue::Unparseable { .. }]
        ));
    }

    #[test]
    fn test_common_events_edit_is_checked() {
        let original = CommonEventsV3::synthetic_messages_v2(&[("Ev001", MESSAGE)])
            .dump()
            .unwrap();
        let planned = plan(&[("CommonEvents/Ev001/0/0", "Hello")]);
        let output = inject_common_events(&original, &translations(&planned), &v2())
            .unwrap()
            .0;
        assert!(
            verify_common_events_edit("CommonEvent.dat", &original, &output, &planned).is_empty()
        );
        assert!(matches!(
            verify_common_events_edit("CommonEvent.dat", &original, &output, &HashMap::new())
                .as_slice(),
            [IntegrityIssue::UnplannedChange { .. }]
        ));
    }

    fn archive_files(
    ) -> [crate::engines::wolf::decrypt::legacy_xor::test_archives::DxFile<'static>; 2] {
        use crate::engines::wolf::decrypt::legacy_xor::test_archives::DxFile;
        [
            DxFile {
                dir: Some("MapData"),
                name: "Map001.mps",
                content: b"original map bytes",
                compress: true,
            },
            DxFile {
                dir: None,
                name: "Game.dat",
                content: b"game settings",
                compress: false,
            },
        ]
    }

    fn rebuild(archive: &[u8], replacements: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
        let mut out = Vec::new();
        crate::engines::wolf::decrypt::writer::rebuild_archive(archive, replacements, &mut out)
            .unwrap();
        out
    }

    fn sample_archives() -> Vec<(u8, Vec<u8>)> {
        use crate::engines::wolf::decrypt::legacy_xor::test_archives::{
            make_v6_dx_archive, make_v8_dx_archive,
        };
        use crate::engines::wolf::decrypt::legacy_xor::{WOLF_KEYS, WOLF_V8_KEY_STRINGS};
        let v6_key = WOLF_KEYS.iter().find(|(n, _)| *n == "v2.20").unwrap().1;
        let v8_key = WOLF_V8_KEY_STRINGS
            .iter()
            .find(|(n, _, _)| *n == "v3.14")
            .unwrap()
            .2;
        let mut v8_files = archive_files();
        v8_files[0].compress = false; // the v8 test builder stores raw files
        vec![
            (
                6,
                make_v6_dx_archive(&v6_key, &archive_files(), 932, true).bytes,
            ),
            (8, make_v8_dx_archive(v8_key, 314, &v8_files)),
        ]
    }

    #[test]
    fn test_rebuilt_archive_passes_and_tampering_is_reported() {
        let replacements = BTreeMap::from([(
            "/MapData/Map001.mps".to_string(),
            b"translated map".to_vec(),
        )]);
        for (version, original) in sample_archives() {
            let rebuilt = rebuild(&original, &replacements);
            assert_eq!(
                verify_archive_rebuild("Data.wolf", &original, &rebuilt, &replacements),
                Vec::new(),
                "v{version}"
            );

            // The replacement read back differs from the plan.
            let other =
                BTreeMap::from([("/MapData/Map001.mps".to_string(), b"another text".to_vec())]);
            assert!(matches!(
                verify_archive_rebuild("Data.wolf", &original, &rebuilt, &other).as_slice(),
                [IntegrityIssue::PlannedValueMissing { pointer, .. }] if pointer == "/MapData/Map001.mps"
            ));

            // An unchanged file altered in the rebuilt archive.
            let mut tampered = rebuilt.clone();
            let layout = read_layout(&rebuilt).unwrap();
            let entry = layout
                .entries
                .iter()
                .find(|e| e.path == "/Game.dat")
                .unwrap();
            let at = (layout.base_offset + entry.data_offset) as usize;
            tampered[at] ^= 0xFF;
            let issues = verify_archive_rebuild("Data.wolf", &original, &tampered, &replacements);
            assert!(
                !issues.is_empty(),
                "v{version}: altered Game.dat not reported"
            );
        }
    }

    /// A database whose type header carries `STRING_INDICATOR` and its string.
    fn dat_with_indicator(value: &str) -> Vec<u8> {
        let mut dat = make_minimal_dat_pub(value);
        dat[19..23].copy_from_slice(&0x0001_D4C0u32.to_le_bytes());
        let (label, _, _) = encoding_rs::SHIFT_JIS.encode("表示名");
        let mut extra = ((label.len() + 1) as u32).to_le_bytes().to_vec();
        extra.extend_from_slice(&label);
        extra.push(0);
        let tail = dat.split_off(27);
        dat.extend(extra);
        dat.extend(tail);
        dat
    }

    #[test]
    fn test_dat_edit_is_checked_and_unknown1_kept() {
        let project = make_minimal_project_pub("Type", "name", "entry");
        let original = dat_with_indicator(MESSAGE);
        let parsed = dat_parser::parse_database(&project, &original).unwrap();
        assert_eq!(parsed.types[0].unknown1, 0x0001_D4C0);
        assert_eq!(parsed.types[0].unknown1_string.as_deref(), Some("表示名"));

        let (unchanged, _) = inject_dat(&project, &original, &[], &v2()).unwrap();
        assert_eq!(
            unchanged, original,
            "unknown1 and its string written back as read"
        );

        let planned = plan(&[("Database/DataBase/0/0/name", "Traveller")]);
        let (output, _) = inject_dat(&project, &original, &translations(&planned), &v2()).unwrap();
        assert!(verify_dat_edit(
            "DataBase.dat",
            "DataBase",
            &project,
            &original,
            &output,
            &planned
        )
        .is_empty());
        assert!(matches!(
            verify_dat_edit(
                "DataBase.dat",
                "DataBase",
                &project,
                &original,
                &output,
                &HashMap::new()
            )
            .as_slice(),
            [IntegrityIssue::UnplannedChange { .. }]
        ));
    }
}
