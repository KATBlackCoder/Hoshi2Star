//! Tauri command for project-wide segment search (concordance search).
//!
//! Unlike the grid's per-file client-side filter, this searches every segment
//! of a project in SQL and returns hits with their file name so the UI can
//! group results by file.

use serde::Deserialize;
use sqlx::SqlitePool;

use crate::{
    domain::types::{SegmentSearchHit, SegmentSearchResult},
    state::AppState,
};

/// Maximum hits per request — the frontend fetches successive batches
/// (`offset` 0, 500, 1000, …) until [`SegmentSearchResult::total`] is
/// reached, so every match is eventually loaded.
pub const SEARCH_BATCH_SIZE: i64 = 500;

/// Which segment column(s) the query is matched against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchScope {
    Both,
    Source,
    Target,
}

/// Search all segments of a project. Returns hits ordered by file name then
/// insertion order, so file groups are contiguous for the results view —
/// including across successive `offset` batches (the ORDER BY is stable).
#[tauri::command]
pub async fn search_segments(
    project_id: String,
    query: String,
    scope: SearchScope,
    limit: Option<i64>,
    offset: Option<i64>,
    state: tauri::State<'_, AppState>,
) -> Result<SegmentSearchResult, String> {
    search_project_segments(
        &state.db,
        &project_id,
        &query,
        scope,
        limit.unwrap_or(SEARCH_BATCH_SIZE),
        offset.unwrap_or(0),
    )
    .await
    .map_err(|e| e.to_string())
}

