//! Debug export (plan DX): a developer tool that writes `hoshi2star-debug.zip`
//! with the translations ready so far, to test them in game before the project
//! is complete.
//!
//! Untranslated segments and translations with a critical QA error stay as in
//! the original game; the integrity gate and the game-update check (R2) are
//! those of the normal export, which keeps refusing incomplete or critical
//! projects. Each test drives the real command layer on a tiny synthetic game.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use hoshi2star_lib::commands::export::{export_debug_project, export_project, DebugExport};
use hoshi2star_lib::commands::project::{
    get_segments, get_source_files, open_project, update_segment,
};
use hoshi2star_lib::db;
use hoshi2star_lib::domain::types::{Project, Segment};
use hoshi2star_lib::state::AppState;
use serde_json::{json, Value};
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::Manager;

const INTEGRITY_PREFIX: &str = "Export blocked: the integrity check found";
const DEBUG_ZIP: &str = "hoshi2star-debug.zip";
const RELEASE_ZIP: &str = "hoshi2star.zip";

const GREETING: &str = "おはようございます。";
const PENDING: &str = "こんばんは、旅の方。";
const GOLD: &str = "所持金は\\V[1]ゴールドです。";

async fn mock_app(db_path: &str) -> tauri::App<tauri::test::MockRuntime> {
    let pool = db::pool::init(db_path).await.expect("pool init");
    mock_builder()
        .manage(
            AppState::new(pool, PathBuf::from(format!("{db_path}.data")))
                .expect("terminology service"),
        )
        .build(mock_context(noop_assets()))
        .expect("mock app build")
}

fn command(code: i64, parameters: Value) -> Value {
    json!({ "code": code, "indent": 0, "parameters": parameters })
}

/// MV page: three Show Text blocks, each with its own face.
fn mv_page() -> Vec<Value> {
    vec![
        command(101, json!(["Actor1", 0, 0, 2])),
        command(401, json!([GREETING])),
        command(101, json!(["Actor2", 1, 0, 2])),
        command(401, json!([PENDING])),
        command(101, json!(["Actor3", 2, 0, 2])),
        command(401, json!([GOLD])),
        command(0, json!([])),
    ]
}

fn mv_map(list: Vec<Value>) -> Value {
    json!({
        "displayName": "",
        "events": [null, { "id": 1, "name": "EV001", "pages": [{ "list": list }] }]
    })
}

struct Game {
    _temporary: tempfile::TempDir,
    app: tauri::App<tauri::test::MockRuntime>,
    project: Project,
    game_root: PathBuf,
    segments: Vec<Segment>,
}

impl Game {
    fn segment(&self, source: &str) -> &Segment {
        self.segments
            .iter()
            .find(|segment| segment.source_text == source)
            .unwrap_or_else(|| panic!("segment {source:?} in {:?}", self.sources()))
    }

    fn sources(&self) -> Vec<&str> {
        self.segments
            .iter()
            .map(|segment| segment.source_text.as_str())
            .collect()
    }

    async fn translate(&self, source: &str, target: &str) {
        update_segment(
            self.segment(source).id.clone(),
            target.to_string(),
            self.app.state(),
        )
        .await
        .expect("update_segment");
    }

    async fn debug_export(&self) -> Result<DebugExport, String> {
        export_debug_project(self.project.id.clone(), self.app.state()).await
    }
}

/// Open the game at `game_root` and list every extracted segment.
async fn opened(temporary: tempfile::TempDir, game_root: PathBuf) -> Game {
    let db_file = temporary.path().join("h2s.db");
    let app = mock_app(db_file.to_str().unwrap()).await;
    let project = open_project(
        game_root.to_string_lossy().to_string(),
        Some("ja".to_string()),
        Some("en".to_string()),
        app.state(),
    )
    .await
    .expect("open synthetic game")
    .project;
    let mut segments = Vec::new();
    for file in get_source_files(project.id.clone(), app.state())
        .await
        .unwrap()
    {
        let page = get_segments(project.id.clone(), file.id, 0, 10_000, app.state())
            .await
            .unwrap();
        segments.extend(page.items);
    }
    Game {
        _temporary: temporary,
        app,
        project,
        game_root,
        segments,
    }
}

