//! Export after a game update (audit S-X1 / F, scenario P-2).
//!
//! MV/MZ segments are addressed by JSON pointer. A game update that inserts an
//! event command shifts every later pointer of the page: an export must then
//! refuse, never write a translation under a pointer whose original value is no
//! longer the recorded source (a face name, another line, a missing key).
//!
//! Each test drives the real command layer (`open_project` → `update_segment`
//! → game file changed on disk → `export_project`) on a tiny synthetic game.

use std::io::Read;
use std::path::{Path, PathBuf};

use hoshi2star_lib::commands::export::export_project;
use hoshi2star_lib::commands::project::{
    get_segments, get_source_files, open_project, update_segment,
};
use hoshi2star_lib::db;
use hoshi2star_lib::domain::types::Project;
use hoshi2star_lib::state::AppState;
use serde_json::{json, Value};
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::Manager;

const INTEGRITY_PREFIX: &str = "Export blocked: the integrity check found";

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

fn map_with(list: Vec<Value>) -> Value {
    json!({
        "displayName": "",
        "events": [null, { "id": 1, "name": "EV001", "pages": [{ "list": list }] }]
    })
}

/// MV page: two Show Text blocks, each with its own face.
fn mv_page() -> Vec<Value> {
    vec![
        command(101, json!(["Actor1", 0, 0, 2])),
        command(401, json!(["おはようございます。"])),
        command(101, json!(["Actor2", 1, 0, 2])),
        command(401, json!(["こんばんは、旅の方。"])),
        command(0, json!([])),
    ]
}

/// MZ page: two speakers, then a choice.
fn mz_page() -> Vec<Value> {
    vec![
        command(101, json!(["Actor1", 0, 0, 2, "村人"])),
        command(401, json!(["ようこそ、私たちの村へ。"])),
        command(101, json!(["Actor1", 1, 0, 2, "旅人"])),
        command(401, json!(["どこへ行けばいい？"])),
        command(102, json!([["北の森へ", "南の港へ"], 1, 0, 2, 0])),
        command(0, json!([])),
    ]
}

/// The same page after a game update: one dialogue line added at `at`.
fn with_inserted_line(mut list: Vec<Value>, at: usize) -> Vec<Value> {
    list.insert(at, command(401, json!(["新しく追加された台詞です。"])));
    list
}

struct Game {
    _temporary: tempfile::TempDir,
    app: tauri::App<tauri::test::MockRuntime>,
    project: Project,
    map_path: PathBuf,
    zip_path: PathBuf,
}

/// Write a synthetic game (`www/data` for MV, `data` for MZ), open it and
/// translate every segment as `EN<n>`.
async fn opened_and_translated(data_dir: &str, page: Vec<Value>) -> Game {
    let temporary = tempfile::tempdir().unwrap();
    let game_root = temporary.path().join("game");
    let data = game_root.join(data_dir);
    std::fs::create_dir_all(&data).unwrap();
    std::fs::write(
        data.join("System.json"),
        json!({ "gameTitle": "Test", "terms": {} }).to_string(),
    )
    .unwrap();
    let map_path = data.join("Map001.json");
    std::fs::write(&map_path, map_with(page).to_string()).unwrap();

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

    let files = get_source_files(project.id.clone(), app.state())
        .await
        .unwrap();
    let mut index = 0;
    for file in files {
        let page = get_segments(project.id.clone(), file.id.clone(), 0, 10_000, app.state())
            .await
            .unwrap();
        for segment in page.items {
            index += 1;
            update_segment(segment.id, format!("EN{index}"), app.state())
                .await
                .unwrap();
        }
    }
    assert!(index > 0, "the synthetic game must yield segments");

    Game {
        zip_path: game_root.join("hoshi2star.zip"),
        _temporary: temporary,
        app,
        project,
        map_path,
    }
}

/// `Map001.json` as exported in `zip_path` (`entry` = path inside the zip).
fn exported_map(zip_path: &Path, entry: &str) -> Value {
    let file = std::fs::File::open(zip_path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut bytes = Vec::new();
    archive
        .by_name(entry)
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// The page's command list as found in an exported map (for failure output).
fn exported_list(map: &Value) -> String {
    map.pointer("/events/1/pages/0/list")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .map(|cmd| format!("{} {}", cmd["code"], cmd["parameters"]))
                .collect::<Vec<_>>()
                .join("\n  ")
        })
        .unwrap_or_default()
}

