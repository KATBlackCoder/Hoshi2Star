//! Characterisation corpus of text fitting (plan DD, L1-0).
//!
//! Freezes what the commands give for a set of MZ and Wolf texts under every
//! setting that changes the result: the text-fitting view, the scan (counts,
//! notable rows, characters missing from the font), the MZ previews and the
//! exported files. The expected output is versioned in
//! `tests/fixtures/text_fit_corpus/*.json`; refactors of the layout code must
//! leave it unchanged.
//!
//! To write the expected output again (only on purpose, on the commit the
//! corpus is meant to describe): `H2S_BLESS_CORPUS=1 cargo test --test
//! text_fit_corpus`.
//!
//! Segment and file ids are replaced by stable names (`<file>#<key>`), and the
//! temporary folders by `<tmp>`.

use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::path::{Path, PathBuf};

use hoshi2star_lib::commands::export::export_project;
use hoshi2star_lib::commands::project::{
    get_segments, get_source_files, open_project, update_segment,
};
use hoshi2star_lib::commands::text_fit::{
    get_text_fit, preview_text_fit, scan_text_fit, set_text_fit,
};
use hoshi2star_lib::core::text_fit::TextFitSettings;
use hoshi2star_lib::db;
use hoshi2star_lib::engines::detector::WolfVersion;
use hoshi2star_lib::engines::mv_mz::layout::fit::{LineBreaks, Overflow};
use hoshi2star_lib::engines::wolf::extractor::extract_map_segments;
use hoshi2star_lib::state::AppState;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::Manager;

type App = tauri::App<tauri::test::MockRuntime>;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn corpus_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/text_fit_corpus")
        .join(format!("{name}.json"))
}

/// Compare `actual` with the versioned corpus, or write it when blessing.
fn check_corpus(name: &str, actual: Value) {
    let path = corpus_path(name);
    let text = serde_json::to_string_pretty(&actual).unwrap() + "\n";
    if std::env::var_os("H2S_BLESS_CORPUS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e} (bless with H2S_BLESS_CORPUS=1)", path.display()));
    if expected != text {
        let actual_path = std::env::temp_dir().join(format!("text_fit_corpus_{name}.actual.json"));
        std::fs::write(&actual_path, &text).unwrap();
        let first = expected
            .lines()
            .zip(text.lines())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| expected.lines().count().min(text.lines().count()));
        panic!(
            "text fitting corpus `{name}` changed (first different line: {}); actual output in {}",
            first + 1,
            actual_path.display()
        );
    }
}

async fn mock_app(root: &Path) -> App {
    let pool = db::pool::init(root.join("h2s.db").to_str().unwrap())
        .await
        .unwrap();
    mock_builder()
        .manage(AppState::new(pool, root.join("app-data")).unwrap())
        .build(mock_context(noop_assets()))
        .unwrap()
}

/// `(segment id, stable name, source)` of every segment, and the file ids by
/// stable name.
async fn segments(
    app: &App,
    project_id: &str,
) -> (Vec<(String, String, String)>, Vec<(String, String)>) {
    let mut out = Vec::new();
    let mut files = Vec::new();
    for file in get_source_files(project_id.to_string(), app.state())
        .await
        .unwrap()
    {
        files.push((file.id.clone(), file.file_name.clone()));
        let page = get_segments(
            project_id.to_string(),
            file.id.clone(),
            0,
            100_000,
            app.state(),
        )
        .await
        .unwrap();
        for segment in page.items {
            out.push((
                segment.id,
                format!("{}#{}", file.file_name, segment.json_key),
                segment.source_text,
            ));
        }
    }
    (out, files)
}

/// Serialise `value` with ids and temporary paths replaced by stable names.
fn stable(value: &impl serde::Serialize, names: &HashMap<String, String>, tmp: &Path) -> Value {
    fn walk(value: &mut Value, names: &HashMap<String, String>, tmp: &str) {
        match value {
            Value::String(text) => {
                if let Some(name) = names.get(text.as_str()) {
                    *text = name.clone();
                } else if text.contains(tmp) {
                    *text = text.replace(tmp, "<tmp>");
                }
            }
            Value::Array(items) => items.iter_mut().for_each(|v| walk(v, names, tmp)),
            Value::Object(map) => map.values_mut().for_each(|v| walk(v, names, tmp)),
            _ => {}
        }
    }
    let mut value = serde_json::to_value(value).unwrap();
    walk(&mut value, names, &tmp.to_string_lossy());
    value
}