/// Testable core of [`search_segments`] (no Tauri `State`).
///
/// Trims the query; fewer than 2 characters returns an empty result rather
/// than an error (the UI enforces the same minimum). `\`, `%` and `_` in the
/// query are escaped so they match literally instead of acting as LIKE
/// wildcards. `limit` is clamped to `1..=SEARCH_BATCH_SIZE` per request;
/// `offset` (clamped to `>= 0`) lets the caller page through all matches.
async fn search_project_segments(
    pool: &SqlitePool,
    project_id: &str,
    query: &str,
    scope: SearchScope,
    limit: i64,
    offset: i64,
) -> Result<SegmentSearchResult, sqlx::Error> {
    let q = query.trim();
    if q.chars().count() < 2 {
        return Ok(SegmentSearchResult {
            items: vec![],
            total: 0,
        });
    }
    let limit = limit.clamp(1, SEARCH_BATCH_SIZE);
    let offset = offset.max(0);

    let escaped = q
        .replace('\\', r"\\")
        .replace('%', r"\%")
        .replace('_', r"\_");
    let pattern = format!("%{escaped}%");

    // Static predicate per scope — user input only ever enters via binds.
    let (predicate, binds) = match scope {
        SearchScope::Both => (
            r"(s.source_text LIKE ? ESCAPE '\' OR s.target_text LIKE ? ESCAPE '\')",
            2,
        ),
        SearchScope::Source => (r"s.source_text LIKE ? ESCAPE '\'", 1),
        SearchScope::Target => (r"s.target_text LIKE ? ESCAPE '\'", 1),
    };

    let count_sql = format!(
        "SELECT COUNT(*) FROM segments s \
         JOIN source_files sf ON s.source_file_id = sf.id \
         WHERE sf.project_id = ? AND {predicate}"
    );
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql).bind(project_id);
    for _ in 0..binds {
        count_query = count_query.bind(&pattern);
    }
    let total = count_query.fetch_one(pool).await?;

    let items_sql = format!(
        "SELECT s.id, s.source_file_id, s.json_key, s.source_text, s.target_text, \
                s.status, s.qa_score, s.created_at, s.updated_at, sf.file_name \
         FROM segments s \
         JOIN source_files sf ON s.source_file_id = sf.id \
         WHERE sf.project_id = ? AND {predicate} \
         ORDER BY sf.file_name, s.rowid \
         LIMIT ? OFFSET ?"
    );
    let mut items_query = sqlx::query_as::<_, SegmentSearchHit>(&items_sql).bind(project_id);
    for _ in 0..binds {
        items_query = items_query.bind(&pattern);
    }
    let items = items_query.bind(limit).bind(offset).fetch_all(pool).await?;

    Ok(SegmentSearchResult { items, total })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Fresh migrated DB with two projects: p1 owns f1 "Actors.json" and
    /// f2 "Map001.json", p2 owns f3. Segments are inserted per-test.
    async fn seeded_db() -> (SqlitePool, tempfile::NamedTempFile) {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let pool = crate::db::pool::init(tmp.path().to_str().unwrap())
            .await
            .unwrap();
        for (id, name) in [("p1", "One"), ("p2", "Two")] {
            sqlx::query(
                "INSERT INTO projects (id, name, engine, game_path) VALUES (?, ?, 'mv_mz', '/tmp')",
            )
            .bind(id)
            .bind(name)
            .execute(&pool)
            .await
            .unwrap();
        }
        for (id, project, name) in [
            ("f1", "p1", "Actors.json"),
            ("f2", "p1", "Map001.json"),
            ("f3", "p2", "Actors.json"),
        ] {
            sqlx::query(
                "INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
                 VALUES (?, ?, ?, ?, 'actors')",
            )
            .bind(id)
            .bind(project)
            .bind(name)
            .bind(format!("data/{name}"))
            .execute(&pool)
            .await
            .unwrap();
        }
        (pool, tmp)
    }

    async fn insert_segment(pool: &SqlitePool, id: &str, file: &str, source: &str, target: &str) {
        sqlx::query(
            "INSERT INTO segments (id, source_file_id, json_key, source_text, target_text) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(file)
        .bind(format!("/{id}"))
        .bind(source)
        .bind(target)
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_scope_source_excludes_target_hits() {
        let (pool, _tmp) = seeded_db().await;
        // "cool" appears only in the TARGET of s1.
        insert_segment(&pool, "s1", "f1", "かっこいい", "so cool").await;

        let source = search_project_segments(&pool, "p1", "cool", SearchScope::Source, 500, 0)
            .await
            .unwrap();
        assert_eq!(source.total, 0);
        assert!(source.items.is_empty());

        let target = search_project_segments(&pool, "p1", "cool", SearchScope::Target, 500, 0)
            .await
            .unwrap();
        assert_eq!(target.total, 1);
        assert_eq!(target.items[0].id, "s1");
        assert_eq!(target.items[0].file_name, "Actors.json");

        let both = search_project_segments(&pool, "p1", "cool", SearchScope::Both, 500, 0)
            .await
            .unwrap();
        assert_eq!(both.total, 1);
    }

    #[tokio::test]
    async fn test_like_wildcards_escaped() {
        let (pool, _tmp) = seeded_db().await;
        insert_segment(&pool, "s1", "f1", "100% done", "").await;
        insert_segment(&pool, "s2", "f1", "100x done", "").await;
        insert_segment(&pool, "s3", "f1", "a_b", "").await;
        insert_segment(&pool, "s4", "f1", "aXb", "").await;

        // Unescaped, "100%" would match both s1 and s2 ('%' = any run).
        let percent = search_project_segments(&pool, "p1", "100%", SearchScope::Source, 500, 0)
            .await
            .unwrap();
        assert_eq!(percent.total, 1);
        assert_eq!(percent.items[0].id, "s1");

        // Unescaped, "a_b" would match both s3 and s4 ('_' = any char).
        let underscore = search_project_segments(&pool, "p1", "a_b", SearchScope::Source, 500, 0)
            .await
            .unwrap();
        assert_eq!(underscore.total, 1);
        assert_eq!(underscore.items[0].id, "s3");
    }

    #[tokio::test]
    async fn test_cap_and_total() {
        let (pool, _tmp) = seeded_db().await;
        for i in 0..7 {
            insert_segment(&pool, &format!("s{i}"), "f1", "hero appears", "").await;
        }

        let result = search_project_segments(&pool, "p1", "hero", SearchScope::Source, 5, 0)
            .await
            .unwrap();
        assert_eq!(result.items.len(), 5);
        assert_eq!(result.total, 7);
    }

    #[tokio::test]
    async fn test_offset_batches_are_contiguous() {
        let (pool, _tmp) = seeded_db().await;
        // 4 hits on f1 (Actors.json) + 3 on f2 (Map001.json), interleaved at
        // insertion. Batch 2 must resume exactly where batch 1 stopped —
        // including mid-file (batch 1 ends inside Map001.json's run).
        insert_segment(&pool, "s1", "f2", "hero one", "").await;
        insert_segment(&pool, "s2", "f1", "hero two", "").await;
        insert_segment(&pool, "s3", "f2", "hero three", "").await;
        insert_segment(&pool, "s4", "f1", "hero four", "").await;
        insert_segment(&pool, "s5", "f1", "hero five", "").await;
        insert_segment(&pool, "s6", "f2", "hero six", "").await;
        insert_segment(&pool, "s7", "f1", "hero seven", "").await;

        let full = search_project_segments(&pool, "p1", "hero", SearchScope::Source, 500, 0)
            .await
            .unwrap();
        let batch1 = search_project_segments(&pool, "p1", "hero", SearchScope::Source, 5, 0)
            .await
            .unwrap();
        let batch2 = search_project_segments(&pool, "p1", "hero", SearchScope::Source, 5, 5)
            .await
            .unwrap();

        assert_eq!(batch1.items.len(), 5);
        assert_eq!(batch2.items.len(), 2);
        assert_eq!(batch1.total, 7);
        assert_eq!(batch2.total, 7);

        // Concatenated batches == the full ordering, no gap, no overlap.
        let full_ids: Vec<&str> = full.items.iter().map(|h| h.id.as_str()).collect();
        let stitched: Vec<&str> = batch1
            .items
            .iter()
            .chain(batch2.items.iter())
            .map(|h| h.id.as_str())
            .collect();
        assert_eq!(stitched, full_ids);
        // f1 (Actors.json) run first, then f2 — grouping survives the stitch.
        assert_eq!(stitched, ["s2", "s4", "s5", "s7", "s1", "s3", "s6"]);
    }

    #[tokio::test]
    async fn test_ordering_groups_contiguous() {
        let (pool, _tmp) = seeded_db().await;
        // Interleave inserts across f1/f2 — results must still group by file.
        insert_segment(&pool, "s1", "f2", "hero one", "").await;
        insert_segment(&pool, "s2", "f1", "hero two", "").await;
        insert_segment(&pool, "s3", "f2", "hero three", "").await;
        insert_segment(&pool, "s4", "f1", "hero four", "").await;

        let result = search_project_segments(&pool, "p1", "hero", SearchScope::Source, 500, 0)
            .await
            .unwrap();
        let files: Vec<&str> = result.items.iter().map(|h| h.file_name.as_str()).collect();
        // Actors.json (f1) sorts before Map001.json (f2); rowid order inside.
        assert_eq!(
            files,
            ["Actors.json", "Actors.json", "Map001.json", "Map001.json"]
        );
        let ids: Vec<&str> = result.items.iter().map(|h| h.id.as_str()).collect();
        assert_eq!(ids, ["s2", "s4", "s1", "s3"]);
    }

    #[tokio::test]
    async fn test_project_scoping() {
        let (pool, _tmp) = seeded_db().await;
        insert_segment(&pool, "s1", "f1", "hero of p1", "").await;
        insert_segment(&pool, "s2", "f3", "hero of p2", "").await;

        let result = search_project_segments(&pool, "p1", "hero", SearchScope::Source, 500, 0)
            .await
            .unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.items[0].id, "s1");
    }

    #[tokio::test]
    async fn test_short_query_returns_empty() {
        let (pool, _tmp) = seeded_db().await;
        insert_segment(&pool, "s1", "f1", "abc", "").await;

        let result = search_project_segments(&pool, "p1", " a ", SearchScope::Both, 500, 0)
            .await
            .unwrap();
        assert_eq!(result.total, 0);
        assert!(result.items.is_empty());
    }
}
