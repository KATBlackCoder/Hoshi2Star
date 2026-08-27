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
use hoshi2star_lib::core::qa::{self, QaSemanticContext};
use hoshi2star_lib::core::terminology::repository;
use hoshi2star_lib::core::terminology::resolver;
use hoshi2star_lib::core::terminology::translator;
use hoshi2star_lib::core::terminology::types::{Enforcement, ReviewStatus, UpsertTranslationInput};
use hoshi2star_lib::db;
use hoshi2star_lib::domain::types::ResourceProfile;
use hoshi2star_lib::llm::provider::{
    LlmProvider, OpenAiCompatibleProvider, PromptContextPolicy, ProviderCallMetrics,
    TranslationContext, DEFAULT_OLLAMA_MODEL, DEFAULT_OLLAMA_URL,
};
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

#[derive(Debug, Clone, sqlx::FromRow)]
struct PilotTerm {
    id: String,
    segment_id: String,
    source_text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OllamaLanguageMetrics {
    target_language: String,
    translated_terms: Vec<(String, String)>,
    term_call: ProviderCallMetrics,
    baseline_call: ProviderCallMetrics,
    hinted_call: ProviderCallMetrics,
    prompt_token_overhead_percent: f64,
    resolved_hints: usize,
    translated_segment: String,
    qa_score: u8,
    qa_has_critical: bool,
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

async fn disposable_project(
    source: &Path,
    target_language: &str,
) -> (
    tempfile::TempDir,
    tauri::App<tauri::test::MockRuntime>,
    String,
    PathBuf,
) {
    let workspace = tempfile::tempdir().unwrap();
    let game = workspace.path().join("game-copy");
    copy_tree(source, &game);
    let database = workspace.path().join("pilot.db");
    let pool = db::pool::init(database.to_str().unwrap()).await.unwrap();
    let app = mock_builder()
        .manage(AppState::new(pool).unwrap())
        .build(mock_context(noop_assets()))
        .unwrap();
    let opened = open_project(
        game.to_string_lossy().into_owned(),
        Some("ja".into()),
        Some(target_language.into()),
        app.state(),
    )
    .await
    .unwrap();
    (workspace, app, opened.project.id, database)
}

fn translation_context(
    target_language: &str,
    hints: Vec<hoshi2star_lib::llm::provider::TerminologyHint>,
) -> TranslationContext {
    TranslationContext {
        source_lang: "ja".into(),
        target_lang: target_language.into(),
        terminology_hints: hints,
        engine: "mv_mz".into(),
        batch_size: 1,
        batch_delay_ms: 0,
        prompt_context_policy: PromptContextPolicy::Disabled,
        segment_contexts: Vec::new(),
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

    let (workspace, app, project_id, database) = disposable_project(&source, "en").await;
    let state = app.state::<AppState>();
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
    drop(workspace);
}

#[tokio::test]
#[ignore = "requires StandGirl and a local Ollama server"]
async fn standgirl_ollama_translates_terms_and_reports_exact_prompt_tokens() {
    let source = PathBuf::from(
        std::env::var("H2S_STANDGIRL_PATH")
            .expect("set H2S_STANDGIRL_PATH to the private StandGirl game directory"),
    );
    let ollama_url =
        std::env::var("H2S_OLLAMA_URL").unwrap_or_else(|_| DEFAULT_OLLAMA_URL.to_string());
    let model =
        std::env::var("H2S_OLLAMA_MODEL").unwrap_or_else(|_| DEFAULT_OLLAMA_MODEL.to_string());
    let (_workspace, app, project_id, _database) = disposable_project(&source, "en").await;
    let state = app.state::<AppState>();
    state
        .terminology
        .scan_project(&state.db, &project_id, None)
        .await
        .unwrap();

    let terms = sqlx::query_as::<_, PilotTerm>(
        "SELECT DISTINCT entry.id, occurrence.segment_id, \
         segment.source_text FROM terminology_entries entry \
         JOIN terminology_occurrences occurrence ON occurrence.entry_id = entry.id \
         JOIN segments segment ON segment.id = occurrence.segment_id \
         WHERE occurrence.project_id = ? AND entry.origin = 'engine' \
           AND entry.semantic_type != 'general' \
           AND trim(segment.source_text) = trim(entry.canonical_text) \
         ORDER BY CASE entry.semantic_type WHEN 'character' THEN 0 WHEN 'item' THEN 1 \
                  WHEN 'skill' THEN 2 ELSE 3 END, entry.canonical_text LIMIT 3",
    )
    .bind(&project_id)
    .fetch_all(&state.db)
    .await
    .unwrap();
    assert_eq!(terms.len(), 3, "expected three stable engine-owned terms");
    let entry_ids = terms.iter().map(|term| term.id.clone()).collect::<Vec<_>>();
    let provider = OpenAiCompatibleProvider::new_for_preset(
        "ollama",
        &ollama_url,
        &model,
        None,
        Duration::from_secs(180),
    );
    provider.health_check().await.unwrap();

    let mut language_reports = Vec::new();
    for target_language in ["en", "fr"] {
        let summary = translator::translate_selected(
            &state.db,
            &provider,
            &entry_ids,
            target_language,
            Some(&project_id),
            ResourceProfile::Fast,
            "ollama",
            &model,
            None,
        )
        .await
        .unwrap();
        assert_eq!((summary.requested, summary.proposed), (3, 3));
        let term_call = provider.drain_metrics().into_iter().next().unwrap();
        assert!(term_call.success);
        assert!(term_call.prompt_tokens.is_some());

        let translated_terms = sqlx::query_as::<_, (String, String)>(
            "SELECT entry.canonical_text, translation.target_text \
             FROM terminology_translations translation \
             JOIN terminology_entries entry ON entry.id = translation.entry_id \
             WHERE translation.project_id = ? AND translation.target_language = ? \
               AND translation.entry_id IN (?, ?, ?) ORDER BY entry.canonical_text",
        )
        .bind(&project_id)
        .bind(target_language)
        .bind(&entry_ids[0])
        .bind(&entry_ids[1])
        .bind(&entry_ids[2])
        .fetch_all(&state.db)
        .await
        .unwrap();
        assert_eq!(translated_terms.len(), 3);
        let review_statuses: Vec<String> = sqlx::query_scalar(
            "SELECT review_status FROM terminology_translations \
             WHERE project_id = ? AND target_language = ? AND entry_id IN (?, ?, ?)",
        )
        .bind(&project_id)
        .bind(target_language)
        .bind(&entry_ids[0])
        .bind(&entry_ids[1])
        .bind(&entry_ids[2])
        .fetch_all(&state.db)
        .await
        .unwrap();
        assert_eq!(review_statuses, vec!["proposed"; 3]);
        let required_target: String = sqlx::query_scalar(
            "SELECT target_text FROM terminology_translations \
             WHERE entry_id = ? AND target_language = ? AND project_id = ?",
        )
        .bind(&terms[0].id)
        .bind(target_language)
        .bind(&project_id)
        .fetch_one(&state.db)
        .await
        .unwrap();
        repository::upsert_translation(
            &state.db,
            &UpsertTranslationInput {
                entry_id: terms[0].id.clone(),
                target_language: target_language.into(),
                project_id: Some(project_id.clone()),
                target_text: required_target.clone(),
                review_status: ReviewStatus::Locked,
                enforcement: Enforcement::Required,
                confidence: 1.0,
                provider_id: Some("ollama".into()),
                model: Some(model.clone()),
                accepted_variants: Vec::new(),
            },
        )
        .await
        .unwrap();

        let estimated_prompt_chars = 2_000 + terms[0].source_text.chars().count();
        let hints = resolver::resolve_for_request(
            &state.db,
            std::slice::from_ref(&terms[0].segment_id),
            target_language,
            estimated_prompt_chars,
        )
        .await
        .unwrap();
        assert!(!hints.is_empty());
        assert!(hints.len() <= 20);

        let _baseline = provider
            .translate(
                vec![terms[0].source_text.clone()],
                translation_context(target_language, Vec::new()),
            )
            .await
            .unwrap();
        let baseline_call = provider.drain_metrics().into_iter().next().unwrap();
        let translated_segment = provider
            .translate(
                vec![terms[0].source_text.clone()],
                translation_context(target_language, hints.clone()),
            )
            .await
            .unwrap()
            .remove(0);
        let hinted_call = provider.drain_metrics().into_iter().next().unwrap();
        let baseline_tokens = baseline_call.prompt_tokens.unwrap();
        let hinted_tokens = hinted_call.prompt_tokens.unwrap();
        let overhead =
            100.0 * hinted_tokens.saturating_sub(baseline_tokens) as f64 / baseline_tokens as f64;
        assert!(
            overhead <= 10.0,
            "terminology prompt token overhead exceeded 10%: {overhead:.2}%"
        );

        let qa_rules = resolver::resolve_qa_rules_for_segments(
            &state.db,
            std::slice::from_ref(&terms[0].segment_id),
            target_language,
        )
        .await
        .unwrap()
        .remove(&terms[0].segment_id)
        .unwrap();
        let qa_result = qa::check_with_context(
            &terms[0].source_text,
            &translated_segment,
            &qa_rules,
            "mv_mz",
            &QaSemanticContext {
                source_language: "ja",
                target_language,
                segment_kind: "database",
                neighbor_sources: &[],
            },
        );
        assert!(!qa_result.has_critical_errors());
        assert!(translated_segment
            .to_lowercase()
            .contains(&required_target.to_lowercase()));

        language_reports.push(OllamaLanguageMetrics {
            target_language: target_language.into(),
            translated_terms,
            term_call,
            baseline_call,
            hinted_call,
            prompt_token_overhead_percent: overhead,
            resolved_hints: hints.len(),
            translated_segment,
            qa_score: qa_result.score,
            qa_has_critical: qa_result.has_critical_errors(),
        });
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&language_reports).unwrap()
    );
}
