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

use hoshi2star_lib::commands::coverage::{set_text_categories, CategoryChange};
use hoshi2star_lib::commands::export::export_project;
use hoshi2star_lib::commands::project::{
    get_segments, get_source_files, open_project, update_segment,
};
use hoshi2star_lib::commands::resync::resync_project;
use hoshi2star_lib::db;
use hoshi2star_lib::domain::types::Project;
use hoshi2star_lib::engines::mv_mz::profiles::ProfileSet;
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
    temporary: tempfile::TempDir,
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
        temporary,
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

// ---------------------------------------------------------------------------
// Realignment (R2b)
// ---------------------------------------------------------------------------

/// `(key, source, target, status)` of every segment of `Map001.json`, in the
/// order the editor lists them.
async fn map_segments(game: &Game) -> Vec<(String, String, String, String)> {
    let files = get_source_files(game.project.id.clone(), game.app.state())
        .await
        .unwrap();
    let file = files
        .iter()
        .find(|file| file.file_name == "Map001.json")
        .unwrap();
    get_segments(
        game.project.id.clone(),
        file.id.clone(),
        0,
        10_000,
        game.app.state(),
    )
    .await
    .unwrap()
    .items
    .into_iter()
    .map(|s| (s.json_key, s.source_text, s.target_text, s.status))
    .collect()
}

async fn export(game: &Game) -> Result<String, String> {
    export_project(game.project.id.clone(), None, true, None, game.app.state()).await
}

/// After an inserted line, realigning moves the translation with its line:
/// the preview writes nothing, applying makes the export pass, the face name
/// stays and the new line is untranslated.
#[tokio::test]
async fn mv_resync_moves_translations_after_an_inserted_line() {
    let game = opened_and_translated("www/data", mv_page()).await;
    std::fs::write(
        &game.map_path,
        map_with(with_inserted_line(mv_page(), 2)).to_string(),
    )
    .unwrap();
    assert!(export(&game).await.is_err());

    let preview = resync_project(
        game.project.id.clone(),
        false,
        game.app.handle().clone(),
        game.app.state(),
    )
    .await
    .expect("preview");
    assert!(!preview.applied);
    assert_eq!((preview.moved, preview.added, preview.gone), (1, 1, 0));
    assert_eq!(preview.files.len(), 1);
    assert!(export(&game).await.is_err(), "a preview writes nothing");

    let applied = resync_project(
        game.project.id.clone(),
        true,
        game.app.handle().clone(),
        game.app.state(),
    )
    .await
    .expect("apply");
    assert!(applied.applied);
    assert_eq!((applied.moved, applied.added, applied.gone), (1, 1, 0));

    let list = "/events/1/pages/0/list";
    let segments = map_segments(&game).await;
    let keys: Vec<&str> = segments.iter().map(|s| s.0.as_str()).collect();
    assert_eq!(
        keys,
        vec![
            format!("{list}/1/parameters/0"),
            format!("{list}/2/parameters/0"),
            format!("{list}/4/parameters/0"),
        ],
        "the editor lists the lines in the new order"
    );
    assert_eq!(segments[1].2, "", "the new line is untranslated");
    assert_eq!(segments[1].3, "untranslated");
    assert_eq!(segments[2].1, "こんばんは、旅の方。");
    assert_eq!(segments[2].2, "EN2");

    let path = export(&game).await.expect("export after realignment");
    let map = exported_map(Path::new(&path), "www/data/Map001.json");
    assert_eq!(
        map.pointer(&format!("{list}/3/parameters/0")).unwrap(),
        "Actor2"
    );
    assert_eq!(
        map.pointer(&format!("{list}/4/parameters/0")).unwrap(),
        "EN2"
    );
    assert_eq!(
        map.pointer(&format!("{list}/2/parameters/0")).unwrap(),
        "新しく追加された台詞です。"
    );

    let again = resync_project(
        game.project.id.clone(),
        false,
        game.app.handle().clone(),
        game.app.state(),
    )
    .await
    .unwrap();
    assert!(again.files.is_empty(), "nothing left to realign: {again:?}");
}

