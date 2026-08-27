use std::fs;

use hoshi2star_lib::commands::export::export_project;
use hoshi2star_lib::core::report;
use hoshi2star_lib::llm::provider::ProviderCallMetrics;
use serde::Serialize;

use super::{PilotConfig, PilotWorkspace, VariantSummary};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Comparison {
    model: String,
    source_fingerprint: String,
    baseline: VariantSummary,
    terminology: VariantSummary,
}

pub async fn summarize(workspace: &PilotWorkspace) -> Result<VariantSummary, String> {
    let state = workspace.state();
    let (translated, needs_review, untranslated): (i64, i64, i64) = sqlx::query_as(
        "SELECT SUM(CASE WHEN segment.status = 'translated' THEN 1 ELSE 0 END), \
                SUM(CASE WHEN segment.status = 'needs_review' THEN 1 ELSE 0 END), \
                SUM(CASE WHEN segment.status = 'untranslated' THEN 1 ELSE 0 END) \
         FROM segments segment JOIN source_files file ON file.id = segment.source_file_id \
         WHERE file.project_id = ?",
    )
    .bind(&workspace.project_id)
    .fetch_one(&state.db)
    .await
    .map_err(|error| error.to_string())?;
    let (qa, details) = report::preview_project(&state.db, &workspace.project_id)
        .await
        .map_err(|error| error.to_string())?;
    let details_json = serde_json::to_vec_pretty(&details).map_err(|error| error.to_string())?;
    fs::write(workspace.root.join("qa-details.json"), details_json)
        .map_err(|error| error.to_string())?;
    let inconsistent_repeated_sources: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM (SELECT segment.source_text FROM segments segment \
         JOIN source_files file ON file.id = segment.source_file_id \
         WHERE file.project_id = ? AND trim(segment.target_text) != '' \
         GROUP BY segment.source_text HAVING COUNT(DISTINCT segment.target_text) > 1)",
    )
    .bind(&workspace.project_id)
    .fetch_one(&state.db)
    .await
    .map_err(|error| error.to_string())?;
    let metrics = read_metrics(&workspace.metrics_path())?;
    let export =
        match export_project(workspace.project_id.clone(), None, false, workspace.state()).await {
            Ok(path) => format!("valid:{path}"),
            Err(error) => format!("blocked:{error}"),
        };
    Ok(VariantSummary {
        variant: workspace.variant.clone(),
        total_segments: qa.total_segments,
        translated,
        needs_review,
        untranslated,
        qa_ok: qa.ok_count,
        qa_errors: qa.error_count,
        qa_critical: qa.critical_count,
        errors_by_type: qa.errors_by_type,
        terminology_issues: qa.terminology_issues.len(),
        inconsistent_repeated_sources,
        provider_calls: metrics.len(),
        provider_attempts: metrics.iter().map(|metric| metric.attempts).sum(),
        prompt_tokens: sum_optional(&metrics, |metric| metric.prompt_tokens),
        completion_tokens: sum_optional(&metrics, |metric| metric.completion_tokens),
        total_tokens: sum_optional(&metrics, |metric| metric.total_tokens),
        provider_duration_ms: metrics.iter().map(|metric| metric.duration_ms).sum(),
        calls_with_hints: metrics
            .iter()
            .filter(|metric| metric.terminology_hints > 0)
            .count(),
        terminology_hints: metrics.iter().map(|metric| metric.terminology_hints).sum(),
        export,
    })
}

pub fn write_comparison(
    config: &PilotConfig,
    source_fingerprint: String,
    baseline: VariantSummary,
    terminology: VariantSummary,
) -> Result<(), String> {
    let comparison = Comparison {
        model: config.model.clone(),
        source_fingerprint,
        baseline,
        terminology,
    };
    let output = serde_json::to_vec_pretty(&comparison).map_err(|error| error.to_string())?;
    fs::write(config.root.join("comparison.json"), &output).map_err(|error| error.to_string())?;
    println!("{}", String::from_utf8_lossy(&output));
    Ok(())
}

fn read_metrics(path: &std::path::Path) -> Result<Vec<ProviderCallMetrics>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    fs::read_to_string(path)
        .map_err(|error| error.to_string())?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).map_err(|error| error.to_string()))
        .collect()
}

fn sum_optional(
    metrics: &[ProviderCallMetrics],
    select: impl Fn(&ProviderCallMetrics) -> Option<u64>,
) -> Option<u64> {
    metrics
        .iter()
        .map(select)
        .collect::<Option<Vec<_>>>()
        .map(|values| values.into_iter().sum())
}
