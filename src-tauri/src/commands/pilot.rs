//! Non-destructive MV/MZ pilot preparation.
//!
//! This command deliberately receives no [`crate::state::AppState`]. It reads
//! the game through the shared extractor, uses a disposable SQLite database,
//! and returns only a representative sample plus an isolation audit.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use tauri::Emitter;

use crate::commands::project::{
    insert_segments, insert_source_file, normalize_language, visit_extracted_files,
};
use crate::core::qa::{self, QaResult};
use crate::domain::types::ProviderConfig;
use crate::engines::detector::{detect_engine, Engine};
use crate::llm::context::{self, SegmentPromptContext};
use crate::llm::pipeline::{self, TranslationResult};
use crate::llm::provider::{
    LlmProvider, OpenAiCompatibleProvider, PromptContextPolicy, ProviderCallMetrics,
    TranslationContext,
};
use crate::llm::tokenizer::{Engine as TokenizerEngine, Tokenizer};

const PILOT_PROJECT_ID: &str = "isolated-mv-mz-pilot";
const DEFAULT_SOURCE_LANG: &str = "ja";
const DEFAULT_TARGET_LANG: &str = "fr";
const MAX_SAMPLE_SIZE: usize = 200;
const PILOT_PROVIDER_TIMEOUT_SECS: u64 = 180;

const CATEGORIES: [PilotSampleCategory; 6] = [
    PilotSampleCategory::Dialogue,
    PilotSampleCategory::ChoiceBranch,
    PilotSampleCategory::Database,
    PilotSampleCategory::NamesUi,
    PilotSampleCategory::PlaceholderMultiline,
    PilotSampleCategory::Uncertain,
];

/// Stable pilot strata. Each future engine must define its own categorisation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PilotSampleCategory {
    Dialogue,
    ChoiceBranch,
    Database,
    NamesUi,
    PlaceholderMultiline,
    Uncertain,
}

impl PilotSampleCategory {
    const fn weight(self) -> usize {
        match self {
            Self::Dialogue => 40,
            Self::ChoiceBranch | Self::Database => 16,
            Self::NamesUi | Self::PlaceholderMultiline => 12,
            Self::Uncertain => 4,
        }
    }
}

/// One source unit selected for the human pilot review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PilotSampleSegment {
    pub stable_key: String,
    pub file_name: String,
    pub json_key: String,
    pub source_text: String,
    pub segment_kind: String,
    pub scene_id: Option<String>,
    pub speaker: Option<String>,
    pub branch_path: Option<String>,
    pub category: PilotSampleCategory,
    pub has_placeholders: bool,
    pub context: Option<SegmentPromptContext>,
}

/// Proof returned after the disposable database has been closed and removed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PilotIsolation {
    pub personal_database_accessed: bool,
    pub game_files_written: usize,
    pub database_removed: bool,
}

