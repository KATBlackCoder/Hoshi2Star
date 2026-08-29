-- Terminology translations are usable by their existence. Review/lock state is
-- deliberately removed; project/global scope is the persistence lifecycle.
-- Legacy proposed rows are not trusted and therefore become untranslated.

CREATE TABLE terminology_translations_v9 (
    id               TEXT PRIMARY KEY NOT NULL,
    entry_id         TEXT NOT NULL REFERENCES terminology_entries(id) ON DELETE CASCADE,
    target_language  TEXT NOT NULL,
    project_id       TEXT REFERENCES projects(id) ON DELETE CASCADE,
    target_text      TEXT NOT NULL CHECK(length(trim(target_text)) > 0),
    enforcement      TEXT NOT NULL
                         CHECK(enforcement IN ('contextual', 'preferred', 'required')),
    confidence       REAL NOT NULL DEFAULT 1.0
                         CHECK(confidence >= 0.0 AND confidence <= 1.0),
    provider_id      TEXT,
    model            TEXT,
    created_at       TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at       TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO terminology_translations_v9 (
    id, entry_id, target_language, project_id, target_text, enforcement,
    confidence, provider_id, model, created_at, updated_at
)
SELECT
    id, entry_id, target_language, project_id, target_text, enforcement,
    confidence, provider_id, model, created_at, updated_at
FROM terminology_translations
WHERE review_status IN ('approved', 'locked');

CREATE TABLE terminology_target_variants_v9 (
    id               TEXT PRIMARY KEY NOT NULL,
    translation_id   TEXT NOT NULL REFERENCES terminology_translations_v9(id) ON DELETE CASCADE,
    text             TEXT NOT NULL,
    normalized_text  TEXT NOT NULL,
    UNIQUE(translation_id, normalized_text)
);

INSERT INTO terminology_target_variants_v9 (id, translation_id, text, normalized_text)
SELECT variant.id, variant.translation_id, variant.text, variant.normalized_text
FROM terminology_target_variants variant
JOIN terminology_translations_v9 translation ON translation.id = variant.translation_id;

DROP TABLE terminology_target_variants;
DROP TABLE terminology_translations;
ALTER TABLE terminology_translations_v9 RENAME TO terminology_translations;
ALTER TABLE terminology_target_variants_v9 RENAME TO terminology_target_variants;

CREATE UNIQUE INDEX idx_terminology_translation_global
    ON terminology_translations(entry_id, target_language)
    WHERE project_id IS NULL;
CREATE UNIQUE INDEX idx_terminology_translation_project
    ON terminology_translations(entry_id, target_language, project_id)
    WHERE project_id IS NOT NULL;
CREATE INDEX idx_terminology_translation_scope
    ON terminology_translations(target_language, project_id, enforcement);
