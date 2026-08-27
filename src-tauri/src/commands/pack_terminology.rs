//! Compatibility mapping between the multilingual terminology store and the
//! historical `glossary.json` section of `.h2s` format v1.

use std::collections::HashMap;

use crate::core::h2s_pack::PackGlossaryTerm;
use crate::core::terminology::normalize::normalize_term;

pub(super) fn split_lang_pair(lang_pair: &str) -> Result<(&str, &str), String> {
    let (source, target) = lang_pair
        .split_once('-')
        .ok_or_else(|| format!("invalid language pair: {lang_pair:?}"))?;
    if source.trim().is_empty() || target.trim().is_empty() {
        return Err(format!("invalid language pair: {lang_pair:?}"));
    }
    Ok((source.trim(), target.trim()))
}

pub(super) async fn export_terms(
    db: &sqlx::SqlitePool,
    project_id: &str,
    target_language: &str,
) -> Result<Vec<PackGlossaryTerm>, String> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT entry.canonical_text, translation.target_text, entry.semantic_type \
         FROM terminology_entries entry \
         JOIN terminology_translations translation ON translation.id = (\
             SELECT candidate.id FROM terminology_translations candidate \
             WHERE candidate.entry_id = entry.id AND candidate.target_language = ? \
               AND (candidate.project_id = ? OR candidate.project_id IS NULL) \
             ORDER BY CASE WHEN candidate.project_id = ? THEN 0 ELSE 1 END, \
                      candidate.updated_at DESC LIMIT 1\
         ) \
         WHERE entry.status = 'active' \
           AND translation.review_status IN ('approved', 'locked') \
           AND trim(translation.target_text) != '' \
           AND (translation.project_id = ? OR EXISTS (\
               SELECT 1 FROM terminology_occurrences occurrence \
               WHERE occurrence.entry_id = entry.id AND occurrence.project_id = ?\
           )) \
         ORDER BY entry.normalized_text, entry.semantic_type",
    )
    .bind(target_language)
    .bind(project_id)
    .bind(project_id)
    .bind(project_id)
    .bind(project_id)
    .fetch_all(db)
    .await
    .map_err(|error| error.to_string())?;
    Ok(rows
        .into_iter()
        .map(|(source_text, target_text, domain)| PackGlossaryTerm {
            source_text,
            target_text,
            domain,
        })
        .collect())
}

#[derive(sqlx::FromRow)]
struct ExistingPackTerm {
    entry_id: String,
    normalized_text: String,
    semantic_type: String,
    project_id: Option<String>,
    target_text: Option<String>,
    review_status: Option<String>,
}

pub(super) async fn preview_import(
    db: &sqlx::SqlitePool,
    project_id: &str,
    lang_pair: &str,
    terms: &[PackGlossaryTerm],
) -> Result<(u32, u32, u32), String> {
    let (source_language, target_language) = split_lang_pair(lang_pair)?;
    let rows: Vec<ExistingPackTerm> = sqlx::query_as(
        "SELECT entry.id AS entry_id, entry.normalized_text, entry.semantic_type, \
                translation.project_id, translation.target_text, translation.review_status \
         FROM terminology_entries entry \
         LEFT JOIN terminology_translations translation \
           ON translation.entry_id = entry.id AND translation.target_language = ? \
          AND (translation.project_id = ? OR translation.project_id IS NULL) \
         WHERE entry.source_language = ? AND entry.status = 'active' \
           AND entry.sense_key = ''",
    )
    .bind(target_language)
    .bind(project_id)
    .bind(source_language)
    .fetch_all(db)
    .await
    .map_err(|error| error.to_string())?;
    let mut existing: HashMap<(String, String), Vec<ExistingPackTerm>> = HashMap::new();
    for row in rows {
        existing
            .entry((row.normalized_text.clone(), row.semantic_type.clone()))
            .or_default()
            .push(row);
    }

    let (mut creates, mut updates, mut conflicts) = (0_u32, 0_u32, 0_u32);
    for term in non_empty_terms(terms) {
        let normalized = normalize_term(&term.source_text, source_language)
            .map_err(|error| error.to_string())?;
        let semantic_type = pack_semantic_type(&term.domain);
        let Some(candidates) = existing.get(&(normalized, semantic_type)) else {
            creates += 1;
            continue;
        };
        let entry_id = &candidates[0].entry_id;
        let effective = candidates
            .iter()
            .filter(|candidate| &candidate.entry_id == entry_id)
            .find(|candidate| candidate.project_id.as_deref() == Some(project_id))
            .or_else(|| {
                candidates
                    .iter()
                    .filter(|candidate| &candidate.entry_id == entry_id)
                    .find(|candidate| candidate.project_id.is_none())
            });
        match effective {
            Some(current) if current.target_text.as_deref() == Some(term.target_text.trim()) => {}
            Some(current) if current.review_status.as_deref() == Some("locked") => conflicts += 1,
            _ => updates += 1,
        }
    }
    Ok((creates, updates, conflicts))
}

