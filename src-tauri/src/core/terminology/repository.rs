use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use sqlx::{FromRow, QueryBuilder, Sqlite, SqlitePool};

use super::normalize::{clean_optional_text, normalize_language, normalize_term};
use super::types::{
    CreateTermInput, Enforcement, EntryStatus, PaginatedTerminology, PartOfSpeech, ReviewStatus,
    TermOrigin, TerminologyContextView, TerminologyEntryView, TerminologyQuery, TerminologyStats,
    TerminologyTranslationView, UpdateTermInput, UpsertTranslationInput,
};
use super::{Result, TerminologyError};

const MAX_PAGE_SIZE: i64 = 200;
const CONTEXTS_PER_ENTRY: i64 = 3;

#[derive(Debug, FromRow)]
struct EntryRow {
    id: String,
    source_language: String,
    canonical_text: String,
    normalized_text: String,
    reading: Option<String>,
    part_of_speech: String,
    semantic_type: String,
    sense_key: String,
    status: String,
    origin: String,
    confidence: f64,
    occurrence_count: i64,
    translation_id: Option<String>,
    target_language: Option<String>,
    translation_project_id: Option<String>,
    target_text: Option<String>,
    review_status: Option<String>,
    enforcement: Option<String>,
    translation_confidence: Option<f64>,
    provider_id: Option<String>,
    model: Option<String>,
}

#[derive(Debug, FromRow)]
struct ContextRow {
    entry_id: String,
    segment_id: String,
    surface_text: String,
    source_text: String,
    engine_kind: String,
    occurrence_count: i64,
}

#[derive(Debug, FromRow)]
struct TranslationRow {
    id: String,
    target_language: String,
    project_id: Option<String>,
    target_text: String,
    review_status: String,
    enforcement: String,
    confidence: f64,
    provider_id: Option<String>,
    model: Option<String>,
}

fn validate_query(query: &TerminologyQuery) -> Result<()> {
    normalize_language(&query.source_language)?;
    normalize_language(&query.target_language)?;
    if query.page < 0 {
        return Err(TerminologyError::InvalidInput(
            "page must be zero or greater".to_string(),
        ));
    }
    if !(1..=MAX_PAGE_SIZE).contains(&query.page_size) {
        return Err(TerminologyError::InvalidInput(format!(
            "page size must be between 1 and {MAX_PAGE_SIZE}"
        )));
    }
    Ok(())
}

fn push_occurrence_join<'args>(
    builder: &mut QueryBuilder<'args, Sqlite>,
    project_id: Option<&'args str>,
) {
    builder.push(
        " LEFT JOIN (SELECT entry_id, SUM(occurrence_count) AS occurrence_count \
          FROM terminology_occurrences",
    );
    if let Some(project_id) = project_id {
        builder.push(" WHERE project_id = ").push_bind(project_id);
    }
    builder.push(" GROUP BY entry_id) occurrence ON occurrence.entry_id = entry.id ");
}

fn push_common_filters<'args>(
    builder: &mut QueryBuilder<'args, Sqlite>,
    query: &'args TerminologyQuery,
) {
    builder
        .push(" WHERE entry.source_language = ")
        .push_bind(query.source_language.trim().to_lowercase());

    if let Some(search) = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let pattern = format!("%{}%", escape_like(search));
        builder
            .push(" AND (entry.canonical_text LIKE ")
            .push_bind(pattern.clone())
            .push(" ESCAPE '\\' OR entry.reading LIKE ")
            .push_bind(pattern)
            .push(" ESCAPE '\\')");
    }
    if let Some(part_of_speech) = query.part_of_speech {
        builder
            .push(" AND entry.part_of_speech = ")
            .push_bind(part_of_speech.as_str());
    }
    if let Some(semantic_type) = query
        .semantic_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        builder
            .push(" AND entry.semantic_type = ")
            .push_bind(semantic_type.to_lowercase());
    }
    if let Some(status) = query.status {
        builder
            .push(" AND entry.status = ")
            .push_bind(status.as_str());
    }
    if let Some(project_id) = query.project_id.as_deref() {
        builder.push(
            " AND (COALESCE(occurrence.occurrence_count, 0) > 0 OR EXISTS (\
                SELECT 1 FROM terminology_translations project_translation \
                WHERE project_translation.entry_id = entry.id \
                  AND project_translation.project_id = ",
        );
        builder
            .push_bind(project_id)
            .push(" AND project_translation.target_language = ")
            .push_bind(query.target_language.trim().to_lowercase())
            .push("))");
    }
}

