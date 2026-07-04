//! Tauri commands for QA checks and Translation Memory queries.

use std::collections::HashMap;

use sqlx::SqlitePool;

use crate::{
    core::{glossary, qa, tm},
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
/// When `project_id` is provided, loads the project glossary (global +
/// project terms, `ja-en`) and resolves the project engine from the DB so
/// live QA matches the batch pipeline. Engine precedence: explicit param >
/// `projects.engine` > `"mv_mz"`.
#[tauri::command]
pub async fn qa_check_segment(
    source_text: String,
    target_text: String,
    engine: Option<String>,
    project_id: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<qa::QaResult, String> {
    Ok(check_segment_live(
        &state.db,
        &source_text,
        &target_text,
        engine.as_deref(),
        project_id.as_deref(),
    )
    .await)
}

/// Testable core of [`qa_check_segment`] (no Tauri `State`). Best-effort:
/// glossary/engine lookups degrade to defaults on DB errors, never fail QA.
async fn check_segment_live(
    db: &SqlitePool,
    source_text: &str,
    target_text: &str,
    engine: Option<&str>,
    project_id: Option<&str>,
) -> qa::QaResult {
    let (glossary_terms, db_engine) = match project_id {
        Some(pid) => {
            let db_engine: Option<String> =
                sqlx::query_scalar("SELECT engine FROM projects WHERE id = ?")
                    .bind(pid)
                    .fetch_optional(db)
                    .await
                    .ok()
                    .flatten();
            let terms = glossary::relevant_terms(db, pid, "ja-en", &[source_text]).await;
            (terms, db_engine)
        }
        None => (Vec::new(), None),
    };

    let engine = engine
        .map(str::to_string)
        .or(db_engine)
        .unwrap_or_else(|| "mv_mz".to_string());

    qa::check(source_text, target_text, &glossary_terms, &engine)
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::qa::QaError;

    /// Fresh migrated DB seeded with one project (`p1`, given engine) and one
    /// project glossary term 魔法使い → Mage (ja-en).
    async fn seeded_db(engine: &str) -> (SqlitePool, tempfile::NamedTempFile) {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let pool = crate::db::pool::init(tmp.path().to_str().unwrap())
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO projects (id, name, engine, game_path) VALUES ('p1','T',?,'/tmp')",
        )
        .bind(engine)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO glossary_terms (id, source_text, target_text, lang_pair, project_id) \
             VALUES ('g1','魔法使い','Mage','ja-en','p1')",
        )
        .execute(&pool)
        .await
        .unwrap();
        (pool, tmp)
    }

    #[tokio::test]
    async fn test_live_qa_flags_glossary_mismatch() {
        let (db, _tmp) = seeded_db("mv_mz").await;

        let result = check_segment_live(
            &db,
            "魔法使いが現れた",
            "A wizard appeared",
            None,
            Some("p1"),
        )
        .await;

        assert_eq!(result.score, 85); // GlossaryMismatch = −15
        assert!(matches!(
            result.errors.as_slice(),
            [QaError::GlossaryMismatch { source_term, expected_target }]
                if source_term == "魔法使い" && expected_target == "Mage"
        ));
    }

    #[tokio::test]
    async fn test_live_qa_glossary_respected_no_error() {
        let (db, _tmp) = seeded_db("mv_mz").await;

        let result =
            check_segment_live(&db, "魔法使いが現れた", "A Mage appeared", None, Some("p1")).await;

        assert_eq!(result.score, 100);
        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_live_qa_without_project_id_skips_glossary() {
        let (db, _tmp) = seeded_db("mv_mz").await;

        let result =
            check_segment_live(&db, "魔法使いが現れた", "A wizard appeared", None, None).await;

        assert_eq!(result.score, 100);
        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_live_qa_engine_resolved_from_project() {
        let (db, _tmp) = seeded_db("wolf").await;

        // 49 half-width units: over wolf's 520/13 = 40, under MV/MZ's 720/13 ≈ 55.
        let long_line = "A Mage appeared and it was a very long line here!";
        assert_eq!(long_line.len(), 49);

        // Engine resolved from projects.engine ('wolf') → line too long.
        let with_project =
            check_segment_live(&db, "魔法使いが現れた", long_line, None, Some("p1")).await;
        assert!(with_project
            .errors
            .iter()
            .any(|e| matches!(e, QaError::LineTooLong { .. })));

        // Explicit param takes precedence over the DB engine → MV/MZ box, no error.
        let param_override = check_segment_live(
            &db,
            "魔法使いが現れた",
            long_line,
            Some("mv_mz"),
            Some("p1"),
        )
        .await;
        assert!(param_override.errors.is_empty());

        // No project_id → mv_mz default, no error.
        let without_project =
            check_segment_live(&db, "魔法使いが現れた", long_line, None, None).await;
        assert!(without_project.errors.is_empty());
    }
}
