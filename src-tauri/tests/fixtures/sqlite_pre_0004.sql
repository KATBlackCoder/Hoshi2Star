-- Immutable snapshot of a database installed through migrations 0001-0003.
--
-- Keep both the historical schema and migration ledger literal. In particular,
-- do not generate this fixture from the current SQLx migrator: its fixed SHA-384
-- checksums are intended to expose any later rewrite of migrations 0001-0003.

CREATE TABLE IF NOT EXISTS _sqlx_migrations (
    version BIGINT PRIMARY KEY,
    description TEXT NOT NULL,
    installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    success BOOLEAN NOT NULL,
    checksum BLOB NOT NULL,
    execution_time BIGINT NOT NULL
);

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

CREATE TABLE IF NOT EXISTS tm_entries (
    id          TEXT    PRIMARY KEY NOT NULL,
    source_hash TEXT    NOT NULL,
    source_text TEXT    NOT NULL,
    target_text TEXT    NOT NULL,
    engine      TEXT    NOT NULL,
    lang_pair   TEXT    NOT NULL,
    confidence  REAL    NOT NULL DEFAULT 1.0,
    created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_tm_hash_lang ON tm_entries(source_hash, lang_pair);

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

CREATE INDEX IF NOT EXISTS idx_glossary_project_lang
    ON glossary_terms(project_id, lang_pair);

CREATE INDEX IF NOT EXISTS idx_glossary_global_lang
    ON glossary_terms(lang_pair) WHERE project_id IS NULL;

INSERT INTO _sqlx_migrations
    (version, description, installed_on, success, checksum, execution_time)
VALUES
    (
        1,
        'initial',
        '2026-05-24 00:00:01',
        TRUE,
        X'42fc75cad660730b3fe93fb5c838297a5465e841b1180e95168fbce932f05fea40e11374bd81b75a09046466fe9afb80',
        0
    ),
    (
        2,
        'tm',
        '2026-05-24 00:00:02',
        TRUE,
        X'afc0f2aadf402217f873a6020d9492f62b0f83667ad4f04ee78c835dd4cbea4579278a0fc57bcb3582325da4a0070c4c',
        0
    ),
    (
        3,
        'glossary',
        '2026-05-24 00:00:03',
        TRUE,
        X'3729d6b6593f3fdee56a9830a0babf8582353e10cedf70825ff343b89eca11e530785d85102b4c33a473325de00db89b',
        0
    );
