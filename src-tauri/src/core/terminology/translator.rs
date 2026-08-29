use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, QueryBuilder, Sqlite, SqlitePool};

use super::normalize::{normalize_language, normalize_term};
use super::types::{Enforcement, PartOfSpeech};
use super::{Result, TerminologyError};
use crate::domain::types::ResourceProfile;
use crate::llm::prompts::{lang_code_to_name, terminology_for};
use crate::llm::provider::LlmProvider;

const MAX_SELECTED_TERMS: usize = 500;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TerminologyTranslationProgress {
    pub processed: usize,
    pub total: usize,
}

pub type TranslationProgressSink = Arc<dyn Fn(TerminologyTranslationProgress) + Send + Sync>;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranslateTerminologySummary {
    pub requested: usize,
    pub translated: usize,
    pub scopes_written: usize,
    pub batches: usize,
}

#[derive(Debug, Clone, FromRow)]
struct CandidateRow {
    id: String,
    source_language: String,
    canonical_text: String,
    reading: Option<String>,
    part_of_speech: String,
    semantic_type: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PromptTerm {
    id: String,
    source: String,
    part_of_speech: String,
    semantic_type: String,
    reading: Option<String>,
    contexts: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ProviderTerm {
    id: String,
    target: String,
    confidence: f64,
}

#[derive(Debug)]
struct ValidatedTerm {
    id: String,
    target: String,
    confidence: f64,
    part_of_speech: PartOfSpeech,
}

#[allow(clippy::too_many_arguments)]
pub async fn translate_selected<P: LlmProvider>(
    pool: &SqlitePool,
    provider: &P,
    entry_ids: &[String],
    target_language: &str,
    project_id: Option<&str>,
    also_global: bool,
    profile: ResourceProfile,
    provider_id: &str,
    model: &str,
    progress_sink: Option<TranslationProgressSink>,
) -> Result<TranslateTerminologySummary> {
    if also_global && project_id.is_none() {
        return Err(TerminologyError::InvalidInput(
            "project + global translation requires a project".to_string(),
        ));
    }
    let target_language = normalize_language(target_language)?;
    let unique_ids = unique_requested_ids(entry_ids)?;
    let context_limit = policy(profile).1;
    let batch_size = policy(profile).0;
    let candidates = load_candidates(pool, &unique_ids).await?;
    if candidates.len() != unique_ids.len() {
        return Err(TerminologyError::InvalidInput(
            "one or more terminology entry IDs do not exist or are archived".to_string(),
        ));
    }
    let source_languages = candidates
        .iter()
        .map(|candidate| candidate.source_language.as_str())
        .collect::<HashSet<_>>();
    if source_languages.len() != 1 {
        return Err(TerminologyError::InvalidInput(
            "a terminology translation request must use one source language".to_string(),
        ));
    }
    let source_language = candidates[0].source_language.clone();
    let contexts = load_contexts(pool, &unique_ids, project_id, context_limit).await?;
    let total = candidates.len();
    let mut validated = Vec::with_capacity(total);
    let mut batches = 0;
    for batch in candidates.chunks(batch_size) {
        let prompt_terms = batch
            .iter()
            .map(|candidate| PromptTerm {
                id: candidate.id.clone(),
                source: candidate.canonical_text.clone(),
                part_of_speech: candidate.part_of_speech.clone(),
                semantic_type: candidate.semantic_type.clone(),
                reading: candidate.reading.clone(),
                contexts: contexts.get(&candidate.id).cloned().unwrap_or_default(),
            })
            .collect::<Vec<_>>();
        let template = terminology_for(&target_language);
        let system = template.render(
            &template.system,
            &[
                ("source_lang", lang_code_to_name(&source_language)),
                ("target_lang", lang_code_to_name(&target_language)),
            ],
        );
        let terms_json = serde_json::to_string(&prompt_terms)
            .map_err(|error| TerminologyError::Analyzer(error.to_string()))?;
        let user = template.render(
            &template.user,
            &[
                ("target_lang", lang_code_to_name(&target_language)),
                ("terms", &terms_json),
            ],
        );
        let raw = provider
            .chat(&system, &user)
            .await
            .map_err(|error| TerminologyError::Analyzer(error.to_string()))?;
        validated.extend(validate_response(
            &raw,
            batch,
            &source_language,
            &target_language,
        )?);
        batches += 1;
        if let Some(sink) = &progress_sink {
            sink(TerminologyTranslationProgress {
                processed: validated.len(),
                total,
            });
        }
        let delay = match profile {
            ResourceProfile::Eco => 500,
            ResourceProfile::Balanced => 100,
            ResourceProfile::Fast => 0,
        };
        if delay > 0 && validated.len() < total {
            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
        }
    }

    let mut transaction = pool.begin().await?;
    let mut translated = 0;
    let mut scopes_written = 0;
    for term in validated {
        let enforcement = default_enforcement(term.part_of_speech);
        let scopes = if also_global {
            vec![project_id, None]
        } else {
            vec![project_id]
        };
        for scope_project_id in scopes {
            let translation_id = uuid::Uuid::new_v4().to_string();
            let query = if scope_project_id.is_some() {
                "INSERT INTO terminology_translations (\
                    id, entry_id, target_language, project_id, target_text, enforcement, \
                    confidence, provider_id, model\
                 ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) \
                 ON CONFLICT(entry_id, target_language, project_id) WHERE project_id IS NOT NULL \
                 DO UPDATE SET target_text = excluded.target_text, \
                    enforcement = excluded.enforcement, confidence = excluded.confidence, \
                    provider_id = excluded.provider_id, model = excluded.model, updated_at = datetime('now')"
            } else {
                "INSERT INTO terminology_translations (\
                    id, entry_id, target_language, project_id, target_text, enforcement, \
                    confidence, provider_id, model\
                 ) VALUES (?, ?, ?, NULL, ?, ?, ?, ?, ?) \
                 ON CONFLICT(entry_id, target_language) WHERE project_id IS NULL \
                 DO UPDATE SET target_text = excluded.target_text, \
                    enforcement = excluded.enforcement, confidence = excluded.confidence, \
                    provider_id = excluded.provider_id, model = excluded.model, updated_at = datetime('now')"
            };
            let mut query = sqlx::query(query)
                .bind(translation_id)
                .bind(&term.id)
                .bind(&target_language);
            if let Some(scope_project_id) = scope_project_id {
                query = query.bind(scope_project_id);
            }
            query
                .bind(&term.target)
                .bind(enforcement.as_str())
                .bind(term.confidence)
                .bind(provider_id)
                .bind(model)
                .execute(&mut *transaction)
                .await?;
            scopes_written += 1;
        }
        translated += 1;
    }
    transaction.commit().await?;
    Ok(TranslateTerminologySummary {
        requested: total,
        translated,
        scopes_written,
        batches,
    })
}

fn unique_requested_ids(entry_ids: &[String]) -> Result<Vec<String>> {
    if entry_ids.is_empty() || entry_ids.len() > MAX_SELECTED_TERMS {
        return Err(TerminologyError::InvalidInput(format!(
            "select between 1 and {MAX_SELECTED_TERMS} terminology entries"
        )));
    }
    let mut seen = HashSet::new();
    let mut ids = Vec::with_capacity(entry_ids.len());
    for id in entry_ids {
        let id = id.trim();
        if id.is_empty() || !seen.insert(id.to_string()) {
            return Err(TerminologyError::InvalidInput(
                "terminology entry IDs must be non-empty and unique".to_string(),
            ));
        }
        ids.push(id.to_string());
    }
    Ok(ids)
}

async fn load_candidates(pool: &SqlitePool, ids: &[String]) -> Result<Vec<CandidateRow>> {
    let mut builder = QueryBuilder::<Sqlite>::new(
        "SELECT id, source_language, canonical_text, reading, part_of_speech, semantic_type \
         FROM terminology_entries WHERE status = 'active' AND id IN (",
    );
    let mut separated = builder.separated(",");
    for id in ids {
        separated.push_bind(id);
    }
    separated.push_unseparated(") ORDER BY id");
    builder
        .build_query_as()
        .fetch_all(pool)
        .await
        .map_err(Into::into)
}

async fn load_contexts(
    pool: &SqlitePool,
    ids: &[String],
    project_id: Option<&str>,
    limit: i64,
) -> Result<HashMap<String, Vec<String>>> {
    let mut builder = QueryBuilder::<Sqlite>::new(
        "WITH ranked AS (SELECT occurrence.entry_id, segment.source_text, \
         ROW_NUMBER() OVER (PARTITION BY occurrence.entry_id ORDER BY occurrence.occurrence_count DESC, segment.id) AS rank \
         FROM terminology_occurrences occurrence JOIN segments segment ON segment.id = occurrence.segment_id \
         WHERE occurrence.entry_id IN (",
    );
    let mut separated = builder.separated(",");
    for id in ids {
        separated.push_bind(id);
    }
    separated.push_unseparated(")");
    if let Some(project_id) = project_id {
        builder
            .push(" AND occurrence.project_id = ")
            .push_bind(project_id);
    }
    builder
        .push(") SELECT entry_id, source_text FROM ranked WHERE rank <= ")
        .push_bind(limit);
    let rows: Vec<(String, String)> = builder.build_query_as().fetch_all(pool).await?;
    let mut contexts = HashMap::new();
    for (entry_id, source_text) in rows {
        contexts
            .entry(entry_id)
            .or_insert_with(Vec::new)
            .push(source_text);
    }
    Ok(contexts)
}

fn validate_response(
    raw: &str,
    candidates: &[CandidateRow],
    source_language: &str,
    target_language: &str,
) -> Result<Vec<ValidatedTerm>> {
    let response: Vec<ProviderTerm> = serde_json::from_str(raw.trim()).map_err(|error| {
        TerminologyError::InvalidInput(format!("invalid terminology JSON response: {error}"))
    })?;
    let expected = candidates
        .iter()
        .map(|row| row.id.as_str())
        .collect::<HashSet<_>>();
    let by_id = candidates
        .iter()
        .map(|row| (row.id.as_str(), row))
        .collect::<HashMap<_, _>>();
    let mut seen = HashSet::new();
    let mut validated = Vec::with_capacity(response.len());
    for term in response {
        if !expected.contains(term.id.as_str()) || !seen.insert(term.id.clone()) {
            return Err(TerminologyError::InvalidInput(
                "terminology response contains an unknown or duplicate ID".to_string(),
            ));
        }
        let target = term.target.trim();
        if target.is_empty() || !(0.0..=1.0).contains(&term.confidence) {
            return Err(TerminologyError::InvalidInput(
                "terminology response contains an empty target or invalid confidence".to_string(),
            ));
        }
        let candidate = by_id[term.id.as_str()];
        if source_language.starts_with("ja")
            && matches!(target_language, "en" | "fr")
            && normalize_term(target, target_language)?
                == normalize_term(&candidate.canonical_text, source_language)?
        {
            return Err(TerminologyError::InvalidInput(
                "terminology response copied Japanese source text".to_string(),
            ));
        }
        validated.push(ValidatedTerm {
            id: term.id,
            target: target.to_string(),
            confidence: term.confidence,
            part_of_speech: PartOfSpeech::from_str(&candidate.part_of_speech)?,
        });
    }
    if seen.len() != expected.len() {
        return Err(TerminologyError::InvalidInput(
            "terminology response omitted one or more IDs".to_string(),
        ));
    }
    Ok(validated)
}

fn policy(profile: ResourceProfile) -> (usize, i64) {
    match profile {
        ResourceProfile::Eco => (8, 1),
        ResourceProfile::Balanced => (20, 2),
        ResourceProfile::Fast => (50, 3),
    }
}

fn default_enforcement(part_of_speech: PartOfSpeech) -> Enforcement {
    match part_of_speech {
        PartOfSpeech::Noun | PartOfSpeech::ProperNoun | PartOfSpeech::Unknown => {
            Enforcement::Preferred
        }
        PartOfSpeech::Verb
        | PartOfSpeech::Adjective
        | PartOfSpeech::Adverb
        | PartOfSpeech::Expression => Enforcement::Contextual,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::llm::provider::{LlmError, TranslationContext};

    enum Reply {
        Valid(&'static str),
        Raw(String),
    }

    struct MockProvider {
        reply: Reply,
        batch_sizes: Mutex<Vec<usize>>,
    }

    impl MockProvider {
        fn valid(prefix: &'static str) -> Self {
            Self {
                reply: Reply::Valid(prefix),
                batch_sizes: Mutex::new(Vec::new()),
            }
        }
    }

    impl LlmProvider for MockProvider {
        async fn translate(
            &self,
            segments: Vec<String>,
            _context: TranslationContext,
        ) -> std::result::Result<Vec<String>, LlmError> {
            Ok(segments)
        }

        async fn health_check(&self) -> std::result::Result<(), LlmError> {
            Ok(())
        }

        async fn chat(&self, _system: &str, user: &str) -> std::result::Result<String, LlmError> {
            if let Reply::Raw(raw) = &self.reply {
                return Ok(raw.clone());
            }
            let start = user
                .find('[')
                .ok_or_else(|| LlmError::ResponseFormat("missing terms".into()))?;
            let inputs: Vec<serde_json::Value> = serde_json::from_str(&user[start..])
                .map_err(|error| LlmError::ResponseFormat(error.to_string()))?;
            self.batch_sizes.lock().unwrap().push(inputs.len());
            let Reply::Valid(prefix) = self.reply else {
                unreachable!()
            };
            Ok(serde_json::to_string(
                &inputs
                    .iter()
                    .map(|input| {
                        serde_json::json!({
                            "id": input["id"],
                            "target": format!("{prefix} {}", input["id"].as_str().unwrap()),
                            "confidence": 0.9
                        })
                    })
                    .collect::<Vec<_>>(),
            )
            .unwrap())
        }
    }

    async fn pool_with_entries(count: usize) -> (tempfile::TempDir, SqlitePool, Vec<String>) {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("translator.db");
        let pool = crate::db::pool::init(database.to_str().unwrap())
            .await
            .unwrap();
        let mut ids = Vec::new();
        for index in 0..count {
            let id = format!("entry-{index:03}");
            let part_of_speech = if index % 2 == 0 { "noun" } else { "verb" };
            sqlx::query(
                "INSERT INTO terminology_entries (\
                    id, source_language, canonical_text, normalized_text, part_of_speech, \
                    semantic_type, sense_key, status, origin, confidence\
                 ) VALUES (?, 'ja', ?, ?, ?, 'general', '', 'active', 'manual', 1.0)",
            )
            .bind(&id)
            .bind(format!("用語{index}"))
            .bind(format!("用語{index}"))
            .bind(part_of_speech)
            .execute(&pool)
            .await
            .unwrap();
            ids.push(id);
        }
        (directory, pool, ids)
    }

    #[tokio::test]
    async fn japanese_terms_become_usable_translations_for_english_and_french() {
        for (language, prefix) in [("en", "Term"), ("fr", "Terme")] {
            let (_directory, pool, ids) = pool_with_entries(2).await;
            let provider = MockProvider::valid(prefix);
            let summary = translate_selected(
                &pool,
                &provider,
                &ids,
                language,
                None,
                false,
                ResourceProfile::Balanced,
                "mock",
                "mock-model",
                None,
            )
            .await
            .unwrap();
            assert_eq!((summary.translated, summary.batches), (2, 1));
            let rows: Vec<(String, String)> = sqlx::query_as(
                "SELECT enforcement, target_language \
                 FROM terminology_translations ORDER BY entry_id",
            )
            .fetch_all(&pool)
            .await
            .unwrap();
            assert_eq!(rows[0], ("preferred".into(), language.into()));
            assert_eq!(rows[1], ("contextual".into(), language.into()));
        }
    }

    #[tokio::test]
    async fn invalid_protocol_persists_nothing() {
        let (_directory, pool, ids) = pool_with_entries(2).await;
        for raw in [
            "not json".to_string(),
            format!(
                r#"[{{"id":"{}","target":"Hero","confidence":0.9}}]"#,
                ids[0]
            ),
            format!(
                r#"[{{"id":"unknown","target":"Hero","confidence":0.9}},{{"id":"{}","target":"Run","confidence":0.9}}]"#,
                ids[1]
            ),
        ] {
            let provider = MockProvider {
                reply: Reply::Raw(raw),
                batch_sizes: Mutex::new(Vec::new()),
            };
            assert!(translate_selected(
                &pool,
                &provider,
                &ids,
                "en",
                None,
                false,
                ResourceProfile::Balanced,
                "mock",
                "model",
                None,
            )
            .await
            .is_err());
        }
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM terminology_translations")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn project_and_global_scope_writes_both_rows_atomically() {
        let (_directory, pool, ids) = pool_with_entries(1).await;
        sqlx::query(
            "INSERT INTO projects (id, name, engine, game_path, source_language, target_language) \
             VALUES ('p1', 'Game', 'mv_mz', '/tmp/game', 'ja', 'en')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let summary = translate_selected(
            &pool,
            &MockProvider::valid("Term"),
            &ids,
            "en",
            Some("p1"),
            true,
            ResourceProfile::Fast,
            "mock",
            "model",
            None,
        )
        .await
        .unwrap();
        assert_eq!((summary.translated, summary.scopes_written), (1, 2));
        let scopes: Vec<Option<String>> = sqlx::query_scalar(
            "SELECT project_id FROM terminology_translations \
             WHERE entry_id = ? ORDER BY project_id IS NULL",
        )
        .bind(&ids[0])
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(scopes, vec![Some("p1".into()), None]);
    }

    #[tokio::test]
    async fn profiles_bound_sequential_batches_and_explicit_retranslation_overwrites() {
        let (_directory, pool, ids) = pool_with_entries(21).await;
        sqlx::query(
            "INSERT INTO terminology_translations (\
                id, entry_id, target_language, project_id, target_text, enforcement, confidence\
             ) VALUES ('existing', ?, 'en', NULL, 'Old term', 'required', 1.0)",
        )
        .bind(&ids[0])
        .execute(&pool)
        .await
        .unwrap();
        let provider = MockProvider::valid("Term");
        let summary = translate_selected(
            &pool,
            &provider,
            &ids,
            "en",
            None,
            false,
            ResourceProfile::Balanced,
            "mock",
            "model",
            None,
        )
        .await
        .unwrap();
        assert_eq!(provider.batch_sizes.lock().unwrap().as_slice(), &[20, 1]);
        assert_eq!((summary.translated, summary.scopes_written), (21, 21));
        let target: String = sqlx::query_scalar(
            "SELECT target_text FROM terminology_translations WHERE entry_id = ? AND project_id IS NULL",
        )
        .bind(&ids[0])
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(target, "Term entry-000");
    }
}
