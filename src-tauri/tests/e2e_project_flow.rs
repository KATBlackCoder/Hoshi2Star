//! End-to-end integration tests exercising the real Tauri command layer
//! (`open_project` → `update_segment` → `export_project`) against decrypted
//! small, versioned synthetic game fixtures.
//!
//! MV/MZ coverage must never depend on a user's private games or a gitignored
//! local directory. Optional Wolf coverage still skips when its legacy fixture
//! is absent. The flow is driven through the actual
//! `#[tauri::command]` functions — `mock_builder` only holds the managed
//! `AppState`; commands are called directly with a `State` from `Manager`.
//!
//! An ASCII marker is used as the translation: ASCII bytes are identical under
//! both UTF-8 (MV/MZ JSON) and Shift-JIS (Wolf v2 `.dat`), so the marker can be
//! located as a raw byte substring in the exported archive without depending on
//! any crate-internal (`pub(crate)`) parser.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use hoshi2star_lib::commands::export::export_project;
use hoshi2star_lib::commands::pilot::prepare_mv_mz_pilot;
use hoshi2star_lib::commands::project::{
    debug_dump_segments, get_segments, get_source_files, open_project, update_segment,
};
use hoshi2star_lib::commands::qa::get_qa_report;
use hoshi2star_lib::db;
use hoshi2star_lib::domain::types::Project;
use hoshi2star_lib::engines::filter;
use hoshi2star_lib::llm::tokenizer::Engine as TokEngine;
use hoshi2star_lib::state::AppState;
use sha2::{Digest, Sha256};
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::Manager;

const MARKER: &str = "H2S_E2E_MARKER";

/// Clearing a manual translation must restore the untranslated state, remove
/// the stale QA score and avoid adding an empty entry to translation memory.
#[tokio::test]
async fn empty_manual_translation_restores_untranslated_state() {
    let tmp = tempfile::tempdir().unwrap();
    let db_file = tmp.path().join("h2s.db");
    let app = mock_app(db_file.to_str().unwrap()).await;
    let db = &app.state::<AppState>().db;

    sqlx::query(
        "INSERT INTO projects (id, name, engine, game_path) \
         VALUES ('p1', 'Test', 'mv_mz', ?)",
    )
    .bind(tmp.path().to_string_lossy().as_ref())
    .execute(db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
         VALUES ('f1', 'p1', 'Actors.json', '/tmp/Actors.json', 'actors')",
    )
    .execute(db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO segments \
         (id, source_file_id, json_key, source_text, segment_kind, scene_id, \
          sequence_index, speaker, branch_path, context_json, target_text, status, qa_score) \
         VALUES ('s1', 'f1', '/1/name', '主人公', 'dialogue', 'Map001.json:event:1:page:0', \
                 1, '勇者', 'if:0', '{\"schemaVersion\":1}', 'Hero', 'translated', 100)",
    )
    .execute(db)
    .await
    .unwrap();

    let segment = update_segment("s1".to_string(), "   ".to_string(), app.state())
        .await
        .expect("clear manual translation");
    let tm_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tm_entries")
        .fetch_one(db)
        .await
        .unwrap();
    let report = get_qa_report("p1".to_string(), app.state())
        .await
        .expect("QA report");

    assert_eq!(segment.target_text, "");
    assert_eq!(segment.status, "untranslated");
    assert_eq!(segment.qa_score, None);
    assert_eq!(segment.segment_kind, "dialogue");
    assert_eq!(
        segment.scene_id.as_deref(),
        Some("Map001.json:event:1:page:0")
    );
    assert_eq!(segment.sequence_index, Some(1));
    assert_eq!(segment.speaker.as_deref(), Some("勇者"));
    assert_eq!(segment.branch_path.as_deref(), Some("if:0"));
    assert_eq!(
        segment.context_json.as_deref(),
        Some(r#"{"schemaVersion":1}"#)
    );
    assert_eq!(tm_count, 0);
    assert_eq!(report.total_segments, 0);
    assert_eq!(report.ok_count, 0);
}

/// Repo `test/` directory (sibling of `src-tauri/`).
fn test_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("test")
}

