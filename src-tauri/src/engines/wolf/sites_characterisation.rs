//! Characterisation of the Wolf message sites (plan DD, L3-1): the segments
//! the extractor reads from the Show Message (101) and Choices (102) commands
//! of maps and common events, and the bytes the injector writes back for a
//! known translation. Frozen before the extractor and the injector share one
//! walk over these commands; the expected output is versioned in
//! `tests/fixtures/wolf_sites.json`.
//!
//! To write it again (only on purpose): `H2S_BLESS_CORPUS=1 cargo test
//! --lib wolf::sites_characterisation`.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::extractor::{extract_common_events, extract_map_segments};
use super::injector::{inject_common_events, inject_map, WolfTranslation};
use super::v3_format::command::{Command, CID_CHOICES, CID_MESSAGE};
use super::v3_format::common_events::{self, CommonEventsV3};
use super::v3_format::compression::pack_map;
use super::v3_format::map::MapV3;
use crate::engines::detector::WolfVersion;

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn command(cid: u32, strings: &[&str], v35: bool) -> Command {
    Command {
        cid,
        args: Vec::new(),
        indent: 0,
        string_args: strings.iter().map(|s| (*s).to_owned()).collect(),
        move_data: None,
        v35_unknown: if v35 { vec![0xAB] } else { Vec::new() },
    }
}

/// Every message a site may hold: text, blank, whitespace only, a text the
/// filter skips (no translatable character), choices with a blank one.
fn map_bytes(version: u32, is_utf8: bool) -> Vec<u8> {
    let v35 = version >= 0x67;
    let mut map = MapV3::synthetic_messages(
        version,
        is_utf8,
        &["こんにちは", "", "   ", "@1\n顔つきの台詞", "123"],
        &["はい", "", "いいえ"],
    );
    // A second page and a second event, with a message, choices and a
    // message holding two strings (only the first is a text).
    let mut page = map.events[0].pages[0].clone();
    page.commands = vec![
        command(CID_CHOICES, &["上", "下"], v35),
        command(CID_MESSAGE, &["二ページ目", "二つ目の文字列"], v35),
    ];
    map.events[0].pages.push(page);
    let mut event = map.events[0].clone();
    event.id = 1;
    event.pages.truncate(1);
    event.pages[0].commands = vec![command(CID_MESSAGE, &["二つ目のイベント"], v35)];
    map.events.push(event);
    pack_map(map.dump().unwrap()).unwrap()
}

fn common_event_bytes(v35: bool) -> Vec<u8> {
    let events = [("会話", "共通の台詞"), ("空", ""), ("選択", "どうする？")];
    let mut common = if v35 {
        CommonEventsV3::synthetic_messages(&events)
    } else {
        CommonEventsV3::synthetic_messages_v2(&events)
    };
    common.events[2]
        .commands
        .push(command(CID_CHOICES, &["はい", "  ", "いいえ"], v35));
    common.events[0]
        .commands
        .push(command(CID_MESSAGE, &["@2\n二つ目"], v35));
    common_events::pack(common.dump().unwrap()).unwrap()
}

/// Segments read, then the file written with `T<n>` for every segment and
/// for every blank site (which the injector must leave alone), read back.
fn round_trip(
    extract: impl Fn(&[u8]) -> Vec<(String, String)>,
    inject: impl Fn(&[u8], &[WolfTranslation]) -> (Vec<u8>, usize),
    bytes: &[u8],
    blank_keys: &[String],
) -> Value {
    let segments = extract(bytes);
    let translations: Vec<WolfTranslation> = segments
        .iter()
        .map(|(key, _)| key.clone())
        .chain(blank_keys.iter().cloned())
        .enumerate()
        .map(|(n, key)| WolfTranslation {
            key,
            text: format!("T{n}"),
        })
        .collect();
    let (injected, updated) = inject(bytes, &translations);
    json!({
        "segments": segments,
        "updated": updated,
        "injected_sha256": sha256_hex(&injected),
        "read_back": extract(&injected),
    })
}

#[test]
fn wolf_message_sites_are_unchanged() {
    let mut actual = serde_json::Map::new();
    for (label, version, is_utf8) in [
        ("map_v2_sjis", 0x64u32, false),
        ("map_v3_utf8", 0x67u32, true),
    ] {
        let wolf = WolfVersion {
            major: if is_utf8 { 3 } else { 2 },
            minor: 0,
        };
        let blank = [
            "MapData/Map001/events/0/pages/0/1".to_string(),
            "MapData/Map001/events/0/pages/0/2".to_string(),
            "MapData/Map001/events/0/pages/0/5/choices/1".to_string(),
        ];
        let value = round_trip(
            |bytes| {
                extract_map_segments("Map001", bytes, &wolf)
                    .unwrap()
                    .into_iter()
                    .map(|s| (s.key, s.source_text))
                    .collect()
            },
            |bytes, translations| {
                let (out, result) = inject_map("Map001", bytes, translations, &wolf).unwrap();
                (out, result.updated_count)
            },
            &map_bytes(version, is_utf8),
            &blank,
        );
        actual.insert(label.to_string(), value);
    }
    for (label, v35) in [("common_events_v2", false), ("common_events_v35", true)] {
        let wolf = WolfVersion {
            major: if v35 { 3 } else { 2 },
            minor: 0,
        };
        let blank = [
            "CommonEvents/空/1/0".to_string(),
            "CommonEvents/選択/2/1/choices/1".to_string(),
        ];
        let value = round_trip(
            |bytes| {
                extract_common_events(bytes, &wolf)
                    .unwrap()
                    .into_iter()
                    .map(|s| (s.key, s.source_text))
                    .collect()
            },
            |bytes, translations| {
                let (out, result) = inject_common_events(bytes, translations, &wolf).unwrap();
                (out, result.updated_count)
            },
            &common_event_bytes(v35),
            &blank,
        );
        actual.insert(label.to_string(), value);
    }

    let actual = Value::Object(actual);
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/wolf_sites.json");
    if std::env::var_os("H2S_BLESS_CORPUS").is_some() {
        std::fs::write(&path, serde_json::to_string_pretty(&actual).unwrap() + "\n").unwrap();
        return;
    }
    let expected: Value = serde_json::from_str(
        &std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{}: {e} (bless with H2S_BLESS_CORPUS=1)", path.display())),
    )
    .unwrap();
    assert_eq!(expected, actual, "Wolf message sites changed");
}
