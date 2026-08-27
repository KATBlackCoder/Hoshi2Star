//! Provider-neutral translation context dispatch.
//!
//! Each engine must own its extraction and neighbour-selection rules. A
//! missing adapter returns no context instead of guessing from another engine.

mod mv_mz;

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Prompt-shaping policy owned by an engine adapter.
///
/// The provider remains generic: it receives already-sanitised context and
/// never needs to know RPG Maker segment kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PromptContextClass {
    Dialogue,
    Branch,
    Canonical,
    Isolated,
}

/// MV/MZ semantic groups used by context selection, request grouping and
/// canonical translation reuse. Future engines must define their own mapping.
pub fn mv_mz_context_class(segment_kind: &str) -> PromptContextClass {
    match segment_kind {
        "dialogue" | "scrolling_text" | "plugin_text" => PromptContextClass::Dialogue,
        "choice" => PromptContextClass::Branch,
        "speaker" | "actor_name" | "actor_nickname" | "class_name" | "item_name" | "skill_name"
        | "enemy_name" | "state_name" | "map_name" | "common_event_name" | "system_term"
        | "game_title" => PromptContextClass::Canonical,
        _ => PromptContextClass::Isolated,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeighborLine {
    pub segment_kind: String,
    pub speaker: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentPromptContext {
    pub segment_kind: String,
    pub scene_id: Option<String>,
    pub speaker: Option<String>,
    pub branch_path: Option<String>,
    pub previous: Vec<NeighborLine>,
    pub following: Vec<NeighborLine>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QaNeighborContext {
    pub sources: Vec<String>,
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SegmentContextBundle {
    pub prompt: Option<SegmentPromptContext>,
    pub qa_neighbors: QaNeighborContext,
}

/// Build contexts in the same order as `segment_ids`.
///
/// The engine string is parsed only at this persistence boundary. Unsupported
/// engines deliberately receive no context until they gain a dedicated module.
pub async fn build_for_segments(
    db: &SqlitePool,
    engine: &str,
    segment_ids: &[String],
) -> Result<Vec<Option<SegmentPromptContext>>, sqlx::Error> {
    Ok(build_context_bundles(db, engine, segment_ids)
        .await?
        .into_iter()
        .map(|bundle| bundle.prompt)
        .collect())
}

pub async fn build_context_bundles(
    db: &SqlitePool,
    engine: &str,
    segment_ids: &[String],
) -> Result<Vec<SegmentContextBundle>, sqlx::Error> {
    match engine {
        "mv_mz" => mv_mz::build(db, segment_ids).await,
        _ => Ok(vec![SegmentContextBundle::default(); segment_ids.len()]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unsupported_engines_do_not_reuse_mv_mz_context_logic() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let pool = crate::db::pool::init(temp.path().to_str().unwrap())
            .await
            .unwrap();

        let contexts = build_for_segments(&pool, "wolf", &["missing".to_string()])
            .await
            .unwrap();

        assert_eq!(contexts, vec![None]);
    }
}