fn zip_entries(zip_path: &str) -> BTreeMap<String, Vec<u8>> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(zip_path).unwrap()).unwrap();
    let mut entries = BTreeMap::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).unwrap();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        entries.insert(entry.name().to_string(), bytes);
    }
    entries
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn setting_name(settings: &TextFitSettings) -> String {
    format!(
        "{:?}/{:?}/min{}/dialogue{:?}/width{:?}/lines{:?}/base{:?}",
        settings.line_breaks,
        settings.overflow,
        settings.min_font_size,
        settings.dialogue_font_size,
        settings.wolf_width_em,
        settings.wolf_max_lines,
        settings.wolf_base_font_size
    )
}

// ---------------------------------------------------------------------------
// MZ
// ---------------------------------------------------------------------------

fn copy_tree(source: &Path, destination: &Path) {
    std::fs::create_dir_all(destination).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// The translations of `tests/text_fit.rs` (same fixture).
const MZ_TRANSLATIONS: &[(&str, &str)] = &[
    (
        "長い台詞の一行目。",
        "This is a deliberately long first line of dialogue that will not fit in the box.",
    ),
    ("二行目。", "Second line."),
    ("一つ目の文。", "The first sentence of this rather talkative portrait message."),
    ("二つ目の文。", "The second sentence keeps going so that the box overflows."),
    ("三つ目の文。", "A third sentence follows, still long enough to need wrapping."),
    ("四つ目の文。", "And the fourth sentence finally ends this long message here."),
    ("長い前置き。", "A long introduction that certainly runs past the edge of the box, sadly."),
    ("そして。", "And then."),
    ("\\|時間の一。", "\\|The first timed line, which is long enough to be wrapped soon."),
    ("時間の二。", "The second timed line also carries plenty of words to wrap."),
    ("時間の三。", "The third timed line is here, making the message rather long."),
    (
        "時間の四。",
        "The fourth timed line ends it after a few more words than before.",
    ),
    ("どうする？", "What will you do?"),
    ("共通の台詞。", "Common line."),
    ("体力を少し回復する。", "Restores a little health."),
    (
        "体力を全回復する。\n戦闘中にも使える。",
        "Fully restores health and cures every ailment that the party might suffer from.\nCan also be used in battle.",
    ),
    ("若き魔術師。\n禁断の魔術を追う。", "A young magician.\nShe chases forbidden magic."),
];

/// Texts written by the export: every Show Text line and every description
/// or profile of the fitted files, by pointer.
fn mz_exported_texts(entries: &BTreeMap<String, Vec<u8>>) -> Value {
    let mut out = serde_json::Map::new();
    for (name, bytes) in entries {
        let Ok(value) = serde_json::from_slice::<Value>(bytes) else {
            continue;
        };
        let mut texts = Vec::new();
        collect_mz_texts(&value, String::new(), &mut texts);
        if !texts.is_empty() {
            out.insert(name.clone(), json!(texts));
        }
    }
    Value::Object(out)
}

fn collect_mz_texts(value: &Value, pointer: String, out: &mut Vec<Value>) {
    match value {
        Value::Object(map) => {
            if map.get("code").and_then(Value::as_u64) == Some(401) {
                if let Some(text) = map.get("parameters").and_then(|p| p.get(0)) {
                    out.push(json!([pointer, text]));
                }
            }
            for key in ["description", "profile"] {
                if let Some(text) = map.get(key).filter(|v| v.is_string()) {
                    out.push(json!([format!("{pointer}/{key}"), text]));
                }
            }
            for (key, child) in map {
                collect_mz_texts(child, format!("{pointer}/{key}"), out);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                collect_mz_texts(child, format!("{pointer}/{index}"), out);
            }
        }
        _ => {}
    }
}

#[tokio::test]
async fn text_fit_corpus_mz() {
    let temporary = tempfile::tempdir().unwrap();
    let game = temporary.path().join("game");
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mv_mz/mz_fit"),
        &game,
    );
    let app = mock_app(temporary.path()).await;
    let project_id = open_project(
        game.to_string_lossy().into_owned(),
        Some("ja".into()),
        Some("en".into()),
        app.state(),
    )
    .await
    .expect("open the MZ fixture")
    .project
    .id;
    let known: HashMap<&str, &str> = MZ_TRANSLATIONS.iter().copied().collect();
    let (all, files) = segments(&app, &project_id).await;
    let mut names: HashMap<String, String> = HashMap::new();
    names.insert(project_id.clone(), "<project>".into());
    for (id, name) in &files {
        names.insert(id.clone(), format!("<file {name}>"));
    }
    let mut stand_ins: HashMap<String, String> = HashMap::new();
    for (id, name, source) in &all {
        names.insert(id.clone(), name.clone());
        let next = stand_ins.len();
        let target = match known.get(source.as_str()) {
            Some(text) => text.to_string(),
            None => stand_ins
                .entry(source.clone())
                .or_insert_with(|| format!("Text {next}"))
                .clone(),
        };
        update_segment(id.clone(), target, app.state())
            .await
            .expect("update_segment");
    }

    let mut cases = Vec::new();
    let mut settings_list = Vec::new();
    for line_breaks in [LineBreaks::Reflow, LineBreaks::Keep] {
        for overflow in [Overflow::Shrink, Overflow::NewPage, Overflow::Leave] {
            settings_list.push(TextFitSettings {
                enabled_dialogue: true,
                enabled_descriptions: true,
                line_breaks,
                overflow,
                ..TextFitSettings::default()
            });
        }
    }
    // A dialogue font size and a lower minimum change page prefixes and shrinks.
    settings_list.push(TextFitSettings {
        enabled_dialogue: true,
        enabled_descriptions: true,
        overflow: Overflow::Shrink,
        min_font_size: 12,
        dialogue_font_size: Some(24),
        ..TextFitSettings::default()
    });

    for settings in settings_list {
        set_text_fit(project_id.clone(), settings.clone(), app.state())
            .await
            .expect("MZ settings are accepted");
        let view = get_text_fit(project_id.clone(), app.state()).await.unwrap();
        let scan = scan_text_fit(project_id.clone(), None, app.state())
            .await
            .unwrap();
        let mut previews = serde_json::Map::new();
        for (id, name, _) in &all {
            let preview = preview_text_fit(project_id.clone(), id.clone(), None, app.state())
                .await
                .unwrap();
            if let Some(preview) = preview {
                previews.insert(name.clone(), stable(&preview, &names, temporary.path()));
            }
        }
        let mut exports = serde_json::Map::new();
        for (font_size, replace) in [(None, true), (Some(22), true), (Some(22), false)] {
            let label = format!("font{font_size:?}/replace{replace}");
            let result =
                export_project(project_id.clone(), font_size, replace, None, app.state()).await;
            let value = match result {
                Ok(zip) => {
                    let entries = zip_entries(&zip);
                    json!({
                        "entries": entries
                            .iter()
                            .map(|(name, bytes)| (name.clone(), json!(sha256_hex(bytes))))
                            .collect::<serde_json::Map<_, _>>(),
                        "texts": mz_exported_texts(&entries),
                    })
                }
                Err(error) => json!({ "error": error }),
            };
            exports.insert(label, value);
        }
        cases.push(json!({
            "settings": setting_name(&settings),
            "view": stable(&view, &names, temporary.path()),
            "scan": stable(&scan, &names, temporary.path()),
            "previews": previews,
            "exports": exports,
        }));
    }
    check_corpus("mz", json!(cases));
}