fn versioned_fixture(name: &str, relative_data_dir: &Path) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("mv_mz")
        .join(name);
    assert!(
        root.join(relative_data_dir).join("System.json").is_file(),
        "versioned {name} fixture is incomplete: {}",
        root.display()
    );
    root
}

fn required_mv_fixture() -> PathBuf {
    versioned_fixture("mv", Path::new("www/data"))
}

fn required_mz_fixture() -> PathBuf {
    versioned_fixture("mz", Path::new("data"))
}

fn escape_json_pointer_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

/// Enumerate player-visible event strings that Hoshi2Star explicitly supports.
/// This is intentionally narrower than "every string parameter": plugin names,
/// script source, asset paths and editor comments must not become translations.
fn collect_supported_event_texts(
    value: &serde_json::Value,
    pointer: &str,
    output: &mut BTreeMap<String, String>,
) {
    match value {
        serde_json::Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                collect_supported_event_texts(item, &format!("{pointer}/{index}"), output);
            }
        }
        serde_json::Value::Object(object) => {
            let code = object.get("code").and_then(serde_json::Value::as_i64);
            let params = object
                .get("parameters")
                .and_then(serde_json::Value::as_array);
            if let (Some(code), Some(params)) = (code, params) {
                let params_pointer = format!("{pointer}/parameters");
                let mut insert_string = |index: usize, kind_pointer: String| {
                    if let Some(text) = params.get(index).and_then(serde_json::Value::as_str) {
                        if filter::needs_translation(text, TokEngine::MvMz) {
                            output.insert(kind_pointer, text.to_string());
                        }
                    }
                };

                match code {
                    101 => insert_string(4, format!("{params_pointer}/4")),
                    401 | 405 => insert_string(0, format!("{params_pointer}/0")),
                    102 => {
                        if let Some(choices) = params.first().and_then(serde_json::Value::as_array)
                        {
                            for (index, choice) in choices.iter().enumerate() {
                                if let Some(text) = choice.as_str() {
                                    if filter::needs_translation(text, TokEngine::MvMz) {
                                        output.insert(
                                            format!("{params_pointer}/0/{index}"),
                                            text.to_string(),
                                        );
                                    }
                                }
                            }
                        }
                    }
                    357 if params.first().and_then(serde_json::Value::as_str)
                        == Some("TextPicture")
                        && params.get(1).and_then(serde_json::Value::as_str) == Some("set") =>
                    {
                        if let Some(text) = params
                            .get(3)
                            .and_then(serde_json::Value::as_object)
                            .and_then(|args| args.get("text"))
                            .and_then(serde_json::Value::as_str)
                        {
                            if filter::needs_translation(text, TokEngine::MvMz) {
                                output.insert(format!("{params_pointer}/3/text"), text.to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }

            for (key, child) in object {
                collect_supported_event_texts(
                    child,
                    &format!("{pointer}/{}", escape_json_pointer_token(key)),
                    output,
                );
            }
        }
        _ => {}
    }
}

fn insert_supported_string(
    value: &serde_json::Value,
    pointer: String,
    output: &mut BTreeMap<String, String>,
) {
    if let Some(text) = value.pointer(&pointer).and_then(serde_json::Value::as_str) {
        if filter::needs_translation(text, TokEngine::MvMz) {
            output.insert(pointer, text.to_string());
        }
    }
}

fn collect_array_fields(
    value: &serde_json::Value,
    fields: &[&str],
    output: &mut BTreeMap<String, String>,
) {
    let Some(entries) = value.as_array() else {
        return;
    };
    for (index, entry) in entries.iter().enumerate() {
        if entry.is_null() {
            continue;
        }
        for field in fields {
            insert_supported_string(value, format!("/{index}/{field}"), output);
        }
    }
}

/// Independently enumerate the database and System.json fields that the
/// player can see and that the MV/MZ adapter claims to support.
fn collect_supported_database_texts(
    file_name: &str,
    value: &serde_json::Value,
    output: &mut BTreeMap<String, String>,
) {
    match file_name {
        "Actors.json" => collect_array_fields(value, &["name", "nickname", "profile"], output),
        "Armors.json" | "Items.json" | "Weapons.json" => {
            collect_array_fields(value, &["name", "description"], output);
        }
        "Classes.json" | "Enemies.json" | "MapInfos.json" => {
            collect_array_fields(value, &["name"], output);
        }
        "Skills.json" => collect_array_fields(
            value,
            &["name", "description", "message1", "message2"],
            output,
        ),
        "States.json" => collect_array_fields(
            value,
            &["name", "message1", "message2", "message3", "message4"],
            output,
        ),
        "System.json" => {
            if value
                .pointer("/gameTitle")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|title| !title.trim().is_empty())
            {
                let title = value.pointer("/gameTitle").unwrap().as_str().unwrap();
                output.insert("/gameTitle".to_string(), title.to_string());
            }
            insert_supported_string(value, "/currencyUnit".to_string(), output);
            for group in ["basic", "commands", "params"] {
                if let Some(terms) = value
                    .pointer(&format!("/terms/{group}"))
                    .and_then(serde_json::Value::as_array)
                {
                    for index in 0..terms.len() {
                        insert_supported_string(value, format!("/terms/{group}/{index}"), output);
                    }
                }
            }
            if let Some(messages) = value
                .pointer("/terms/messages")
                .and_then(serde_json::Value::as_object)
            {
                for key in messages.keys() {
                    insert_supported_string(
                        value,
                        format!("/terms/messages/{}", escape_json_pointer_token(key)),
                        output,
                    );
                }
            }
        }
        _ => {}
    }
}

/// Verify both extraction precision (every segment points to the exact source
/// string) and recall for the player-visible event types supported above.
fn verify_dump_against_game(dump_path: &Path, data_dir: &Path) -> (usize, usize, usize) {
    let dump: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dump_path).expect("read extraction dump"))
            .expect("parse extraction dump");
    let mut extracted_count = 0usize;
    let mut expected_visible_count = 0usize;
    let mut expected_supported_count = 0usize;

    for file in dump["files"].as_array().expect("dump files array") {
        let file_name = file["file_name"].as_str().expect("dump file name");
        let source_path = data_dir.join(file_name);
        let source_json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&source_path)
                .unwrap_or_else(|error| panic!("read {}: {error}", source_path.display())),
        )
        .unwrap_or_else(|error| panic!("parse {}: {error}", source_path.display()));

        let mut extracted = BTreeMap::new();
        for segment in file["segments"].as_array().expect("dump segments array") {
            let key = segment["key"].as_str().expect("segment key");
            let source_text = segment["source_text"]
                .as_str()
                .expect("segment source text");
            let original = source_json
                .pointer(key)
                .and_then(serde_json::Value::as_str)
                .unwrap_or_else(|| panic!("{file_name}{key} must point to an original string"));
            assert_eq!(
                source_text, original,
                "{file_name}{key} must preserve the exact game text"
            );
            assert!(
                filter::needs_translation(source_text, TokEngine::MvMz),
                "{file_name}{key} is not useful translation content: {source_text:?}"
            );
            extracted.insert(key.to_string(), source_text.to_string());
            extracted_count += 1;
        }

        let mut expected_visible = BTreeMap::new();
        collect_supported_event_texts(&source_json, "", &mut expected_visible);
        let mut expected_supported = expected_visible.clone();
        collect_supported_database_texts(file_name, &source_json, &mut expected_supported);
        for (key, text) in &expected_supported {
            assert_eq!(
                extracted.get(key),
                Some(text),
                "supported player-visible text missing from {file_name}{key}: {text:?}"
            );
        }
        expected_visible_count += expected_visible.len();
        expected_supported_count += expected_supported.len();
    }

    (
        extracted_count,
        expected_visible_count,
        expected_supported_count,
    )
}

