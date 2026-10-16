-- Text fitting (RPG Maker MZ): wrap dialogue and descriptions to the real
-- text boxes at export. Translations themselves are never rewritten.

-- Per-project settings. Off until the user enables it.
CREATE TABLE IF NOT EXISTS project_text_fit (
    project_id           TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
    enabled_dialogue     INTEGER NOT NULL DEFAULT 0 CHECK (enabled_dialogue IN (0, 1)),
    enabled_descriptions INTEGER NOT NULL DEFAULT 0 CHECK (enabled_descriptions IN (0, 1)),
    line_breaks          TEXT NOT NULL DEFAULT 'reflow' CHECK (line_breaks IN ('reflow', 'keep')),
    overflow             TEXT NOT NULL DEFAULT 'new_page'
                         CHECK (overflow IN ('new_page', 'shrink', 'leave')),
    min_font_size        INTEGER NOT NULL DEFAULT 20,
    -- Dialogue font size emitted at every page start (NULL = the game's).
    dialogue_font_size   INTEGER,
    updated_at           TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Per-message / per-description choices. `pointer` is the 101 command (or the
-- description field); commands are never inserted, so it is stable.
-- `source_hash` is the original text the choice was made for: a game update
-- that changes the text makes the override stale (ignored).
CREATE TABLE IF NOT EXISTS text_fit_overrides (
    project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    file_name   TEXT NOT NULL,
    pointer     TEXT NOT NULL,
    source_hash TEXT NOT NULL,
    line_breaks TEXT CHECK (line_breaks IN ('reflow', 'keep')),
    overflow    TEXT CHECK (overflow IN ('new_page', 'shrink', 'leave')),
    disabled    INTEGER NOT NULL DEFAULT 0 CHECK (disabled IN (0, 1)),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (project_id, file_name, pointer)
);
