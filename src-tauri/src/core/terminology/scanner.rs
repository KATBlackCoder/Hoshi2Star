use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, Sqlite, SqlitePool, Transaction};

use super::analyzer::filters::is_terminology_candidate;
use super::analyzer::{LinguisticToken, MorphologicalAnalyzer};
use super::normalize::normalize_term;
use super::types::PartOfSpeech;
use super::{Result, TerminologyError};
use crate::engines::terminology::{adapter_for, EngineSegmentContext, EngineTermSeed};

pub const SCAN_CHUNK_SIZE: i64 = 250;
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanStatus {
    Completed,
    Cancelled,
}

impl ScanStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub scan_id: String,
    pub project_id: String,
    pub processed: i64,
    pub total: i64,
    pub discovered: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub scan_id: String,
    pub project_id: String,
    pub status: ScanStatus,
    pub processed: i64,
    pub analyzed: i64,
    pub skipped: i64,
    pub discovered: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScanDone {
    pub scan_id: String,
    pub project_id: String,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum ScanEvent {
    Progress(ScanProgress),
    Done(ScanDone),
}

pub type ScanEventSink = Arc<dyn Fn(ScanEvent) + Send + Sync>;

#[derive(Debug, Clone, FromRow)]
struct ProjectRow {
    engine: String,
    source_language: String,
}

#[derive(Debug, Clone, FromRow)]
struct SegmentRow {
    id: String,
    source_text: String,
    segment_kind: String,
    json_key: String,
    speaker: Option<String>,
    context_json: Option<String>,
    file_name: String,
    file_type: String,
    previous_hash: Option<String>,
}

#[derive(Debug)]
struct AnalyzedSegment {
    segment: SegmentRow,
    source_hash: String,
    terms: Vec<AnalyzedTerm>,
}

#[derive(Debug, Clone)]
struct AnalyzedTerm {
    canonical_text: String,
    surface_text: String,
    reading: Option<String>,
    part_of_speech: PartOfSpeech,
    semantic_type: String,
    origin: &'static str,
    confidence: f64,
    occurrence_count: i64,
}

pub async fn scan_project(
    pool: &SqlitePool,
    analyzer: Arc<dyn MorphologicalAnalyzer>,
    scan_id: &str,
    project_id: &str,
    cancelled: Arc<AtomicBool>,
    event_sink: Option<ScanEventSink>,
) -> Result<ScanSummary> {
    let project: ProjectRow =
        sqlx::query_as("SELECT engine, source_language FROM projects WHERE id = ?")
            .bind(project_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| TerminologyError::NotFound(project_id.to_string()))?;
    let adapter = adapter_for(&project.engine).ok_or_else(|| {
        TerminologyError::InvalidInput(format!(
            "terminology scanning is not supported for engine `{}`",
            project.engine
        ))
    })?;
    let analyzer_version = format!("{}+{}", analyzer.version(), adapter.version());
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM segments segment \
         JOIN source_files file ON file.id = segment.source_file_id \
         WHERE file.project_id = ?",
    )
    .bind(project_id)
    .fetch_one(pool)
    .await?;

    sqlx::query(
        "INSERT INTO terminology_scans (\
            id, project_id, source_language, status, analyzer_version, total_segments\
         ) VALUES (?, ?, ?, 'running', ?, ?)",
    )
    .bind(scan_id)
    .bind(project_id)
    .bind(&project.source_language)
    .bind(&analyzer_version)
    .bind(total)
    .execute(pool)
    .await?;

    let result = scan_pages(
        pool,
        analyzer,
        scan_id,
        project_id,
        &project,
        &analyzer_version,
        total,
        cancelled,
        event_sink,
    )
    .await;

    if let Err(error) = &result {
        let _ = sqlx::query(
            "UPDATE terminology_scans SET status = 'failed', error = ?, \
             finished_at = datetime('now') WHERE id = ?",
        )
        .bind(error.to_string())
        .bind(scan_id)
        .execute(pool)
        .await;
    }
    result
}

