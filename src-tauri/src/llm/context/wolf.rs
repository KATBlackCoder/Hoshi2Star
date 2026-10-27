//! Wolf RPG segment facts. No prompt context yet: only each segment's key,
//! which tells a message from a choice or a database field (plan WR-E6).

use std::collections::HashMap;

use sqlx::{QueryBuilder, Sqlite, SqlitePool};

use super::SegmentContextBundle;

/// Ids bound per query: the `IN (...)` list is chunked as in MV/MZ.
const ID_CHUNK: usize = 400;

pub(super) async fn build(
    db: &SqlitePool,
    segment_ids: &[String],
) -> Result<Vec<SegmentContextBundle>, sqlx::Error> {
    let mut keys: HashMap<String, String> = HashMap::with_capacity(segment_ids.len());
    for chunk in segment_ids.chunks(ID_CHUNK) {
        let mut query =
            QueryBuilder::<Sqlite>::new("SELECT id, json_key FROM segments WHERE id IN (");
        {
            let mut ids = query.separated(", ");
            for id in chunk {
                ids.push_bind(id);
            }
        }
        query.push(")");
        let rows: Vec<(String, String)> = query.build_query_as().fetch_all(db).await?;
        keys.extend(rows);
    }
    Ok(segment_ids
        .iter()
        .map(|id| SegmentContextBundle {
            json_key: keys.get(id).cloned(),
            ..SegmentContextBundle::default()
        })
        .collect())
}
