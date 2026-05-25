//! Batch helpers for the LLM pipeline.
//!
//! ## group_segments
//! Splits a flat list of segment IDs into fixed-size batches ready to be sent
//! to the LLM provider.
//!
//! ## dedup_by_hash
//! Removes duplicate source texts (by hash) from a batch before sending them
//! to the LLM, then returns a mapping so results can be spread back to all
//! original positions.  This avoids paying for the same translation twice when
//! a game reuses the same text string (very common for item descriptions).

use crate::core::tm::hash_source;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A deduplicated segment ready to be sent to the LLM.
#[derive(Debug, Clone)]
pub struct UniqueSegment {
    /// The segment ID representative of this group (first occurrence).
    pub id: String,
    /// Source text (original, not tokenized — tokenization happens in pipeline).
    pub text: String,
    /// SHA-256 hash of the normalised source text.
    pub hash: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Split `ids` into batches of at most `batch_size` elements.
///
/// The last batch may be smaller.  An empty input returns an empty `Vec`.
pub fn group_segments(ids: Vec<String>, batch_size: usize) -> Vec<Vec<String>> {
    assert!(batch_size > 0, "batch_size must be > 0");
    ids.chunks(batch_size).map(|c| c.to_vec()).collect()
}

/// Deduplicate a list of `(id, source_text)` pairs by hash.
///
/// Returns:
/// - `unique`    — one `UniqueSegment` per distinct hash (in first-occurrence order)
/// - `idx_map`   — maps each hash to the list of original indices (into the input
///   slice) that share it
///
/// The caller uses `idx_map` to spread a single translation back to every
/// position that had the same source text.
pub fn dedup_by_hash(
    segments: Vec<(String, String)>,
) -> (Vec<UniqueSegment>, HashMap<String, Vec<usize>>) {
    let mut unique: Vec<UniqueSegment> = Vec::new();
    let mut idx_map: HashMap<String, Vec<usize>> = HashMap::new();
    let mut seen: HashMap<String, ()> = HashMap::new();

    for (orig_idx, (id, text)) in segments.into_iter().enumerate() {
        let hash = hash_source(&text);
        idx_map.entry(hash.clone()).or_default().push(orig_idx);
        if seen.insert(hash.clone(), ()).is_none() {
            unique.push(UniqueSegment { id, text, hash });
        }
    }

    (unique, idx_map)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_segments_even() {
        let ids: Vec<String> = (0..6).map(|i| i.to_string()).collect();
        let batches = group_segments(ids, 2);
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0], vec!["0", "1"]);
        assert_eq!(batches[2], vec!["4", "5"]);
    }

    #[test]
    fn test_group_segments_remainder() {
        let ids: Vec<String> = (0..5).map(|i| i.to_string()).collect();
        let batches = group_segments(ids, 2);
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[2], vec!["4"]);
    }

    #[test]
    fn test_group_segments_empty() {
        assert!(group_segments(vec![], 10).is_empty());
    }

    #[test]
    fn test_dedup_no_duplicates() {
        let segs = vec![
            ("a".to_string(), "Hello".to_string()),
            ("b".to_string(), "World".to_string()),
        ];
        let (unique, idx_map) = dedup_by_hash(segs);
        assert_eq!(unique.len(), 2);
        assert!(idx_map.values().all(|v| v.len() == 1));
    }

    #[test]
    fn test_dedup_with_duplicates() {
        let segs = vec![
            ("a".to_string(), "Hello".to_string()),
            ("b".to_string(), "World".to_string()),
            ("c".to_string(), "Hello".to_string()), // duplicate of "a"
        ];
        let (unique, idx_map) = dedup_by_hash(segs);

        // Only 2 unique hashes
        assert_eq!(unique.len(), 2);

        // Hash for "hello" (normalised) maps to indices 0 and 2
        let hello_hash = hash_source("Hello");
        let indices = idx_map.get(&hello_hash).expect("hello hash");
        assert_eq!(indices.len(), 2);
        assert!(indices.contains(&0));
        assert!(indices.contains(&2));
    }

    #[test]
    fn test_dedup_normalisation() {
        // "Hello" and "  hello  " normalise to the same hash
        let segs = vec![
            ("a".to_string(), "Hello".to_string()),
            ("b".to_string(), "  hello  ".to_string()),
        ];
        let (unique, _) = dedup_by_hash(segs);
        assert_eq!(unique.len(), 1);
    }
}