/// Import the historical v1 payload as project-scoped approved terminology.
/// Locked effective translations are never overwritten or masked.
pub(super) async fn import_terms(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    project_id: &str,
    lang_pair: &str,
    terms: &[PackGlossaryTerm],
) -> Result<(u32, u32, u32), String> {
    let (source_language, target_language) = split_lang_pair(lang_pair)?;
    let (mut added, mut updated, mut conflicts) = (0_u32, 0_u32, 0_u32);
    for term in non_empty_terms(terms) {
        let normalized_text = normalize_term(&term.source_text, source_language)
            .map_err(|error| error.to_string())?;
        let semantic_type = pack_semantic_type(&term.domain);
        let existing_entry: Option<String> = sqlx::query_scalar(
            "SELECT id FROM terminology_entries \
             WHERE source_language = ? AND normalized_text = ? \
               AND semantic_type = ? AND sense_key = '' LIMIT 1",
        )
        .bind(source_language)
        .bind(&normalized_text)
        .bind(&semantic_type)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| error.to_string())?;
        let created = existing_entry.is_none();
        let entry_id = existing_entry.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        if created {
            sqlx::query(
                "INSERT INTO terminology_entries (\
                    id, source_language, canonical_text, normalized_text, part_of_speech, \
                    semantic_type, sense_key, status, origin, confidence\
                 ) VALUES (?, ?, ?, ?, 'unknown', ?, '', 'active', 'import', 1.0)",
            )
            .bind(&entry_id)
            .bind(source_language)
            .bind(term.source_text.trim())
            .bind(&normalized_text)
            .bind(&semantic_type)
            .execute(&mut **tx)
            .await
            .map_err(|error| error.to_string())?;
        }

        let effective: Option<(String, String, String, Option<String>)> = sqlx::query_as(
            "SELECT id, target_text, review_status, project_id \
             FROM terminology_translations \
             WHERE entry_id = ? AND target_language = ? \
               AND (project_id = ? OR project_id IS NULL) \
             ORDER BY CASE WHEN project_id = ? THEN 0 ELSE 1 END, updated_at DESC LIMIT 1",
        )
        .bind(&entry_id)
        .bind(target_language)
        .bind(project_id)
        .bind(project_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| error.to_string())?;
        if effective
            .as_ref()
            .is_some_and(|(_, target, _, _)| target == term.target_text.trim())
        {
            continue;
        }
        if effective
            .as_ref()
            .is_some_and(|(_, _, review, _)| review == "locked")
        {
            conflicts += 1;
            continue;
        }

        let project_translation: Option<String> = sqlx::query_scalar(
            "SELECT id FROM terminology_translations \
             WHERE entry_id = ? AND target_language = ? AND project_id = ?",
        )
        .bind(&entry_id)
        .bind(target_language)
        .bind(project_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| error.to_string())?;
        if let Some(translation_id) = project_translation {
            sqlx::query(
                "UPDATE terminology_translations SET target_text = ?, \
                    review_status = 'approved', enforcement = 'preferred', confidence = 1.0, \
                    provider_id = NULL, model = NULL, updated_at = datetime('now') WHERE id = ?",
            )
            .bind(term.target_text.trim())
            .bind(translation_id)
            .execute(&mut **tx)
            .await
            .map_err(|error| error.to_string())?;
            updated += 1;
        } else {
            sqlx::query(
                "INSERT INTO terminology_translations (\
                    id, entry_id, target_language, project_id, target_text, \
                    review_status, enforcement, confidence\
                 ) VALUES (?, ?, ?, ?, ?, 'approved', 'preferred', 1.0)",
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(&entry_id)
            .bind(target_language)
            .bind(project_id)
            .bind(term.target_text.trim())
            .execute(&mut **tx)
            .await
            .map_err(|error| error.to_string())?;
            if created {
                added += 1;
            } else {
                updated += 1;
            }
        }
    }
    Ok((added, updated, conflicts))
}

fn non_empty_terms(terms: &[PackGlossaryTerm]) -> impl Iterator<Item = &PackGlossaryTerm> {
    terms
        .iter()
        .filter(|term| !term.source_text.trim().is_empty() && !term.target_text.trim().is_empty())
}

fn pack_semantic_type(domain: &str) -> String {
    let normalized = domain.trim().to_lowercase();
    if normalized.is_empty() {
        "general".to_string()
    } else {
        normalized
    }
}