#[allow(clippy::too_many_arguments)]
async fn scan_pages(
    pool: &SqlitePool,
    analyzer: Arc<dyn MorphologicalAnalyzer>,
    scan_id: &str,
    project_id: &str,
    project: &ProjectRow,
    analyzer_version: &str,
    total: i64,
    cancelled: Arc<AtomicBool>,
    event_sink: Option<ScanEventSink>,
) -> Result<ScanSummary> {
    let mut processed = 0_i64;
    let mut analyzed = 0_i64;
    let mut skipped = 0_i64;
    let mut discovered = 0_i64;
    let mut cursor: Option<String> = None;
    let mut last_progress = Instant::now() - PROGRESS_INTERVAL;

    loop {
        if cancelled.load(Ordering::Relaxed) {
            break;
        }
        let rows = load_segment_page(pool, project_id, cursor.as_deref()).await?;
        if rows.is_empty() {
            break;
        }
        cursor = rows.last().map(|row| row.id.clone());

        let current = rows
            .into_iter()
            .map(|segment| {
                let source_hash = segment_hash(&segment, analyzer_version);
                (segment, source_hash)
            })
            .collect::<Vec<_>>();
        let unchanged = current
            .iter()
            .filter(|(segment, hash)| segment.previous_hash.as_deref() == Some(hash.as_str()))
            .count() as i64;
        let changed = current
            .into_iter()
            .filter(|(segment, hash)| segment.previous_hash.as_deref() != Some(hash.as_str()))
            .collect::<Vec<_>>();

        let analyzer_for_worker = analyzer.clone();
        let cancelled_for_worker = cancelled.clone();
        let source_language = project.source_language.clone();
        let engine = project.engine.clone();
        let analyzed_segments = tokio::task::spawn_blocking(move || {
            let adapter = adapter_for(&engine).expect("registered adapter disappeared");
            let mut output = Vec::with_capacity(changed.len());
            for (segment, source_hash) in changed {
                if cancelled_for_worker.load(Ordering::Relaxed) {
                    break;
                }
                let terms = analyze_segment(
                    analyzer_for_worker.as_ref(),
                    adapter,
                    &segment,
                    &source_language,
                )?;
                output.push(AnalyzedSegment {
                    segment,
                    source_hash,
                    terms,
                });
            }
            Result::<Vec<AnalyzedSegment>>::Ok(output)
        })
        .await
        .map_err(|error| TerminologyError::Analyzer(error.to_string()))??;

        let chunk_analyzed = analyzed_segments.len() as i64;
        if !analyzed_segments.is_empty() {
            let mut transaction = pool.begin().await?;
            for segment in &analyzed_segments {
                discovered += persist_segment(
                    &mut transaction,
                    project_id,
                    &project.source_language,
                    analyzer_version,
                    segment,
                )
                .await?;
            }
            transaction.commit().await?;
        }

        analyzed += chunk_analyzed;
        skipped += unchanged;
        processed += unchanged + chunk_analyzed;
        update_scan_progress(pool, scan_id, processed, discovered).await?;
        if last_progress.elapsed() >= PROGRESS_INTERVAL || processed == total {
            emit_progress(
                &event_sink,
                scan_id,
                project_id,
                processed,
                total,
                discovered,
            );
            last_progress = Instant::now();
        }

        if cancelled.load(Ordering::Relaxed) {
            break;
        }
    }

    let status = if cancelled.load(Ordering::Relaxed) {
        ScanStatus::Cancelled
    } else {
        ScanStatus::Completed
    };
    sqlx::query(
        "UPDATE terminology_scans SET status = ?, processed_segments = ?, \
         discovered_entries = ?, finished_at = datetime('now') WHERE id = ?",
    )
    .bind(status.as_str())
    .bind(processed)
    .bind(discovered)
    .bind(scan_id)
    .execute(pool)
    .await?;
    emit_progress(
        &event_sink,
        scan_id,
        project_id,
        processed,
        total,
        discovered,
    );

    Ok(ScanSummary {
        scan_id: scan_id.to_string(),
        project_id: project_id.to_string(),
        status,
        processed,
        analyzed,
        skipped,
        discovered,
        total,
    })
}