// ---------------------------------------------------------------------------
// Wolf
// ---------------------------------------------------------------------------

/// Database magic of a Shift-JIS (Wolf 2.x) game (see `tests/e2e_project_flow.rs`).
const WOLF_DB_MAGIC_SJIS: [u8; 9] = [0x57, 0x00, 0x00, 0x4F, 0x4C, 0x00, 0x46, 0x4D, 0x00];

/// The decrypted `Data/BasicData/` of a Wolf 2.x game (copy of the helper of
/// `tests/e2e_project_flow.rs`).
fn write_wolf_v2_basic_data(game_root: &Path) {
    let basic_data = game_root.join("Data").join("BasicData");
    std::fs::create_dir_all(&basic_data).unwrap();
    let mut sys_database = vec![0x00]; // indicator = unencrypted
    sys_database.extend_from_slice(&WOLF_DB_MAGIC_SJIS);
    sys_database.push(0xC1);
    std::fs::write(basic_data.join("SysDatabase.dat"), sys_database).unwrap();
    std::fs::write(basic_data.join("DataBase.project"), b"").unwrap();
}

/// Length-prefixed Wolf string (byte count includes the trailing NUL).
fn wolf_string(bytes: &[u8]) -> Vec<u8> {
    let mut out = ((bytes.len() + 1) as u32).to_le_bytes().to_vec();
    out.extend_from_slice(bytes);
    out.push(0);
    out
}

