//! Opt-in integration benchmark for the private StandGirl MV/MZ pilot.
//!
//! The source path is never opened as a Hoshi2Star project. The test first
//! copies it to a temporary directory, then uses a disposable SQLite database.
//! Run with `H2S_STANDGIRL_PATH=/path/to/game cargo test --test
//! terminology_real_pilot -- --ignored --nocapture`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use hoshi2star_lib::commands::project::open_project;
use hoshi2star_lib::db;
use hoshi2star_lib::state::AppState;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::Manager;

const EXPECTED_SEGMENTS: i64 = 1_191;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PilotMetrics {
    extracted_segments: i64,
    cold_scan_ms: u128,
    warm_scan_ms: u128,
    cold_analyzed: i64,
    warm_analyzed: i64,
    warm_skipped: i64,
    entries: i64,
    occurrences: i64,
    by_part_of_speech: BTreeMap<String, i64>,
    by_semantic_type: BTreeMap<String, i64>,
    database_bytes: u64,
    rss_before_kib: Option<u64>,
    rss_after_kib: Option<u64>,
    rss_delta_kib: Option<i64>,
    idle_cpu_ticks_500ms: Option<u64>,
    segment_fingerprint_preserved: bool,
    active_scan_after_completion: bool,
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&source_path, &destination_path);
        } else {
            fs::copy(source_path, destination_path).unwrap();
        }
    }
}

fn rss_kib() -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    status.lines().find_map(|line| {
        line.strip_prefix("VmRSS:")?
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    })
}

fn cpu_ticks() -> Option<u64> {
    let stat = fs::read_to_string("/proc/self/stat").ok()?;
    let fields = stat
        .rsplit_once(") ")?
        .1
        .split_whitespace()
        .collect::<Vec<_>>();
    let user: u64 = fields.get(11)?.parse().ok()?;
    let system: u64 = fields.get(12)?.parse().ok()?;
    Some(user + system)
}

