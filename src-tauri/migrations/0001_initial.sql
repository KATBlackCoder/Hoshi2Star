-- Hoshi2Star — initial schema
-- F1 : projects · source_files · segments
--
-- Tests SQL (manual, exécuter contre une DB temporaire) :
--   INSERT INTO projects (id, name, engine, game_path) VALUES ('p1', 'Test', 'mv_mz', '/tmp/game');
--   INSERT INTO source_files (id, project_id, file_name, file_path, file_type) VALUES ('f1', 'p1', 'Actors.json', '/tmp/game/data/Actors.json', 'actors');
--   INSERT INTO segments (id, source_file_id, json_key, source_text) VALUES ('s1', 'f1', '/1/name', '主人公');
--   SELECT s.source_text, s.status FROM segments s JOIN source_files sf ON s.source_file_id = sf.id WHERE sf.project_id = 'p1';
--   UPDATE segments SET target_text = 'Hero', status = 'translated', updated_at = datetime('now') WHERE id = 's1';
--   DELETE FROM projects WHERE id = 'p1'; -- cascade → source_files + segments deleted

CREATE TABLE IF NOT EXISTS projects (
    id         TEXT PRIMARY KEY NOT NULL,
    name       TEXT NOT NULL,
    engine     TEXT NOT NULL,
    game_path  TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS source_files (
    id         TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    file_name  TEXT NOT NULL,
    file_path  TEXT NOT NULL,
    file_type  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_source_files_project ON source_files(project_id);

CREATE TABLE IF NOT EXISTS segments (
    id             TEXT PRIMARY KEY NOT NULL,
    source_file_id TEXT NOT NULL REFERENCES source_files(id) ON DELETE CASCADE,
    json_key       TEXT NOT NULL,
    source_text    TEXT NOT NULL,
    target_text    TEXT NOT NULL DEFAULT '',
    status         TEXT NOT NULL DEFAULT 'untranslated'
                       CHECK(status IN ('untranslated', 'translated', 'reviewed', 'needs_review')),
    qa_score       INTEGER,
    created_at     TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at     TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_segments_source_file ON segments(source_file_id);