/// A Wolf 2.x map stored in clear: one event, one page, one Show Message
/// (`101`) command carrying `message` (copy of the helper of
/// `tests/e2e_project_flow.rs`).
fn wolf_v2_map(message: &str) -> Vec<u8> {
    let (sjis, _, had_errors) = encoding_rs::SHIFT_JIS.encode(message);
    assert!(!had_errors, "test message must be Shift-JIS");
    let mut b = Vec::new();
    b.extend_from_slice(&[0u8; 10]);
    b.extend_from_slice(b"WOLFM");
    b.extend_from_slice(&[0u8; 5]); // NUL, Shift-JIS flag (byte 16), padding
    b.extend_from_slice(&0x64u32.to_le_bytes()); // version: stored in clear
    b.push(0); // unknown2
    b.extend(wolf_string(b"map"));
    for v in [1u32, 1, 1, 1] {
        b.extend_from_slice(&v.to_le_bytes()); // tileset, width, height, events
    }
    b.extend_from_slice(&[0u8; 12]); // 1x1 tiles, 3 layers
    b.push(0x6F); // event
    b.extend_from_slice(&[0x39, 0x30, 0x00, 0x00]);
    b.extend_from_slice(&0u32.to_le_bytes()); // id
    b.extend(wolf_string(b"EV001"));
    b.extend_from_slice(&0u32.to_le_bytes()); // x
    b.extend_from_slice(&0u32.to_le_bytes()); // y
    b.extend_from_slice(&1u32.to_le_bytes()); // page count
    b.extend_from_slice(&[0u8; 4]); // event magic 2
    b.push(0x79); // page
    b.extend_from_slice(&0u32.to_le_bytes()); // unknown1
    b.extend(wolf_string(b"")); // graphic name
    b.extend_from_slice(&[0u8; 4]); // direction, frame, opacity, render mode
    b.extend_from_slice(&[0u8; 37]); // conditions
    b.extend_from_slice(&[0u8; 4]); // movement
    b.extend_from_slice(&[0, 0]); // flags, route flags
    b.extend_from_slice(&0u32.to_le_bytes()); // route count
    b.extend_from_slice(&1u32.to_le_bytes()); // command count
    b.push(1); // args count + 1
    b.extend_from_slice(&101u32.to_le_bytes()); // Show Message
    b.push(0); // indent
    b.push(1); // string count
    b.extend(wolf_string(&sjis));
    b.push(0); // command terminator
    b.extend_from_slice(&0u32.to_le_bytes()); // features
    b.extend_from_slice(&[0, 0, 0]); // shadow, collision width / height
    b.push(0x7A); // end of page
    b.push(0x70); // end of event
    b.push(0x66); // end of map
    b
}

/// `(source, translation)`, one message per map. With the window of the
/// settings below (10 em, 3 lines, no font file: 20 half-width characters per
/// line), they cover: fits as is, rewrapped, a stray break before the final
/// dot, too many lines (shrunk with `\f[n]` when the base size is set), a word
/// wider than the window, the face prefix `@N⏎`, a font size code, a
/// positioning code, a speaker label, blank lines and an indented line.
const WOLF_MESSAGES: &[(&str, &str)] = &[
    ("こんにちは。", "Hello there."),
    (
        "こんにちは、旅の人。\nようこそ、川のそばの町へ。",
        "aaaa bbbb cccc dddd eeee \n.",
    ),
    (
        "これはとても長い話です。\n窓には収まらないかもしれません。",
        "This message is long enough to need more than three lines in the window, so it must shrink or stay too long.",
    ),
    (
        "単語。",
        "Supercalifragilisticexpialidocious word here.",
    ),
    (
        "@1\n顔つきの台詞です。\n二行目。",
        "@1\nA line spoken with a face graphic that is long enough to wrap.",
    ),
    (
        "\\f[16]大きな文字。",
        "\\f[16]Big letters that will probably need to be wrapped twice.",
    ),
    (
        "\\ax[10]位置指定の台詞。",
        "\\ax[10]A positioned line that is long enough to need wrapping.",
    ),
    (
        "魚淫さん：\nいらっしゃい。",
        "Gyoin:\nWelcome to the shop, traveller, take a good look around.",
    ),
    (
        "一行目。\n\n三行目。",
        "First part of the text.\n\nThird part, after a blank line, also long.",
    ),
    (
        "見出し。\n　字下げの行。",
        "A heading line here.\n\u{3000}An indented line that is fairly long too.",
    ),
    (
        "\\c[2]色付き\\c[0]の文。",
        "\\c[2]Coloured\\c[0] words inside a sentence that wraps somewhere.",
    ),
];

