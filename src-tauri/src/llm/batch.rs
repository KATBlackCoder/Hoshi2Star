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
use crate::llm::context::SegmentPromptContext;
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
    /// Context-sensitive key used only to deduplicate this request batch.
    pub dedup_key: DedupKey,
    /// Position of the representative segment in the original input.
    pub original_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DedupKey {
    pub source_hash: String,
    pub prompt_context: Option<SegmentPromptContext>,
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
    segments: &[(String, String)],
) -> (Vec<UniqueSegment>, HashMap<String, Vec<usize>>) {
    let contexts = vec![None; segments.len()];
    let (unique, contextual_map) = dedup_with_context(segments, &contexts);
    let idx_map = contextual_map
        .into_iter()
        .map(|(key, positions)| (key.source_hash, positions))
        .collect();
    (unique, idx_map)
}

/// Deduplicate equal source strings only when their prompt contexts are also
/// equal. This prevents one ambiguous line from borrowing another speaker or
/// scene merely because the source text is identical.
pub fn dedup_with_context(
    segments: &[(String, String)],
    contexts: &[Option<SegmentPromptContext>],
) -> (Vec<UniqueSegment>, HashMap<DedupKey, Vec<usize>>) {
    let mut unique: Vec<UniqueSegment> = Vec::with_capacity(segments.len());
    let mut idx_map: HashMap<DedupKey, Vec<usize>> = HashMap::with_capacity(segments.len());

    for (orig_idx, (id, text)) in segments.iter().enumerate() {
        let hash = hash_source(text);
        let dedup_key = DedupKey {
            source_hash: hash.clone(),
            prompt_context: contexts.get(orig_idx).cloned().flatten(),
        };
        let positions = idx_map.entry(dedup_key.clone()).or_default();
        let is_first = positions.is_empty();
        positions.push(orig_idx);
        if is_first {
            unique.push(UniqueSegment {
                id: id.clone(),
                text: text.clone(),
                hash,
                dedup_key,
                original_index: orig_idx,
            });
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
        let (unique, idx_map) = dedup_by_hash(&segs);
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
        let (unique, idx_map) = dedup_by_hash(&segs);

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
        let (unique, _) = dedup_by_hash(&segs);
        assert_eq!(unique.len(), 1);
    }

    fn prompt_context(speaker: &str) -> Option<SegmentPromptContext> {
        Some(SegmentPromptContext {
            segment_kind: "dialogue".to_string(),
            scene_id: Some("Map001.json:event:1:page:0".to_string()),
            speaker: Some(speaker.to_string()),
            branch_path: None,
            previous: vec![],
            following: vec![],
        })
    }

    #[test]
    fn equal_text_with_different_context_is_not_deduplicated() {
        let segments = vec![
            ("a".to_string(), "そうです".to_string()),
            ("b".to_string(), "そうです".to_string()),
        ];
        let contexts = vec![prompt_context("勇者"), prompt_context("魔王")];

        let (unique, _) = dedup_with_context(&segments, &contexts);

        assert_eq!(unique.len(), 2);
    }

    #[test]
    fn equal_text_with_equal_context_is_still_deduplicated() {
        let segments = vec![
            ("a".to_string(), "そうです".to_string()),
            ("b".to_string(), "そうです".to_string()),
        ];
        let contexts = vec![prompt_context("勇者"), prompt_context("勇者")];

        let (unique, index_map) = dedup_with_context(&segments, &contexts);

        assert_eq!(unique.len(), 1);
        assert_eq!(index_map.values().next().unwrap(), &[0, 1]);
    }
}
