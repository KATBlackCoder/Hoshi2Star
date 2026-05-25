-- Hoshi2Star — Translation Memory schema
-- F2 : table tm_entries (ADR-003 : TM globale cross-projet)
--
-- source_hash  : SHA-256 du texte source normalisé (trim + lowercase)
-- lang_pair    : ex. 'ja-en', 'ja-fr' — index composite avec source_hash
-- confidence   : 1.0 pour saisie manuelle, < 1.0 pour suggestions LLM

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