#[tokio::test]
async fn text_fit_corpus_wolf() {
    let temporary = tempfile::tempdir().unwrap();
    let game_root = temporary.path().join("game");
    let map_dir = game_root.join("Data").join("MapData");
    std::fs::create_dir_all(&map_dir).unwrap();
    for (index, (source, _)) in WOLF_MESSAGES.iter().enumerate() {
        std::fs::write(
            map_dir.join(format!("Map{:03}.mps", index + 1)),
            wolf_v2_map(source),
        )
        .unwrap();
    }
    std::fs::write(game_root.join("Game.ini"), b"").unwrap();
    write_wolf_v2_basic_data(&game_root);

    let app = mock_app(temporary.path()).await;
    let project_id = open_project(
        game_root.to_string_lossy().into_owned(),
        None,
        None,
        app.state(),
    )
    .await
    .expect("a Wolf 2.x game opens")
    .project
    .id;
    let translations: HashMap<&str, &str> = WOLF_MESSAGES.iter().copied().collect();
    let (all, files) = segments(&app, &project_id).await;
    let mut names: HashMap<String, String> = HashMap::new();
    names.insert(project_id.clone(), "<project>".into());
    for (id, name) in &files {
        names.insert(id.clone(), format!("<file {name}>"));
    }
    for (id, name, source) in &all {
        names.insert(id.clone(), name.clone());
        let target = translations
            .get(source.as_str())
            .unwrap_or_else(|| panic!("no translation for {source:?}"));
        update_segment(id.clone(), target.to_string(), app.state())
            .await
            .expect("update_segment");
    }

    let window = TextFitSettings {
        enabled_dialogue: true,
        wolf_width_em: Some(10.0),
        wolf_max_lines: Some(3),
        wolf_base_font_size: Some(12),
        ..TextFitSettings::default_for("wolf")
    };
    let settings_list = [
        window.clone(),
        TextFitSettings {
            line_breaks: LineBreaks::Keep,
            ..window.clone()
        },
        TextFitSettings {
            overflow: Overflow::Leave,
            ..window.clone()
        },
        TextFitSettings {
            wolf_base_font_size: None,
            ..window.clone()
        },
        TextFitSettings {
            min_font_size: 11,
            ..window.clone()
        },
    ];

    let version = WolfVersion { major: 2, minor: 0 };
    let mut cases = Vec::new();
    for settings in settings_list {
        set_text_fit(project_id.clone(), settings.clone(), app.state())
            .await
            .expect("Wolf settings are accepted");
        let view = get_text_fit(project_id.clone(), app.state()).await.unwrap();
        let scan = scan_text_fit(project_id.clone(), None, app.state())
            .await
            .unwrap();
        let mut exports = serde_json::Map::new();
        for (font_size, replace) in [(None, false), (Some(12), true)] {
            let label = format!("font{font_size:?}/replace{replace}");
            let result =
                export_project(project_id.clone(), font_size, replace, None, app.state()).await;
            let value = match result {
                Ok(zip) => {
                    let entries = zip_entries(&zip);
                    let mut maps = serde_json::Map::new();
                    for (name, bytes) in &entries {
                        let Some(stem) = name
                            .strip_prefix("Data/MapData/")
                            .and_then(|n| n.strip_suffix(".mps"))
                        else {
                            continue;
                        };
                        let messages: Vec<String> = extract_map_segments(stem, bytes, &version)
                            .expect("exported map reads back")
                            .into_iter()
                            .map(|segment| segment.source_text)
                            .collect();
                        maps.insert(
                            name.clone(),
                            json!({ "sha256": sha256_hex(bytes), "messages": messages }),
                        );
                    }
                    json!({
                        "entries": entries
                            .iter()
                            .map(|(name, bytes)| (name.clone(), json!(sha256_hex(bytes))))
                            .collect::<serde_json::Map<_, _>>(),
                        "maps": maps,
                    })
                }
                Err(error) => json!({ "error": error }),
            };
            exports.insert(label, value);
        }
        cases.push(json!({
            "settings": setting_name(&settings),
            "view": stable(&view, &names, temporary.path()),
            "scan": stable(&scan, &names, temporary.path()),
            "exports": exports,
        }));
    }
    check_corpus("wolf", json!(cases));
}