/// MZ: speaker names and choices follow their commands too.
#[tokio::test]
async fn mz_resync_moves_speakers_and_choices() {
    let game = opened_and_translated("data", mz_page()).await;
    std::fs::write(
        &game.map_path,
        map_with(with_inserted_line(mz_page(), 2)).to_string(),
    )
    .unwrap();
    let before = map_segments(&game).await;
    let target_of = |segments: &[(String, String, String, String)], source: &str| {
        segments
            .iter()
            .find(|s| s.1 == source)
            .map(|s| s.2.clone())
            .unwrap()
    };

    let summary = resync_project(
        game.project.id.clone(),
        true,
        game.app.handle().clone(),
        game.app.state(),
    )
    .await
    .unwrap();
    assert_eq!((summary.moved, summary.added, summary.gone), (4, 1, 0));

    let path = export(&game).await.expect("export after realignment");
    let map = exported_map(Path::new(&path), "data/Map001.json");
    let list = "/events/1/pages/0/list";
    assert_eq!(
        map.pointer(&format!("{list}/3/parameters/4")).unwrap(),
        &json!(target_of(&before, "旅人"))
    );
    assert_eq!(
        map.pointer(&format!("{list}/3/parameters/0")).unwrap(),
        "Actor1",
        "face name untouched"
    );
    assert_eq!(
        map.pointer(&format!("{list}/5/parameters/0")).unwrap(),
        &json!([
            target_of(&before, "北の森へ"),
            target_of(&before, "南の港へ")
        ])
    );
}

/// Removed and rewritten lines: their translations are kept aside and in the
/// translation memory, never paired approximately, and the export passes.
#[tokio::test]
async fn resync_keeps_the_translations_of_removed_and_rewritten_lines() {
    let game = opened_and_translated("www/data", mv_page()).await;
    let mut updated = mv_page();
    updated.remove(1); // おはようございます。
    updated[2] = command(401, json!(["こんばんは、旅の皆さん。"]));
    std::fs::write(&game.map_path, map_with(updated).to_string()).unwrap();

    let summary = resync_project(
        game.project.id.clone(),
        true,
        game.app.handle().clone(),
        game.app.state(),
    )
    .await
    .unwrap();
    assert_eq!(
        (summary.moved, summary.added, summary.gone, summary.orphaned),
        (0, 1, 2, 2)
    );
    assert_eq!(summary.gone_translations.len(), 2);

    let db = &game.app.state::<AppState>().db;
    let orphans: Vec<(String, String)> = sqlx::query_as(
        "SELECT source_text, target_text FROM orphaned_segments WHERE project_id = ? \
         ORDER BY source_text",
    )
    .bind(&game.project.id)
    .fetch_all(db)
    .await
    .unwrap();
    assert_eq!(
        orphans,
        vec![
            ("おはようございます。".to_string(), "EN1".to_string()),
            ("こんばんは、旅の方。".to_string(), "EN2".to_string()),
        ]
    );
    let in_memory: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tm_entries WHERE source_text IN \
         ('おはようございます。', 'こんばんは、旅の方。')",
    )
    .fetch_one(db)
    .await
    .unwrap();
    assert_eq!(in_memory, 2);

    let segments = map_segments(&game).await;
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].1, "こんばんは、旅の皆さん。");
    assert_eq!(
        segments[0].2, "",
        "a rewritten line is a new, untranslated line"
    );
    export(&game).await.expect("export after realignment");
}

/// A data file added by the update joins the project; other files are left
/// as they are.
#[tokio::test]
async fn resync_adds_a_new_map_file() {
    let game = opened_and_translated("www/data", mv_page()).await;
    std::fs::write(
        &game.map_path,
        map_with(with_inserted_line(mv_page(), 2)).to_string(),
    )
    .unwrap();
    let new_map = game.temporary.path().join("game/www/data/Map002.json");
    std::fs::write(
        &new_map,
        map_with(vec![
            command(101, json!(["", 0, 0, 2])),
            command(401, json!(["二つ目の村です。"])),
            command(0, json!([])),
        ])
        .to_string(),
    )
    .unwrap();

    let summary = resync_project(
        game.project.id.clone(),
        true,
        game.app.handle().clone(),
        game.app.state(),
    )
    .await
    .unwrap();
    let new_file = summary
        .files
        .iter()
        .find(|file| file.file == "Map002.json")
        .expect("the new map is listed");
    assert!(new_file.new_file);
    assert_eq!(new_file.added, 1);

    let files = get_source_files(game.project.id.clone(), game.app.state())
        .await
        .unwrap();
    assert!(files.iter().any(|file| file.file_name == "Map002.json"));
    export(&game).await.expect("export after realignment");
}