/// A synthetic MV game (`www/data`) with the three-message page.
async fn mv_game() -> Game {
    let temporary = tempfile::tempdir().unwrap();
    let game_root = temporary.path().join("game");
    let data = game_root.join("www").join("data");
    std::fs::create_dir_all(&data).unwrap();
    std::fs::write(
        data.join("System.json"),
        json!({ "gameTitle": "Test", "terms": {} }).to_string(),
    )
    .unwrap();
    std::fs::write(data.join("Map001.json"), mv_map(mv_page()).to_string()).unwrap();
    opened(temporary, game_root).await
}

/// MV game with one translation, one untranslated line and one translation
/// that loses its `\V[1]` (critical QA error).
async fn mixed_mv_game() -> Game {
    let game = mv_game().await;
    game.translate(GREETING, "Good morning.").await;
    game.translate(GOLD, "You have some gold.").await;
    game
}

fn zip_entries(zip_path: &Path) -> BTreeMap<String, Vec<u8>> {
    let file = std::fs::File::open(zip_path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut entries = BTreeMap::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).unwrap();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        entries.insert(entry.name().to_string(), bytes);
    }
    entries
}

fn zip_comment(zip_path: &Path) -> String {
    let file = std::fs::File::open(zip_path).unwrap();
    let archive = zip::ZipArchive::new(file).unwrap();
    String::from_utf8_lossy(archive.comment()).into_owned()
}

