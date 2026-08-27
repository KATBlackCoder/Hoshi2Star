use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use hoshi2star_lib::core::terminology::analyzer::{LinguisticToken, MorphologicalAnalyzer};
use hoshi2star_lib::core::terminology::scanner::{ScanEvent, ScanEventSink};
use hoshi2star_lib::core::terminology::service::TerminologyService;
use hoshi2star_lib::core::terminology::types::PartOfSpeech;
use hoshi2star_lib::core::terminology::{Result, TerminologyError};
use hoshi2star_lib::db;
use sqlx::SqlitePool;

struct FakeAnalyzer {
    calls: AtomicUsize,
    slow: AtomicBool,
}

impl FakeAnalyzer {
    fn new() -> Self {
        Self {
            calls: AtomicUsize::new(0),
            slow: AtomicBool::new(false),
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::Relaxed)
    }
}

impl MorphologicalAnalyzer for FakeAnalyzer {
    fn version(&self) -> &str {
        "fake-ja-v1"
    }

    fn analyze(&self, text: &str) -> Result<Vec<LinguisticToken>> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if self.slow.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(1));
        }
        if text.contains("FAIL") {
            return Err(TerminologyError::Analyzer("intentional failure".into()));
        }
        Ok(text
            .split_whitespace()
            .filter(|word| !word.is_empty())
            .map(|word| LinguisticToken {
                surface: word.to_string(),
                lemma: word.to_string(),
                reading: None,
                part_of_speech: if word.ends_with('る') {
                    PartOfSpeech::Verb
                } else {
                    PartOfSpeech::Noun
                },
                pos_detail: None,
                conjugation: None,
                byte_start: 0,
                byte_end: word.len(),
                is_unknown: false,
            })
            .collect())
    }
}

async fn test_pool() -> (tempfile::TempDir, SqlitePool) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("terminology.db");
    let pool = db::pool::init(path.to_str().unwrap()).await.unwrap();
    (directory, pool)
}

async fn seed_project(pool: &SqlitePool, project_id: &str) {
    sqlx::query(
        "INSERT INTO projects (\
            id, name, engine, game_path, source_language, target_language\
         ) VALUES (?, 'Game', 'mv_mz', '/tmp/game', 'ja', 'en')",
    )
    .bind(project_id)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
         VALUES (?, ?, 'Map001.json', '/tmp/Map001.json', 'map')",
    )
    .bind(format!("file-{project_id}"))
    .bind(project_id)
    .execute(pool)
    .await
    .unwrap();
}