/// Verified profile of a synthetic plugin whose `LOG` command shows its
/// `message` argument in game (optional category).
const LOG_PROFILE: &str = r#"
    [plugin]
    name = "Test/Log"
    title = "Test log"
    verified_versions = ["1.0"]
    reference = "synthetic"

    [[command_arg]]
    command = "LOG"
    arg = "message"
    title = "Test log"
    visible_in = "Battle log"
    message = true
"#;

const LOG_CATEGORY: &str = "plugin_command.Test/Log.LOG.message";

/// A game whose map shows a line, then runs the profiled plugin's `LOG`
/// command; the plugin command category is enabled and everything translated
/// (`Welcome.` / `The enemy fled!`).
struct PluginGame {
    _temporary: tempfile::TempDir,
    app: tauri::App<tauri::test::MockRuntime>,
    project: Project,
    map_path: PathBuf,
    plugin_path: PathBuf,
    page: Vec<Value>,
}

async fn plugin_game() -> PluginGame {
    let temporary = tempfile::tempdir().unwrap();
    let game_root = temporary.path().join("game");
    let data = game_root.join("data");
    let plugins = game_root.join("js/plugins/Test");
    std::fs::create_dir_all(&data).unwrap();
    std::fs::create_dir_all(&plugins).unwrap();
    std::fs::write(
        data.join("System.json"),
        json!({ "gameTitle": "Test", "terms": {} }).to_string(),
    )
    .unwrap();
    std::fs::write(plugins.join("Log.js"), "/*:\n * @version 1.0\n */\n").unwrap();
    std::fs::write(
        game_root.join("js/plugins.js"),
        "var $plugins =\n[\n{\"name\":\"Test/Log\",\"status\":true,\"description\":\"\",\"parameters\":{}}\n];\n",
    )
    .unwrap();
    let page = vec![
        command(401, json!(["ようこそ。"])),
        command(
            357,
            json!(["Test/Log", "LOG", "ログ", { "message": "敵が逃げた！" }]),
        ),
        command(0, json!([])),
    ];
    let map_path = data.join("Map001.json");
    std::fs::write(&map_path, map_with(page.clone()).to_string()).unwrap();

    let db_file = temporary.path().join("h2s.db");
    let pool = db::pool::init(db_file.to_str().unwrap()).await.unwrap();
    let mut state = AppState::new(pool, temporary.path().join("app-data")).unwrap();
    state.plugin_profiles =
        std::sync::Arc::new(ProfileSet::parse_all(&[("log.toml", LOG_PROFILE)]).unwrap());
    let app = mock_builder()
        .manage(state)
        .build(mock_context(noop_assets()))
        .unwrap();
    let project = open_project(
        game_root.to_string_lossy().into_owned(),
        Some("ja".into()),
        Some("en".into()),
        app.state(),
    )
    .await
    .unwrap()
    .project;
    set_text_categories(
        project.id.clone(),
        vec![CategoryChange {
            category_key: LOG_CATEGORY.into(),
            enabled: true,
            accept_consequences: true,
        }],
        app.state(),
    )
    .await
    .expect("enable the plugin command category");

    let mut translated = Vec::new();
    for file in get_source_files(project.id.clone(), app.state())
        .await
        .unwrap()
    {
        let page = get_segments(project.id.clone(), file.id, 0, 10_000, app.state())
            .await
            .unwrap();
        for segment in page.items {
            translated.push((segment.text_category.clone(), segment.json_key.clone()));
            let target = if segment.text_category.is_some() {
                "The enemy fled!"
            } else {
                "Welcome."
            };
            update_segment(segment.id, target.to_string(), app.state())
                .await
                .unwrap();
        }
    }
    assert!(
        translated.contains(&(
            Some(LOG_CATEGORY.to_string()),
            "/events/1/pages/0/list/1/parameters/3/message".to_string()
        )),
        "{translated:?}"
    );

    PluginGame {
        _temporary: temporary,
        app,
        project,
        map_path,
        plugin_path: plugins.join("Log.js"),
        page,
    }
}