pub async fn list(pool: &SqlitePool, query: &TerminologyQuery) -> Result<PaginatedTerminology> {
    validate_query(query)?;

    let mut count_builder =
        QueryBuilder::<Sqlite>::new("SELECT COUNT(*) FROM terminology_entries entry");
    push_occurrence_join(&mut count_builder, query.project_id.as_deref());
    push_common_filters(&mut count_builder, query);
    let total: i64 = count_builder.build_query_scalar().fetch_one(pool).await?;

    let mut page_builder = QueryBuilder::<Sqlite>::new(
        "SELECT entry.id, entry.source_language, entry.canonical_text, \
                entry.normalized_text, entry.reading, entry.part_of_speech, \
                entry.semantic_type, entry.sense_key, entry.status, entry.origin, \
                entry.confidence, COALESCE(occurrence.occurrence_count, 0) AS occurrence_count, \
                translation.id AS translation_id, \
                translation.target_language AS target_language, \
                translation.project_id AS translation_project_id, \
                translation.target_text AS target_text, \
                translation.review_status AS review_status, \
                translation.enforcement AS enforcement, \
                translation.confidence AS translation_confidence, \
                translation.provider_id AS provider_id, translation.model AS model \
         FROM terminology_entries entry",
    );
    push_occurrence_join(&mut page_builder, query.project_id.as_deref());
    page_builder.push(
        " LEFT JOIN terminology_translations translation ON translation.id = (\
             SELECT candidate.id FROM terminology_translations candidate \
             WHERE candidate.entry_id = entry.id AND candidate.target_language = ",
    );
    page_builder.push_bind(query.target_language.trim().to_lowercase());
    if let Some(project_id) = query.project_id.as_deref() {
        page_builder
            .push(" AND (candidate.project_id = ")
            .push_bind(project_id)
            .push(" OR candidate.project_id IS NULL) ORDER BY CASE WHEN candidate.project_id = ")
            .push_bind(project_id)
            .push(" THEN 0 ELSE 1 END, candidate.updated_at DESC LIMIT 1) ");
    } else {
        page_builder
            .push(" AND candidate.project_id IS NULL ORDER BY candidate.updated_at DESC LIMIT 1) ");
    }
    push_common_filters(&mut page_builder, query);
    page_builder
        .push(" ORDER BY entry.normalized_text ASC, entry.semantic_type ASC LIMIT ")
        .push_bind(query.page_size)
        .push(" OFFSET ")
        .push_bind(query.page * query.page_size);

    let rows: Vec<EntryRow> = page_builder.build_query_as().fetch_all(pool).await?;
    let entry_ids = rows.iter().map(|row| row.id.clone()).collect::<Vec<_>>();
    let translation_ids = rows
        .iter()
        .filter_map(|row| row.translation_id.clone())
        .collect::<Vec<_>>();
    let contexts = load_contexts(pool, &entry_ids, query.project_id.as_deref()).await?;
    let variants = load_target_variants(pool, &translation_ids).await?;

    let items = rows
        .into_iter()
        .map(|row| row.into_view(&contexts, &variants))
        .collect::<Result<Vec<_>>>()?;

    Ok(PaginatedTerminology {
        items,
        total,
        page: query.page,
        page_size: query.page_size,
    })
}

