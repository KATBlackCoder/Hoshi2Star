//! Synthetic end-to-end terminology workflow on a disposable MV fixture.
//! Real extraction, Lindera analysis, SQLite, QA and ZIP injection are used;
//! only provider responses are deterministic and local.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use hoshi2star_lib::commands::export::export_project;
use hoshi2star_lib::commands::project::{delete_project, open_project};
use hoshi2star_lib::core::report;
use hoshi2star_lib::core::terminology::repository;
use hoshi2star_lib::core::terminology::translator;
use hoshi2star_lib::core::terminology::types::{Enforcement, UpsertTranslationInput};
use hoshi2star_lib::db;
use hoshi2star_lib::domain::types::ResourceProfile;
use hoshi2star_lib::llm::pipeline;
use hoshi2star_lib::llm::provider::{
    LlmError, LlmProvider, PromptContextPolicy, TranslationContext,
};
use hoshi2star_lib::state::AppState;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::Manager;

struct WorkflowProvider {
    term_targets: BTreeMap<String, String>,
    chat_calls: AtomicUsize,
    translate_calls: AtomicUsize,
    translation_contexts: Mutex<Vec<TranslationContext>>,
}

impl WorkflowProvider {
    fn new(term_targets: BTreeMap<String, String>) -> Self {
        Self {
            term_targets,
            chat_calls: AtomicUsize::new(0),
            translate_calls: AtomicUsize::new(0),
            translation_contexts: Mutex::new(Vec::new()),
        }
    }
}

impl LlmProvider for WorkflowProvider {
    async fn translate(
        &self,
        segments: Vec<String>,
        context: TranslationContext,
    ) -> Result<Vec<String>, LlmError> {
        self.translate_calls.fetch_add(1, Ordering::Relaxed);
        let target = context
            .terminology_hints
            .first()
            .map(|hint| hint.target.clone())
            .ok_or_else(|| LlmError::ResponseFormat("expected an exact terminology hint".into()))?;
        self.translation_contexts.lock().unwrap().push(context);
        Ok(segments.into_iter().map(|_| target.clone()).collect())
    }

    async fn health_check(&self) -> Result<(), LlmError> {
        Ok(())
    }

    async fn chat(&self, _system: &str, _user: &str) -> Result<String, LlmError> {
        self.chat_calls.fetch_add(1, Ordering::Relaxed);
        serde_json::to_string(
            &self
                .term_targets
                .iter()
                .map(|(id, target)| {
                    serde_json::json!({"id": id, "target": target, "confidence": 0.99})
                })
                .collect::<Vec<_>>(),
        )
        .map_err(|error| LlmError::ResponseFormat(error.to_string()))
    }
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mv_mz/mv")
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

async fn mock_app(database: &Path) -> tauri::App<tauri::test::MockRuntime> {
    let pool = db::pool::init(database.to_str().unwrap()).await.unwrap();
    mock_builder()
        .manage(AppState::new(pool).unwrap())
        .build(mock_context(noop_assets()))
        .unwrap()
}

async fn engine_term(
    pool: &sqlx::SqlitePool,
    project_id: &str,
    canonical: &str,
    semantic_type: &str,
) -> (String, String) {
    sqlx::query_as(
        "SELECT DISTINCT entry.id, occurrence.segment_id \
         FROM terminology_entries entry \
         JOIN terminology_occurrences occurrence ON occurrence.entry_id = entry.id \
         WHERE occurrence.project_id = ? AND entry.canonical_text = ? \
           AND entry.semantic_type = ? AND entry.origin = 'engine' LIMIT 1",
    )
    .bind(project_id)
    .bind(canonical)
    .bind(semantic_type)
    .fetch_one(pool)
    .await
    .unwrap()
}

fn assert_valid_export(zip_path: &Path, expected_target: &str) {
    let file = fs::File::open(zip_path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut actor_json = String::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).unwrap();
        if entry.name().ends_with("Actors.json") {
            entry.read_to_string(&mut actor_json).unwrap();
            break;
        }
    }
    let actors: serde_json::Value = serde_json::from_str(&actor_json).unwrap();
    assert_eq!(
        actors.pointer("/1/name").and_then(|value| value.as_str()),
        Some(expected_target)
    );
}