/// The texts of the exported page, in command order.
fn exported_page_texts(map: &[u8]) -> Vec<String> {
    let map: Value = serde_json::from_slice(map).unwrap();
    map.pointer("/events/1/pages/0/list")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .filter(|cmd| cmd["code"] == 401)
        .map(|cmd| cmd["parameters"][0].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn mv_debug_export_delivers_only_the_ready_translations() {
    let game = mixed_mv_game().await;

    let result = game.debug_export().await.expect("debug export");

    let untranslated = game.segments.len() as i64 - 2;
    assert_eq!(
        (
            result.exported,
            result.untranslated,
            result.skipped_critical
        ),
        (1, untranslated, 1)
    );
    let zip_path = PathBuf::from(&result.zip_path);
    assert!(
        zip_path.ends_with(DEBUG_ZIP) && zip_path.exists(),
        "{zip_path:?}"
    );
    assert!(
        !game.game_root.join(RELEASE_ZIP).exists(),
        "a debug export never writes the release archive"
    );
    assert!(
        zip_comment(&zip_path).contains("DEBUG"),
        "the archive says it is a debug export: {:?}",
        zip_comment(&zip_path)
    );

    let entries = zip_entries(&zip_path);
    let map = entries
        .get("www/data/Map001.json")
        .unwrap_or_else(|| panic!("exported map in {:?}", entries.keys()));
    assert_eq!(
        exported_page_texts(map),
        vec!["Good morning.", PENDING, GOLD],
        "the untranslated line and the critical translation stay as in the original"
    );
}

#[tokio::test]
async fn release_export_still_refuses_what_the_debug_export_leaves_out() {
    let game = mixed_mv_game().await;

    let error = export_project(game.project.id.clone(), None, true, None, game.app.state())
        .await
        .expect_err("a critical QA error still blocks the normal export");
    assert!(error.contains("Export bloqué"), "{error}");
    assert!(!game.game_root.join(RELEASE_ZIP).exists());

    game.debug_export()
        .await
        .expect("the debug export is not blocked by the same error");
    assert!(
        !game.game_root.join(RELEASE_ZIP).exists(),
        "the debug export leaves the release archive alone"
    );
}

#[tokio::test]
async fn debug_export_refuses_a_project_without_translation() {
    let game = mv_game().await;

    let error = game.debug_export().await.expect_err("nothing to export");
    assert!(error.contains("no translation"), "{error}");
    assert!(!game.game_root.join(DEBUG_ZIP).exists());

    // A translation that only carries a critical error is not exportable either.
    game.translate(GOLD, "You have some gold.").await;
    game.debug_export()
        .await
        .expect_err("only a critical translation: nothing to export");
    assert!(!game.game_root.join(DEBUG_ZIP).exists());
}

#[tokio::test]
async fn debug_export_refuses_after_a_game_update() {
    let game = mixed_mv_game().await;

    // A game update inserts a line before the translated one (scenario P-2).
    let mut page = mv_page();
    page.insert(1, command(401, json!(["新しく追加された台詞です。"])));
    std::fs::write(
        game.game_root.join("www").join("data").join("Map001.json"),
        mv_map(page).to_string(),
    )
    .unwrap();

    let error = game
        .debug_export()
        .await
        .expect_err("the game-update check applies to the debug export");
    assert!(error.starts_with(INTEGRITY_PREFIX), "{error}");
    assert!(error.contains("source changed since extraction"), "{error}");
    assert!(!game.game_root.join(DEBUG_ZIP).exists());
}

/// Length-prefixed Wolf string (byte count includes the trailing NUL).
fn wolf_string(bytes: &[u8]) -> Vec<u8> {
    let mut out = ((bytes.len() + 1) as u32).to_le_bytes().to_vec();
    out.extend_from_slice(bytes);
    out.push(0);
    out
}

fn shift_jis(text: &str) -> Vec<u8> {
    let (bytes, _, had_errors) = encoding_rs::SHIFT_JIS.encode(text);
    assert!(!had_errors, "test message must be Shift-JIS");
    bytes.into_owned()
}

/// A Wolf 2.x map stored in clear (version `0x64`, Shift-JIS): one event, one
/// page, one Show Message (`101`) command per message (same layout as the
/// e2e test map of `e2e_project_flow.rs`).
fn wolf_v2_map(messages: &[&str]) -> Vec<u8> {
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
    b.extend_from_slice(&(messages.len() as u32).to_le_bytes()); // command count
    for message in messages {
        b.push(1); // args count + 1
        b.extend_from_slice(&101u32.to_le_bytes()); // Show Message
        b.push(0); // indent
        b.push(1); // string count
        b.extend(wolf_string(&shift_jis(message)));
        b.push(0); // command terminator
    }
    b.extend_from_slice(&0u32.to_le_bytes()); // features
    b.extend_from_slice(&[0, 0, 0]); // shadow, collision width / height
    b.push(0x7A); // end of page
    b.push(0x70); // end of event
    b.push(0x66); // end of map
    b
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

#[tokio::test]
async fn wolf_debug_export_delivers_only_the_ready_translations() {
    const HELLO: &str = "こんにちは、旅の人";
    const LATER: &str = "また明日来てください";
    const COPIED: &str = "気をつけて行ってらっしゃい";

    let temporary = tempfile::tempdir().unwrap();
    let game_root = temporary.path().join("game");
    let map_dir = game_root.join("Data").join("MapData");
    std::fs::create_dir_all(&map_dir).unwrap();
    std::fs::write(
        map_dir.join("Map001.mps"),
        wolf_v2_map(&[HELLO, LATER, COPIED]),
    )
    .unwrap();
    std::fs::write(game_root.join("Game.ini"), b"").unwrap();
    let game = opened(temporary, game_root).await;
    assert_eq!(game.project.engine, "wolf");

    game.translate(HELLO, "Hello, traveller").await;
    // Copying the source is a critical QA error: left as in the original.
    game.translate(COPIED, COPIED).await;

    let result = game.debug_export().await.expect("Wolf debug export");
    assert_eq!((result.exported, result.skipped_critical), (1, 1));

    let entries = zip_entries(Path::new(&result.zip_path));
    let map = entries
        .get("Data/MapData/Map001.mps")
        .unwrap_or_else(|| panic!("exported map in {:?}", entries.keys()));
    assert!(contains(map, b"Hello, traveller"));
    assert!(!contains(map, &shift_jis(HELLO)));
    assert!(
        contains(map, &shift_jis(LATER)),
        "the untranslated message stays as in the original"
    );
    assert!(
        contains(map, &shift_jis(COPIED)),
        "the critical message stays as in the original"
    );
}