/// Recursively copy `src` into `dst` (directories created as needed).
fn copy_tree(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let target = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

fn tree_digest(root: &Path) -> [u8; 32] {
    fn collect_files(root: &Path, current: &Path, files: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(current).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                collect_files(root, &entry.path(), files);
            } else {
                files.push(entry.path().strip_prefix(root).unwrap().to_path_buf());
            }
        }
    }

    let mut files = Vec::new();
    collect_files(root, root, &mut files);
    files.sort();
    let mut hasher = Sha256::new();
    for relative in files {
        hasher.update(relative.to_string_lossy().as_bytes());
        hasher.update([0]);
        hasher.update(std::fs::read(root.join(relative)).unwrap());
    }
    hasher.finalize().into()
}

/// Build a mock Tauri app whose managed state wraps a fresh migrated DB.
async fn mock_app(db_path: &str) -> tauri::App<tauri::test::MockRuntime> {
    let pool = db::pool::init(db_path).await.expect("pool init");
    mock_builder()
        .manage(AppState::new(pool).expect("terminology service"))
        .build(mock_context(noop_assets()))
        .expect("mock app build")
}

/// Copy the committed MV fixture into a disposable game directory and open it
/// through the real command layer. Keeping this setup in one place prevents QA
/// and export tests from slowly growing different pseudo-games.
async fn opened_mv_fixture(
    target_language: &str,
) -> (
    tempfile::TempDir,
    tauri::App<tauri::test::MockRuntime>,
    Project,
) {
    let fixture = required_mv_fixture();
    let temporary = tempfile::tempdir().unwrap();
    let game_root = temporary.path().join("game");
    copy_tree(
        &fixture.join("www").join("data"),
        &game_root.join("www").join("data"),
    );

    let db_file = temporary.path().join("h2s.db");
    let app = mock_app(db_file.to_str().unwrap()).await;
    let project = open_project(
        game_root.to_string_lossy().to_string(),
        Some("ja".to_string()),
        Some(target_language.to_string()),
        app.state(),
    )
    .await
    .expect("open synthetic MV fixture")
    .project;

    (temporary, app, project)
}

