use std::collections::HashMap;

use sqlx::{FromRow, QueryBuilder, Sqlite, SqlitePool};

use super::{mv_mz_context_class, NeighborLine, PromptContextClass, SegmentPromptContext};

const NEIGHBOR_RADIUS: i64 = 2;

pub(super) async fn build(
    db: &SqlitePool,
    segment_ids: &[String],
) -> Result<Vec<Option<SegmentPromptContext>>, sqlx::Error> {
    if segment_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut query = QueryBuilder::<Sqlite>::new(
        "SELECT c.id AS center_id, c.segment_kind AS center_kind, \
                c.scene_id AS center_scene_id, c.sequence_index AS center_sequence, \
                c.speaker AS center_speaker, c.branch_path AS center_branch_path, \
                n.segment_kind AS neighbor_kind, n.sequence_index AS neighbor_sequence, \
                n.speaker AS neighbor_speaker, n.source_text AS neighbor_text \
         FROM segments c \
         LEFT JOIN segments n \
           ON c.scene_id IS NOT NULL \
          AND c.sequence_index IS NOT NULL \
          AND c.segment_kind IN ('dialogue', 'scrolling_text', 'plugin_text') \
          AND n.source_file_id = c.source_file_id \
          AND n.scene_id = c.scene_id \
          AND n.branch_path IS c.branch_path \
          AND n.sequence_index BETWEEN c.sequence_index - ",
    );
    query
        .push_bind(NEIGHBOR_RADIUS)
        .push(" AND c.sequence_index + ")
        .push_bind(NEIGHBOR_RADIUS)
        .push(" AND n.id <> c.id WHERE c.id IN (");
    {
        let mut ids = query.separated(", ");
        for id in segment_ids {
            ids.push_bind(id);
        }
    }
    query.push(") ORDER BY c.rowid, n.sequence_index");

    let rows = query.build_query_as::<JoinedRow>().fetch_all(db).await?;
    let mut contexts: HashMap<String, SegmentPromptContext> =
        HashMap::with_capacity(segment_ids.len());

    for row in rows {
        let class = mv_mz_context_class(&row.center_kind);
        let context = contexts.entry(row.center_id).or_insert_with(|| {
            let (scene_id, speaker, branch_path) = match class {
                PromptContextClass::Dialogue => (
                    row.center_scene_id,
                    row.center_speaker,
                    row.center_branch_path,
                ),
                PromptContextClass::Branch => (None, None, row.center_branch_path),
                PromptContextClass::Canonical | PromptContextClass::Isolated => (None, None, None),
            };
            SegmentPromptContext {
                segment_kind: row.center_kind,
                scene_id,
                speaker,
                branch_path,
                previous: Vec::with_capacity(NEIGHBOR_RADIUS as usize),
                following: Vec::with_capacity(NEIGHBOR_RADIUS as usize),
            }
        });

        let Some((neighbor_sequence, neighbor_kind, neighbor_text)) = row
            .neighbor_sequence
            .zip(row.neighbor_kind)
            .zip(row.neighbor_text)
            .map(|((sequence, kind), text)| (sequence, kind, text))
        else {
            continue;
        };
        let Some(center_sequence) = row.center_sequence else {
            continue;
        };
        let line = NeighborLine {
            segment_kind: neighbor_kind,
            speaker: row.neighbor_speaker,
            text: neighbor_text,
        };
        if neighbor_sequence < center_sequence {
            context.previous.push(line);
        } else {
            context.following.push(line);
        }
    }

    Ok(segment_ids
        .iter()
        .map(|id| contexts.get(id).cloned())
        .collect())
}