async fn insert_segment(
    pool: &SqlitePool,
    project_id: &str,
    id: &str,
    source_text: &str,
    segment_kind: &str,
) {
    sqlx::query(
        "INSERT INTO segments (\
            id, source_file_id, json_key, source_text, segment_kind\
         ) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(format!("file-{project_id}"))
    .bind(format!("/{id}"))
    .bind(source_text)
    .bind(segment_kind)
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn scan_is_incremental_merges_occurrences_and_preserves_engine_semantics() {
    let (_directory, pool) = test_pool().await;
    seed_project(&pool, "p1").await;
    insert_segment(&pool, "p1", "s1", "勇者 勇者 走る", "dialogue").await;
    insert_segment(&pool, "p1", "s2", "勇者", "actor_name").await;
    let analyzer = Arc::new(FakeAnalyzer::new());
    let service = TerminologyService::new(analyzer.clone(), 1);

    let first = service.scan_project(&pool, "p1", None).await.unwrap();
    assert_eq!((first.total, first.analyzed, first.skipped), (2, 2, 0));
    assert_eq!(first.discovered, 3);
    assert_eq!(analyzer.calls(), 2);

    let occurrences: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT entry.semantic_type, occurrence.surface_text, occurrence.occurrence_count \
         FROM terminology_occurrences occurrence \
         JOIN terminology_entries entry ON entry.id = occurrence.entry_id \
         ORDER BY entry.semantic_type, occurrence.surface_text",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(occurrences.contains(&("general".into(), "勇者".into(), 2)));
    assert!(occurrences.contains(&("character".into(), "勇者".into(), 1)));

    let second = service.scan_project(&pool, "p1", None).await.unwrap();
    assert_eq!((second.analyzed, second.skipped), (0, 2));
    assert_eq!(analyzer.calls(), 2, "unchanged scan must not call analyzer");

    sqlx::query("UPDATE segments SET source_text = '魔王 走る' WHERE id = 's1'")
        .execute(&pool)
        .await
        .unwrap();
    let changed = service.scan_project(&pool, "p1", None).await.unwrap();
    assert_eq!((changed.analyzed, changed.skipped), (1, 1));
    assert_eq!(analyzer.calls(), 3);
    let old_general_occurrences: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM terminology_occurrences occurrence \
         JOIN terminology_entries entry ON entry.id = occurrence.entry_id \
         WHERE entry.semantic_type = 'general' AND entry.canonical_text = '勇者'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(old_general_occurrences, 0);

    sqlx::query("DELETE FROM segments WHERE id = 's2'")
        .execute(&pool)
        .await
        .unwrap();
    let character_entries: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM terminology_entries \
         WHERE semantic_type = 'character' AND canonical_text = '勇者'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let character_occurrences: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM terminology_occurrences occurrence \
         JOIN terminology_entries entry ON entry.id = occurrence.entry_id \
         WHERE entry.semantic_type = 'character' AND entry.canonical_text = '勇者'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((character_entries, character_occurrences), (1, 0));
}

#[tokio::test]
async fn analyzer_failure_is_recorded_and_does_not_leave_project_locked() {
    let (_directory, pool) = test_pool().await;
    seed_project(&pool, "failure").await;
    insert_segment(&pool, "failure", "failure-s1", "FAIL", "dialogue").await;
    let analyzer = Arc::new(FakeAnalyzer::new());
    let service = TerminologyService::new(analyzer, 1);

    let error = service
        .scan_project(&pool, "failure", None)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("intentional failure"));
    assert!(service.active_scan_id("failure").await.is_none());
    let recorded: (String, Option<String>) =
        sqlx::query_as("SELECT status, error FROM terminology_scans WHERE project_id = 'failure'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(recorded.0, "failed");
    assert!(recorded.1.unwrap().contains("intentional failure"));
}

#[tokio::test]
async fn scan_can_be_cancelled_resumed_and_rejects_same_project_concurrency() {
    let (_directory, pool) = test_pool().await;
    seed_project(&pool, "large").await;
    for index in 0..260 {
        insert_segment(
            &pool,
            "large",
            &format!("segment-{index:04}"),
            &format!("勇者{index}"),
            "dialogue",
        )
        .await;
    }
    let analyzer = Arc::new(FakeAnalyzer::new());
    analyzer.slow.store(true, Ordering::Relaxed);
    let service = Arc::new(TerminologyService::new(analyzer.clone(), 1));
    let scan_service = service.clone();
    let scan_pool = pool.clone();
    let scan = tokio::spawn(async move {
        scan_service
            .scan_project(&scan_pool, "large", None)
            .await
            .unwrap()
    });

    let scan_id = loop {
        if let Some(scan_id) = service.active_scan_id("large").await {
            break scan_id;
        }
        tokio::task::yield_now().await;
    };
    let concurrent = service
        .scan_project(&pool, "large", None)
        .await
        .unwrap_err();
    assert!(concurrent.to_string().contains("already running"));
    while analyzer.calls() < 250 {
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    assert!(service.cancel_scan(&scan_id).await);
    let cancelled = scan.await.unwrap();
    assert_eq!(
        cancelled.status,
        hoshi2star_lib::core::terminology::scanner::ScanStatus::Cancelled
    );
    assert!(cancelled.processed <= 260);

    analyzer.slow.store(false, Ordering::Relaxed);
    let resumed = service.scan_project(&pool, "large", None).await.unwrap();
    assert_eq!(
        resumed.status,
        hoshi2star_lib::core::terminology::scanner::ScanStatus::Completed
    );
    assert_eq!(resumed.processed, 260);
    let scan_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM terminology_segment_scans WHERE project_id = 'large'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(scan_rows, 260);
}

#[tokio::test]
async fn regular_segment_edits_remain_responsive_during_cpu_heavy_scan() {
    let (_directory, pool) = test_pool().await;
    seed_project(&pool, "responsive").await;
    for index in 0..260 {
        insert_segment(
            &pool,
            "responsive",
            &format!("responsive-{index:04}"),
            "勇者",
            "dialogue",
        )
        .await;
    }
    let analyzer = Arc::new(FakeAnalyzer::new());
    analyzer.slow.store(true, Ordering::Relaxed);
    let service = Arc::new(TerminologyService::new(analyzer.clone(), 1));
    let scan_service = service.clone();
    let scan_pool = pool.clone();
    let scan = tokio::spawn(async move {
        scan_service
            .scan_project(&scan_pool, "responsive", None)
            .await
            .unwrap()
    });
    while analyzer.calls() < 20 {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }

    let started = Instant::now();
    sqlx::query("UPDATE segments SET target_text = 'Hero' WHERE id = 'responsive-0000'")
        .execute(&pool)
        .await
        .unwrap();
    let edit_latency = started.elapsed();
    assert!(
        edit_latency < Duration::from_millis(250),
        "edit was blocked for {edit_latency:?}"
    );

    let completed = scan.await.unwrap();
    assert_eq!(completed.processed, 260);
}

#[tokio::test]
async fn scan_events_are_compact_and_finish_exactly_once() {
    let (_directory, pool) = test_pool().await;
    seed_project(&pool, "events").await;
    insert_segment(&pool, "events", "events-s1", "勇者", "dialogue").await;
    let service = TerminologyService::new(Arc::new(FakeAnalyzer::new()), 1);
    let events = Arc::new(std::sync::Mutex::new(Vec::<ScanEvent>::new()));
    let captured = events.clone();
    let sink: ScanEventSink = Arc::new(move |event| captured.lock().unwrap().push(event));

    service
        .scan_project(&pool, "events", Some(sink))
        .await
        .unwrap();
    let events = events.lock().unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, ScanEvent::Done(_)))
            .count(),
        1
    );
    let serialized = serde_json::to_string(&*events).unwrap();
    assert!(!serialized.contains("canonicalText"));
    assert!(!serialized.contains("tokens"));
    assert!(!serialized.contains("勇者"));
}