/// Assert that the archive at `zip_path` contains `entry_name` and that its
/// bytes include `needle`.
fn zip_entry_contains(zip_path: &Path, entry_name: &str, needle: &[u8]) -> bool {
    let file = std::fs::File::open(zip_path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut entry = match archive.by_name(entry_name) {
        Ok(e) => e,
        Err(_) => return false,
    };
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).unwrap();
    bytes.windows(needle.len()).any(|w| w == needle)
}

/// MV/MZ round-trip: open → translate one segment → export → the marker is
/// present in the exported `www/data/*.json`.
#[tokio::test]
async fn mv_open_translate_export_round_trip() {
    let (_temporary, app, project) = opened_mv_fixture("fr").await;
    assert_eq!(project.engine, "mv_mz");
    assert_eq!(project.source_lang, "ja");
    assert_eq!(project.target_lang, "fr");

    let files = get_source_files(project.id.clone(), app.state())
        .await
        .expect("get_source_files");
    let file = files.first().expect("at least one source file");

    let page = get_segments(project.id.clone(), file.id.clone(), 0, 100_000, app.state())
        .await
        .expect("get_segments");
    let seg = page.items.first().expect("at least one segment");

    update_segment(seg.id.clone(), MARKER.to_string(), app.state())
        .await
        .expect("update_segment");

    let zip_path = export_project(project.id.clone(), None, true, app.state())
        .await
        .expect("export_project");

    // The exported file lives under www/data/ — find the one carrying the marker.
    let file_zip_path = format!("www/data/{}", file.file_name);
    assert!(
        zip_entry_contains(Path::new(&zip_path), &file_zip_path, MARKER.as_bytes()),
        "exported {file_zip_path} must contain the injected translation"
    );
}

/// A width warning requires review but must not make a valid patch impossible.
#[tokio::test]
async fn mv_export_allows_a_non_critical_width_warning() {
    let (_temporary, app, project) = opened_mv_fixture("en").await;
    let files = get_source_files(project.id.clone(), app.state())
        .await
        .expect("get_source_files");
    let first_file = files.first().expect("at least one source file");
    let segment = get_segments(
        project.id.clone(),
        first_file.id.clone(),
        0,
        100_000,
        app.state(),
    )
    .await
    .expect("get_segments")
    .items
    .into_iter()
    .next()
    .expect("at least one segment");

    let updated = update_segment(
        segment.id,
        "This translated line is deliberately wider than the default message box.".to_string(),
        app.state(),
    )
    .await
    .expect("save warning-only translation");
    assert_eq!(updated.qa_score, Some(90));

    export_project(project.id, None, true, app.state())
        .await
        .expect("a warning-only project must remain exportable");
}