async fn run_target(target_language: &str, character_target: &str, item_target: &str) {
    let workspace = tempfile::tempdir().unwrap();
    let game = workspace.path().join(format!("game-{target_language}"));
    copy_tree(&fixture_root(), &game);
    let app = mock_app(&workspace.path().join("workflow.db")).await;
    let opened = open_project(
        game.to_string_lossy().into_owned(),
        Some("ja".into()),
        Some(target_language.into()),
        app.state(),
    )
    .await
    .unwrap();
    let project_id = opened.project.id;
    let pool = &app.state::<AppState>().db;

    let scan = app
        .state::<AppState>()
        .terminology
        .scan_project(pool, &project_id, None)
        .await
        .unwrap();
    assert!(scan.analyzed > 0);
    let semantic_types: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT entry.semantic_type FROM terminology_entries entry \
         JOIN terminology_occurrences occurrence ON occurrence.entry_id = entry.id \
         WHERE occurrence.project_id = ? ORDER BY entry.semantic_type",
    )
    .bind(&project_id)
    .fetch_all(pool)
    .await
    .unwrap();
    assert!(semantic_types.iter().any(|value| value == "character"));
    assert!(semantic_types.iter().any(|value| value == "item"));

    let character = engine_term(pool, &project_id, "勇者アオイ", "character").await;
    let item = engine_term(pool, &project_id, "回復薬", "item").await;
    let provider = WorkflowProvider::new(BTreeMap::from([
        (character.0.clone(), character_target.to_string()),
        (item.0.clone(), item_target.to_string()),
    ]));
    assert_eq!(provider.chat_calls.load(Ordering::Relaxed), 0);
    assert_eq!(provider.translate_calls.load(Ordering::Relaxed), 0);

    let summary = translator::translate_selected(
        pool,
        &provider,
        &[character.0.clone(), item.0.clone()],
        target_language,
        Some(&project_id),
        false,
        ResourceProfile::Fast,
        "e2e-local",
        "deterministic",
        None,
    )
    .await
    .unwrap();
    assert_eq!((summary.requested, summary.translated), (2, 2));
    assert_eq!(provider.chat_calls.load(Ordering::Relaxed), 1);

    for (entry_id, target_text, enforcement) in [
        (&character.0, character_target, Enforcement::Required),
        (&item.0, item_target, Enforcement::Preferred),
    ] {
        repository::upsert_translation(
            pool,
            &UpsertTranslationInput {
                entry_id: entry_id.clone(),
                target_language: target_language.into(),
                project_id: Some(project_id.clone()),
                target_text: target_text.into(),
                enforcement,
                confidence: 1.0,
                provider_id: Some("e2e-local".into()),
                model: Some("deterministic".into()),
                accepted_variants: Vec::new(),
            },
        )
        .await
        .unwrap();
    }

    let source_text: String = sqlx::query_scalar("SELECT source_text FROM segments WHERE id = ?")
        .bind(&character.1)
        .fetch_one(pool)
        .await
        .unwrap();
    let results = pipeline::run_inner(
        vec![(character.1.clone(), source_text)],
        &provider,
        TranslationContext {
            source_lang: "ja".into(),
            target_lang: target_language.into(),
            terminology_hints: Vec::new(),
            engine: "mv_mz".into(),
            batch_size: 4,
            batch_delay_ms: 0,
            prompt_context_policy: PromptContextPolicy::EngineOwned,
            segment_contexts: Vec::new(),
        },
        pool,
        None,
        None,
        None,
        |_, _| {},
    )
    .await
    .unwrap();
    assert_eq!(results[0].translated_text, character_target);
    {
        let contexts = provider.translation_contexts.lock().unwrap();
        assert_eq!(contexts.len(), 1);
        assert!(contexts[0]
            .terminology_hints
            .iter()
            .any(|hint| hint.source == "勇者アオイ" && hint.target == character_target));
        assert!(contexts[0].terminology_hints.len() <= 20);
    }

    let (qa, _) = report::preview_project(pool, &project_id).await.unwrap();
    assert_eq!(qa.critical_count, 0);
    assert!(qa.terminology_issues.is_empty());
    let zip_path = export_project(project_id.clone(), None, false, app.state())
        .await
        .unwrap();
    assert_valid_export(Path::new(&zip_path), character_target);

    delete_project(project_id.clone(), app.state())
        .await
        .unwrap();
    let surviving_source: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM terminology_entries WHERE id = ?")
            .bind(&character.0)
            .fetch_one(pool)
            .await
            .unwrap();
    let local_occurrences: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM terminology_occurrences WHERE project_id = ?")
            .bind(&project_id)
            .fetch_one(pool)
            .await
            .unwrap();
    let local_translations: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM terminology_translations WHERE project_id = ?")
            .bind(&project_id)
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(surviving_source, 1);
    assert_eq!((local_occurrences, local_translations), (0, 0));
}

#[tokio::test]
async fn mv_terminology_workflow_is_operational_for_english_and_french() {
    run_target("en", "Hero Aoi", "Potion").await;
    run_target("fr", "Héros Aoi", "Potion de soin").await;
}
