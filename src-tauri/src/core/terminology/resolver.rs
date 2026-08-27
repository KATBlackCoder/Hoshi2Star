use std::collections::HashMap;
use std::str::FromStr;

use sqlx::{FromRow, QueryBuilder, Sqlite, SqlitePool};

use super::types::{Enforcement, PartOfSpeech, ReviewStatus};
use super::Result;
use crate::llm::provider::{terminology_prompt_fragment, TerminologyHint};

const MAX_HINTS: usize = 20;

fn fit_prompt_budget(
    candidates: impl IntoIterator<Item = TerminologyHint>,
    estimated_prompt_chars: usize,
) -> Vec<TerminologyHint> {
    let char_budget = estimated_prompt_chars / 10;
    let mut hints = Vec::new();
    for hint in candidates.into_iter().take(MAX_HINTS) {
        let mut candidate = hints.clone();
        candidate.push(hint.clone());
        if terminology_prompt_fragment(&candidate).chars().count() <= char_budget {
            hints.push(hint);
        }
    }
    hints
}

#[derive(Debug, FromRow)]
struct HintRow {
    source: String,
    semantic_type: String,
    part_of_speech: String,
    translation_id: String,
    target: String,
    review_status: String,
    enforcement: String,
}

pub async fn resolve_for_request(
    pool: &SqlitePool,
    segment_ids: &[String],
    target_language: &str,
    estimated_prompt_chars: usize,
) -> Result<Vec<TerminologyHint>> {
    if segment_ids.is_empty() {
        return Ok(Vec::new());
    }
    let Some(project_id) = project_for_segments(pool, segment_ids).await? else {
        return Ok(Vec::new());
    };
    let mut builder = QueryBuilder::<Sqlite>::new(
        "SELECT entry.canonical_text AS source, \
                entry.semantic_type, entry.part_of_speech, \
                translation.id AS translation_id, translation.target_text AS target, \
                translation.review_status, translation.enforcement \
         FROM terminology_occurrences occurrence \
         JOIN terminology_entries entry ON entry.id = occurrence.entry_id \
         JOIN terminology_translations translation ON translation.id = (\
             SELECT candidate.id FROM terminology_translations candidate \
             WHERE candidate.entry_id = entry.id AND candidate.target_language = ",
    );
    builder.push_bind(target_language.trim().to_lowercase());
    builder
        .push(" AND (candidate.project_id = ")
        .push_bind(project_id.clone());
    builder.push(" OR candidate.project_id IS NULL) ORDER BY CASE WHEN candidate.project_id = ");
    builder.push_bind(project_id);
    builder.push(
        " THEN 0 ELSE 1 END, candidate.updated_at DESC LIMIT 1) \
         WHERE entry.status = 'active' AND trim(translation.target_text) != '' \
           AND occurrence.segment_id IN (",
    );
    let mut separated = builder.separated(",");
    for segment_id in segment_ids {
        separated.push_bind(segment_id);
    }
    separated.push_unseparated(
        ") GROUP BY entry.id, translation.id ORDER BY \
        CASE translation.enforcement WHEN 'required' THEN 0 WHEN 'preferred' THEN 1 ELSE 2 END, \
        SUM(occurrence.occurrence_count) DESC, entry.normalized_text LIMIT ",
    );
    builder.push_bind(MAX_HINTS as i64);
    let rows: Vec<HintRow> = builder.build_query_as().fetch_all(pool).await?;
    let translation_ids = rows
        .iter()
        .map(|row| row.translation_id.clone())
        .collect::<Vec<_>>();
    let variants = load_variants(pool, &translation_ids).await?;
    let mut candidates = Vec::with_capacity(rows.len());
    for row in rows {
        let review_status = ReviewStatus::from_str(&row.review_status)?;
        let mut enforcement = Enforcement::from_str(&row.enforcement)?;
        if review_status == ReviewStatus::Proposed && enforcement == Enforcement::Required {
            enforcement = Enforcement::Preferred;
        }
        let accepted_targets = variants
            .get(&row.translation_id)
            .cloned()
            .unwrap_or_default();
        candidates.push(TerminologyHint {
            source: row.source,
            target: row.target,
            semantic_type: row.semantic_type,
            part_of_speech: PartOfSpeech::from_str(&row.part_of_speech)?,
            enforcement,
            accepted_targets,
        });
    }
    Ok(fit_prompt_budget(candidates, estimated_prompt_chars))
}

