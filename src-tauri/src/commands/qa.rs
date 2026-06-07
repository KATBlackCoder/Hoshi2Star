//! Tauri commands for QA checks and Translation Memory queries.

use std::collections::HashMap;

use crate::{
    core::{qa, tm},
    domain::types::QaReport,
    state::AppState,
};

/// Return TM fuzzy suggestions for a given source text.
/// Exact matches (score 1.0) sort to the top; fuzzy matches follow.
#[tauri::command]
pub async fn get_tm_suggestions(
    source_text: String,
    lang_pair: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<tm::TmSuggestion>, String> {
    tm::lookup_fuzzy(&source_text, &lang_pair, 0.80, 5, &state.db)
        .await
        .map_err(|e| e.to_string())
}

/// Run QA checks on a (source, target) pair and return the result.
///
/// `engine` is optional; defaults to `"mv_mz"` when absent.
/// Does not touch the database — useful for live checking in the UI.
#[tauri::command]
pub fn qa_check_segment(
    source_text: String,
    target_text: String,
    engine: Option<String>,
) -> qa::QaResult {
    qa::check(
        &source_text,
        &target_text,
        &[],
        engine.as_deref().unwrap_or("mv_mz"),
    )
}

/// Return a QA summary for all segments in a project.
#[tauri::command]
pub async fn get_qa_report(
    project_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<QaReport, String> {
    let total_segments: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM segments s \
         JOIN source_files sf ON s.source_file_id = sf.id \
         WHERE sf.project_id = ?",
    )
    .bind(&project_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let ok_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM segments s \
         JOIN source_files sf ON s.source_file_id = sf.id \
         WHERE sf.project_id = ? AND (qa_score = 100 OR qa_score IS NULL)",
    )
    .bind(&project_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    // Fetch all non-null qa_score to compute errors_by_type
    let qa_scores: Vec<i64> = sqlx::query_scalar(
        "SELECT qa_score FROM segments s \
         JOIN source_files sf ON s.source_file_id = sf.id \
         WHERE sf.project_id = ? AND qa_score IS NOT NULL AND qa_score < 100",
    )
    .bind(&project_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let error_count = qa_scores.len() as i64;

    // errors_by_type: approximate from score ranges
    // (exact type breakdown would require storing error types in DB — F3 improvement)
    let mut errors_by_type: HashMap<String, usize> = HashMap::new();
    for score in &qa_scores {
        if *score <= 75 {
            *errors_by_type.entry("placeholder".to_string()).or_insert(0) += 1;
        } else if *score <= 90 {
            *errors_by_type
                .entry("line_too_long".to_string())
                .or_insert(0) += 1;
        } else {
            *errors_by_type.entry("bom".to_string()).or_insert(0) += 1;
        }
    }

    Ok(QaReport {
        total_segments,
        ok_count,
        error_count,
        errors_by_type,
    })
}