async fn load_contexts(
    pool: &SqlitePool,
    entry_ids: &[String],
    project_id: Option<&str>,
) -> Result<HashMap<String, Vec<TerminologyContextView>>> {
    if entry_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let mut builder = QueryBuilder::<Sqlite>::new(
        "WITH ranked AS (\
           SELECT occurrence.entry_id, occurrence.segment_id, occurrence.surface_text, \
                  segment.source_text, occurrence.engine_kind, occurrence.occurrence_count, \
                  ROW_NUMBER() OVER (\
                    PARTITION BY occurrence.entry_id \
                    ORDER BY occurrence.occurrence_count DESC, occurrence.segment_id ASC\
                  ) AS context_rank \
           FROM terminology_occurrences occurrence \
           JOIN segments segment ON segment.id = occurrence.segment_id \
           WHERE occurrence.entry_id IN (",
    );
    let mut separated = builder.separated(", ");
    for entry_id in entry_ids {
        separated.push_bind(entry_id);
    }
    separated.push_unseparated(")");
    if let Some(project_id) = project_id {
        builder
            .push(" AND occurrence.project_id = ")
            .push_bind(project_id);
    }
    builder.push(
        ") SELECT entry_id, segment_id, surface_text, source_text, engine_kind, occurrence_count \
           FROM ranked WHERE context_rank <= ",
    );
    builder
        .push_bind(CONTEXTS_PER_ENTRY)
        .push(" ORDER BY entry_id, context_rank");

    let rows: Vec<ContextRow> = builder.build_query_as().fetch_all(pool).await?;
    let mut contexts: HashMap<String, Vec<TerminologyContextView>> = HashMap::new();
    for row in rows {
        contexts
            .entry(row.entry_id)
            .or_default()
            .push(TerminologyContextView {
                segment_id: row.segment_id,
                surface_text: row.surface_text,
                source_text: row.source_text,
                engine_kind: row.engine_kind,
                occurrence_count: row.occurrence_count,
            });
    }
    Ok(contexts)
}

async fn load_target_variants(
    pool: &SqlitePool,
    translation_ids: &[String],
) -> Result<HashMap<String, Vec<String>>> {
    if translation_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let mut builder = QueryBuilder::<Sqlite>::new(
        "SELECT translation_id, text FROM terminology_target_variants WHERE translation_id IN (",
    );
    let mut separated = builder.separated(", ");
    for translation_id in translation_ids {
        separated.push_bind(translation_id);
    }
    separated.push_unseparated(") ORDER BY translation_id, normalized_text");

    let rows: Vec<(String, String)> = builder.build_query_as().fetch_all(pool).await?;
    let mut variants: HashMap<String, Vec<String>> = HashMap::new();
    for (translation_id, text) in rows {
        variants.entry(translation_id).or_default().push(text);
    }
    Ok(variants)
}

impl EntryRow {
    fn into_view(
        self,
        contexts: &HashMap<String, Vec<TerminologyContextView>>,
        variants: &HashMap<String, Vec<String>>,
    ) -> Result<TerminologyEntryView> {
        let translation = match (
            self.translation_id,
            self.target_language,
            self.target_text,
            self.review_status,
            self.enforcement,
            self.translation_confidence,
        ) {
            (
                Some(id),
                Some(target_language),
                Some(target_text),
                Some(review),
                Some(enforcement),
                Some(confidence),
            ) => Some(TerminologyTranslationView {
                accepted_variants: variants.get(&id).cloned().unwrap_or_default(),
                id,
                target_language,
                project_id: self.translation_project_id,
                target_text,
                review_status: ReviewStatus::from_str(&review)?,
                enforcement: Enforcement::from_str(&enforcement)?,
                confidence,
                provider_id: self.provider_id,
                model: self.model,
            }),
            (None, None, None, None, None, None) => None,
            _ => {
                return Err(TerminologyError::InvalidInput(format!(
                    "incomplete translation row for entry {}",
                    self.id
                )))
            }
        };

        Ok(TerminologyEntryView {
            contexts: contexts.get(&self.id).cloned().unwrap_or_default(),
            id: self.id,
            source_language: self.source_language,
            canonical_text: self.canonical_text,
            normalized_text: self.normalized_text,
            reading: self.reading,
            part_of_speech: PartOfSpeech::from_str(&self.part_of_speech)?,
            semantic_type: self.semantic_type,
            sense_key: self.sense_key,
            status: EntryStatus::from_str(&self.status)?,
            origin: TermOrigin::from_str(&self.origin)?,
            confidence: self.confidence,
            occurrence_count: self.occurrence_count,
            translation,
        })
    }
}

