-- Optional text categories (plugin / system UI text beyond core extraction).
--
-- Version 0009 is intentionally skipped: it is taken by
-- `0009_terminology_translation_scope.sql` on the codex/hoshiyomi-redesign
-- branch. SQLx applies unapplied lower versions in order, so either branch can
-- merge later without rewriting an already-applied migration.
--
-- Model: a category is enabled exactly when its segments exist. Core
-- extraction rows keep `text_category = NULL`.
ALTER TABLE segments ADD COLUMN text_category TEXT;

CREATE INDEX IF NOT EXISTS idx_segments_text_category
    ON segments(source_file_id, text_category)
    WHERE text_category IS NOT NULL;

-- A category pointer can only exist once per file (no such rows exist yet, so
-- the partial unique index cannot fail on historical data).
CREATE UNIQUE INDEX IF NOT EXISTS idx_segments_category_pointer
    ON segments(source_file_id, json_key)
    WHERE text_category IS NOT NULL;

-- User decisions per project. The candidate list itself is recomputed from the
-- pristine snapshot; only what the user chose (and when they accepted the
-- consequences of a linked category) is stored.
CREATE TABLE IF NOT EXISTS project_text_categories (
    project_id   TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    category_key TEXT NOT NULL,
    enabled      INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    accepted_at  TEXT,
    updated_at   TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (project_id, category_key)
);

-- Translations of a disabled category, kept so re-enabling restores them
-- (matched by file + pointer, and only while the source text is unchanged).
CREATE TABLE IF NOT EXISTS parked_segments (
    project_id   TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    category_key TEXT NOT NULL,
    file_name    TEXT NOT NULL,
    json_key     TEXT NOT NULL,
    source_text  TEXT NOT NULL,
    target_text  TEXT NOT NULL,
    status       TEXT NOT NULL,
    qa_score     INTEGER,
    parked_at    TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (project_id, file_name, json_key)
);
