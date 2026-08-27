//! Executable QA contracts shared by the editor, pipeline, report and export.
//!
//! These tests intentionally cover both Japanese → English and Japanese →
//! French. The language used to display a QA report is a separate concern from
//! the project's target language.

use hoshi2star_lib::core::{qa, report};
use hoshi2star_lib::db;

fn semantic_check(
    source: &str,
    target: &str,
    target_language: &str,
    segment_kind: &str,
) -> qa::QaResult {
    qa::check_with_context(
        source,
        target,
        &[],
        "mv_mz",
        &qa::QaSemanticContext {
            source_language: "ja",
            target_language,
            segment_kind,
            neighbor_sources: &[],
            neighbor_targets: &[],
        },
    )
}

#[test]
fn japanese_to_english_preserves_engine_codes_without_false_positive() {
    let result = semantic_check(
        r"\C[2]勇者\C[0]：所持金は\V[1]です。",
        r"\C[2]Hero\C[0]: You have \V[1] gold.",
        "en",
        "dialogue",
    );

    assert_eq!(result.score, 100);
    assert!(result.errors.is_empty());
}

#[test]
fn japanese_to_french_preserves_engine_codes_without_false_positive() {
    let result = semantic_check(
        r"\C[2]勇者\C[0]：所持金は\V[1]です。",
        r"\C[2]Héros\C[0] : Vous avez \V[1] pièces d'or.",
        "fr",
        "dialogue",
    );

    assert_eq!(result.score, 100);
    assert!(result.errors.is_empty());
}

#[test]
fn missing_placeholder_is_a_blocker_even_with_score_75() {
    let result = semantic_check(
        r"所持金は\V[1]ゴールドです。",
        "You have some gold.",
        "en",
        "dialogue",
    );

    assert_eq!(result.score, 75);
    assert!(result.errors.iter().any(
        |error| matches!(error, qa::QaError::MissingPlaceholder { placeholder } if placeholder == r"\V[1]")
    ));
    assert!(
        result.has_critical_errors(),
        "export decisions must use explicit severity, not a score threshold"
    );
}

#[test]
fn width_warning_does_not_block_export_by_itself() {
    let result = semantic_check(
        "これは長さ確認用の文章です。",
        "This translated line is deliberately wider than the default message box.",
        "en",
        "dialogue",
    );

    assert_eq!(result.score, 90);
    assert!(matches!(
        result.errors.as_slice(),
        [qa::QaError::LineTooLong { .. }]
    ));
    assert!(!result.has_critical_errors());
}

#[test]
fn untranslated_japanese_is_rejected_for_both_target_languages() {
    for target_language in ["en", "fr"] {
        let result = semantic_check(
            "冒険を始めます。",
            "冒険を始めます。",
            target_language,
            "dialogue",
        );

        assert!(result
            .errors
            .iter()
            .any(|error| matches!(error, qa::QaError::UnchangedSource)));
        assert!(result.errors.iter().any(|error| matches!(
            error,
            qa::QaError::SourceScriptRemaining { source_language } if source_language == "ja"
        )));
        assert!(result.has_critical_errors());
    }
}

#[test]
fn report_labels_are_independent_from_the_translation_target() {
    let error = qa::QaError::MissingPlaceholder {
        placeholder: r"\V[1]".to_string(),
    };

    assert_eq!(error.label("fr"), r"Placeholder manquant : \V[1]");
    assert_eq!(error.label("en"), r"Missing placeholder: \V[1]");
}

#[tokio::test]
async fn qa_report_preview_is_read_only_and_explicit_audit_persists() {
    let temporary = tempfile::NamedTempFile::new().unwrap();
    let pool = db::pool::init(temporary.path().to_str().unwrap())
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO projects \
         (id, name, engine, game_path, source_language, target_language) \
         VALUES ('p1', 'QA contract', 'mv_mz', '/tmp', 'ja', 'en')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
         VALUES ('f1', 'p1', 'Map001.json', '/tmp/Map001.json', 'map')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO segments \
         (id, source_file_id, json_key, source_text, target_text, status, qa_score, segment_kind) \
         VALUES ('s1', 'f1', '/events/1', '所持金は\\V[1]です。', \
                 'You have gold.', 'translated', 100, 'dialogue')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let before: (String, Option<i64>) =
        sqlx::query_as("SELECT status, qa_score FROM segments WHERE id = 's1'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let _preview = report::preview_project(&pool, "p1").await.unwrap();
    let after: (String, Option<i64>) =
        sqlx::query_as("SELECT status, qa_score FROM segments WHERE id = 's1'")
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(
        after, before,
        "reading a QA report must not mutate project data"
    );

    let (summary, _) = report::audit_project(&pool, "p1").await.unwrap();
    let audited: (String, Option<i64>) =
        sqlx::query_as("SELECT status, qa_score FROM segments WHERE id = 's1'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(summary.critical_count, 1);
    assert_eq!(audited.0, "needs_review");
    assert!(audited.1.unwrap_or(100) < 100);
}
