use std::fs;

use hoshi2star_lib::core::terminology::repository;
use hoshi2star_lib::core::terminology::translator;
use hoshi2star_lib::core::terminology::types::{Enforcement, ReviewStatus, UpsertTranslationInput};
use hoshi2star_lib::domain::types::ResourceProfile;
use hoshi2star_lib::llm::provider::LlmProvider;
use serde::Serialize;

use super::translation::append_metrics;
use super::PilotWorkspace;

const REVIEWED_TYPES: &str = "'character','speaker','item','weapon','armor','skill',\
    'enemy','state','class','place','title'";

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct ReviewRow {
    source: String,
    target: String,
    semantic_type: String,
    part_of_speech: String,
    confidence: f64,
    review_status: String,
    enforcement: String,
}

pub async fn prepare<P: LlmProvider>(
    workspace: &PilotWorkspace,
    provider: &P,
    model: &str,
) -> Result<(), String> {
    let state = workspace.state();
    let scan = state
        .terminology
        .scan_project(&state.db, &workspace.project_id, None)
        .await
        .map_err(|error| error.to_string())?;
    eprintln!(
        "[terminology] scan analyzed={}, skipped={}, discovered={}",
        scan.analyzed, scan.skipped, scan.discovered
    );

    let selection_sql = format!(
        "SELECT DISTINCT entry.id FROM terminology_entries entry \
         JOIN terminology_occurrences occurrence ON occurrence.entry_id = entry.id \
         WHERE occurrence.project_id = ? AND entry.origin = 'engine' \
           AND entry.semantic_type IN ({REVIEWED_TYPES}) \
           AND NOT EXISTS (SELECT 1 FROM terminology_translations translation \
             WHERE translation.entry_id = entry.id AND translation.target_language = 'en' \
               AND translation.project_id = ?) ORDER BY entry.id"
    );
    let missing_ids: Vec<String> = sqlx::query_scalar(&selection_sql)
        .bind(&workspace.project_id)
        .bind(&workspace.project_id)
        .fetch_all(&state.db)
        .await
        .map_err(|error| error.to_string())?;
    let metrics_path = workspace.root.join("terminology-provider-metrics.jsonl");
    for chunk in missing_ids.chunks(3) {
        match translate(provider, workspace, model, chunk).await {
            Ok(()) => append_metrics(&metrics_path, &provider.drain_metrics())?,
            Err(batch_error) if chunk.len() > 1 => {
                append_metrics(&metrics_path, &provider.drain_metrics())?;
                eprintln!("[terminology] batch retry as singles: {batch_error}");
                for entry_id in chunk {
                    translate(provider, workspace, model, std::slice::from_ref(entry_id)).await?;
                    append_metrics(&metrics_path, &provider.drain_metrics())?;
                }
            }
            Err(error) => return Err(error),
        }
    }

    for (source, target) in [("六花", "Rikka"), ("凛", "Rin")] {
        lock_known_name(workspace, source, target, model).await?;
    }
    write_review_snapshot(workspace).await
}

async fn translate<P: LlmProvider>(
    provider: &P,
    workspace: &PilotWorkspace,
    model: &str,
    entry_ids: &[String],
) -> Result<(), String> {
    translator::translate_selected(
        &workspace.state().db,
        provider,
        entry_ids,
        "en",
        Some(&workspace.project_id),
        ResourceProfile::Fast,
        "ollama",
        model,
        None,
    )
    .await
    .map(|summary| {
        eprintln!(
            "[terminology] proposed {}/{} terms",
            summary.proposed, summary.requested
        );
    })
    .map_err(|error| error.to_string())
}

async fn lock_known_name(
    workspace: &PilotWorkspace,
    source: &str,
    target: &str,
    model: &str,
) -> Result<(), String> {
    let ids: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT entry.id FROM terminology_entries entry \
         JOIN terminology_occurrences occurrence ON occurrence.entry_id = entry.id \
         WHERE occurrence.project_id = ? AND entry.canonical_text = ? \
           AND entry.semantic_type IN ('character', 'speaker')",
    )
    .bind(&workspace.project_id)
    .bind(source)
    .fetch_all(&workspace.state().db)
    .await
    .map_err(|error| error.to_string())?;
    for entry_id in ids {
        repository::upsert_translation(
            &workspace.state().db,
            &UpsertTranslationInput {
                entry_id,
                target_language: "en".into(),
                project_id: Some(workspace.project_id.clone()),
                target_text: target.into(),
                review_status: ReviewStatus::Locked,
                enforcement: Enforcement::Required,
                confidence: 1.0,
                provider_id: Some("manual-pilot".into()),
                model: Some(model.into()),
                accepted_variants: Vec::new(),
            },
        )
        .await
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

async fn write_review_snapshot(workspace: &PilotWorkspace) -> Result<(), String> {
    let sql = format!(
        "SELECT entry.canonical_text AS source, translation.target_text AS target, \
                entry.semantic_type, entry.part_of_speech, translation.confidence, \
                translation.review_status, translation.enforcement \
         FROM terminology_entries entry \
         JOIN terminology_occurrences occurrence ON occurrence.entry_id = entry.id \
         JOIN terminology_translations translation ON translation.entry_id = entry.id \
           AND translation.target_language = 'en' AND translation.project_id = ? \
         WHERE occurrence.project_id = ? AND entry.semantic_type IN ({REVIEWED_TYPES}) \
         GROUP BY entry.id, translation.id ORDER BY entry.semantic_type, entry.canonical_text"
    );
    let rows = sqlx::query_as::<_, ReviewRow>(&sql)
        .bind(&workspace.project_id)
        .bind(&workspace.project_id)
        .fetch_all(&workspace.state().db)
        .await
        .map_err(|error| error.to_string())?;
    fs::write(
        workspace.root.join("terminology-review.json"),
        serde_json::to_vec_pretty(&rows).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}