pub async fn create_entry(
    pool: &SqlitePool,
    input: &CreateTermInput,
) -> Result<TerminologyEntryView> {
    let source_language = normalize_language(&input.source_language)?;
    let canonical_text = input.canonical_text.trim();
    let normalized_text = normalize_term(canonical_text, &source_language)?;
    let semantic_type = normalized_semantic_type(&input.semantic_type)?;
    let id = uuid::Uuid::new_v4().to_string();
    let reading = clean_optional_text(input.reading.as_deref());
    let sense_key = input.sense_key.trim();

    sqlx::query(
        "INSERT INTO terminology_entries (\
            id, source_language, canonical_text, normalized_text, reading, \
            part_of_speech, semantic_type, sense_key, status, origin, confidence\
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'active', 'manual', 1.0)",
    )
    .bind(&id)
    .bind(&source_language)
    .bind(canonical_text)
    .bind(&normalized_text)
    .bind(&reading)
    .bind(input.part_of_speech.as_str())
    .bind(&semantic_type)
    .bind(sense_key)
    .execute(pool)
    .await?;

    Ok(TerminologyEntryView {
        id,
        source_language,
        canonical_text: canonical_text.to_string(),
        normalized_text,
        reading,
        part_of_speech: input.part_of_speech,
        semantic_type,
        sense_key: sense_key.to_string(),
        status: EntryStatus::Active,
        origin: TermOrigin::Manual,
        confidence: 1.0,
        occurrence_count: 0,
        translation: None,
        contexts: Vec::new(),
    })
}

pub async fn update_entry(
    pool: &SqlitePool,
    input: &UpdateTermInput,
) -> Result<TerminologyEntryView> {
    let source_language: String =
        sqlx::query_scalar("SELECT source_language FROM terminology_entries WHERE id = ?")
            .bind(&input.id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| TerminologyError::NotFound(input.id.clone()))?;
    let canonical_text = input.canonical_text.trim();
    let normalized_text = normalize_term(canonical_text, &source_language)?;
    let semantic_type = normalized_semantic_type(&input.semantic_type)?;
    let reading = clean_optional_text(input.reading.as_deref());
    let sense_key = input.sense_key.trim();

    sqlx::query(
        "UPDATE terminology_entries SET canonical_text = ?, normalized_text = ?, reading = ?, \
            part_of_speech = ?, semantic_type = ?, sense_key = ?, status = ?, \
            updated_at = datetime('now') WHERE id = ?",
    )
    .bind(canonical_text)
    .bind(&normalized_text)
    .bind(&reading)
    .bind(input.part_of_speech.as_str())
    .bind(&semantic_type)
    .bind(sense_key)
    .bind(input.status.as_str())
    .bind(&input.id)
    .execute(pool)
    .await?;

    Ok(TerminologyEntryView {
        id: input.id.clone(),
        source_language,
        canonical_text: canonical_text.to_string(),
        normalized_text,
        reading,
        part_of_speech: input.part_of_speech,
        semantic_type,
        sense_key: sense_key.to_string(),
        status: input.status,
        origin: current_origin(pool, &input.id).await?,
        confidence: current_confidence(pool, &input.id).await?,
        occurrence_count: occurrence_count(pool, &input.id, None).await?,
        translation: None,
        contexts: Vec::new(),
    })
}

