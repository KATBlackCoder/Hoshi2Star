CREATE TABLE IF NOT EXISTS glossary_terms (
    id             TEXT    PRIMARY KEY NOT NULL,
    source_text    TEXT    NOT NULL,
    target_text    TEXT    NOT NULL,
    lang_pair      TEXT    NOT NULL,            -- 'ja-en', 'ja-fr', ...
    domain         TEXT    NOT NULL DEFAULT '', -- 'character', 'skill', 'item', 'state', ''
    project_id     TEXT    REFERENCES projects(id) ON DELETE CASCADE, -- NULL = global
    auto_generated INTEGER NOT NULL DEFAULT 0, -- 1 si généré par LLM
    created_at     TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at     TEXT    NOT NULL DEFAULT (datetime('now'))
);
-- Index composite : lookup par projet + paire de langues
CREATE INDEX IF NOT EXISTS idx_glossary_project_lang
    ON glossary_terms(project_id, lang_pair);
-- Index lookup global (project_id IS NULL)
CREATE INDEX IF NOT EXISTS idx_glossary_global_lang
    ON glossary_terms(lang_pair) WHERE project_id IS NULL;
