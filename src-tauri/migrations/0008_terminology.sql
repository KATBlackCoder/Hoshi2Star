-- Hoshi2Star terminology library.
--
-- Source entries are global and intentionally do not reference projects.
-- Project deletion removes only occurrences, scan state and project-scoped
-- translations. Historical glossary data is copied once and left untouched
-- so a failed application upgrade never destroys the old representation.

CREATE TABLE terminology_entries (
    id               TEXT PRIMARY KEY NOT NULL,
    source_language  TEXT NOT NULL,
    canonical_text   TEXT NOT NULL,
    normalized_text  TEXT NOT NULL,
    reading          TEXT,
    part_of_speech   TEXT NOT NULL
                         CHECK(part_of_speech IN (
                             'noun', 'proper_noun', 'verb', 'adjective',
                             'adverb', 'expression', 'unknown'
                         )),
    semantic_type    TEXT NOT NULL,
    sense_key        TEXT NOT NULL DEFAULT '',
    status           TEXT NOT NULL DEFAULT 'active'
                         CHECK(status IN ('active', 'ignored', 'archived')),
    origin           TEXT NOT NULL
                         CHECK(origin IN (
                             'engine', 'lindera', 'manual',
                             'legacy_glossary', 'import'
                         )),
    confidence       REAL NOT NULL DEFAULT 1.0
                         CHECK(confidence >= 0.0 AND confidence <= 1.0),
    created_at       TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at       TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(source_language, normalized_text, semantic_type, sense_key)
);

CREATE INDEX idx_terminology_entries_lookup
    ON terminology_entries(source_language, normalized_text);
CREATE INDEX idx_terminology_entries_filters
    ON terminology_entries(source_language, part_of_speech, semantic_type, status);

