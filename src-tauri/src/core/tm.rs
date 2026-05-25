//! Translation Memory — exact-match lookup backed by SQLite.
//!
//! ## Hashing
//! Source text is normalised before hashing: trim whitespace + lowercase.
//! This ensures that "　Hello 　" and "hello" map to the same bucket and
//! avoids duplicates caused by surrounding whitespace.
//!
//! ## Storage
//! The TM is global (ADR-003): one `tm_entries` table per installation,
//! shared across all projects. `engine` and `lang_pair` columns allow
//! scoped queries while preserving cross-project fuzzy potential (F3).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, SqlitePool};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct TmEntry {
    pub id: String,
    pub source_hash: String,
    pub source_text: String,
    pub target_text: String,
    pub engine: String,
    pub lang_pair: String,
    pub confidence: f64,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// Hash helper
// ---------------------------------------------------------------------------

/// SHA-256 of the normalised source text (trim + lowercase).
///
/// Same input always produces the same hash, regardless of surrounding
/// whitespace or letter casing.
pub fn hash_source(text: &str) -> String {
    let normalised = text.trim().to_lowercase();
    let digest = Sha256::digest(normalised.as_bytes());
    hex::encode(digest)
}

// ---------------------------------------------------------------------------
// Database operations
// ---------------------------------------------------------------------------

/// Insert (or replace) a TM entry.
///
/// Uses `INSERT OR REPLACE` so that re-saving a segment updates the
/// target text without creating a duplicate row.
pub async fn insert(
    source_text: &str,
    target_text: &str,
    engine: &str,
    lang_pair: &str,
    db: &SqlitePool,
) -> Result<(), sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let source_hash = hash_source(source_text);

    sqlx::query(
        "INSERT OR REPLACE INTO tm_entries \
             (id, source_hash, source_text, target_text, engine, lang_pair, confidence) \
         VALUES (?, ?, ?, ?, ?, ?, 1.0)",
    )
    .bind(&id)
    .bind(&source_hash)
    .bind(source_text)
    .bind(target_text)
    .bind(engine)
    .bind(lang_pair)
    .execute(db)
    .await?;

    Ok(())
}

/// Exact-match lookup: returns the most recently saved entry for
/// `(source_hash, lang_pair)`, or `None` if no match exists.
pub async fn lookup_exact(
    source_hash: &str,
    lang_pair: &str,
    db: &SqlitePool,
) -> Result<Option<TmEntry>, sqlx::Error> {
    sqlx::query_as::<_, TmEntry>(
        "SELECT id, source_hash, source_text, target_text, engine, lang_pair, \
                confidence, created_at \
         FROM tm_entries \
         WHERE source_hash = ? AND lang_pair = ? \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(source_hash)
    .bind(lang_pair)
    .fetch_optional(db)
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

    #[test]
    fn test_hash_normalisation() {
        // Trim + lowercase → same hash
        assert_eq!(hash_source("  Hello  "), hash_source("hello"));
        assert_eq!(hash_source("ABC"), hash_source("abc"));
        // Different content → different hash
        assert_ne!(hash_source("hello"), hash_source("world"));
    }

    #[test]
    fn test_hash_deterministic() {
        let h1 = hash_source("主人公");
        let h2 = hash_source("主人公");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // SHA-256 hex string
    }

    #[tokio::test]
    async fn test_insert_and_lookup_exact() {
        let (db, _file) = test_db().await;

        insert("主人公", "Hero", "mv_mz", "ja-en", &db)
            .await
            .expect("insert");

        let hash = hash_source("主人公");
        let entry = lookup_exact(&hash, "ja-en", &db)
            .await
            .expect("lookup")
            .expect("should find entry");

        assert_eq!(entry.source_text, "主人公");
        assert_eq!(entry.target_text, "Hero");
        assert_eq!(entry.engine, "mv_mz");
        assert_eq!(entry.lang_pair, "ja-en");
        assert!((entry.confidence - 1.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_no_match_returns_none() {
        let (db, _file) = test_db().await;

        let hash = hash_source("nonexistent text");
        let result = lookup_exact(&hash, "ja-en", &db).await.expect("lookup");

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_lookup_scoped_to_lang_pair() {
        let (db, _file) = test_db().await;

        insert("主人公", "Hero", "mv_mz", "ja-en", &db)
            .await
            .expect("insert");

        let hash = hash_source("主人公");

        // ja-en → found
        assert!(lookup_exact(&hash, "ja-en", &db).await.unwrap().is_some());
        // ja-fr → not found
        assert!(lookup_exact(&hash, "ja-fr", &db).await.unwrap().is_none());
    }
}
