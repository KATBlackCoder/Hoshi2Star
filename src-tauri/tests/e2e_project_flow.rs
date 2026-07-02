//! End-to-end integration tests exercising the real Tauri command layer
//! (`open_project` → `update_segment` → `export_project`) against decrypted
//! game fixtures in `test/`.
//!
//! The fixtures are gitignored, so each test returns early when its fixture is
//! absent (fresh checkout / CI). The flow is driven through the actual
//! `#[tauri::command]` functions — `mock_builder` only holds the managed
//! `AppState`; commands are called directly with a `State` from `Manager`.
//!
//! An ASCII marker is used as the translation: ASCII bytes are identical under
//! both UTF-8 (MV/MZ JSON) and Shift-JIS (Wolf v2 `.dat`), so the marker can be
//! located as a raw byte substring in the exported archive without depending on
//! any crate-internal (`pub(crate)`) parser.

use std::io::Read;
use std::path::{Path, PathBuf};

use hoshi2star_lib::commands::export::export_project;
use hoshi2star_lib::commands::project::{
    get_segments, get_source_files, open_project, update_segment,
};
use hoshi2star_lib::db;
use hoshi2star_lib::state::AppState;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::Manager;

const MARKER: &str = "H2S_E2E_MARKER";

/// Repo `test/` directory (sibling of `src-tauri/`).
fn test_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("test")
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

/// Build a mock Tauri app whose managed state wraps a fresh migrated DB.
async fn mock_app(db_path: &str) -> tauri::App<tauri::test::MockRuntime> {
    let pool = db::pool::init(db_path).await.expect("pool init");
    mock_builder()
        .manage(AppState { db: pool })
        .build(mock_context(noop_assets()))
        .expect("mock app build")
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
    let fixture = test_dir().join("性処理係のある学校");
    if !fixture.exists() {
        return;
    }

    // Copy only www/data (the assets under www/ are ~600 MB and unused).
    let tmp = tempfile::tempdir().unwrap();
    let game_root = tmp.path().join("game");
    copy_tree(
        &fixture.join("www").join("data"),
        &game_root.join("www").join("data"),
    );

    let db_file = tmp.path().join("h2s.db");
    let app = mock_app(db_file.to_str().unwrap()).await;

    let project = open_project(game_root.to_str().unwrap().to_string(), app.state())
        .await
        .expect("open_project")
        .project;
    assert_eq!(project.engine, "mv_mz");

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

    let project = open_project(game_root.to_str().unwrap().to_string(), app.state())
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