CREATE TABLE terminology_translations (
    id               TEXT PRIMARY KEY NOT NULL,
    entry_id         TEXT NOT NULL REFERENCES terminology_entries(id) ON DELETE CASCADE,
    target_language  TEXT NOT NULL,
    project_id       TEXT REFERENCES projects(id) ON DELETE CASCADE,
    target_text      TEXT NOT NULL CHECK(length(trim(target_text)) > 0),
    review_status    TEXT NOT NULL
                         CHECK(review_status IN ('proposed', 'approved', 'locked')),
    enforcement      TEXT NOT NULL
                         CHECK(enforcement IN ('contextual', 'preferred', 'required')),
    confidence       REAL NOT NULL DEFAULT 1.0
                         CHECK(confidence >= 0.0 AND confidence <= 1.0),
    provider_id      TEXT,
    model            TEXT,
    created_at       TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at       TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX idx_terminology_translation_global
    ON terminology_translations(entry_id, target_language)
    WHERE project_id IS NULL;
CREATE UNIQUE INDEX idx_terminology_translation_project
    ON terminology_translations(entry_id, target_language, project_id)
    WHERE project_id IS NOT NULL;
CREATE INDEX idx_terminology_translation_review
    ON terminology_translations(target_language, review_status, enforcement);

CREATE TABLE terminology_source_variants (
    id               TEXT PRIMARY KEY NOT NULL,
    entry_id         TEXT NOT NULL REFERENCES terminology_entries(id) ON DELETE CASCADE,
    surface_text     TEXT NOT NULL,
    normalized_text  TEXT NOT NULL,
    variant_kind     TEXT NOT NULL
                         CHECK(variant_kind IN ('inflection', 'alias', 'orthography', 'reading')),
    UNIQUE(entry_id, normalized_text, variant_kind)
);

CREATE INDEX idx_terminology_source_variants_lookup
    ON terminology_source_variants(normalized_text);

CREATE TABLE terminology_target_variants (
    id               TEXT PRIMARY KEY NOT NULL,
    translation_id   TEXT NOT NULL REFERENCES terminology_translations(id) ON DELETE CASCADE,
    text             TEXT NOT NULL,
    normalized_text  TEXT NOT NULL,
    UNIQUE(translation_id, normalized_text)
);

CREATE TABLE terminology_occurrences (
    entry_id          TEXT NOT NULL REFERENCES terminology_entries(id) ON DELETE CASCADE,
    project_id        TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    segment_id        TEXT NOT NULL REFERENCES segments(id) ON DELETE CASCADE,
    surface_text      TEXT NOT NULL,
    engine_kind       TEXT NOT NULL,
    occurrence_count  INTEGER NOT NULL DEFAULT 1 CHECK(occurrence_count > 0),
    PRIMARY KEY(entry_id, segment_id, surface_text)
);

CREATE INDEX idx_terminology_occurrences_project_segment
    ON terminology_occurrences(project_id, segment_id);
CREATE INDEX idx_terminology_occurrences_project_entry
    ON terminology_occurrences(project_id, entry_id);

CREATE TABLE terminology_segment_scans (
    segment_id        TEXT PRIMARY KEY NOT NULL REFERENCES segments(id) ON DELETE CASCADE,
    project_id        TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    source_hash       TEXT NOT NULL,
    analyzer_version  TEXT NOT NULL,
    scanned_at        TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_terminology_segment_scans_project
    ON terminology_segment_scans(project_id);

CREATE TABLE terminology_scans (
    id                   TEXT PRIMARY KEY NOT NULL,
    project_id           TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    source_language      TEXT NOT NULL,
    status               TEXT NOT NULL
                             CHECK(status IN ('running', 'completed', 'cancelled', 'failed')),
    analyzer_version     TEXT NOT NULL,
    processed_segments   INTEGER NOT NULL DEFAULT 0 CHECK(processed_segments >= 0),
    total_segments       INTEGER NOT NULL DEFAULT 0 CHECK(total_segments >= 0),
    discovered_entries   INTEGER NOT NULL DEFAULT 0 CHECK(discovered_entries >= 0),
    started_at           TEXT NOT NULL DEFAULT (datetime('now')),
    finished_at          TEXT,
    error                TEXT
);

CREATE INDEX idx_terminology_scans_project_started
    ON terminology_scans(project_id, started_at DESC);

-- Migrate one global source entry per normalized source/type. SQLite cannot
-- perform full Unicode NFKC, so 0008 uses trim/lower for legacy rows. All new
-- writes use the Rust normalizer; Japanese legacy terms are unaffected.
WITH legacy_entries AS (
    SELECT
        CASE
            WHEN instr(lang_pair, '-') > 1
                THEN substr(lang_pair, 1, instr(lang_pair, '-') - 1)
            ELSE 'ja'
        END AS source_language,
        trim(source_text) AS canonical_text,
        lower(trim(source_text)) AS normalized_text,
        CASE
            WHEN trim(domain) = '' THEN 'general'
            ELSE lower(trim(domain))
        END AS semantic_type,
        MIN(id) AS legacy_id,
        MAX(CASE WHEN auto_generated = 0 THEN 1.0 ELSE 0.5 END) AS confidence
    FROM glossary_terms
    WHERE trim(source_text) <> ''
    GROUP BY source_language, normalized_text, semantic_type
)
INSERT INTO terminology_entries (
    id, source_language, canonical_text, normalized_text, reading,
    part_of_speech, semantic_type, sense_key, status, origin, confidence
)
SELECT
    'legacy-entry:' || legacy_id,
    source_language,
    canonical_text,
    normalized_text,
    NULL,
    'unknown',
    semantic_type,
    '',
    'active',
    'legacy_glossary',
    confidence
FROM legacy_entries;

-- Keep the newest row if a historical database contains duplicate definitions
-- for exactly the same source, target language and scope.
WITH legacy_translations AS (
    SELECT
        g.id AS legacy_id,
        CASE
            WHEN instr(g.lang_pair, '-') > 1
                THEN substr(g.lang_pair, 1, instr(g.lang_pair, '-') - 1)
            ELSE 'ja'
        END AS source_language,
        CASE
            WHEN instr(g.lang_pair, '-') > 0
                THEN substr(g.lang_pair, instr(g.lang_pair, '-') + 1)
            ELSE 'en'
        END AS target_language,
        lower(trim(g.source_text)) AS normalized_text,
        CASE
            WHEN trim(g.domain) = '' THEN 'general'
            ELSE lower(trim(g.domain))
        END AS semantic_type,
        g.project_id,
        trim(g.target_text) AS target_text,
        g.auto_generated,
        ROW_NUMBER() OVER (
            PARTITION BY
                CASE
                    WHEN instr(g.lang_pair, '-') > 1
                        THEN substr(g.lang_pair, 1, instr(g.lang_pair, '-') - 1)
                    ELSE 'ja'
                END,
                lower(trim(g.source_text)),
                CASE
                    WHEN trim(g.domain) = '' THEN 'general'
                    ELSE lower(trim(g.domain))
                END,
                CASE
                    WHEN instr(g.lang_pair, '-') > 0
                        THEN substr(g.lang_pair, instr(g.lang_pair, '-') + 1)
                    ELSE 'en'
                END,
                coalesce(g.project_id, '')
            ORDER BY g.updated_at DESC, g.id DESC
        ) AS scope_rank
    FROM glossary_terms g
    WHERE trim(g.source_text) <> '' AND trim(g.target_text) <> ''
)
INSERT INTO terminology_translations (
    id, entry_id, target_language, project_id, target_text,
    review_status, enforcement, confidence, provider_id, model
)
SELECT
    'legacy-translation:' || lt.legacy_id,
    te.id,
    lt.target_language,
    lt.project_id,
    lt.target_text,
    CASE WHEN lt.auto_generated = 1 THEN 'proposed' ELSE 'approved' END,
    CASE WHEN lt.auto_generated = 1 THEN 'preferred' ELSE 'required' END,
    CASE WHEN lt.auto_generated = 1 THEN 0.5 ELSE 1.0 END,
    NULL,
    NULL
FROM legacy_translations lt
JOIN terminology_entries te
  ON te.source_language = lt.source_language
 AND te.normalized_text = lt.normalized_text
 AND te.semantic_type = lt.semantic_type
 AND te.sense_key = ''
WHERE lt.scope_rank = 1;
