use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use hoshi2star_lib::commands::terminology::{
    archive_terminology_entry, cancel_terminology_scan, create_terminology_entry,
    get_terminology_stats, list_terminology, start_terminology_scan_with_app,
    upsert_terminology_translation, SCAN_DONE_EVENT,
};
use hoshi2star_lib::core::terminology::analyzer::{LinguisticToken, MorphologicalAnalyzer};
use hoshi2star_lib::core::terminology::service::TerminologyService;
use hoshi2star_lib::core::terminology::types::{
    CreateTermInput, Enforcement, EntryStatus, PartOfSpeech, ReviewStatus, TerminologyQuery,
    UpsertTranslationInput,
};
use hoshi2star_lib::core::terminology::Result;
use hoshi2star_lib::db;
use hoshi2star_lib::state::AppState;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::{Listener, Manager};

struct FakeAnalyzer {
    delay_ms: u64,
    calls: AtomicUsize,
}

impl MorphologicalAnalyzer for FakeAnalyzer {
    fn version(&self) -> &str {
        "command-fake-v1"
    }

    fn analyze(&self, text: &str) -> Result<Vec<LinguisticToken>> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if self.delay_ms > 0 {
            std::thread::sleep(Duration::from_millis(self.delay_ms));
        }
        Ok((!text.trim().is_empty())
            .then(|| LinguisticToken {
                surface: text.trim().to_string(),
                lemma: text.trim().to_string(),
                reading: None,
                part_of_speech: PartOfSpeech::Noun,
                pos_detail: None,
                conjugation: None,
                byte_start: 0,
                byte_end: text.trim().len(),
                is_unknown: false,
            })
            .into_iter()
            .collect())
    }
}

async fn mock_app(delay_ms: u64) -> (tempfile::TempDir, tauri::App<tauri::test::MockRuntime>) {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("commands.db");
    let pool = db::pool::init(database.to_str().unwrap()).await.unwrap();
    let service = Arc::new(TerminologyService::new(
        Arc::new(FakeAnalyzer {
            delay_ms,
            calls: AtomicUsize::new(0),
        }),
        1,
    ));
    let app = mock_builder()
        .manage(AppState::with_terminology(pool, service))
        .build(mock_context(noop_assets()))
        .unwrap();
    (directory, app)
}

async fn seed_project(app: &tauri::App<tauri::test::MockRuntime>, segment_count: usize) {
    let db = &app.state::<AppState>().db;
    sqlx::query(
        "INSERT INTO projects (\
            id, name, engine, game_path, source_language, target_language\
         ) VALUES ('p1', 'Game', 'mv_mz', '/tmp/game', 'ja', 'en')",
    )
    .execute(db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
         VALUES ('f1', 'p1', 'Map001.json', '/tmp/Map001.json', 'map')",
    )
    .execute(db)
    .await
    .unwrap();
    for index in 0..segment_count {
        sqlx::query(
            "INSERT INTO segments (id, source_file_id, json_key, source_text, segment_kind) \
             VALUES (?, 'f1', ?, ?, 'dialogue')",
        )
        .bind(format!("s{index:04}"))
        .bind(format!("/{index}"))
        .bind(format!("勇者{index}"))
        .execute(db)
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn commands_validate_pagination_and_apply_project_translation_override() {
    let (_directory, app) = mock_app(0).await;
    seed_project(&app, 0).await;
    let entry = create_terminology_entry(
        CreateTermInput {
            source_language: "ja".into(),
            canonical_text: "勇者".into(),
            reading: Some("ユウシャ".into()),
            part_of_speech: PartOfSpeech::ProperNoun,
            semantic_type: "character".into(),
            sense_key: String::new(),
        },
        app.state(),
    )
    .await
    .unwrap();
    upsert_terminology_translation(
        UpsertTranslationInput {
            entry_id: entry.id.clone(),
            target_language: "en".into(),
            project_id: None,
            target_text: "Hero".into(),
            review_status: ReviewStatus::Approved,
            enforcement: Enforcement::Required,
            confidence: 1.0,
            provider_id: None,
            model: None,
            accepted_variants: vec!["The Hero".into()],
        },
        app.state(),
    )
    .await
    .unwrap();
    upsert_terminology_translation(
        UpsertTranslationInput {
            entry_id: entry.id.clone(),
            target_language: "en".into(),
            project_id: Some("p1".into()),
            target_text: "Champion".into(),
            review_status: ReviewStatus::Locked,
            enforcement: Enforcement::Required,
            confidence: 1.0,
            provider_id: None,
            model: None,
            accepted_variants: Vec::new(),
        },
        app.state(),
    )
    .await
    .unwrap();

    let page = list_terminology(
        TerminologyQuery {
            project_id: Some("p1".into()),
            status: Some(EntryStatus::Active),
            ..TerminologyQuery::default()
        },
        app.state(),
    )
    .await
    .unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(
        page.items[0].translation.as_ref().unwrap().target_text,
        "Champion"
    );
    let stats = get_terminology_stats("ja".into(), "en".into(), Some("p1".into()), app.state())
        .await
        .unwrap();
    assert_eq!(stats.locked_translations, 1);

    let invalid = list_terminology(
        TerminologyQuery {
            page_size: 201,
            ..TerminologyQuery::default()
        },
        app.state(),
    )
    .await
    .unwrap_err();
    assert!(invalid.contains("page size must be between 1 and 200"));
    archive_terminology_entry(entry.id, app.state())
        .await
        .unwrap();
}

#[tokio::test]
async fn scan_command_returns_immediately_emits_done_and_can_be_cancelled() {
    let (_directory, app) = mock_app(2).await;
    seed_project(&app, 260).await;
    let done_payloads = Arc::new(Mutex::new(Vec::<String>::new()));
    let captured = done_payloads.clone();
    app.listen(SCAN_DONE_EVENT, move |event| {
        captured.lock().unwrap().push(event.payload().to_string());
    });

    let started = std::time::Instant::now();
    let response = start_terminology_scan_with_app(
        "p1".into(),
        app.handle().clone(),
        app.state::<AppState>().inner(),
    )
    .await
    .unwrap();
    assert!(started.elapsed() < Duration::from_millis(100));
    assert!(
        cancel_terminology_scan(response.scan_id.clone(), app.state())
            .await
            .unwrap()
    );

    for _ in 0..200 {
        if !done_payloads.lock().unwrap().is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let payloads = done_payloads.lock().unwrap();
    assert_eq!(payloads.len(), 1);
    let done: serde_json::Value = serde_json::from_str(&payloads[0]).unwrap();
    assert_eq!(done["scanId"], response.scan_id);
    assert_eq!(done["projectId"], "p1");
    assert_eq!(done["status"], "cancelled");
    assert!(done.get("tokens").is_none());
}

#[test]
fn command_inputs_use_camel_case_and_explicit_languages() {
    let query: TerminologyQuery = serde_json::from_value(serde_json::json!({
        "sourceLanguage": "ja",
        "targetLanguage": "fr",
        "projectId": "p1",
        "page": 0,
        "pageSize": 25
    }))
    .unwrap();
    assert_eq!(query.source_language, "ja");
    assert_eq!(query.target_language, "fr");
    assert_eq!(query.project_id.as_deref(), Some("p1"));
}
