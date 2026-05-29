-- F3: persist per-file translation duration so it survives app restarts.
-- SQLite ALTER TABLE only supports ADD COLUMN (no NOT NULL without a default).
ALTER TABLE source_files ADD COLUMN translation_secs INTEGER;
