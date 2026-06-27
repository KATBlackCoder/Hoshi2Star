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

use crate::llm::provider::LlmProvider;
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
// LLM extraction
// ---------------------------------------------------------------------------

/// Local struct for deserializing LLM-generated JSON term suggestions.
#[derive(Debug, Deserialize)]
struct ExtractedTerm {
    source: String,
    target: String,
    #[serde(default)]
    domain: String,
}

/// Query the LLM to identify glossary-worthy terms from a project's name fields.
///
/// Scans `Actors`, `Skills`, `Items`, and `States` segments (json_key `%/name`),
/// sends up to 200 unique source texts to the LLM, and stores up to 50 results.
///
/// Returns only newly created terms (existing source_text entries are skipped).
/// On JSON parse failure, logs a warning and returns an empty Vec — no panic.
pub async fn extract_terms_from_project(
    pool: &SqlitePool,
    provider: &impl LlmProvider,
    project_id: &str,
    lang_pair: &str,
) -> Result<Vec<GlossaryTerm>, String> {
    // 1. Fetch up to 200 unique source texts from name fields
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT s.source_text \
         FROM segments s \
         JOIN source_files sf ON s.source_file_id = sf.id \
         WHERE sf.project_id = ? \
           AND sf.file_type IN ('actors', 'skills', 'items', 'states') \
           AND (s.json_key LIKE '%/name' OR s.json_key LIKE '%/nickname') \
         LIMIT 200",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    if rows.is_empty() {
        return Ok(vec![]);
    }

    let source_list = rows
        .iter()
        .map(|(s,)| s.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    // 2. Build extraction prompt
    let lang_code = lang_pair.split('-').nth(1).unwrap_or("en");
    let tmpl = crate::llm::prompts::glossary_for(lang_code);
    let system = tmpl.system.clone();
    let user = tmpl.render(
        &tmpl.user,
        &[
            (
                "target_lang",
                crate::llm::prompts::lang_code_to_name(lang_code),
            ),
            ("source_list", &source_list),
        ],
    );

    // 3. Call LLM — single chat turn
    let raw = provider
        .chat(&system, &user)
        .await
        .map_err(|e| e.to_string())?;

    // 4. Robustly parse the JSON array from the response
    let extracted = parse_json_array(&raw);

    // 5. Insert new terms in DB (skip duplicates, limit to 50)
    let mut created = Vec::new();
    for term in extracted.into_iter().take(50) {
        if term.source.is_empty() || term.target.is_empty() {
            continue;
        }

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM glossary_terms WHERE source_text = ? AND project_id = ?",
        )
        .bind(&term.source)
        .bind(project_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        if count > 0 {
            continue;
        }

        if let Ok(gt) = insert_term(
            pool,
            &term.source,
            &term.target,
            lang_pair,
            &term.domain,
            Some(project_id),
            true,
        )
        .await
        {
            created.push(gt);
        }
    }

    Ok(created)
}

/// Extract the first JSON array `[…]` from a raw LLM response.
///
/// LLMs often wrap JSON in markdown code fences or add explanatory text —
/// this function searches for the first `[` and the last `]` to find the array.
/// Returns an empty Vec on any parse error.
fn parse_json_array(raw: &str) -> Vec<ExtractedTerm> {
    // Strip <think>…</think> blocks that reasoning models may emit
    let stripped = {
        static THINK_RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
            regex::Regex::new(r"(?s)<think>.*?</think>").expect("valid regex")
        });
        THINK_RE.replace_all(raw, "").into_owned()
    };

    let start = stripped.find('[');
    let end = stripped.rfind(']');

    match (start, end) {
        (Some(s), Some(e)) if s < e => {
            let slice = &stripped[s..=e];
            match serde_json::from_str::<Vec<ExtractedTerm>>(slice) {
                Ok(terms) => terms,
                Err(e) => {
                    eprintln!("[glossary] JSON parse error: {e}");
                    vec![]
                }
            }
        }
        _ => {
            eprintln!("[glossary] no JSON array found in LLM response");
            vec![]
        }
    }
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
    use crate::llm::provider::OllamaProvider;
    use httpmock::prelude::*;
    use serde_json::json;
    use std::time::Duration;
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

    // ---- Extraction tests --------------------------------------------------

    fn make_provider(server: &MockServer) -> OllamaProvider {
        OllamaProvider::new(&server.base_url(), "test-model", Duration::from_secs(5))
    }

    async fn insert_actor_segment(db: &SqlitePool, project_id: &str) {
        sqlx::query(
            "INSERT INTO projects (id, name, engine, game_path) VALUES (?, 'G', 'mv_mz', '/tmp')",
        )
        .bind(project_id)
        .execute(db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO source_files \
             (id, project_id, file_name, file_path, file_type) \
             VALUES (?, ?, 'Actors.json', '/tmp/Actors.json', 'actors')",
        )
        .bind(format!("sf-{project_id}"))
        .bind(project_id)
        .execute(db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO segments (id, source_file_id, json_key, source_text) \
             VALUES (?, ?, '/1/name', '勇者')",
        )
        .bind(format!("s-{project_id}"))
        .bind(format!("sf-{project_id}"))
        .execute(db)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_extract_valid_json() {
        let (db, _file) = test_db().await;
        insert_actor_segment(&db, "p-extract-valid").await;

        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/api/chat");
            then.status(200).json_body(json!({
                "message": {
                    "role": "assistant",
                    "content": r#"[{"source":"勇者","target":"Hero","domain":"character"}]"#
                }
            }));
        });

        let provider = make_provider(&server);
        let terms = extract_terms_from_project(&db, &provider, "p-extract-valid", "ja-en")
            .await
            .expect("extract");

        assert_eq!(terms.len(), 1);
        assert_eq!(terms[0].source_text, "勇者");
        assert_eq!(terms[0].target_text, "Hero");
        assert_eq!(terms[0].domain, "character");
        assert!(terms[0].auto_generated);
        assert_eq!(terms[0].project_id, Some("p-extract-valid".to_string()));
    }

    #[tokio::test]
    async fn test_extract_invalid_json_returns_empty() {
        let (db, _file) = test_db().await;
        insert_actor_segment(&db, "p-extract-invalid").await;

        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/api/chat");
            then.status(200).json_body(json!({
                "message": {
                    "role": "assistant",
                    "content": "Sorry, I cannot help with that."
                }
            }));
        });

        let provider = make_provider(&server);
        let terms = extract_terms_from_project(&db, &provider, "p-extract-invalid", "ja-en")
            .await
            .expect("no panic on invalid JSON");

        assert!(terms.is_empty());
    }

    #[tokio::test]
    async fn test_extract_limit_50() {
        let (db, _file) = test_db().await;
        insert_actor_segment(&db, "p-extract-limit").await;

        // Build a JSON array of 100 terms
        let many_terms: Vec<serde_json::Value> = (1..=100)
            .map(|i| {
                json!({
                    "source": format!("用語{i}"),
                    "target": format!("Term{i}"),
                    "domain": "other"
                })
            })
            .collect();

        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/api/chat");
            then.status(200).json_body(json!({
                "message": {
                    "role": "assistant",
                    "content": serde_json::to_string(&many_terms).unwrap()
                }
            }));
        });

        let provider = make_provider(&server);
        let terms = extract_terms_from_project(&db, &provider, "p-extract-limit", "ja-en")
            .await
            .expect("extract");

        assert_eq!(terms.len(), 50, "should be capped at 50");
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