/// Missing engine variables are blockers even though their legacy numeric
/// score (75) is above the UI's former `score < 70` critical threshold.
#[tokio::test]
async fn mv_export_blocks_a_missing_placeholder_at_score_75() {
    let (_temporary, app, project) = opened_mv_fixture("en").await;
    let files = get_source_files(project.id.clone(), app.state())
        .await
        .expect("get_source_files");
    let mut placeholder_segment = None;
    for file in files {
        let page = get_segments(project.id.clone(), file.id, 0, 100_000, app.state())
            .await
            .expect("get_segments");
        if let Some(segment) = page
            .items
            .into_iter()
            .find(|segment| segment.source_text.contains(r"\V[1]"))
        {
            placeholder_segment = Some(segment);
            break;
        }
    }
    let segment = placeholder_segment.expect("fixture segment containing \\V[1]");
    let updated = update_segment(segment.id, "You have some gold.".to_string(), app.state())
        .await
        .expect("save structurally invalid translation");
    assert_eq!(updated.qa_score, Some(75));

    let error = export_project(project.id, None, true, app.state())
        .await
        .expect_err("a missing placeholder must block export");
    assert!(error.contains("Export bloqué"));
}

/// Characterization test (Phase 8, étape 0): `debug_dump_segments` must extract
/// exactly the same `(json_key, source_text)` segments, in the same per-file
/// order, as `open_project` persists to the DB. The two share one engine
/// extractor after the Phase 8 dispatch refactor; this locks them together so
/// the refactor cannot silently diverge the debug path — which the round-trip
/// tests do not exercise. `get_segments` returns `ORDER BY rowid` (insertion =
/// extraction order), so ordered comparison is faithful.
#[tokio::test]
async fn mv_debug_dump_matches_open_project_extraction() {
    let fixture = required_mv_fixture();

    let tmp = tempfile::tempdir().unwrap();
    let game_root = tmp.path().join("game");
    copy_tree(
        &fixture.join("www").join("data"),
        &game_root.join("www").join("data"),
    );

    let db_file = tmp.path().join("h2s.db");
    let app = mock_app(db_file.to_str().unwrap()).await;

    let game_path = game_root.to_str().unwrap().to_string();
    let project = open_project(game_path.clone(), None, None, app.state())
        .await
        .expect("open_project")
        .project;

    // Map file_name -> [(json_key, source_text)] as persisted by open_project.
    let files = get_source_files(project.id.clone(), app.state())
        .await
        .expect("get_source_files");
    let mut from_db: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    let mut contextual_event_segments = 0_usize;
    for f in &files {
        let page = get_segments(project.id.clone(), f.id.clone(), 0, 1_000_000, app.state())
            .await
            .expect("get_segments");
        for segment in &page.items {
            assert_ne!(
                segment.segment_kind, "unknown",
                "MV/MZ must assign a semantic kind to every extracted segment"
            );
            if segment.scene_id.is_some() {
                assert!(
                    segment.sequence_index.is_some(),
                    "an MV/MZ scene segment must have an order"
                );
                contextual_event_segments += 1;
            }
        }
        from_db.insert(
            f.file_name.clone(),
            page.items
                .into_iter()
                .map(|s| (s.json_key, s.source_text))
                .collect(),
        );
    }

    // Same map, parsed from the JSON written by debug_dump_segments.
    let dump_path = debug_dump_segments(game_path)
        .await
        .expect("debug_dump_segments");
    let json = std::fs::read_to_string(&dump_path).unwrap();
    let dump: serde_json::Value = serde_json::from_str(&json).unwrap();
    let mut from_dump: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for f in dump["files"].as_array().unwrap() {
        let name = f["file_name"].as_str().unwrap().to_string();
        let segs = f["segments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| {
                (
                    s["key"].as_str().unwrap().to_string(),
                    s["source_text"].as_str().unwrap().to_string(),
                )
            })
            .collect();
        from_dump.insert(name, segs);
    }

    assert!(
        !from_db.is_empty(),
        "MV fixture must yield at least one source file"
    );
    assert!(
        contextual_event_segments > 0,
        "MV fixture must yield context-aware event segments"
    );
    assert_eq!(
        from_db, from_dump,
        "debug_dump_segments must extract the same segments per file as open_project persists"
    );

    let (extracted, visible, supported) =
        verify_dump_against_game(Path::new(&dump_path), &game_root.join("www").join("data"));
    println!(
        "MV extraction audit: {extracted} exact segments, {visible} visible event texts, \
         {supported} supported game-content texts"
    );
}

