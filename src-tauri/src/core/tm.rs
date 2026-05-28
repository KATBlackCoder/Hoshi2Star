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

/// A TM suggestion enriched with a similarity score and match type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TmSuggestion {
    pub entry: TmEntry,
    /// Normalised similarity score in [0.0, 1.0].
    pub score: f32,
    /// `"exact"` when score == 1.0, `"fuzzy"` otherwise.
    pub match_type: String,
}

// ---------------------------------------------------------------------------
// Fuzzy matching — Levenshtein distance
// ---------------------------------------------------------------------------

/// Wagner-Fischer algorithm using two rows (O(m) space).
/// Operates on Unicode `char`s, not bytes — required for correct Japanese scoring.
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let n = a.len();
    let m = b.len();

    if n == 0 {
        return m;
    }
    if m == 0 {
        return n;
    }

    let mut prev: Vec<usize> = (0..=m).collect();
    let mut curr: Vec<usize> = vec![0; m + 1];

    for i in 1..=n {
        curr[0] = i;
        for j in 1..=m {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[m]
}

/// Normalised similarity: `1.0 - edit_distance / max(len_a, len_b)`.
/// Returns `1.0` for identical strings (including two empty strings).
pub fn similarity_score(a: &str, b: &str) -> f32 {
    let max_len = a.chars().count().max(b.chars().count());
    if max_len == 0 {
        return 1.0;
    }
    let dist = levenshtein(a, b);
    1.0 - (dist as f32 / max_len as f32)
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

/// Fuzzy lookup: scans all TM entries for `lang_pair` in memory, scores each
/// against `source_text` using normalised Levenshtein, and returns the top
/// `limit` suggestions with score >= `threshold`.
///
/// Exact matches (score == 1.0) sort to the top naturally.
///
/// # Performance
/// Scans all TM entries in memory. Acceptable for ~5k entries.
/// For larger TMs, consider a trigram index (backlog F5).
pub async fn lookup_fuzzy(
    source_text: &str,
    lang_pair: &str,
    threshold: f32,
    limit: usize,
    db: &SqlitePool,
) -> Result<Vec<TmSuggestion>, sqlx::Error> {
    let entries = sqlx::query_as::<_, TmEntry>(
        "SELECT id, source_hash, source_text, target_text, engine, lang_pair, \
                confidence, created_at \
         FROM tm_entries WHERE lang_pair = ?",
    )
    .bind(lang_pair)
    .fetch_all(db)
    .await?;

    let normalised_query = source_text.trim().to_lowercase();

    let mut suggestions: Vec<TmSuggestion> = entries
        .into_iter()
        .filter_map(|entry| {
            let normalised_entry = entry.source_text.trim().to_lowercase();
            let score = similarity_score(&normalised_query, &normalised_entry);
            if score >= threshold {
                let match_type = if (score - 1.0_f32).abs() < f32::EPSILON {
                    "exact".to_string()
                } else {
                    "fuzzy".to_string()
                };
                Some(TmSuggestion {
                    entry,
                    score,
                    match_type,
                })
            } else {
                None
            }
        })
        .collect();

    suggestions.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    suggestions.truncate(limit);

    Ok(suggestions)
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

    #[test]
    fn test_levenshtein_known_values() {
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(levenshtein("abc", ""), 3);
        assert_eq!(levenshtein("abc", "abc"), 0);
        assert_eq!(levenshtein("", ""), 0);
    }

    #[test]
    fn test_similarity_identical() {
        assert!((similarity_score("こんにちは", "こんにちは") - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_similarity_close() {
        // "こんにちは" vs "こんにちわ" — 1 char differs out of 5
        let score = similarity_score("こんにちは", "こんにちわ");
        assert!(score >= 0.80, "expected >= 0.80, got {score}");
    }

    #[test]
    fn test_similarity_empty() {
        assert!((similarity_score("", "") - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_similarity_ascii() {
        // "hello" vs "helo" → dist=1, max_len=5 → 0.80
        let score = similarity_score("hello", "helo");
        assert!((score - 0.80).abs() < 0.001, "expected ~0.80, got {score}");
    }

    #[tokio::test]
    async fn test_fuzzy_exact_match_returns_score_1() {
        let (db, _file) = test_db().await;
        insert("こんにちは", "Hello", "mv_mz", "ja-en", &db)
            .await
            .expect("insert");

        let results = lookup_fuzzy("こんにちは", "ja-en", 0.80, 10, &db)
            .await
            .expect("fuzzy lookup");

        assert_eq!(results.len(), 1);
        assert!((results[0].score - 1.0).abs() < f32::EPSILON);
        assert_eq!(results[0].match_type, "exact");
        assert_eq!(results[0].entry.target_text, "Hello");
    }

    #[tokio::test]
    async fn test_fuzzy_similar_returns_high_score() {
        let (db, _file) = test_db().await;
        insert("こんにちは", "Hello", "mv_mz", "ja-en", &db)
            .await
            .expect("insert");

        // "こんにちわ" differs by 1 char out of 5 → score ~0.80
        let results = lookup_fuzzy("こんにちわ", "ja-en", 0.80, 10, &db)
            .await
            .expect("fuzzy lookup");

        assert_eq!(results.len(), 1);
        assert!(results[0].score >= 0.80, "score was {}", results[0].score);
        assert_eq!(results[0].match_type, "fuzzy");
    }

    #[tokio::test]
    async fn test_fuzzy_dissimilar_filtered_out() {
        let (db, _file) = test_db().await;
        insert("ABC", "Hello", "mv_mz", "ja-en", &db)
            .await
            .expect("insert");

        let results = lookup_fuzzy("XYZ", "ja-en", 0.80, 10, &db)
            .await
            .expect("fuzzy lookup");

        assert!(
            results.is_empty(),
            "expected no results for dissimilar text"
        );
    }

    #[tokio::test]
    async fn test_fuzzy_threshold_respected() {
        let (db, _file) = test_db().await;
        // "abcde" vs "abcde" → score 1.0 (above)
        insert("abcde", "T1", "mv_mz", "ja-en", &db)
            .await
            .expect("insert");
        // "abcde" vs "abcxx" → dist=2, max=5 → score 0.60 (below 0.80)
        insert("abcxx", "T2", "mv_mz", "ja-en", &db)
            .await
            .expect("insert");

        let results = lookup_fuzzy("abcde", "ja-en", 0.80, 10, &db)
            .await
            .expect("fuzzy lookup");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entry.target_text, "T1");
    }

    #[tokio::test]
    async fn test_fuzzy_sorted_by_score_descending() {
        let (db, _file) = test_db().await;
        // Exact match
        insert("abcde", "Exact", "mv_mz", "ja-en", &db)
            .await
            .expect("insert");
        // 1 diff → score 0.80
        insert("abcdx", "Near", "mv_mz", "ja-en", &db)
            .await
            .expect("insert");

        let results = lookup_fuzzy("abcde", "ja-en", 0.80, 10, &db)
            .await
            .expect("fuzzy lookup");

        assert!(results.len() >= 2);
        assert!(
            results[0].score >= results[1].score,
            "results not sorted descending"
        );
        assert_eq!(results[0].entry.target_text, "Exact");
    }

    #[tokio::test]
    async fn test_fuzzy_empty_tm_returns_empty() {
        let (db, _file) = test_db().await;

        let results = lookup_fuzzy("anything", "ja-en", 0.80, 10, &db)
            .await
            .expect("fuzzy lookup");

        assert!(results.is_empty());
    }
}
