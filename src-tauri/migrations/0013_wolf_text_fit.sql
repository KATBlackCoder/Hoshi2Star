-- Text fitting for Wolf RPG messages (plan WR). The window is measured on the
-- game's original messages; these columns override the measurement.
-- NULL = the measured value (or, for the font size, unknown).

-- Line width in em of the game's default font size.
ALTER TABLE project_text_fit ADD COLUMN wolf_width_em REAL;
-- Lines per message window.
ALTER TABLE project_text_fit ADD COLUMN wolf_max_lines INTEGER;
-- The game's default font size, in `\f[n]` units: needed to fit a text that
-- holds a font size code (the export font size) and to shrink.
ALTER TABLE project_text_fit ADD COLUMN wolf_base_font_size INTEGER;