#[tokio::test]
async fn mz_debug_dump_matches_original_game_content() {
    let fixture = required_mz_fixture();
    let tmp = tempfile::tempdir().unwrap();
    let game_root = tmp.path().join("game");
    let data_dir = game_root.join("data");
    copy_tree(&fixture.join("data"), &data_dir);

    let dump_path = debug_dump_segments(game_root.to_string_lossy().to_string())
        .await
        .expect("debug_dump_segments MZ");
    let dump_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&dump_path).expect("read MZ debug dump"))
            .expect("parse MZ debug dump");
    let dump_segments: Vec<&serde_json::Value> = dump_json["files"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|file| file["segments"].as_array().unwrap())
        .collect();
    assert!(dump_segments.iter().all(|segment| {
        segment["kind"]
            .as_str()
            .is_some_and(|kind| kind != "unknown")
    }));
    assert!(dump_segments.iter().any(|segment| {
        segment["scene_id"].is_string() && segment["sequence_index"].is_number()
    }));
    let (extracted, visible, supported) =
        verify_dump_against_game(Path::new(&dump_path), &data_dir);

    println!(
        "MZ extraction audit: {extracted} exact segments, {visible} visible event texts, \
         {supported} supported game-content texts"
    );
    assert!(extracted > 0, "MZ fixture must yield translated content");
    assert!(
        visible > 0,
        "MZ fixture must exercise player-visible event text"
    );
    assert_eq!(
        extracted, supported,
        "the synthetic fixture must not hide unsupported extraction"
    );
}

#[tokio::test]
async fn mv_mz_pilot_preparation_is_deterministic_and_non_destructive() {
    for (fixture, relative_data_dir) in [
        (required_mv_fixture(), PathBuf::from("www/data")),
        (required_mz_fixture(), PathBuf::from("data")),
    ] {
        let temporary = tempfile::tempdir().unwrap();
        let game_root = temporary.path().join("game");
        let copied_data = game_root.join(&relative_data_dir);
        copy_tree(&fixture.join(&relative_data_dir), &copied_data);
        let before = tree_digest(&game_root);

        let preparation = prepare_mv_mz_pilot(
            game_root.to_string_lossy().to_string(),
            Some("ja".to_string()),
            Some("fr".to_string()),
            50,
        )
        .await
        .expect("prepare isolated MV/MZ pilot");
        let unique_keys = preparation
            .sample
            .iter()
            .map(|segment| segment.stable_key.as_str())
            .collect::<std::collections::HashSet<_>>();

        let expected_sample_size = 50.min(preparation.total_segments);
        assert_eq!(preparation.sample_size, expected_sample_size);
        assert_eq!(unique_keys.len(), expected_sample_size);
        assert!(preparation.sample.iter().all(|item| item.context.is_some()));
        assert!(!preparation.isolation.personal_database_accessed);
        assert_eq!(preparation.isolation.game_files_written, 0);
        assert!(preparation.isolation.database_removed);
        assert_eq!(tree_digest(&game_root), before);
        assert!(!game_root.join(".hoshi2star.json").exists());
        assert!(!game_root.join("hoshi2star_debug_extract.json").exists());
    }
}