async fn segment_fingerprint(pool: &sqlx::SqlitePool, project_id: &str) -> String {
    let rows = sqlx::query_as::<_, (String, String, String)>(
        "SELECT segment.id, segment.json_key, segment.source_text \
         FROM segments segment JOIN source_files file ON file.id = segment.source_file_id \
         WHERE file.project_id = ? ORDER BY segment.id",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
    .unwrap();
    let mut digest = Sha256::new();
    for (id, json_key, source_text) in rows {
        digest.update(id.as_bytes());
        digest.update([0x1f]);
        digest.update(json_key.as_bytes());
        digest.update([0x1f]);
        digest.update(source_text.as_bytes());
        digest.update([0x1e]);
    }
    hex::encode(digest.finalize())
}

async fn grouped_counts(
    pool: &sqlx::SqlitePool,
    project_id: &str,
    column: &str,
) -> BTreeMap<String, i64> {
    let sql = format!(
        "SELECT entry.{column}, COUNT(DISTINCT entry.id) \
         FROM terminology_entries entry \
         JOIN terminology_occurrences occurrence ON occurrence.entry_id = entry.id \
         WHERE occurrence.project_id = ? GROUP BY entry.{column} ORDER BY entry.{column}"
    );
    sqlx::query_as::<_, (String, i64)>(&sql)
        .bind(project_id)
        .fetch_all(pool)
        .await
        .unwrap()
        .into_iter()
        .collect()
}

#[tokio::test]
#[ignore = "requires the private StandGirl pilot path via H2S_STANDGIRL_PATH"]
async fn standgirl_scan_meets_real_project_gates_on_a_disposable_copy() {
    let source = PathBuf::from(
        std::env::var("H2S_STANDGIRL_PATH")
            .expect("set H2S_STANDGIRL_PATH to the private StandGirl game directory"),
    );
    assert!(source.is_dir());

    let workspace = tempfile::tempdir().unwrap();
    let game = workspace.path().join("game-copy");
    copy_tree(&source, &game);

    let database = workspace.path().join("pilot.db");
    let pool = db::pool::init(database.to_str().unwrap()).await.unwrap();
    let app = mock_builder()
        .manage(AppState::new(pool).unwrap())
        .build(mock_context(noop_assets()))
        .unwrap();
    let state = app.state::<AppState>();

    let opened = open_project(
        game.to_string_lossy().into_owned(),
        Some("ja".into()),
        Some("en".into()),
        state.clone(),
    )
    .await
    .unwrap();
    let project_id = opened.project.id;
    let extracted_segments: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM segments segment JOIN source_files file \
         ON file.id = segment.source_file_id WHERE file.project_id = ?",
    )
    .bind(&project_id)
    .fetch_one(&state.db)
    .await
    .unwrap();
    assert_eq!(extracted_segments, EXPECTED_SEGMENTS);
    let fingerprint_before = segment_fingerprint(&state.db, &project_id).await;
    let rss_before = rss_kib();

    let cold_started = Instant::now();
    let cold = state
        .terminology
        .scan_project(&state.db, &project_id, None)
        .await
        .unwrap();
    let cold_scan_ms = cold_started.elapsed().as_millis();

    let warm_started = Instant::now();
    let warm = state
        .terminology
        .scan_project(&state.db, &project_id, None)
        .await
        .unwrap();
    let warm_scan_ms = warm_started.elapsed().as_millis();
    let rss_after = rss_kib();
    let fingerprint_after = segment_fingerprint(&state.db, &project_id).await;

    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&state.db)
        .await
        .unwrap();
    let entries: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT entry.id) FROM terminology_entries entry \
         JOIN terminology_occurrences occurrence ON occurrence.entry_id = entry.id \
         WHERE occurrence.project_id = ?",
    )
    .bind(&project_id)
    .fetch_one(&state.db)
    .await
    .unwrap();
    let occurrences: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM terminology_occurrences WHERE project_id = ?")
            .bind(&project_id)
            .fetch_one(&state.db)
            .await
            .unwrap();

    let cpu_before_idle = cpu_ticks();
    tokio::time::sleep(Duration::from_millis(500)).await;
    let cpu_after_idle = cpu_ticks();
    let idle_cpu_ticks_500ms = cpu_before_idle
        .zip(cpu_after_idle)
        .map(|(before, after)| after.saturating_sub(before));
    let active_scan_after_completion = state
        .terminology
        .active_scan_id(&project_id)
        .await
        .is_some();

    let metrics = PilotMetrics {
        extracted_segments,
        cold_scan_ms,
        warm_scan_ms,
        cold_analyzed: cold.analyzed,
        warm_analyzed: warm.analyzed,
        warm_skipped: warm.skipped,
        entries,
        occurrences,
        by_part_of_speech: grouped_counts(&state.db, &project_id, "part_of_speech").await,
        by_semantic_type: grouped_counts(&state.db, &project_id, "semantic_type").await,
        database_bytes: fs::metadata(&database).unwrap().len(),
        rss_before_kib: rss_before,
        rss_after_kib: rss_after,
        rss_delta_kib: rss_before
            .zip(rss_after)
            .map(|(before, after)| after as i64 - before as i64),
        idle_cpu_ticks_500ms,
        segment_fingerprint_preserved: fingerprint_before == fingerprint_after,
        active_scan_after_completion,
    };

    println!("{}", serde_json::to_string_pretty(&metrics).unwrap());
    assert!(cold_scan_ms <= 5_000, "cold scan took {cold_scan_ms} ms");
    assert!(warm_scan_ms <= 1_000, "warm scan took {warm_scan_ms} ms");
    assert_eq!(cold.analyzed, EXPECTED_SEGMENTS);
    assert_eq!((warm.analyzed, warm.skipped), (0, EXPECTED_SEGMENTS));
    assert_eq!(fingerprint_before, fingerprint_after);
    assert!(!active_scan_after_completion);
    if let (Some(before), Some(after)) = (rss_before, rss_after) {
        assert!(
            after.saturating_sub(before) <= 180 * 1_024,
            "RSS delta exceeded 180 MiB"
        );
    }
}
