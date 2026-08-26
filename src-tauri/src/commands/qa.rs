//! Tauri commands for QA checks and Translation Memory queries.

use sqlx::SqlitePool;

use crate::{
    core::{glossary, qa, report, tm},
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
/// project terms for that project's language pair) and resolves the project engine from the DB so
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
    let (glossary_terms, db_context) = match project_id {
        Some(pid) => {
            let project_context: Option<(String, String, String)> = sqlx::query_as(
                "SELECT engine, COALESCE(source_language, 'ja'), \
                        COALESCE(target_language, 'en') FROM projects WHERE id = ?",
            )
            .bind(pid)
            .fetch_optional(db)
            .await
            .ok()
            .flatten();
            match project_context {
                Some((db_engine, source_lang, target_lang)) => {
                    let lang_pair = format!("{source_lang}-{target_lang}");
                    let terms = glossary::relevant_terms(db, pid, &lang_pair, &[source_text]).await;
                    (terms, Some((db_engine, source_lang, target_lang)))
                }
                None => (Vec::new(), None),
            }
        }
        None => (Vec::new(), None),
    };

    let resolved_engine = engine
        .map(str::to_string)
        .or_else(|| db_context.as_ref().map(|value| value.0.clone()))
        .unwrap_or_else(|| "mv_mz".to_string());
    let source_language = db_context
        .as_ref()
        .map(|value| value.1.as_str())
        .unwrap_or("ja");
    let target_language = db_context
        .as_ref()
        .map(|value| value.2.as_str())
        .unwrap_or("fr");
    qa::check_with_context(
        source_text,
        target_text,
        &glossary_terms,
        &resolved_engine,
        &qa::QaSemanticContext {
            source_language,
            target_language,
            segment_kind: "unknown",
            neighbor_sources: &[],
        },
    )
}

/// Return a QA summary for all segments in a project.
#[tauri::command]
pub async fn get_qa_report(
    project_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<QaReport, String> {
    let project_languages: (String, String) = sqlx::query_as(
        "SELECT COALESCE(source_language, 'ja'), COALESCE(target_language, 'en') \
         FROM projects WHERE id = ?",
    )
    .bind(&project_id)
    .fetch_one(&state.db)
    .await
    .map_err(|error| error.to_string())?;
    let sources: Vec<String> = sqlx::query_scalar(
        "SELECT s.source_text FROM segments s \
         JOIN source_files sf ON s.source_file_id = sf.id \
         WHERE sf.project_id = ? AND (s.status != 'untranslated' OR s.target_text != '')",
    )
    .bind(&project_id)
    .fetch_all(&state.db)
    .await
    .map_err(|error| error.to_string())?;
    let source_refs: Vec<&str> = sources.iter().map(String::as_str).collect();
    let lang_pair = format!("{}-{}", project_languages.0, project_languages.1);
    let terms = glossary::relevant_terms(&state.db, &project_id, &lang_pair, &source_refs).await;
    let (summary, _) = report::audit_project(&state.db, &project_id, &terms)
        .await
        .map_err(|error| error.to_string())?;
    Ok(summary)
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
