-- Durable translation context contract.
--
-- Only the RPG Maker MV/MZ extractor populates these fields for now. Other
-- engines intentionally retain the defaults until they gain their own
-- engine-specific context implementation.
ALTER TABLE segments ADD COLUMN segment_kind TEXT NOT NULL DEFAULT 'unknown';
ALTER TABLE segments ADD COLUMN scene_id TEXT;
ALTER TABLE segments ADD COLUMN sequence_index INTEGER;
ALTER TABLE segments ADD COLUMN speaker TEXT;
ALTER TABLE segments ADD COLUMN branch_path TEXT;
ALTER TABLE segments ADD COLUMN context_json TEXT;

CREATE INDEX IF NOT EXISTS idx_segments_scene_sequence
    ON segments(source_file_id, scene_id, sequence_index);