pub async fn archive_entry(pool: &SqlitePool, entry_id: &str) -> Result<()> {
    let result = sqlx::query(
        "UPDATE terminology_entries SET status = 'archived', updated_at = datetime('now') \
         WHERE id = ?",
    )
    .bind(entry_id)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(TerminologyError::NotFound(entry_id.to_string()));
    }
    Ok(())
}

pub async fn upsert_translation(
    pool: &SqlitePool,
    input: &UpsertTranslationInput,
) -> Result<TerminologyTranslationView> {
    let target_language = normalize_language(&input.target_language)?;
    let target_text = input.target_text.trim();
    if target_text.is_empty() {
        return Err(TerminologyError::InvalidInput(
            "translation text must not be empty".to_string(),
        ));
    }
    if !(0.0..=1.0).contains(&input.confidence) {
        return Err(TerminologyError::InvalidInput(
            "translation confidence must be between 0 and 1".to_string(),
        ));
    }

    let mut transaction = pool.begin().await?;
    let new_id = uuid::Uuid::new_v4().to_string();
    if let Some(project_id) = input.project_id.as_deref() {
        sqlx::query(
            "INSERT INTO terminology_translations (\
                id, entry_id, target_language, project_id, target_text, review_status, \
                enforcement, confidence, provider_id, model\
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(entry_id, target_language, project_id) WHERE project_id IS NOT NULL \
             DO UPDATE SET target_text = excluded.target_text, \
                review_status = excluded.review_status, enforcement = excluded.enforcement, \
                confidence = excluded.confidence, provider_id = excluded.provider_id, \
                model = excluded.model, updated_at = datetime('now')",
        )
        .bind(&new_id)
        .bind(&input.entry_id)
        .bind(&target_language)
        .bind(project_id)
        .bind(target_text)
        .bind(input.review_status.as_str())
        .bind(input.enforcement.as_str())
        .bind(input.confidence)
        .bind(&input.provider_id)
        .bind(&input.model)
        .execute(&mut *transaction)
        .await?;
    } else {
        sqlx::query(
            "INSERT INTO terminology_translations (\
                id, entry_id, target_language, project_id, target_text, review_status, \
                enforcement, confidence, provider_id, model\
             ) VALUES (?, ?, ?, NULL, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(entry_id, target_language) WHERE project_id IS NULL \
             DO UPDATE SET target_text = excluded.target_text, \
                review_status = excluded.review_status, enforcement = excluded.enforcement, \
                confidence = excluded.confidence, provider_id = excluded.provider_id, \
                model = excluded.model, updated_at = datetime('now')",
        )
        .bind(&new_id)
        .bind(&input.entry_id)
        .bind(&target_language)
        .bind(target_text)
        .bind(input.review_status.as_str())
        .bind(input.enforcement.as_str())
        .bind(input.confidence)
        .bind(&input.provider_id)
        .bind(&input.model)
        .execute(&mut *transaction)
        .await?;
    }

    let row: TranslationRow = sqlx::query_as(
        "SELECT id, target_language, project_id, target_text, review_status, enforcement, \
                confidence, provider_id, model FROM terminology_translations \
         WHERE entry_id = ? AND target_language = ? AND \
               ((project_id = ?) OR (project_id IS NULL AND ? IS NULL))",
    )
    .bind(&input.entry_id)
    .bind(&target_language)
    .bind(input.project_id.as_deref())
    .bind(input.project_id.as_deref())
    .fetch_one(&mut *transaction)
    .await?;

    sqlx::query("DELETE FROM terminology_target_variants WHERE translation_id = ?")
        .bind(&row.id)
        .execute(&mut *transaction)
        .await?;
    let mut unique_variants = HashSet::new();
    let mut accepted_variants = Vec::new();
    for variant in &input.accepted_variants {
        let text = variant.trim();
        if text.is_empty() {
            continue;
        }
        let normalized = normalize_term(text, &target_language)?;
        if unique_variants.insert(normalized.clone()) {
            sqlx::query(
                "INSERT INTO terminology_target_variants \
                 (id, translation_id, text, normalized_text) VALUES (?, ?, ?, ?)",
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(&row.id)
            .bind(text)
            .bind(normalized)
            .execute(&mut *transaction)
            .await?;
            accepted_variants.push(text.to_string());
        }
    }
    transaction.commit().await?;

    Ok(TerminologyTranslationView {
        id: row.id,
        target_language: row.target_language,
        project_id: row.project_id,
        target_text: row.target_text,
        review_status: ReviewStatus::from_str(&row.review_status)?,
        enforcement: Enforcement::from_str(&row.enforcement)?,
        confidence: row.confidence,
        provider_id: row.provider_id,
        model: row.model,
        accepted_variants,
    })
}

pub async fn stats(
    pool: &SqlitePool,
    source_language: &str,
    target_language: &str,
    project_id: Option<&str>,
) -> Result<TerminologyStats> {
    let query = TerminologyQuery {
        source_language: normalize_language(source_language)?,
        target_language: normalize_language(target_language)?,
        project_id: project_id.map(ToOwned::to_owned),
        page_size: 1,
        ..TerminologyQuery::default()
    };
    let mut builder = QueryBuilder::<Sqlite>::new(
        "SELECT COUNT(*), \
           SUM(CASE WHEN effective.review_status IS NULL THEN 1 ELSE 0 END), \
           SUM(CASE WHEN effective.review_status = 'proposed' THEN 1 ELSE 0 END), \
           SUM(CASE WHEN effective.review_status = 'approved' THEN 1 ELSE 0 END), \
           SUM(CASE WHEN effective.review_status = 'locked' THEN 1 ELSE 0 END) \
         FROM terminology_entries entry",
    );
    push_occurrence_join(&mut builder, project_id);
    builder.push(
        " LEFT JOIN terminology_translations effective ON effective.id = (\
           SELECT candidate.id FROM terminology_translations candidate \
           WHERE candidate.entry_id = entry.id AND candidate.target_language = ",
    );
    builder.push_bind(query.target_language.clone());
    if let Some(project_id) = project_id {
        builder
            .push(" AND (candidate.project_id = ")
            .push_bind(project_id)
            .push(" OR candidate.project_id IS NULL) ORDER BY CASE WHEN candidate.project_id = ")
            .push_bind(project_id)
            .push(" THEN 0 ELSE 1 END LIMIT 1) ");
    } else {
        builder.push(" AND candidate.project_id IS NULL LIMIT 1) ");
    }
    push_common_filters(&mut builder, &query);

    let counts: (i64, Option<i64>, Option<i64>, Option<i64>, Option<i64>) =
        builder.build_query_as().fetch_one(pool).await?;
    Ok(TerminologyStats {
        total_entries: counts.0,
        untranslated_entries: counts.1.unwrap_or(0),
        proposed_translations: counts.2.unwrap_or(0),
        approved_translations: counts.3.unwrap_or(0),
        locked_translations: counts.4.unwrap_or(0),
    })
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn normalized_semantic_type(value: &str) -> Result<String> {
    let normalized = value.trim().to_lowercase();
    if normalized.is_empty()
        || !normalized
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return Err(TerminologyError::InvalidInput(format!(
            "invalid semantic type `{value}`"
        )));
    }
    Ok(normalized)
}

async fn current_origin(pool: &SqlitePool, entry_id: &str) -> Result<TermOrigin> {
    let value: String = sqlx::query_scalar("SELECT origin FROM terminology_entries WHERE id = ?")
        .bind(entry_id)
        .fetch_one(pool)
        .await?;
    TermOrigin::from_str(&value)
}

async fn current_confidence(pool: &SqlitePool, entry_id: &str) -> Result<f64> {
    Ok(
        sqlx::query_scalar("SELECT confidence FROM terminology_entries WHERE id = ?")
            .bind(entry_id)
            .fetch_one(pool)
            .await?,
    )
}

async fn occurrence_count(
    pool: &SqlitePool,
    entry_id: &str,
    project_id: Option<&str>,
) -> Result<i64> {
    let count = if let Some(project_id) = project_id {
        sqlx::query_scalar(
            "SELECT COALESCE(SUM(occurrence_count), 0) FROM terminology_occurrences \
             WHERE entry_id = ? AND project_id = ?",
        )
        .bind(entry_id)
        .bind(project_id)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar(
            "SELECT COALESCE(SUM(occurrence_count), 0) FROM terminology_occurrences \
             WHERE entry_id = ?",
        )
        .bind(entry_id)
        .fetch_one(pool)
        .await?
    };
    Ok(count)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;
    use crate::db;

    async fn database() -> (tempfile::NamedTempFile, SqlitePool) {
        let file = tempfile::NamedTempFile::new().unwrap();
        let pool = db::pool::init(file.path().to_str().unwrap()).await.unwrap();
        sqlx::query(
            "INSERT INTO projects \
             (id, name, engine, game_path, source_language, target_language) \
             VALUES ('p1', 'Terms', 'mv_mz', '/tmp', 'ja', 'en')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
             VALUES ('f1', 'p1', 'Actors.json', '/tmp/Actors.json', 'actors')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO segments (id, source_file_id, json_key, source_text, segment_kind) \
             VALUES ('s1', 'f1', '/1/name', '勇者', 'actor_name')",
        )
        .execute(&pool)
        .await
        .unwrap();
        (file, pool)
    }

    fn new_term(text: &str) -> CreateTermInput {
        CreateTermInput {
            source_language: "ja".to_string(),
            canonical_text: text.to_string(),
            reading: None,
            part_of_speech: PartOfSpeech::ProperNoun,
            semantic_type: "character".to_string(),
            sense_key: String::new(),
        }
    }

    fn translation(
        entry_id: &str,
        target: &str,
        project_id: Option<&str>,
    ) -> UpsertTranslationInput {
        UpsertTranslationInput {
            entry_id: entry_id.to_string(),
            target_language: "en".to_string(),
            project_id: project_id.map(ToOwned::to_owned),
            target_text: target.to_string(),
            review_status: ReviewStatus::Approved,
            enforcement: Enforcement::Preferred,
            confidence: 1.0,
            provider_id: None,
            model: None,
            accepted_variants: vec![format!("{target} variant")],
        }
    }

    #[tokio::test]
    async fn project_translation_overrides_global_without_duplicate_entries() {
        let (_file, pool) = database().await;
        let entry = create_entry(&pool, &new_term("勇者")).await.unwrap();
        upsert_translation(&pool, &translation(&entry.id, "Hero", None))
            .await
            .unwrap();
        upsert_translation(&pool, &translation(&entry.id, "Champion", Some("p1")))
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO terminology_occurrences \
             (entry_id, project_id, segment_id, surface_text, engine_kind, occurrence_count) \
             VALUES (?, 'p1', 's1', '勇者', 'actor_name', 2)",
        )
        .bind(&entry.id)
        .execute(&pool)
        .await
        .unwrap();

        let page = list(
            &pool,
            &TerminologyQuery {
                project_id: Some("p1".to_string()),
                ..TerminologyQuery::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].occurrence_count, 2);
        assert_eq!(page.items[0].contexts.len(), 1);
        let effective = page.items[0].translation.as_ref().unwrap();
        assert_eq!(effective.target_text, "Champion");
        assert_eq!(effective.project_id.as_deref(), Some("p1"));
        assert_eq!(effective.accepted_variants, vec!["Champion variant"]);
    }

    #[tokio::test]
    async fn crud_normalizes_updates_archives_and_rejects_empty_targets() {
        let (_file, pool) = database().await;
        let entry = create_entry(&pool, &new_term("  ＲＰＧ　勇者  "))
            .await
            .unwrap();
        assert_eq!(entry.normalized_text, "RPG 勇者");

        let updated = update_entry(
            &pool,
            &UpdateTermInput {
                id: entry.id.clone(),
                canonical_text: "星の勇者".to_string(),
                reading: Some("ほしのゆうしゃ".to_string()),
                part_of_speech: PartOfSpeech::ProperNoun,
                semantic_type: "character".to_string(),
                sense_key: String::new(),
                status: EntryStatus::Ignored,
            },
        )
        .await
        .unwrap();
        assert_eq!(updated.reading.as_deref(), Some("ほしのゆうしゃ"));
        assert_eq!(updated.status, EntryStatus::Ignored);

        let mut invalid_translation = translation(&entry.id, "", None);
        assert!(upsert_translation(&pool, &invalid_translation)
            .await
            .is_err());
        invalid_translation.target_text = "Star Hero".to_string();
        upsert_translation(&pool, &invalid_translation)
            .await
            .unwrap();

        archive_entry(&pool, &entry.id).await.unwrap();
        let status: String =
            sqlx::query_scalar("SELECT status FROM terminology_entries WHERE id = ?")
                .bind(&entry.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(status, "archived");
    }

    #[tokio::test]
    async fn pagination_search_filters_and_stats_are_server_side() {
        let (_file, pool) = database().await;
        let hero = create_entry(&pool, &new_term("勇者")).await.unwrap();
        let demon = create_entry(&pool, &new_term("魔王")).await.unwrap();
        upsert_translation(&pool, &translation(&hero.id, "Hero", None))
            .await
            .unwrap();

        let page = list(
            &pool,
            &TerminologyQuery {
                search: Some("魔".to_string()),
                page_size: 1,
                ..TerminologyQuery::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].id, demon.id);

        let stats = stats(&pool, "ja", "en", None).await.unwrap();
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.untranslated_entries, 1);
        assert_eq!(stats.approved_translations, 1);
    }

    #[tokio::test]
    async fn ten_thousand_entries_remain_bounded_to_one_page() {
        let (_file, pool) = database().await;
        sqlx::query(
            "WITH RECURSIVE counter(value) AS (\
               SELECT 1 UNION ALL SELECT value + 1 FROM counter WHERE value < 10000\
             ) INSERT INTO terminology_entries (\
               id, source_language, canonical_text, normalized_text, part_of_speech, \
               semantic_type, status, origin, confidence\
             ) SELECT printf('bulk-%05d', value), 'ja', printf('用語%05d', value), \
                      printf('用語%05d', value), 'noun', 'general', 'active', 'manual', 1.0 \
               FROM counter",
        )
        .execute(&pool)
        .await
        .unwrap();

        let query = TerminologyQuery {
            page_size: 100,
            ..TerminologyQuery::default()
        };
        let page = list(&pool, &query).await.unwrap();
        assert_eq!(page.total, 10_000);
        assert_eq!(page.items.len(), 100);

        let mut samples = Vec::with_capacity(40);
        for _ in 0..40 {
            let started = Instant::now();
            let measured_page = list(&pool, &query).await.unwrap();
            samples.push(started.elapsed());
            assert_eq!(measured_page.items.len(), 100);
        }
        samples.sort_unstable();
        let p95 = samples[37];
        eprintln!("terminology 10k page query p95: {p95:?}");
        assert!(
            p95 < Duration::from_millis(100),
            "10k page query p95 exceeded 100 ms: {p95:?}"
        );
    }
}