/// Extraction and sampling report produced before any provider is contacted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PilotPreparation {
    pub engine: String,
    pub source_language: String,
    pub target_language: String,
    pub total_files: usize,
    pub total_segments: usize,
    pub sample_size: usize,
    pub by_kind: BTreeMap<String, usize>,
    pub by_category: BTreeMap<PilotSampleCategory, usize>,
    pub sample: Vec<PilotSampleSegment>,
    pub isolation: PilotIsolation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PilotPhase {
    Preparing,
    Baseline,
    Contextual,
    QualityReview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PilotProgressPayload {
    pub phase: PilotPhase,
    pub done: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PilotQualityFlag {
    EmptyTranslation,
    SourceScriptRemaining,
    ContextLeak,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PilotVariantResult {
    pub translated_text: String,
    pub qa: QaResult,
    pub quality_flags: Vec<PilotQualityFlag>,
    pub needs_review: bool,
    pub from_tm: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PilotComparison {
    pub segment: PilotSampleSegment,
    pub baseline: PilotVariantResult,
    pub contextual: PilotVariantResult,
    pub changed: bool,
    pub context_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PilotMetricsSummary {
    pub calls: Vec<ProviderCallMetrics>,
    pub request_count: usize,
    pub input_units: usize,
    pub prompt_chars: usize,
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub duration_ms: u64,
    pub attempts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PilotVariantSummary {
    pub metrics: PilotMetricsSummary,
    pub average_qa_score: f64,
    pub needs_review_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PilotRunReport {
    pub preparation: PilotPreparation,
    pub comparisons: Vec<PilotComparison>,
    pub baseline: PilotVariantSummary,
    pub contextual: PilotVariantSummary,
    pub changed_count: usize,
}

#[derive(Debug, Clone, FromRow)]
struct PilotCandidate {
    id: String,
    file_name: String,
    json_key: String,
    source_text: String,
    segment_kind: String,
    scene_id: Option<String>,
    speaker: Option<String>,
    branch_path: Option<String>,
}

#[derive(Debug, Clone)]
struct ClassifiedCandidate {
    candidate: PilotCandidate,
    category: PilotSampleCategory,
    has_placeholders: bool,
    rank: [u8; 32],
}

struct PreparedPilot {
    report: PilotPreparation,
    pairs: Vec<(String, String)>,
}

struct PilotWorkspace {
    temporary_directory: tempfile::TempDir,
    database_path: PathBuf,
    pool: sqlx::SqlitePool,
    source_language: String,
    target_language: String,
    total_files: usize,
}

impl PilotWorkspace {
    async fn close(self) -> bool {
        self.pool.close().await;
        let database_path = self.database_path;
        drop(self.temporary_directory);
        !database_path.exists()
    }
}

/// Prepare a representative MV/MZ sample without translating or persisting it.
///
/// The sample size is clamped to `1..=200`. The command has no application
/// state parameter, so the personal SQLite pool is outside its authority.
#[tauri::command]
pub async fn prepare_mv_mz_pilot(
    game_path: String,
    source_lang: Option<String>,
    target_lang: Option<String>,
    sample_size: usize,
) -> Result<PilotPreparation, String> {
    let requested_size = sample_size.clamp(1, MAX_SAMPLE_SIZE);
    let workspace = create_pilot_workspace(game_path, source_lang, target_lang).await?;
    let preparation_result = build_preparation(
        &workspace.pool,
        workspace.total_files,
        requested_size,
        &workspace.source_language,
        &workspace.target_language,
    )
    .await;
    let database_path = workspace.database_path.clone();
    let database_removed = workspace.close().await;
    if !database_removed {
        return Err(format!(
            "temporary pilot database was not removed: {}",
            database_path.display()
        ));
    }

    let mut preparation = preparation_result?.report;
    preparation.isolation = PilotIsolation {
        personal_database_accessed: false,
        game_files_written: 0,
        database_removed,
    };
    Ok(preparation)
}

/// Run an isolated baseline/contextual comparison against one deterministic
/// MV/MZ sample. This command deliberately has no `AppState`: neither the
/// personal database nor the application's translation memory is reachable.
#[tauri::command]
pub async fn run_mv_mz_pilot(
    game_path: String,
    source_lang: Option<String>,
    target_lang: Option<String>,
    sample_size: usize,
    provider_config: ProviderConfig,
    app: tauri::AppHandle,
) -> Result<PilotRunReport, String> {
    emit_pilot_progress(&app, PilotPhase::Preparing, 0, 1);
    let provider = OpenAiCompatibleProvider::new_for_preset(
        &provider_config.provider_id,
        &provider_config.url,
        &provider_config.model,
        provider_config.api_key.as_deref(),
        Duration::from_secs(PILOT_PROVIDER_TIMEOUT_SECS),
    );
    provider
        .health_check()
        .await
        .map_err(|error| format!("pilot provider unavailable: {error}"))?;

    let requested_size = sample_size.clamp(1, MAX_SAMPLE_SIZE);
    let workspace = create_pilot_workspace(game_path, source_lang, target_lang).await?;
    let prepared_result = build_preparation(
        &workspace.pool,
        workspace.total_files,
        requested_size,
        &workspace.source_language,
        &workspace.target_language,
    )
    .await;

    let report_result = match prepared_result {
        Ok(prepared) => {
            emit_pilot_progress(&app, PilotPhase::Preparing, 1, 1);
            run_prepared_pilot(
                &workspace.pool,
                prepared,
                &provider,
                &provider_config,
                |payload| {
                    let _ = app.emit("h2s://pilot/progress", payload);
                },
            )
            .await
        }
        Err(error) => Err(error),
    };

    let database_path = workspace.database_path.clone();
    let database_removed = workspace.close().await;
    if !database_removed {
        return Err(format!(
            "temporary pilot database was not removed: {}",
            database_path.display()
        ));
    }

    let mut report = report_result?;
    report.preparation.isolation = PilotIsolation {
        personal_database_accessed: false,
        game_files_written: 0,
        database_removed,
    };
    Ok(report)
}

fn emit_pilot_progress(app: &tauri::AppHandle, phase: PilotPhase, done: usize, total: usize) {
    let _ = app.emit(
        "h2s://pilot/progress",
        PilotProgressPayload { phase, done, total },
    );
}

async fn run_prepared_pilot<P, F>(
    pool: &sqlx::SqlitePool,
    prepared: PreparedPilot,
    provider: &P,
    provider_config: &ProviderConfig,
    mut on_progress: F,
) -> Result<PilotRunReport, String>
where
    P: LlmProvider,
    F: FnMut(PilotProgressPayload),
{
    let total = prepared.pairs.len();
    let make_context = |prompt_context_policy| TranslationContext {
        source_lang: prepared.report.source_language.clone(),
        target_lang: prepared.report.target_language.clone(),
        glossary_terms: Vec::new(),
        engine: "mv_mz".to_string(),
        batch_size: provider_config.effective_batch_size(),
        batch_delay_ms: provider_config.batch_delay_ms(),
        prompt_context_policy,
        segment_contexts: Vec::new(),
    };

    let baseline_results = pipeline::run_inner(
        prepared.pairs.clone(),
        provider,
        make_context(PromptContextPolicy::Disabled),
        pool,
        None,
        None,
        None,
        |done, total| {
            on_progress(PilotProgressPayload {
                phase: PilotPhase::Baseline,
                done,
                total,
            });
        },
    )
    .await
    .map_err(|error| error.to_string())?;
    let baseline_metrics = provider.drain_metrics();

    reset_pilot_translations(pool).await?;

    let contextual_results = pipeline::run_inner(
        prepared.pairs,
        provider,
        make_context(PromptContextPolicy::EngineOwned),
        pool,
        None,
        None,
        None,
        |done, total| {
            on_progress(PilotProgressPayload {
                phase: PilotPhase::Contextual,
                done,
                total,
            });
        },
    )
    .await
    .map_err(|error| error.to_string())?;
    let contextual_metrics = provider.drain_metrics();

    let mut comparisons = Vec::with_capacity(total);
    for (index, ((segment, baseline), contextual)) in prepared
        .report
        .sample
        .iter()
        .cloned()
        .zip(baseline_results)
        .zip(contextual_results)
        .enumerate()
    {
        let baseline = build_variant(
            &segment,
            baseline,
            &prepared.report.source_language,
            &prepared.report.target_language,
        );
        let contextual = build_variant(
            &segment,
            contextual,
            &prepared.report.source_language,
            &prepared.report.target_language,
        );
        let changed = baseline.translated_text.trim() != contextual.translated_text.trim();
        let context_available = segment.context.is_some();
        comparisons.push(PilotComparison {
            segment,
            baseline,
            contextual,
            changed,
            context_available,
        });
        on_progress(PilotProgressPayload {
            phase: PilotPhase::QualityReview,
            done: index + 1,
            total,
        });
    }

    let baseline = summarize_variant(&comparisons, true, baseline_metrics);
    let contextual = summarize_variant(&comparisons, false, contextual_metrics);
    let changed_count = comparisons
        .iter()
        .filter(|comparison| comparison.changed)
        .count();

    Ok(PilotRunReport {
        preparation: prepared.report,
        comparisons,
        baseline,
        contextual,
        changed_count,
    })
}

async fn reset_pilot_translations(pool: &sqlx::SqlitePool) -> Result<(), String> {
    sqlx::query(
        "UPDATE segments SET target_text = '', status = 'untranslated' \
         WHERE source_file_id IN (SELECT id FROM source_files WHERE project_id = ?)",
    )
    .bind(PILOT_PROJECT_ID)
    .execute(pool)
    .await
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn build_variant(
    segment: &PilotSampleSegment,
    result: TranslationResult,
    source_language: &str,
    target_language: &str,
) -> PilotVariantResult {
    let qa = qa::check(&segment.source_text, &result.translated_text, &[], "mv_mz");
    let mut quality_flags = Vec::new();
    if result.translated_text.trim().is_empty() {
        quality_flags.push(PilotQualityFlag::EmptyTranslation);
    }
    if source_script_remaining(&result.translated_text, source_language, target_language) {
        quality_flags.push(PilotQualityFlag::SourceScriptRemaining);
    }
    if context_leaked(&result.translated_text, segment.context.as_ref()) {
        quality_flags.push(PilotQualityFlag::ContextLeak);
    }
    let needs_review = result.needs_review || !qa.errors.is_empty() || !quality_flags.is_empty();

    PilotVariantResult {
        translated_text: result.translated_text,
        qa,
        quality_flags,
        needs_review,
        from_tm: result.from_tm,
    }
}

fn source_script_remaining(target: &str, source_language: &str, target_language: &str) -> bool {
    if source_language == target_language {
        return false;
    }
    target.chars().any(|character| match source_language {
        "ja" => matches!(
            character,
            '\u{3040}'..='\u{30ff}' | '\u{3400}'..='\u{4dbf}' | '\u{4e00}'..='\u{9fff}'
        ),
        "zh" | "zh-cn" | "zh-tw" => {
            matches!(character, '\u{3400}'..='\u{4dbf}' | '\u{4e00}'..='\u{9fff}')
        }
        "ko" => matches!(character, '\u{1100}'..='\u{11ff}' | '\u{ac00}'..='\u{d7af}'),
        _ => false,
    })
}

fn context_leaked(target: &str, context: Option<&SegmentPromptContext>) -> bool {
    let Some(context) = context else {
        return false;
    };
    context
        .previous
        .iter()
        .chain(&context.following)
        .map(|neighbor| neighbor.text.trim())
        .filter(|text| {
            text.chars()
                .filter(|character| !character.is_whitespace())
                .count()
                >= 3
        })
        .any(|text| target.contains(text))
}

fn summarize_variant(
    comparisons: &[PilotComparison],
    baseline: bool,
    calls: Vec<ProviderCallMetrics>,
) -> PilotVariantSummary {
    let variants = comparisons.iter().map(|comparison| {
        if baseline {
            &comparison.baseline
        } else {
            &comparison.contextual
        }
    });
    let mut qa_total = 0_u64;
    let mut needs_review_count = 0_usize;
    for variant in variants {
        qa_total += u64::from(variant.qa.score);
        needs_review_count += usize::from(variant.needs_review);
    }
    let average_qa_score = if comparisons.is_empty() {
        0.0
    } else {
        qa_total as f64 / comparisons.len() as f64
    };

    PilotVariantSummary {
        metrics: summarize_metrics(calls),
        average_qa_score,
        needs_review_count,
    }
}

fn summarize_metrics(calls: Vec<ProviderCallMetrics>) -> PilotMetricsSummary {
    let request_count = calls.len();
    let input_units = calls.iter().map(|call| call.input_units).sum();
    let prompt_chars = calls.iter().map(|call| call.prompt_chars).sum();
    let prompt_tokens = sum_optional_metrics(&calls, |call| call.prompt_tokens);
    let completion_tokens = sum_optional_metrics(&calls, |call| call.completion_tokens);
    let total_tokens = sum_optional_metrics(&calls, |call| call.total_tokens);
    let duration_ms = calls.iter().map(|call| call.duration_ms).sum();
    let attempts = calls.iter().map(|call| call.attempts).sum();
    PilotMetricsSummary {
        calls,
        request_count,
        input_units,
        prompt_chars,
        prompt_tokens,
        completion_tokens,
        total_tokens,
        duration_ms,
        attempts,
    }
}

fn sum_optional_metrics(
    calls: &[ProviderCallMetrics],
    select: impl Fn(&ProviderCallMetrics) -> Option<u64>,
) -> Option<u64> {
    let values = calls.iter().filter_map(select).collect::<Vec<_>>();
    (!values.is_empty()).then(|| values.into_iter().sum())
}

async fn create_pilot_workspace(
    game_path: String,
    source_lang: Option<String>,
    target_lang: Option<String>,
) -> Result<PilotWorkspace, String> {
    let source_language = normalize_language(source_lang, DEFAULT_SOURCE_LANG)?;
    let target_language = normalize_language(target_lang, DEFAULT_TARGET_LANG)?;
    let game_dir = PathBuf::from(&game_path);
    let detection_path = game_dir.clone();
    let (engine, data_dir) = tokio::task::spawn_blocking(move || {
        let engine = detect_engine(&detection_path).map_err(|error| error.to_string())?;
        if engine != Engine::MvMz {
            return Err(format!(
                "pilot context is implemented only for MV/MZ, detected {}",
                engine.db_str()
            ));
        }
        let data_dir = engine.data_dir(&detection_path)?;
        Ok::<_, String>((engine, data_dir))
    })
    .await
    .map_err(|error| format!("pilot detection worker failed: {error}"))??;

    let temporary_directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let database_path = temporary_directory.path().join("pilot.sqlite");
    let database_path_text = database_path
        .to_str()
        .ok_or_else(|| "temporary pilot database path is not UTF-8".to_string())?;
    let pool = crate::db::pool::init(database_path_text)
        .await
        .map_err(|error| error.to_string())?;
    let total_files_result = seed_isolated_database(
        &pool,
        &engine,
        &game_dir,
        &data_dir,
        &source_language,
        &target_language,
    )
    .await;

    let total_files = match total_files_result {
        Ok(total_files) => total_files,
        Err(error) => {
            pool.close().await;
            drop(temporary_directory);
            return Err(error);
        }
    };

    Ok(PilotWorkspace {
        temporary_directory,
        database_path,
        pool,
        source_language,
        target_language,
        total_files,
    })
}

async fn seed_isolated_database(
    pool: &sqlx::SqlitePool,
    engine: &Engine,
    game_dir: &Path,
    data_dir: &Path,
    source_language: &str,
    target_language: &str,
) -> Result<usize, String> {
    sqlx::query(
        "INSERT INTO projects \
         (id, name, engine, game_path, source_language, target_language) \
         VALUES (?, 'Isolated pilot', 'mv_mz', ?, ?, ?)",
    )
    .bind(PILOT_PROJECT_ID)
    .bind(game_dir.to_string_lossy().as_ref())
    .bind(source_language)
    .bind(target_language)
    .execute(pool)
    .await
    .map_err(|error| error.to_string())?;

    let mut transaction = pool.begin().await.map_err(|error| error.to_string())?;
    let (sender, mut receiver) = tokio::sync::mpsc::channel(2);
    let worker_engine = engine.clone();
    let worker_game_dir = game_dir.to_path_buf();
    let worker_data_dir = data_dir.to_path_buf();
    let extraction_worker = tokio::task::spawn_blocking(move || {
        visit_extracted_files(&worker_engine, &worker_game_dir, &worker_data_dir, |file| {
            sender
                .blocking_send(file)
                .map_err(|_| "pilot extraction consumer closed".to_string())
        })
    });

    let mut total_files = 0_usize;
    while let Some(file) = receiver.recv().await {
        let file_id = uuid::Uuid::new_v4().to_string();
        insert_source_file(
            &mut transaction,
            &file_id,
            PILOT_PROJECT_ID,
            &file.file_name,
            &file.file_path,
            &file.file_type,
        )
        .await
        .map_err(|error| error.to_string())?;
        insert_segments(&mut transaction, &file_id, &file.segments)
            .await
            .map_err(|error| error.to_string())?;
        total_files += 1;
    }

    extraction_worker
        .await
        .map_err(|error| format!("pilot extraction worker failed: {error}"))??;
    transaction
        .commit()
        .await
        .map_err(|error| error.to_string())?;
    Ok(total_files)
}

async fn build_preparation(
    pool: &sqlx::SqlitePool,
    total_files: usize,
    sample_size: usize,
    source_language: &str,
    target_language: &str,
) -> Result<PreparedPilot, String> {
    let candidates = sqlx::query_as::<_, PilotCandidate>(
        "SELECT s.id, sf.file_name, s.json_key, s.source_text, s.segment_kind, \
                s.scene_id, s.speaker, s.branch_path \
         FROM segments s \
         JOIN source_files sf ON sf.id = s.source_file_id \
         WHERE sf.project_id = ? \
         ORDER BY sf.file_name, s.rowid",
    )
    .bind(PILOT_PROJECT_ID)
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?;

    let mut by_kind = BTreeMap::new();
    for candidate in &candidates {
        *by_kind.entry(candidate.segment_kind.clone()).or_insert(0) += 1;
    }

    let selected = select_sample(&candidates, sample_size);
    let selected_ids: Vec<String> = selected
        .iter()
        .map(|item| item.candidate.id.clone())
        .collect();
    let pairs = selected
        .iter()
        .map(|item| {
            (
                item.candidate.id.clone(),
                item.candidate.source_text.clone(),
            )
        })
        .collect();
    let contexts = context::build_for_segments(pool, "mv_mz", &selected_ids)
        .await
        .map_err(|error| error.to_string())?;

    let mut by_category = BTreeMap::new();
    let sample = selected
        .into_iter()
        .zip(contexts)
        .map(|(item, prompt_context)| {
            *by_category.entry(item.category).or_insert(0) += 1;
            let candidate = item.candidate;
            PilotSampleSegment {
                stable_key: stable_key(&candidate),
                file_name: candidate.file_name,
                json_key: candidate.json_key,
                source_text: candidate.source_text,
                segment_kind: candidate.segment_kind,
                scene_id: candidate.scene_id,
                speaker: candidate.speaker,
                branch_path: candidate.branch_path,
                category: item.category,
                has_placeholders: item.has_placeholders,
                context: prompt_context,
            }
        })
        .collect::<Vec<_>>();

    Ok(PreparedPilot {
        report: PilotPreparation {
            engine: "mv_mz".to_string(),
            source_language: source_language.to_string(),
            target_language: target_language.to_string(),
            total_files,
            total_segments: candidates.len(),
            sample_size: sample.len(),
            by_kind,
            by_category,
            sample,
            isolation: PilotIsolation {
                personal_database_accessed: false,
                game_files_written: 0,
                database_removed: false,
            },
        },
        pairs,
    })
}

fn select_sample(candidates: &[PilotCandidate], sample_size: usize) -> Vec<ClassifiedCandidate> {
    let target = sample_size.min(candidates.len());
    if target == 0 {
        return Vec::new();
    }

    let mut classified = candidates
        .iter()
        .cloned()
        .map(classify_candidate)
        .collect::<Vec<_>>();
    classified.sort_by(|left, right| {
        left.rank
            .cmp(&right.rank)
            .then_with(|| stable_key(&left.candidate).cmp(&stable_key(&right.candidate)))
    });

    let quotas = category_quotas(target);
    let mut selected = Vec::with_capacity(target);
    let mut used = HashSet::with_capacity(target);
    for category in CATEGORIES {
        let quota = quotas.get(&category).copied().unwrap_or_default();
        for item in classified
            .iter()
            .filter(|item| item.category == category)
            .take(quota)
        {
            let key = stable_key(&item.candidate);
            if used.insert(key) {
                selected.push(item.clone());
            }
        }
    }

    if selected.len() < target {
        for item in &classified {
            let key = stable_key(&item.candidate);
            if used.insert(key) {
                selected.push(item.clone());
                if selected.len() == target {
                    break;
                }
            }
        }
    }
    selected
}

fn category_quotas(total: usize) -> BTreeMap<PilotSampleCategory, usize> {
    let mut quotas = BTreeMap::new();
    let mut remainders = Vec::with_capacity(CATEGORIES.len());
    let mut allocated = 0_usize;
    for category in CATEGORIES {
        let weighted = total * category.weight();
        let base = weighted / 100;
        allocated += base;
        quotas.insert(category, base);
        remainders.push((weighted % 100, category));
    }
    remainders.sort_by(|left, right| right.cmp(left));
    for (_, category) in remainders.into_iter().take(total - allocated) {
        *quotas.entry(category).or_insert(0) += 1;
    }
    quotas
}

fn classify_candidate(candidate: PilotCandidate) -> ClassifiedCandidate {
    let tokenized = Tokenizer::tokenize(&candidate.source_text, TokenizerEngine::MvMz);
    let has_placeholders = !tokenized.map.is_empty();
    let category = if has_placeholders || candidate.source_text.contains('\n') {
        PilotSampleCategory::PlaceholderMultiline
    } else if candidate.segment_kind == "choice" || candidate.branch_path.is_some() {
        PilotSampleCategory::ChoiceBranch
    } else if matches!(
        candidate.segment_kind.as_str(),
        "dialogue" | "scrolling_text" | "speaker"
    ) {
        PilotSampleCategory::Dialogue
    } else if matches!(
        candidate.segment_kind.as_str(),
        "item_description"
            | "skill_description"
            | "skill_message"
            | "state_message"
            | "actor_profile"
    ) {
        PilotSampleCategory::Database
    } else if matches!(
        candidate.segment_kind.as_str(),
        "actor_name"
            | "actor_nickname"
            | "class_name"
            | "item_name"
            | "skill_name"
            | "enemy_name"
            | "state_name"
            | "map_name"
            | "common_event_name"
            | "system_term"
            | "game_title"
    ) {
        PilotSampleCategory::NamesUi
    } else {
        PilotSampleCategory::Uncertain
    };

    let rank = stable_rank(&candidate);
    ClassifiedCandidate {
        candidate,
        category,
        has_placeholders,
        rank,
    }
}

fn stable_key(candidate: &PilotCandidate) -> String {
    format!("{}::{}", candidate.file_name, candidate.json_key)
}

fn stable_rank(candidate: &PilotCandidate) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(candidate.file_name.as_bytes());
    hasher.update([0]);
    hasher.update(candidate.json_key.as_bytes());
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::provider::{LlmError, ProviderTask};

    struct PilotMockProvider {
        policies: std::sync::Mutex<Vec<PromptContextPolicy>>,
        received_contexts: std::sync::Mutex<Vec<Vec<Option<SegmentPromptContext>>>>,
        metrics: std::sync::Mutex<Vec<ProviderCallMetrics>>,
    }

    impl PilotMockProvider {
        fn new() -> Self {
            Self {
                policies: std::sync::Mutex::new(Vec::new()),
                received_contexts: std::sync::Mutex::new(Vec::new()),
                metrics: std::sync::Mutex::new(Vec::new()),
            }
        }
    }

    impl LlmProvider for PilotMockProvider {
        async fn translate(
            &self,
            segments: Vec<String>,
            context: TranslationContext,
        ) -> Result<Vec<String>, LlmError> {
            self.policies
                .lock()
                .unwrap()
                .push(context.prompt_context_policy);
            self.received_contexts
                .lock()
                .unwrap()
                .push(context.segment_contexts.clone());
            self.metrics.lock().unwrap().push(ProviderCallMetrics {
                task: ProviderTask::Translate,
                model: "pilot-mock".to_string(),
                input_units: segments.len(),
                prompt_chars: segments.iter().map(String::len).sum(),
                prompt_tokens: Some(20),
                completion_tokens: Some(10),
                total_tokens: Some(30),
                duration_ms: 5,
                attempts: 1,
                success: true,
            });
            let prefix = match context.prompt_context_policy {
                PromptContextPolicy::Disabled => "Base",
                PromptContextPolicy::EngineOwned => "Contexte",
            };
            Ok(segments
                .iter()
                .enumerate()
                .map(|(index, _)| format!("{prefix} {index}"))
                .collect())
        }

        async fn health_check(&self) -> Result<(), LlmError> {
            Ok(())
        }

        async fn chat(&self, _system: &str, _user: &str) -> Result<String, LlmError> {
            Ok(String::new())
        }

        fn drain_metrics(&self) -> Vec<ProviderCallMetrics> {
            std::mem::take(&mut *self.metrics.lock().unwrap())
        }
    }

    fn candidate(index: usize, kind: &str, source: &str, branch: Option<&str>) -> PilotCandidate {
        PilotCandidate {
            id: format!("id-{index}"),
            file_name: format!("Map{:03}.json", index % 7),
            json_key: format!("/events/{index}"),
            source_text: source.to_string(),
            segment_kind: kind.to_string(),
            scene_id: Some(format!("scene-{}", index % 5)),
            speaker: None,
            branch_path: branch.map(str::to_string),
        }
    }

    #[test]
    fn fifty_item_sample_uses_expected_stratified_distribution() {
        let mut candidates = Vec::new();
        for index in 0..20 {
            candidates.push(candidate(index, "dialogue", "会話", None));
        }
        for index in 20..28 {
            candidates.push(candidate(index, "choice", "選択", Some("if:1")));
        }
        for index in 28..36 {
            candidates.push(candidate(index, "item_description", "説明", None));
        }
        for index in 36..42 {
            candidates.push(candidate(index, "actor_name", "名前", None));
        }
        for index in 42..48 {
            candidates.push(candidate(index, "dialogue", r"変数\V[1]", None));
        }
        for index in 48..50 {
            candidates.push(candidate(index, "plugin_text", "プラグイン", None));
        }

        let first = select_sample(&candidates, 50);
        let second = select_sample(&candidates, 50);
        let mut distribution = BTreeMap::new();
        let unique = first
            .iter()
            .map(|item| stable_key(&item.candidate))
            .collect::<HashSet<_>>();
        for item in &first {
            *distribution.entry(item.category).or_insert(0) += 1;
        }

        assert_eq!(distribution[&PilotSampleCategory::Dialogue], 20);
        assert_eq!(distribution[&PilotSampleCategory::ChoiceBranch], 8);
        assert_eq!(distribution[&PilotSampleCategory::Database], 8);
        assert_eq!(distribution[&PilotSampleCategory::NamesUi], 6);
        assert_eq!(distribution[&PilotSampleCategory::PlaceholderMultiline], 6);
        assert_eq!(distribution[&PilotSampleCategory::Uncertain], 2);
        assert_eq!(unique.len(), 50);
        assert_eq!(
            first
                .iter()
                .map(|item| stable_key(&item.candidate))
                .collect::<Vec<_>>(),
            second
                .iter()
                .map(|item| stable_key(&item.candidate))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn missing_categories_are_filled_without_duplicates() {
        let candidates = (0..10)
            .map(|index| candidate(index, "dialogue", "会話", None))
            .collect::<Vec<_>>();

        let selected = select_sample(&candidates, 7);
        let unique = selected
            .iter()
            .map(|item| stable_key(&item.candidate))
            .collect::<HashSet<_>>();

        assert_eq!(selected.len(), 7);
        assert_eq!(unique.len(), 7);
        assert!(selected
            .iter()
            .all(|item| item.category == PilotSampleCategory::Dialogue));
    }

    #[tokio::test]
    async fn preparation_never_writes_game_or_keeps_its_database() {
        let game = tempfile::tempdir().unwrap();
        let data = game.path().join("data");
        std::fs::create_dir(&data).unwrap();
        let system_path = data.join("System.json");
        let map_path = data.join("Map001.json");
        std::fs::write(
            &system_path,
            r#"{"gameTitle":"Pilot","currencyUnit":"G","terms":{"basic":["Niveau"]}}"#,
        )
        .unwrap();
        std::fs::write(
            &map_path,
            r#"{"events":[null,{"id":1,"pages":[{"list":[{"code":101,"parameters":["",0,0,2,"勇者"]},{"code":401,"parameters":["待って\\V[1]"]},{"code":102,"parameters":[["はい","いいえ"],0,0,2,0]},{"code":0,"parameters":[]}]}]}]}"#,
        )
        .unwrap();
        let system_before = std::fs::read(&system_path).unwrap();
        let map_before = std::fs::read(&map_path).unwrap();

        let preparation = prepare_mv_mz_pilot(
            game.path().to_string_lossy().to_string(),
            Some("ja".to_string()),
            Some("fr".to_string()),
            5,
        )
        .await
        .unwrap();

        assert_eq!(preparation.engine, "mv_mz");
        assert_eq!(preparation.sample_size, 5.min(preparation.total_segments));
        assert!(preparation
            .sample
            .iter()
            .any(|segment| segment.context.is_some()));
        assert!(!preparation.isolation.personal_database_accessed);
        assert_eq!(preparation.isolation.game_files_written, 0);
        assert!(preparation.isolation.database_removed);
        assert_eq!(std::fs::read(&system_path).unwrap(), system_before);
        assert_eq!(std::fs::read(&map_path).unwrap(), map_before);
        assert!(!game.path().join(".hoshi2star.json").exists());
        assert!(!game.path().join("hoshi2star_debug_extract.json").exists());
    }

    #[tokio::test]
    async fn ab_run_uses_identical_sample_with_separate_context_and_metrics() {
        let game = tempfile::tempdir().unwrap();
        let data = game.path().join("data");
        std::fs::create_dir(&data).unwrap();
        let system_path = data.join("System.json");
        let map_path = data.join("Map001.json");
        std::fs::write(
            &system_path,
            r#"{"gameTitle":"Pilot","currencyUnit":"G","terms":{"basic":["Level"]}}"#,
        )
        .unwrap();
        std::fs::write(
            &map_path,
            r#"{"events":[null,{"id":1,"pages":[{"list":[{"code":101,"parameters":["",0,0,2,"Hero"]},{"code":401,"parameters":["First line"]},{"code":401,"parameters":["Second line"]},{"code":401,"parameters":["Third line"]},{"code":0,"parameters":[]}]}]}]}"#,
        )
        .unwrap();
        let system_before = std::fs::read(&system_path).unwrap();
        let map_before = std::fs::read(&map_path).unwrap();

        let workspace = create_pilot_workspace(
            game.path().to_string_lossy().to_string(),
            Some("en".to_string()),
            Some("fr".to_string()),
        )
        .await
        .unwrap();
        let prepared = build_preparation(
            &workspace.pool,
            workspace.total_files,
            50,
            &workspace.source_language,
            &workspace.target_language,
        )
        .await
        .unwrap();
        let sample_keys = prepared
            .report
            .sample
            .iter()
            .map(|segment| segment.stable_key.clone())
            .collect::<Vec<_>>();
        let provider = PilotMockProvider::new();
        let config = ProviderConfig {
            batch_size: 100,
            ..ProviderConfig::default()
        };
        let mut progress = Vec::new();

        let report = run_prepared_pilot(&workspace.pool, prepared, &provider, &config, |payload| {
            progress.push(payload)
        })
        .await
        .unwrap();
        let tm_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tm_entries")
            .fetch_one(&workspace.pool)
            .await
            .unwrap();
        let database_removed = workspace.close().await;

        assert_eq!(
            report
                .comparisons
                .iter()
                .map(|comparison| comparison.segment.stable_key.clone())
                .collect::<Vec<_>>(),
            sample_keys
        );
        assert!(report.comparisons.iter().all(|comparison| {
            comparison.changed && !comparison.baseline.from_tm && !comparison.contextual.from_tm
        }));
        assert_eq!(report.baseline.metrics.request_count, 1);
        // Contextual requests are intentionally separated by MV/MZ prompt
        // class so names/choices cannot contaminate dialogue.
        assert_eq!(report.contextual.metrics.request_count, 2);
        assert_eq!(report.baseline.metrics.total_tokens, Some(30));
        assert_eq!(report.contextual.metrics.total_tokens, Some(60));
        assert_eq!(tm_count, 0);
        assert!(progress
            .iter()
            .any(|payload| payload.phase == PilotPhase::Baseline));
        assert!(progress
            .iter()
            .any(|payload| payload.phase == PilotPhase::Contextual));
        let policies = provider.policies.lock().unwrap();
        assert_eq!(
            policies.as_slice(),
            [
                PromptContextPolicy::Disabled,
                PromptContextPolicy::EngineOwned,
                PromptContextPolicy::EngineOwned
            ]
        );
        let received = provider.received_contexts.lock().unwrap();
        assert!(received[0].iter().all(Option::is_none));
        assert!(received[1].iter().any(Option::is_some));
        assert!(database_removed);
        assert_eq!(std::fs::read(&system_path).unwrap(), system_before);
        assert_eq!(std::fs::read(&map_path).unwrap(), map_before);
    }
}
