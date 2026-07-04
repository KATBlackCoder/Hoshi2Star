-- Hoshi2Star — enforce TM dedup at the schema level
--
-- idx_tm_hash_lang was non-unique and tm::insert() generates a fresh UUID per
-- call, so its INSERT OR REPLACE never conflicted — every save created a new
-- row for the same (source_hash, lang_pair).
--
-- Dedup first: CREATE UNIQUE INDEX fails if duplicates exist. Keep the newest
-- row per group via MAX(rowid) — created_at has 1-second granularity, so rapid
-- re-saves tie; rowid is insertion-ordered.
DELETE FROM tm_entries
WHERE rowid NOT IN (
    SELECT MAX(rowid) FROM tm_entries
    GROUP BY source_hash, lang_pair
);

DROP INDEX IF EXISTS idx_tm_hash_lang;
CREATE UNIQUE INDEX idx_tm_hash_lang ON tm_entries(source_hash, lang_pair);