async fn project_for_segments(pool: &SqlitePool, segment_ids: &[String]) -> Result<Option<String>> {
    let mut builder = QueryBuilder::<Sqlite>::new(
        "SELECT DISTINCT file.project_id FROM segments segment \
         JOIN source_files file ON file.id = segment.source_file_id \
         WHERE segment.id IN (",
    );
    let mut separated = builder.separated(",");
    for segment_id in segment_ids {
        separated.push_bind(segment_id);
    }
    separated.push_unseparated(") LIMIT 2");
    let projects: Vec<String> = builder.build_query_scalar().fetch_all(pool).await?;
    if projects.len() > 1 {
        return Err(super::TerminologyError::InvalidInput(
            "one provider request cannot mix terminology from multiple projects".into(),
        ));
    }
    Ok(projects.into_iter().next())
}

async fn load_variants(
    pool: &SqlitePool,
    translation_ids: &[String],
) -> Result<HashMap<String, Vec<String>>> {
    if translation_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let mut builder = QueryBuilder::<Sqlite>::new(
        "SELECT translation_id, text FROM terminology_target_variants WHERE translation_id IN (",
    );
    let mut separated = builder.separated(",");
    for translation_id in translation_ids {
        separated.push_bind(translation_id);
    }
    separated.push_unseparated(") ORDER BY translation_id, normalized_text");
    let rows: Vec<(String, String)> = builder.build_query_as().fetch_all(pool).await?;
    let mut variants = HashMap::new();
    for (translation_id, text) in rows {
        variants
            .entry(translation_id)
            .or_insert_with(Vec::new)
            .push(text);
    }
    Ok(variants)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn seeded_pool() -> (tempfile::TempDir, SqlitePool) {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("resolver.db");
        let pool = crate::db::pool::init(database.to_str().unwrap())
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO projects (id, name, engine, game_path, source_language, target_language) \
             VALUES ('p1', 'Game', 'mv_mz', '/tmp/game', 'ja', 'en'); \
             INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
             VALUES ('f1', 'p1', 'Map001.json', '/tmp/Map001.json', 'map'); \
             INSERT INTO segments (id, source_file_id, json_key, source_text, segment_kind) VALUES \
             ('s1', 'f1', '/1', '勇者が来る', 'dialogue'), \
             ('s2', 'f1', '/2', '魔王が来る', 'dialogue');",
        )
        .execute(&pool)
        .await
        .unwrap();
        for (id, source) in [("e1", "勇者"), ("e2", "魔王"), ("e3", "剣")] {
            sqlx::query(
                "INSERT INTO terminology_entries (\
                    id, source_language, canonical_text, normalized_text, part_of_speech, \
                    semantic_type, sense_key, status, origin, confidence\
                 ) VALUES (?, 'ja', ?, ?, 'noun', 'character', '', 'active', 'manual', 1.0)",
            )
            .bind(id)
            .bind(source)
            .bind(source)
            .execute(&pool)
            .await
            .unwrap();
        }
        for (entry, segment, surface) in [("e1", "s1", "勇者"), ("e2", "s2", "魔王")] {
            sqlx::query(
                "INSERT INTO terminology_occurrences \
                 (entry_id, project_id, segment_id, surface_text, engine_kind, occurrence_count) \
                 VALUES (?, 'p1', ?, ?, 'dialogue', 1)",
            )
            .bind(entry)
            .bind(segment)
            .bind(surface)
            .execute(&pool)
            .await
            .unwrap();
        }
        for (id, entry, project, target, review, enforcement) in [
            ("t-global", "e1", None, "Hero", "approved", "required"),
            (
                "t-project",
                "e1",
                Some("p1"),
                "Champion",
                "approved",
                "preferred",
            ),
            (
                "t-proposed",
                "e2",
                None,
                "Demon King",
                "proposed",
                "required",
            ),
            ("t-unused", "e3", None, "Sword", "locked", "required"),
        ] {
            sqlx::query(
                "INSERT INTO terminology_translations (\
                    id, entry_id, target_language, project_id, target_text, review_status, enforcement, confidence\
                 ) VALUES (?, ?, 'en', ?, ?, ?, ?, 1.0)",
            )
            .bind(id)
            .bind(entry)
            .bind(project)
            .bind(target)
            .bind(review)
            .bind(enforcement)
            .execute(&pool)
            .await
            .unwrap();
        }
        sqlx::query(
            "INSERT INTO terminology_target_variants (id, translation_id, text, normalized_text) \
             VALUES ('v1', 't-project', 'The Champion', 'the champion')",
        )
        .execute(&pool)
        .await
        .unwrap();
        (directory, pool)
    }

    #[tokio::test]
    async fn resolves_only_occurring_terms_with_project_override() {
        let (_directory, pool) = seeded_pool().await;
        let hints = resolve_for_request(&pool, &["s1".into()], "en", 2_000)
            .await
            .unwrap();
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].source, "勇者");
        assert_eq!(hints[0].target, "Champion");
        assert_eq!(hints[0].accepted_targets, vec!["The Champion"]);
        assert!(resolve_for_request(&pool, &["missing".into()], "en", 2_000)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn proposed_translation_can_never_be_required() {
        let (_directory, pool) = seeded_pool().await;
        let hints = resolve_for_request(&pool, &["s2".into()], "en", 2_000)
            .await
            .unwrap();
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].target, "Demon King");
        assert_eq!(hints[0].enforcement, Enforcement::Preferred);
    }

    #[tokio::test]
    async fn hint_budget_never_exceeds_ten_percent_of_estimated_prompt() {
        let (_directory, pool) = seeded_pool().await;
        let hints = resolve_for_request(&pool, &["s1".into(), "s2".into()], "en", 100)
            .await
            .unwrap();
        assert!(hints.is_empty(), "a ten-character budget cannot fit a hint");
    }

    #[test]
    fn prompt_budget_holds_across_1191_fixture_segments() {
        #[derive(serde::Deserialize)]
        struct FixtureCase {
            text: String,
        }
        let cases: Vec<FixtureCase> = serde_json::from_str(include_str!(
            "../../../tests/fixtures/terminology/ja_tokens.json"
        ))
        .unwrap();
        let dictionary = [
            ("六花", "Rikka", "character"),
            ("東京", "Tokyo", "location"),
            ("ポーション", "Potion", "item"),
            ("魔王", "Demon King", "character"),
            ("村", "Village", "location"),
            ("夜", "Night", "general"),
            ("町", "Town", "location"),
            ("勇者", "Hero", "character"),
            ("剣", "Sword", "item"),
            ("騎士", "Knight", "character"),
            ("ゴールド", "Gold", "currency"),
            ("星", "Star", "general"),
            ("冒険", "Adventure", "general"),
        ];
        let mut ratios = Vec::with_capacity(1_191);
        for index in 0..1_191 {
            let text = &cases[index % cases.len()].text;
            let candidates = dictionary
                .iter()
                .filter(|(source, _, _)| text.contains(source))
                .map(|(source, target, semantic_type)| TerminologyHint {
                    source: (*source).to_string(),
                    target: (*target).to_string(),
                    semantic_type: (*semantic_type).to_string(),
                    part_of_speech: PartOfSpeech::Noun,
                    enforcement: Enforcement::Preferred,
                    accepted_targets: vec![],
                });
            let estimated_prompt_chars = 2_000 + text.chars().count();
            let hints = fit_prompt_budget(candidates, estimated_prompt_chars);
            let fragment_chars = terminology_prompt_fragment(&hints).chars().count();
            assert!(hints.len() <= MAX_HINTS);
            assert!(fragment_chars <= estimated_prompt_chars / 10);
            ratios.push(fragment_chars as f64 / estimated_prompt_chars as f64);
        }
        ratios.sort_by(f64::total_cmp);
        assert!(ratios[ratios.len() / 2] <= 0.10);
    }
}
