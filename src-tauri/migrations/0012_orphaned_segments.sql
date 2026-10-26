-- Realignment after a game update (lot R2b).
--
-- A translated segment whose line disappeared from the game (removed, or its
-- source rewritten) is never deleted silently: it is kept here, with what it
-- held, before its row leaves `segments`. Its translation is also added to
-- the translation memory.
CREATE TABLE IF NOT EXISTS orphaned_segments (
    -- The former `segments.id`.
    id            TEXT PRIMARY KEY NOT NULL,
    project_id    TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    file_name     TEXT NOT NULL,
    json_key      TEXT NOT NULL,
    source_text   TEXT NOT NULL,
    target_text   TEXT NOT NULL,
    status        TEXT NOT NULL,
    qa_score      INTEGER,
    segment_kind  TEXT NOT NULL,
    text_category TEXT,
    orphaned_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_orphaned_segments_project
    ON orphaned_segments(project_id);