#[derive(Debug, FromRow)]
struct JoinedRow {
    center_id: String,
    center_kind: String,
    center_scene_id: Option<String>,
    center_sequence: Option<i64>,
    center_speaker: Option<String>,
    center_branch_path: Option<String>,
    neighbor_kind: Option<String>,
    neighbor_sequence: Option<i64>,
    neighbor_speaker: Option<String>,
    neighbor_text: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn seeded_db() -> (SqlitePool, tempfile::NamedTempFile) {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let pool = crate::db::pool::init(temp.path().to_str().unwrap())
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO projects (id, name, engine, game_path) \
             VALUES ('p1', 'Test', 'mv_mz', '/tmp')",
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

        for (id, sequence, text, branch, speaker) in [
            ("s0", 0, "前二", "if:1", None),
            ("s1", 1, "前一", "if:1", Some("仲間")),
            ("center", 2, "現在", "if:1", Some("勇者")),
            ("s3", 3, "後一", "if:1", Some("勇者")),
            ("s4", 4, "後二", "if:1", None),
            ("s5", 5, "遠すぎる", "if:1", None),
            ("other", 3, "other branch", "else:6", None),
        ] {
            sqlx::query(
                "INSERT INTO segments \
                 (id, source_file_id, json_key, source_text, segment_kind, scene_id, \
                  sequence_index, speaker, branch_path) \
                 VALUES (?, 'f1', ?, ?, 'dialogue', 'Map001.json:event:1:page:0', ?, ?, ?)",
            )
            .bind(id)
            .bind(format!("/{id}"))
            .bind(text)
            .bind(sequence)
            .bind(speaker)
            .bind(branch)
            .execute(&pool)
            .await
            .unwrap();
        }
        (pool, temp)
    }

    #[tokio::test]
    async fn context_is_bounded_and_does_not_cross_branch_boundaries() {
        let (pool, _temp) = seeded_db().await;

        let contexts = build(&pool, &["center".to_string()]).await.unwrap();
        let center = contexts[0].as_ref().unwrap();

        assert_eq!(center.segment_kind, "dialogue");
        assert_eq!(center.speaker.as_deref(), Some("勇者"));
        assert_eq!(center.previous.len(), 2);
        assert_eq!(center.following.len(), 2);
        assert_eq!(center.previous[0].text, "前二");
        assert_eq!(center.previous[1].text, "前一");
        assert_eq!(center.following[0].text, "後一");
        assert_eq!(center.following[1].text, "後二");
        assert!(center
            .previous
            .iter()
            .chain(&center.following)
            .all(|line| line.text != "other branch" && line.text != "遠すぎる"));
    }

    #[tokio::test]
    async fn contexts_preserve_requested_id_order_and_missing_rows() {
        let (pool, _temp) = seeded_db().await;

        let contexts = build(
            &pool,
            &["s3".to_string(), "missing".to_string(), "s1".to_string()],
        )
        .await
        .unwrap();

        assert_eq!(contexts.len(), 3);
        assert_eq!(
            contexts[0].as_ref().unwrap().speaker.as_deref(),
            Some("勇者")
        );
        assert_eq!(contexts[1], None);
        assert_eq!(
            contexts[2].as_ref().unwrap().speaker.as_deref(),
            Some("仲間")
        );
    }

    #[tokio::test]
    async fn non_dialogue_context_is_sanitized_by_kind() {
        let (pool, _temp) = seeded_db().await;
        for (id, kind, branch) in [
            ("speaker", "speaker", Some("if:1")),
            ("choice", "choice", Some("choice:2")),
            ("term", "system_term", None),
        ] {
            sqlx::query(
                "INSERT INTO segments \
                 (id, source_file_id, json_key, source_text, segment_kind, scene_id, \
                  sequence_index, speaker, branch_path) \
                 VALUES (?, 'f1', ?, '同じ', ?, 'Map001.json:event:1:page:0', 2, '勇者', ?)",
            )
            .bind(id)
            .bind(format!("/{id}"))
            .bind(kind)
            .bind(branch)
            .execute(&pool)
            .await
            .unwrap();
        }

        let contexts = build(&pool, &["speaker".into(), "choice".into(), "term".into()])
            .await
            .unwrap();

        let speaker = contexts[0].as_ref().unwrap();
        assert_eq!(speaker.segment_kind, "speaker");
        assert!(speaker.scene_id.is_none());
        assert!(speaker.speaker.is_none());
        assert!(speaker.branch_path.is_none());
        assert!(speaker.previous.is_empty() && speaker.following.is_empty());

        let choice = contexts[1].as_ref().unwrap();
        assert_eq!(choice.branch_path.as_deref(), Some("choice:2"));
        assert!(choice.previous.is_empty() && choice.following.is_empty());

        let term = contexts[2].as_ref().unwrap();
        assert!(term.scene_id.is_none());
        assert!(term.previous.is_empty() && term.following.is_empty());
    }
}
