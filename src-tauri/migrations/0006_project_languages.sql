-- Hoshi2Star — language pair belongs to the project.
--
-- Historical projects translated Japanese to English, so those values are the
-- migration defaults. New projects may override them through `open_project`.
--
-- Keep the descriptive column names used by the earlier personal-workspace
-- schema. The Rust API aliases them to `source_lang` / `target_lang`, so the
-- TypeScript contract remains `sourceLang` / `targetLang`.
ALTER TABLE projects ADD COLUMN source_language TEXT NOT NULL DEFAULT 'ja';
ALTER TABLE projects ADD COLUMN target_language TEXT NOT NULL DEFAULT 'en';
