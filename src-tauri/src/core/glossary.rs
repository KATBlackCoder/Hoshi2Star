//! Glossary — two-level term store backed by SQLite.
//!
//! ## Two levels
//! - **Global** (`project_id IS NULL`): shared across all projects.
//! - **Project-local** (`project_id = some id`): scoped to a single project.
//!
//! When both levels define the same source term, the project-local term takes
//! precedence and hides the global one. See [`list_for_project`].
//!
//! ## Auto-generated
//! Terms extracted by the LLM pipeline are flagged `auto_generated = true`.
//! Manual additions use `auto_generated = false`.

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct GlossaryTerm {
    pub id: String,
    pub source_text: String,
    pub target_text: String,
    pub lang_pair: String,
    pub domain: String,
    pub project_id: Option<String>,
    pub auto_generated: bool,
    pub created_at: String,
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// CRUD
// ---------------------------------------------------------------------------

/// Insert a new glossary term and return the saved row.
///
/// A fresh UUID v4 is generated for the `id` field.
pub async fn insert_term(
    pool: &SqlitePool,
    source_text: &str,
    target_text: &str,
    lang_pair: &str,
    domain: &str,
    project_id: Option<&str>,
    auto_generated: bool,
) -> Result<GlossaryTerm, sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO glossary_terms \
             (id, source_text, target_text, lang_pair, domain, project_id, auto_generated) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(source_text)
    .bind(target_text)
    .bind(lang_pair)
    .bind(domain)
    .bind(project_id)
    .bind(auto_generated)
    .execute(pool)
    .await?;

    fetch_by_id(pool, &id).await
}

/// Update the editable fields of an existing term and return the updated row.
pub async fn update_term(
    pool: &SqlitePool,
    id: &str,
    source_text: &str,
    target_text: &str,
    domain: &str,
) -> Result<GlossaryTerm, sqlx::Error> {
    sqlx::query(
        "UPDATE glossary_terms \
         SET source_text = ?, target_text = ?, domain = ?, updated_at = datetime('now') \
         WHERE id = ?",
    )
    .bind(source_text)
    .bind(target_text)
    .bind(domain)
    .bind(id)
    .execute(pool)
    .await?;

    fetch_by_id(pool, id).await
}

/// Delete a term by id.
pub async fn delete_term(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM glossary_terms WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Return all terms visible for a given project + lang pair, merging two levels.
///
/// - Global terms (`project_id IS NULL`) come first, sorted by `source_text ASC`.
/// - Project-local terms follow, sorted by `source_text ASC`.
/// - When a project-local term shares the same `source_text` as a global term,
///   the global is hidden (project-local takes precedence).
pub async fn list_for_project(
    pool: &SqlitePool,
    project_id: &str,
    lang_pair: &str,
) -> Result<Vec<GlossaryTerm>, sqlx::Error> {
    let globals = sqlx::query_as::<_, GlossaryTerm>(
        "SELECT id, source_text, target_text, lang_pair, domain, project_id, \
                auto_generated, created_at, updated_at \
         FROM glossary_terms \
         WHERE project_id IS NULL AND lang_pair = ? \
         ORDER BY source_text ASC",
    )
    .bind(lang_pair)
    .fetch_all(pool)
    .await?;

    let locals = sqlx::query_as::<_, GlossaryTerm>(
        "SELECT id, source_text, target_text, lang_pair, domain, project_id, \
                auto_generated, created_at, updated_at \
         FROM glossary_terms \
         WHERE project_id = ? AND lang_pair = ? \
         ORDER BY source_text ASC",
    )
    .bind(project_id)
    .bind(lang_pair)
    .fetch_all(pool)
    .await?;

    // Collect project-local source texts for deduplication
    let local_sources: std::collections::HashSet<&str> =
        locals.iter().map(|t| t.source_text.as_str()).collect();

    // Keep only globals that are not shadowed by a project-local term
    let filtered_globals: Vec<GlossaryTerm> = globals
        .into_iter()
        .filter(|t| !local_sources.contains(t.source_text.as_str()))
        .collect();

    let mut result = filtered_globals;
    result.extend(locals);
    Ok(result)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

async fn fetch_by_id(pool: &SqlitePool, id: &str) -> Result<GlossaryTerm, sqlx::Error> {
    sqlx::query_as::<_, GlossaryTerm>(
        "SELECT id, source_text, target_text, lang_pair, domain, project_id, \
                auto_generated, created_at, updated_at \
         FROM glossary_terms WHERE id = ?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::pool::init;
    use tempfile::NamedTempFile;

    async fn test_db() -> (SqlitePool, NamedTempFile) {
        let file = NamedTempFile::new().expect("tempfile");
        let path = file.path().to_str().expect("utf-8 path").to_string();
        let pool = init(&path).await.expect("pool");
        (pool, file)
    }

    async fn insert_test_project(pool: &SqlitePool, id: &str) {
        sqlx::query(
            "INSERT INTO projects (id, name, engine, game_path) \
             VALUES (?, 'Test Game', 'mv_mz', '/tmp/game')",
        )
        .bind(id)
        .execute(pool)
        .await
        .expect("insert project");
    }

    #[tokio::test]
    async fn test_insert_and_list_global() {
        let (db, _file) = test_db().await;

        let term = insert_term(&db, "勇者", "Hero", "ja-en", "character", None, false)
            .await
            .expect("insert");

        assert_eq!(term.source_text, "勇者");
        assert_eq!(term.target_text, "Hero");
        assert!(term.project_id.is_none());
        assert!(!term.auto_generated);

        // Global term is visible for any project_id
        let terms = list_for_project(&db, "any-project", "ja-en")
            .await
            .expect("list");
        assert_eq!(terms.len(), 1);
        assert_eq!(terms[0].source_text, "勇者");
    }

    #[tokio::test]
    async fn test_project_overrides_global() {
        let (db, _file) = test_db().await;
        insert_test_project(&db, "proj-1").await;

        // Global term
        insert_term(&db, "魔法使い", "Mage", "ja-en", "character", None, false)
            .await
            .expect("global insert");

        // Project-local term for the same source — should shadow the global
        insert_term(
            &db,
            "魔法使い",
            "Sorcerer",
            "ja-en",
            "character",
            Some("proj-1"),
            false,
        )
        .await
        .expect("local insert");

        let terms = list_for_project(&db, "proj-1", "ja-en")
            .await
            .expect("list");

        // Only one term visible — project-local wins
        assert_eq!(terms.len(), 1);
        assert_eq!(terms[0].target_text, "Sorcerer");
        assert_eq!(terms[0].project_id, Some("proj-1".to_string()));
    }

    #[tokio::test]
    async fn test_delete_cascades_with_project() {
        let (db, _file) = test_db().await;
        insert_test_project(&db, "proj-cascade").await;

        insert_term(
            &db,
            "剣士",
            "Swordsman",
            "ja-en",
            "character",
            Some("proj-cascade"),
            false,
        )
        .await
        .expect("insert");

        // Term exists before deletion
        let before = list_for_project(&db, "proj-cascade", "ja-en")
            .await
            .expect("list");
        assert_eq!(before.len(), 1);

        // Deleting the project cascades to glossary_terms
        sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind("proj-cascade")
            .execute(&db)
            .await
            .expect("delete project");

        let after = list_for_project(&db, "proj-cascade", "ja-en")
            .await
            .expect("list after cascade");
        assert!(after.is_empty());
    }
}
