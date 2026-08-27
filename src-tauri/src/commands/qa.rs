//! Tauri commands for QA checks and Translation Memory queries.

use sqlx::SqlitePool;

use crate::{
    core::{qa, report, terminology::resolver, tm},
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
/// When `project_id` and `segment_id` are provided, resolves only terminology
/// occurrences attached to that segment. The project engine is loaded from the
/// DB so live QA matches the batch pipeline. Engine precedence: explicit param
/// > `projects.engine` > `"mv_mz"`.
#[tauri::command]
pub async fn qa_check_segment(
    source_text: String,
    target_text: String,
    engine: Option<String>,
    project_id: Option<String>,
    segment_id: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<qa::QaResult, String> {
    Ok(check_segment_live(
        &state.db,
        &source_text,
        &target_text,
        engine.as_deref(),
        project_id.as_deref(),
        segment_id.as_deref(),
    )
    .await)
}

/// Testable core of [`qa_check_segment`] (no Tauri `State`). Best-effort:
/// terminology/engine lookups degrade to defaults on DB errors, never fail QA.
async fn check_segment_live(
    db: &SqlitePool,
    source_text: &str,
    target_text: &str,
    engine: Option<&str>,
    project_id: Option<&str>,
    segment_id: Option<&str>,
) -> qa::QaResult {
    let db_context = match project_id {
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
            project_context
        }
        None => None,
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
    let terminology_rules = match segment_id {
        Some(segment_id) => {
            resolver::resolve_qa_rules_for_segments(db, &[segment_id.to_string()], target_language)
                .await
                .ok()
                .and_then(|mut rules| rules.remove(segment_id))
                .unwrap_or_default()
        }
        None => Vec::new(),
    };
    qa::check_with_context(
        source_text,
        target_text,
        &terminology_rules,
        &resolved_engine,
        &qa::QaSemanticContext {
            source_language,
            target_language,
            segment_kind: "unknown",
            neighbor_sources: &[],
            neighbor_targets: &[],
        },
    )
}

/// Return a QA summary for all segments in a project.
#[tauri::command]
pub async fn get_qa_report(
    project_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<QaReport, String> {
    let (summary, _) = report::preview_project(&state.db, &project_id)
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
    /// occurrence-scoped terminology rule 魔法使い → Mage (ja-en).
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
            "INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
             VALUES ('f1','p1','Map001.json','/tmp/Map001.json','map'); \
             INSERT INTO segments (id, source_file_id, json_key, source_text, segment_kind) \
             VALUES ('s1','f1','/1','魔法使いが現れた','dialogue'); \
             INSERT INTO terminology_entries (\
                id, source_language, canonical_text, normalized_text, part_of_speech, \
                semantic_type, sense_key, status, origin, confidence\
             ) VALUES ('e1','ja','魔法使い','魔法使い','noun','general','','active','manual',1.0); \
             INSERT INTO terminology_occurrences (\
                entry_id, project_id, segment_id, surface_text, engine_kind, occurrence_count\
             ) VALUES ('e1','p1','s1','魔法使い','dialogue',1); \
             INSERT INTO terminology_translations (\
                id, entry_id, target_language, target_text, review_status, enforcement, confidence\
             ) VALUES ('t1','e1','en','Mage','approved','preferred',1.0)",
        )
        .execute(&pool)
        .await
        .unwrap();
        (pool, tmp)
    }

    #[tokio::test]
    async fn test_live_qa_flags_terminology_mismatch() {
        let (db, _tmp) = seeded_db("mv_mz").await;

        let result = check_segment_live(
            &db,
            "魔法使いが現れた",
            "A wizard appeared",
            None,
            Some("p1"),
            Some("s1"),
        )
        .await;

        assert_eq!(result.score, 85);
        assert!(matches!(
            result.errors.as_slice(),
            [QaError::TerminologyMismatch { source_term, expected_targets, .. }]
                if source_term == "魔法使い" && expected_targets == &["Mage"]
        ));
    }

    #[tokio::test]
    async fn test_live_qa_terminology_respected_no_error() {
        let (db, _tmp) = seeded_db("mv_mz").await;

        let result = check_segment_live(
            &db,
            "魔法使いが現れた",
            "A Mage appeared",
            None,
            Some("p1"),
            Some("s1"),
        )
        .await;

        assert_eq!(result.score, 100);
        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_live_qa_without_segment_id_skips_terminology() {
        let (db, _tmp) = seeded_db("mv_mz").await;

        let result = check_segment_live(
            &db,
            "魔法使いが現れた",
            "A wizard appeared",
            None,
            None,
            None,
        )
        .await;

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
        let with_project = check_segment_live(
            &db,
            "魔法使いが現れた",
            long_line,
            None,
            Some("p1"),
            Some("s1"),
        )
        .await;
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
            Some("s1"),
        )
        .await;
        assert!(param_override.errors.is_empty());

        // No project_id → mv_mz default, no error.
        let without_project =
            check_segment_live(&db, "魔法使いが現れた", long_line, None, None, None).await;
        assert!(without_project.errors.is_empty());
    }
}