/// Wolf CommonEvent round-trip — locks the Phase 1 regression at the command
/// level: open → translate one `wolf_common_events` segment → export → the
/// marker is present in the archive's `Data/BasicData/CommonEvent.dat`.
#[tokio::test]
async fn wolf_common_events_open_translate_export_round_trip() {
    let fixture = test_dir().join("月咲流ホノカver1.03");
    if !fixture.exists() {
        return;
    }

    // BasicData holds CommonEvent.dat + databases; MapData is optional but cheap.
    let tmp = tempfile::tempdir().unwrap();
    let game_root = tmp.path().join("game");
    let data_src = fixture.join("Data");
    let data_dst = game_root.join("Data");
    copy_tree(&data_src.join("BasicData"), &data_dst.join("BasicData"));
    if data_src.join("MapData").exists() {
        copy_tree(&data_src.join("MapData"), &data_dst.join("MapData"));
    }
    // The detector requires a launcher marker at the game root.
    std::fs::write(game_root.join("Game.ini"), b"").unwrap();

    let db_file = tmp.path().join("h2s.db");
    let app = mock_app(db_file.to_str().unwrap()).await;

    let project = open_project(
        game_root.to_str().unwrap().to_string(),
        None,
        None,
        app.state(),
    )
    .await
    .expect("open_project")
    .project;
    assert_eq!(project.engine, "wolf");

    let files = get_source_files(project.id.clone(), app.state())
        .await
        .expect("get_source_files");
    let ce_file = files
        .iter()
        .find(|f| f.file_type == "wolf_common_events")
        .expect("a wolf_common_events source file");

    let page = get_segments(
        project.id.clone(),
        ce_file.id.clone(),
        0,
        100_000,
        app.state(),
    )
    .await
    .expect("get_segments");

    assert!(page.items.iter().all(|segment| {
        segment.segment_kind == "unknown"
            && segment.scene_id.is_none()
            && segment.sequence_index.is_none()
            && segment.speaker.is_none()
            && segment.branch_path.is_none()
            && segment.context_json.is_none()
    }));

    // Translate two segments in TWO DISTINCT common events, each with its own
    // marker, and require BOTH to survive the export. This is what locks Phase 1:
    // the pre-fix bucketing keyed on `CommonEvents/{event_name}` (one bucket per
    // event), and every bucket wrote to the same `CommonEvent.dat` zip path — so
    // the last-written event won and every other event's translations were
    // dropped. A single-event translation would not reproduce that overwrite;
    // two events in distinct buckets does, regardless of iteration order.
    //
    // `event_group` mirrors the pre-fix bucket granularity: the first two
    // '/'-separated components of the key (`CommonEvents/{event_name}`).
    fn event_group(json_key: &str) -> String {
        json_key
            .splitn(3, '/')
            .take(2)
            .collect::<Vec<_>>()
            .join("/")
    }
    let seg_a = page.items.first().expect("at least one CE segment");
    let group_a = event_group(&seg_a.json_key);
    let seg_b = page
        .items
        .iter()
        .find(|s| event_group(&s.json_key) != group_a)
        .expect("Honoka CommonEvent.dat must contain at least two distinct common events");

    const MARKER_A: &str = "H2S_E2E_MARKER_A";
    const MARKER_B: &str = "H2S_E2E_MARKER_B";
    update_segment(seg_a.id.clone(), MARKER_A.to_string(), app.state())
        .await
        .expect("update_segment A");
    update_segment(seg_b.id.clone(), MARKER_B.to_string(), app.state())
        .await
        .expect("update_segment B");

    let zip_path = export_project(project.id.clone(), None, true, app.state())
        .await
        .expect("export_project");

    let ce = "Data/BasicData/CommonEvent.dat";
    let zip = Path::new(&zip_path);
    // BOTH markers must be present — the pre-fix overwrite would keep only one.
    assert!(
        zip_entry_contains(zip, ce, MARKER_A.as_bytes()),
        "exported CommonEvent.dat must contain event A's translation (Phase 1 lock)"
    );
    assert!(
        zip_entry_contains(zip, ce, MARKER_B.as_bytes()),
        "exported CommonEvent.dat must contain event B's translation (Phase 1 lock)"
    );
}