async fn load_segment_page(
    pool: &SqlitePool,
    project_id: &str,
    cursor: Option<&str>,
) -> Result<Vec<SegmentRow>> {
    sqlx::query_as(
        "SELECT segment.id, segment.source_text, segment.segment_kind, segment.json_key, \
                segment.speaker, segment.context_json, file.file_name, file.file_type, \
                scanned.source_hash AS previous_hash \
         FROM segments segment \
         JOIN source_files file ON file.id = segment.source_file_id \
         LEFT JOIN terminology_segment_scans scanned ON scanned.segment_id = segment.id \
         WHERE file.project_id = ? AND (? IS NULL OR segment.id > ?) \
         ORDER BY segment.id LIMIT ?",
    )
    .bind(project_id)
    .bind(cursor)
    .bind(cursor)
    .bind(SCAN_CHUNK_SIZE)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

fn analyze_segment(
    analyzer: &dyn MorphologicalAnalyzer,
    adapter: &dyn crate::engines::terminology::EngineTerminologyAdapter,
    segment: &SegmentRow,
    source_language: &str,
) -> Result<Vec<AnalyzedTerm>> {
    let context = EngineSegmentContext {
        source_text: &segment.source_text,
        segment_kind: &segment.segment_kind,
        file_name: &segment.file_name,
        file_type: &segment.file_type,
        json_key: &segment.json_key,
        speaker: segment.speaker.as_deref(),
        context_json: segment.context_json.as_deref(),
    };
    let seeds = adapter.seeds(&context);
    let seed_keys = seeds
        .iter()
        .filter_map(|seed| normalize_term(&seed.source_text, source_language).ok())
        .collect::<HashSet<_>>();
    let mut terms = seeds
        .into_iter()
        .map(AnalyzedTerm::from)
        .collect::<Vec<_>>();
    let safe_text = adapter.text_for_morphology(&context);
    let tokens = analyzer.analyze(&safe_text)?;
    let mut token_terms: HashMap<(String, String), AnalyzedTerm> = HashMap::new();
    for token in tokens.into_iter().filter(is_terminology_candidate) {
        let normalized = normalize_term(&token.lemma, source_language)?;
        if seed_keys.contains(&normalized) {
            continue;
        }
        let key = (normalized, token.surface.clone());
        token_terms
            .entry(key)
            .and_modify(|term| term.occurrence_count += 1)
            .or_insert_with(|| AnalyzedTerm::from(token));
    }
    terms.extend(token_terms.into_values());
    Ok(terms)
}

impl From<EngineTermSeed> for AnalyzedTerm {
    fn from(seed: EngineTermSeed) -> Self {
        Self {
            surface_text: seed.source_text.clone(),
            canonical_text: seed.source_text,
            reading: None,
            part_of_speech: seed.part_of_speech,
            semantic_type: seed.semantic_type,
            origin: "engine",
            confidence: seed.confidence,
            occurrence_count: 1,
        }
    }
}

impl From<LinguisticToken> for AnalyzedTerm {
    fn from(token: LinguisticToken) -> Self {
        Self {
            canonical_text: token.lemma,
            surface_text: token.surface,
            reading: token.reading,
            part_of_speech: token.part_of_speech,
            semantic_type: "general".to_string(),
            origin: "lindera",
            confidence: if token.is_unknown { 0.5 } else { 0.8 },
            occurrence_count: 1,
        }
    }
}

async fn persist_segment(
    transaction: &mut Transaction<'_, Sqlite>,
    project_id: &str,
    source_language: &str,
    analyzer_version: &str,
    analyzed: &AnalyzedSegment,
) -> Result<i64> {
    sqlx::query("DELETE FROM terminology_occurrences WHERE segment_id = ?")
        .bind(&analyzed.segment.id)
        .execute(&mut **transaction)
        .await?;
    let mut discovered = 0_i64;
    for term in &analyzed.terms {
        let normalized = normalize_term(&term.canonical_text, source_language)?;
        let new_id = uuid::Uuid::new_v4().to_string();
        let entry_id: String = sqlx::query_scalar(
            "INSERT INTO terminology_entries (\
                id, source_language, canonical_text, normalized_text, reading, \
                part_of_speech, semantic_type, sense_key, status, origin, confidence\
             ) VALUES (?, ?, ?, ?, ?, ?, ?, '', 'active', ?, ?) \
             ON CONFLICT(source_language, normalized_text, semantic_type, sense_key) \
             DO UPDATE SET \
                reading = COALESCE(terminology_entries.reading, excluded.reading), \
                origin = CASE \
                    WHEN terminology_entries.origin IN ('manual', 'import') THEN terminology_entries.origin \
                    WHEN excluded.origin = 'engine' THEN 'engine' \
                    ELSE terminology_entries.origin END, \
                confidence = MAX(terminology_entries.confidence, excluded.confidence), \
                updated_at = datetime('now') \
             RETURNING id",
        )
        .bind(&new_id)
        .bind(source_language)
        .bind(term.canonical_text.trim())
        .bind(&normalized)
        .bind(&term.reading)
        .bind(term.part_of_speech.as_str())
        .bind(&term.semantic_type)
        .bind(term.origin)
        .bind(term.confidence)
        .fetch_one(&mut **transaction)
        .await?;
        if entry_id == new_id {
            discovered += 1;
        }

        let normalized_surface = normalize_term(&term.surface_text, source_language)?;
        if normalized_surface != normalized {
            sqlx::query(
                "INSERT INTO terminology_source_variants (\
                    id, entry_id, surface_text, normalized_text, variant_kind\
                 ) VALUES (?, ?, ?, ?, 'inflection') \
                 ON CONFLICT(entry_id, normalized_text, variant_kind) DO NOTHING",
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(&entry_id)
            .bind(&term.surface_text)
            .bind(&normalized_surface)
            .execute(&mut **transaction)
            .await?;
        }
        sqlx::query(
            "INSERT INTO terminology_occurrences (\
                entry_id, project_id, segment_id, surface_text, engine_kind, occurrence_count\
             ) VALUES (?, ?, ?, ?, ?, ?) \
             ON CONFLICT(entry_id, segment_id, surface_text) DO UPDATE SET \
                occurrence_count = excluded.occurrence_count, \
                engine_kind = excluded.engine_kind",
        )
        .bind(&entry_id)
        .bind(project_id)
        .bind(&analyzed.segment.id)
        .bind(&term.surface_text)
        .bind(&analyzed.segment.segment_kind)
        .bind(term.occurrence_count)
        .execute(&mut **transaction)
        .await?;
    }
    sqlx::query(
        "INSERT INTO terminology_segment_scans (\
            segment_id, project_id, source_hash, analyzer_version\
         ) VALUES (?, ?, ?, ?) \
         ON CONFLICT(segment_id) DO UPDATE SET source_hash = excluded.source_hash, \
            analyzer_version = excluded.analyzer_version, scanned_at = datetime('now')",
    )
    .bind(&analyzed.segment.id)
    .bind(project_id)
    .bind(&analyzed.source_hash)
    .bind(analyzer_version)
    .execute(&mut **transaction)
    .await?;
    Ok(discovered)
}

fn segment_hash(segment: &SegmentRow, analyzer_version: &str) -> String {
    let mut hasher = Sha256::new();
    for value in [
        segment.id.as_str(),
        segment.source_text.as_str(),
        segment.segment_kind.as_str(),
        analyzer_version,
    ] {
        hasher.update(value.as_bytes());
        hasher.update([0]);
    }
    hex::encode(hasher.finalize())
}

async fn update_scan_progress(
    pool: &SqlitePool,
    scan_id: &str,
    processed: i64,
    discovered: i64,
) -> Result<()> {
    sqlx::query(
        "UPDATE terminology_scans SET processed_segments = ?, discovered_entries = ? WHERE id = ?",
    )
    .bind(processed)
    .bind(discovered)
    .bind(scan_id)
    .execute(pool)
    .await?;
    Ok(())
}

fn emit_progress(
    sink: &Option<ScanEventSink>,
    scan_id: &str,
    project_id: &str,
    processed: i64,
    total: i64,
    discovered: i64,
) {
    if let Some(sink) = sink {
        sink(ScanEvent::Progress(ScanProgress {
            scan_id: scan_id.to_string(),
            project_id: project_id.to_string(),
            processed,
            total,
            discovered,
        }));
    }
}