/// Optional-category rows inside an event command list (plugin command
/// arguments) are realigned like core lines.
#[tokio::test]
async fn resync_moves_plugin_command_arguments_of_an_optional_category() {
    let PluginGame {
        _temporary: _game_folder,
        app,
        project,
        map_path,
        page,
        ..
    } = plugin_game().await;
    let mut updated = page;
    updated.insert(0, command(230, json!([60])));
    std::fs::write(&map_path, map_with(updated).to_string()).unwrap();
    let refused = export_project(project.id.clone(), None, true, None, app.state())
        .await
        .unwrap_err();
    assert!(
        refused.contains("list/1/parameters/3/message: source changed"),
        "{refused}"
    );

    let summary = resync_project(project.id.clone(), true, app.handle().clone(), app.state())
        .await
        .unwrap();
    assert_eq!((summary.moved, summary.added, summary.gone), (2, 0, 0));
    let path = export_project(project.id.clone(), None, true, None, app.state())
        .await
        .expect("export after realignment");
    let map = exported_map(Path::new(&path), "data/Map001.json");
    assert_eq!(
        map.pointer("/events/1/pages/0/list/2/parameters/3/message")
            .unwrap(),
        "The enemy fled!"
    );
    assert_eq!(
        map.pointer("/events/1/pages/0/list/1/parameters/0")
            .unwrap(),
        "Welcome."
    );
}

/// A category the coverage scan no longer offers (here, the update changed
/// the plugin to a version no profile verifies) is left untouched: its rows
/// are still in the game, so they are neither deleted nor listed as gone.
#[tokio::test]
async fn resync_leaves_a_category_no_longer_offered_untouched() {
    let game = plugin_game().await;
    let mut updated = game.page.clone();
    updated.insert(0, command(230, json!([60])));
    std::fs::write(&game.map_path, map_with(updated).to_string()).unwrap();
    std::fs::write(&game.plugin_path, "/*:\n * @version 1.1\n */\n").unwrap();

    let summary = resync_project(
        game.project.id.clone(),
        true,
        game.app.handle().clone(),
        game.app.state(),
    )
    .await
    .unwrap();
    assert_eq!((summary.moved, summary.gone), (1, 0), "{summary:?}");
    assert_eq!(summary.untouched_categories, vec![LOG_CATEGORY.to_string()]);

    let db = &game.app.state::<AppState>().db;
    let kept: Vec<(String, String)> =
        sqlx::query_as("SELECT json_key, target_text FROM segments WHERE text_category = ?")
            .bind(LOG_CATEGORY)
            .fetch_all(db)
            .await
            .unwrap();
    assert_eq!(
        kept,
        vec![(
            "/events/1/pages/0/list/1/parameters/3/message".to_string(),
            "The enemy fled!".to_string()
        )]
    );
}

/// The translation of a gone line only fills a gap in the translation memory:
/// it never replaces an entry already there for the same source.
#[tokio::test]
async fn resync_never_overwrites_a_translation_memory_entry() {
    let game = opened_and_translated("www/data", mv_page()).await;
    let db = &game.app.state::<AppState>().db;
    sqlx::query("UPDATE tm_entries SET target_text = 'Reviewed' WHERE source_text = ?")
        .bind("おはようございます。")
        .execute(db)
        .await
        .unwrap();
    let mut updated = mv_page();
    updated.remove(1);
    std::fs::write(&game.map_path, map_with(updated).to_string()).unwrap();

    resync_project(
        game.project.id.clone(),
        true,
        game.app.handle().clone(),
        game.app.state(),
    )
    .await
    .unwrap();
    let kept: String =
        sqlx::query_scalar("SELECT target_text FROM tm_entries WHERE source_text = ?")
            .bind("おはようございます。")
            .fetch_one(db)
            .await
            .unwrap();
    assert_eq!(kept, "Reviewed");
}