/// Export after the game update: must be refused with every shifted segment
/// listed, and must write nothing.
async fn assert_refused(game: &Game, entry: &str, expected_pointers: &[&str]) {
    let result = export_project(game.project.id.clone(), None, true, None, game.app.state()).await;
    let error = match result {
        Ok(path) => panic!(
            "export after a game update must be refused, but it wrote {path}; \
             exported page:\n  {}",
            exported_list(&exported_map(Path::new(&path), entry))
        ),
        Err(error) => error,
    };
    assert!(
        error.starts_with(INTEGRITY_PREFIX),
        "the refusal must go through the integrity dialog, got: {error}"
    );
    for pointer in expected_pointers {
        let listed = error.lines().any(|line| {
            line.contains(&format!("Map001.json {pointer}: "))
                && line.contains("source changed since extraction")
        });
        assert!(listed, "{pointer} must be listed as changed in:\n{error}");
    }
    assert!(
        !game.zip_path.exists(),
        "a refused export must not write an archive"
    );
}

/// P-2 in MV: a line inserted before the second Show Text shifts its dialogue
/// pointer onto the face name of the next `101`.
#[tokio::test]
async fn mv_export_refuses_after_a_line_is_inserted() {
    let game = opened_and_translated("www/data", mv_page()).await;
    std::fs::write(
        &game.map_path,
        map_with(with_inserted_line(mv_page(), 2)).to_string(),
    )
    .unwrap();

    assert_refused(
        &game,
        "www/data/Map001.json",
        &["/events/1/pages/0/list/3/parameters/0"],
    )
    .await;
}

/// P-2 with a line that has no Japanese (`♪～`): the shifted dialogue pointer
/// lands on another `401`, so only the page structure tells it moved.
#[tokio::test]
async fn mv_export_refuses_after_a_line_without_japanese_is_inserted() {
    let page = vec![
        command(101, json!(["Actor1", 0, 0, 2])),
        command(401, json!(["こんにちは"])),
        command(0, json!([])),
    ];
    let game = opened_and_translated("www/data", page.clone()).await;
    let mut updated = page;
    updated.insert(1, command(401, json!(["♪～"])));
    std::fs::write(&game.map_path, map_with(updated).to_string()).unwrap();

    assert_refused(
        &game,
        "www/data/Map001.json",
        &["/events/1/pages/0/list/1/parameters/0"],
    )
    .await;
}

/// P-2 in MZ: the inserted line shifts a speaker name (`101[4]`) onto a `401`
/// and a choice (`102[0][n]`) onto a `401`, which used to fail with
/// `key not found in JSON`, and a dialogue line onto a face name.
#[tokio::test]
async fn mz_export_refuses_after_a_line_is_inserted() {
    let game = opened_and_translated("data", mz_page()).await;
    std::fs::write(
        &game.map_path,
        map_with(with_inserted_line(mz_page(), 2)).to_string(),
    )
    .unwrap();

    assert_refused(
        &game,
        "data/Map001.json",
        &[
            "/events/1/pages/0/list/2/parameters/4",
            "/events/1/pages/0/list/3/parameters/0",
            "/events/1/pages/0/list/4/parameters/0/0",
            "/events/1/pages/0/list/4/parameters/0/1",
        ],
    )
    .await;
}

/// Non-regression: an unchanged game exports every translation in place, and
/// exporting again over the unzipped archive gives the same bytes.
#[tokio::test]
async fn unchanged_game_exports_every_translation_in_place() {
    for (data_dir, page) in [("www/data", mv_page()), ("data", mz_page())] {
        let game = opened_and_translated(data_dir, page.clone()).await;
        let entry = format!("{data_dir}/Map001.json");
        let path = export_project(game.project.id.clone(), None, true, None, game.app.state())
            .await
            .expect("export of an unchanged game");
        let map = exported_map(Path::new(&path), &entry);
        let list = map["events"][1]["pages"][0]["list"].as_array().unwrap();
        assert_eq!(list.len(), page.len());
        for (exported, original) in list.iter().zip(&page) {
            assert_eq!(exported["code"], original["code"]);
            match original["code"].as_i64().unwrap() {
                // Face names never change.
                101 => assert_eq!(exported["parameters"][0], original["parameters"][0]),
                401 => assert!(exported["parameters"][0]
                    .as_str()
                    .unwrap()
                    .starts_with("EN")),
                _ => {}
            }
        }

        let bytes = std::fs::read(&game.map_path).unwrap();
        let unzipped = serde_json::to_vec(&map).unwrap();
        std::fs::write(&game.map_path, &unzipped).unwrap();
        let again = export_project(game.project.id.clone(), None, true, None, game.app.state())
            .await
            .expect("export over the unzipped archive");
        assert_eq!(exported_map(Path::new(&again), &entry), map);
        assert_ne!(bytes, unzipped, "the unzipped map carries the translations");
    }
}
