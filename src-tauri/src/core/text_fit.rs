//! Text fitting settings and per-text overrides (RPG Maker MZ; Wolf RPG
//! messages, plan WR), stored per project (`project_text_fit`,
//! `text_fit_overrides`).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::engines::mv_mz::layout::fit::{FitSettings, LineBreaks, Overflow};
use crate::engines::mv_mz::layout::plan::{FitOverride, StoredOverride};

/// Smallest font size the shrink strategy may use.
pub const MIN_FONT_FLOOR: u32 = 8;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextFitSettings {
    pub enabled_dialogue: bool,
    pub enabled_descriptions: bool,
    pub line_breaks: LineBreaks,
    pub overflow: Overflow,
    pub min_font_size: u32,
    /// Dialogue font size emitted at every page start (`None` = the game's).
    pub dialogue_font_size: Option<u32>,
    /// Wolf: message line width in em (`None` = measured on the game).
    #[serde(default)]
    pub wolf_width_em: Option<f32>,
    /// Wolf: lines per message window (`None` = measured on the game).
    #[serde(default)]
    pub wolf_max_lines: Option<u32>,
    /// Wolf: the game's default font size in `\f[n]` units (`None` = unknown).
    #[serde(default)]
    pub wolf_base_font_size: Option<u32>,
}

impl Default for TextFitSettings {
    fn default() -> Self {
        Self {
            enabled_dialogue: false,
            enabled_descriptions: false,
            line_breaks: LineBreaks::Reflow,
            overflow: Overflow::NewPage,
            min_font_size: 20,
            dialogue_font_size: None,
            wolf_width_em: None,
            wolf_max_lines: None,
            wolf_base_font_size: None,
        }
    }
}

impl TextFitSettings {
    pub fn enabled(&self) -> bool {
        self.enabled_dialogue || self.enabled_descriptions
    }

    /// Defaults for an engine: Wolf has no new page (plan WR, E3) and draws
    /// at about 12 by default, so it shrinks down to 8.
    pub fn default_for(engine: &str) -> Self {
        if engine == "wolf" {
            Self {
                overflow: Overflow::Shrink,
                min_font_size: MIN_FONT_FLOOR,
                ..Self::default()
            }
        } else {
            Self::default()
        }
    }

    pub fn fit_settings(&self) -> FitSettings {
        FitSettings {
            line_breaks: self.line_breaks,
            overflow: self.overflow,
            min_font_size: self.min_font_size,
            dialogue_font_size: self.dialogue_font_size,
            strip_stored_font_prefix: false,
        }
    }

    /// Reject sizes the engine could not draw sensibly.
    pub fn validate(&self) -> Result<(), String> {
        if !(MIN_FONT_FLOOR..=96).contains(&self.min_font_size) {
            return Err(format!(
                "The minimum font size must be between {MIN_FONT_FLOOR} and 96 px."
            ));
        }
        if let Some(size) = self.dialogue_font_size {
            if !(MIN_FONT_FLOOR..=96).contains(&size) {
                return Err(format!(
                    "The dialogue font size must be between {MIN_FONT_FLOOR} and 96 px."
                ));
            }
        }
        if let Some(width) = self.wolf_width_em {
            if !(4.0..=200.0).contains(&width) {
                return Err("The message width must be between 4 and 200 em.".into());
            }
        }
        if let Some(lines) = self.wolf_max_lines {
            if !(1..=20).contains(&lines) {
                return Err("The lines per window must be between 1 and 20.".into());
            }
        }
        if let Some(size) = self.wolf_base_font_size {
            if !(4..=96).contains(&size) {
                return Err("The game's font size must be between 4 and 96.".into());
            }
        }
        Ok(())
    }

    /// Settings Wolf cannot apply: no new page, no descriptions.
    pub fn validate_wolf(&self) -> Result<(), String> {
        if self.enabled_descriptions {
            return Err("Wolf RPG text fitting lays out messages only.".into());
        }
        if self.overflow == Overflow::NewPage {
            return Err(
                "Wolf RPG messages cannot continue on a new page: shrink or leave them.".into(),
            );
        }
        Ok(())
    }
}

fn line_breaks_text(value: LineBreaks) -> &'static str {
    match value {
        LineBreaks::Reflow => "reflow",
        LineBreaks::Keep => "keep",
    }
}

fn overflow_text(value: Overflow) -> &'static str {
    match value {
        Overflow::NewPage => "new_page",
        Overflow::Shrink => "shrink",
        Overflow::Leave => "leave",
    }
}

fn parse_line_breaks(text: &str) -> LineBreaks {
    if text == "keep" {
        LineBreaks::Keep
    } else {
        LineBreaks::Reflow
    }
}

fn parse_overflow(text: &str) -> Overflow {
    match text {
        "shrink" => Overflow::Shrink,
        "leave" => Overflow::Leave,
        _ => Overflow::NewPage,
    }
}

/// The project's settings (defaults: off).
pub async fn load_settings(
    db: &SqlitePool,
    project_id: &str,
) -> Result<TextFitSettings, sqlx::Error> {
    load_settings_or(db, project_id, TextFitSettings::default()).await
}

/// The project's settings, with the defaults of `engine` when none are saved.
pub async fn load_settings_for_engine(
    db: &SqlitePool,
    project_id: &str,
    engine: &str,
) -> Result<TextFitSettings, sqlx::Error> {
    load_settings_or(db, project_id, TextFitSettings::default_for(engine)).await
}

/// `enabled_dialogue, enabled_descriptions, line_breaks, overflow,
/// min_font_size, dialogue_font_size, wolf_width_em, wolf_max_lines,
/// wolf_base_font_size`.
type SettingsRow = (
    i64,
    i64,
    String,
    String,
    i64,
    Option<i64>,
    Option<f64>,
    Option<i64>,
    Option<i64>,
);

async fn load_settings_or(
    db: &SqlitePool,
    project_id: &str,
    default: TextFitSettings,
) -> Result<TextFitSettings, sqlx::Error> {
    let row: Option<SettingsRow> = sqlx::query_as(
        "SELECT enabled_dialogue, enabled_descriptions, line_breaks, overflow, min_font_size, \
                dialogue_font_size, wolf_width_em, wolf_max_lines, wolf_base_font_size \
         FROM project_text_fit WHERE project_id = ?",
    )
    .bind(project_id)
    .fetch_optional(db)
    .await?;
    let size = |n: Option<i64>| n.and_then(|n| u32::try_from(n).ok());
    Ok(match row {
        Some((
            dialogue,
            descriptions,
            line_breaks,
            overflow,
            min_font,
            dialogue_font,
            wolf_width,
            wolf_lines,
            wolf_base,
        )) => TextFitSettings {
            enabled_dialogue: dialogue != 0,
            enabled_descriptions: descriptions != 0,
            line_breaks: parse_line_breaks(&line_breaks),
            overflow: parse_overflow(&overflow),
            min_font_size: u32::try_from(min_font).unwrap_or(20),
            dialogue_font_size: size(dialogue_font),
            wolf_width_em: wolf_width.map(|w| w as f32),
            wolf_max_lines: size(wolf_lines),
            wolf_base_font_size: size(wolf_base),
        },
        None => default,
    })
}

pub async fn save_settings(
    db: &SqlitePool,
    project_id: &str,
    settings: &TextFitSettings,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO project_text_fit (project_id, enabled_dialogue, enabled_descriptions, \
             line_breaks, overflow, min_font_size, dialogue_font_size, wolf_width_em, \
             wolf_max_lines, wolf_base_font_size, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now')) \
         ON CONFLICT(project_id) DO UPDATE SET \
             enabled_dialogue = excluded.enabled_dialogue, \
             enabled_descriptions = excluded.enabled_descriptions, \
             line_breaks = excluded.line_breaks, overflow = excluded.overflow, \
             min_font_size = excluded.min_font_size, \
             dialogue_font_size = excluded.dialogue_font_size, \
             wolf_width_em = excluded.wolf_width_em, \
             wolf_max_lines = excluded.wolf_max_lines, \
             wolf_base_font_size = excluded.wolf_base_font_size, updated_at = datetime('now')",
    )
    .bind(project_id)
    .bind(i64::from(settings.enabled_dialogue))
    .bind(i64::from(settings.enabled_descriptions))
    .bind(line_breaks_text(settings.line_breaks))
    .bind(overflow_text(settings.overflow))
    .bind(i64::from(settings.min_font_size))
    .bind(settings.dialogue_font_size.map(i64::from))
    .bind(settings.wolf_width_em.map(f64::from))
    .bind(settings.wolf_max_lines.map(i64::from))
    .bind(settings.wolf_base_font_size.map(i64::from))
    .execute(db)
    .await?;
    Ok(())
}

/// `(file_name, pointer, source_hash, line_breaks, overflow, disabled)`.
type OverrideRow = (String, String, String, Option<String>, Option<String>, i64);

/// Overrides of a project, by `file_name` then pointer.
pub async fn load_overrides(
    db: &SqlitePool,
    project_id: &str,
) -> Result<HashMap<String, HashMap<String, StoredOverride>>, sqlx::Error> {
    let rows: Vec<OverrideRow> = sqlx::query_as(
        "SELECT file_name, pointer, source_hash, line_breaks, overflow, disabled \
         FROM text_fit_overrides WHERE project_id = ?",
    )
    .bind(project_id)
    .fetch_all(db)
    .await?;
    let mut out: HashMap<String, HashMap<String, StoredOverride>> = HashMap::new();
    for (file_name, pointer, source_hash, line_breaks, overflow, disabled) in rows {
        out.entry(file_name).or_default().insert(
            pointer,
            StoredOverride {
                choice: FitOverride {
                    line_breaks: line_breaks.as_deref().map(parse_line_breaks),
                    overflow: overflow.as_deref().map(parse_overflow),
                    disabled: disabled != 0,
                },
                source_hash,
            },
        );
    }
    Ok(out)
}

/// Store (`Some`) or remove (`None`) the choice for one text.
pub async fn save_override(
    db: &SqlitePool,
    project_id: &str,
    file_name: &str,
    pointer: &str,
    source_hash: &str,
    choice: Option<&FitOverride>,
) -> Result<(), sqlx::Error> {
    match choice {
        Some(choice) => {
            sqlx::query(
                "INSERT INTO text_fit_overrides (project_id, file_name, pointer, source_hash, \
                     line_breaks, overflow, disabled, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now')) \
                 ON CONFLICT(project_id, file_name, pointer) DO UPDATE SET \
                     source_hash = excluded.source_hash, line_breaks = excluded.line_breaks, \
                     overflow = excluded.overflow, disabled = excluded.disabled, \
                     updated_at = datetime('now')",
            )
            .bind(project_id)
            .bind(file_name)
            .bind(pointer)
            .bind(source_hash)
            .bind(choice.line_breaks.map(line_breaks_text))
            .bind(choice.overflow.map(overflow_text))
            .bind(i64::from(choice.disabled))
            .execute(db)
            .await?;
        }
        None => {
            sqlx::query(
                "DELETE FROM text_fit_overrides \
                 WHERE project_id = ? AND file_name = ? AND pointer = ?",
            )
            .bind(project_id)
            .bind(file_name)
            .bind(pointer)
            .execute(db)
            .await?;
        }
    }
    Ok(())
}

/// Whether the project fits dialogue / descriptions (MV/MZ; Wolf RPG
/// messages, never descriptions; `false` on any error so QA never fails
/// because of this lookup). `set_text_fit` only switches fitting on where
/// export can fit.
pub async fn enabled_kinds(db: &SqlitePool, project_id: &str) -> (bool, bool) {
    let engine: Option<String> = sqlx::query_scalar("SELECT engine FROM projects WHERE id = ?")
        .bind(project_id)
        .fetch_optional(db)
        .await
        .ok()
        .flatten();
    let wolf = match engine.as_deref() {
        Some("mv_mz") => false,
        Some("wolf") => true,
        _ => return (false, false),
    };
    match load_settings(db, project_id).await {
        Ok(settings) => (
            settings.enabled_dialogue,
            settings.enabled_descriptions && !wolf,
        ),
        Err(_) => (false, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn pool() -> (tempfile::NamedTempFile, SqlitePool) {
        let file = tempfile::NamedTempFile::new().unwrap();
        let pool = crate::db::pool::init(file.path().to_str().unwrap())
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO projects (id, name, engine, game_path) VALUES ('p1', 'P', 'mv_mz', '/g')",
        )
        .execute(&pool)
        .await
        .unwrap();
        (file, pool)
    }

    #[tokio::test]
    async fn settings_default_to_off_and_round_trip() {
        let (_file, pool) = pool().await;
        assert_eq!(
            load_settings(&pool, "p1").await.unwrap(),
            TextFitSettings::default()
        );
        let settings = TextFitSettings {
            enabled_dialogue: true,
            enabled_descriptions: true,
            line_breaks: LineBreaks::Keep,
            overflow: Overflow::Shrink,
            min_font_size: 18,
            dialogue_font_size: Some(22),
            wolf_width_em: Some(21.5),
            wolf_max_lines: Some(4),
            wolf_base_font_size: Some(12),
        };
        save_settings(&pool, "p1", &settings).await.unwrap();
        assert_eq!(load_settings(&pool, "p1").await.unwrap(), settings);
        assert_eq!(enabled_kinds(&pool, "p1").await, (true, true));
    }

    #[tokio::test]
    async fn overrides_are_stored_per_text_and_removable() {
        let (_file, pool) = pool().await;
        let choice = FitOverride {
            line_breaks: None,
            overflow: Some(Overflow::Shrink),
            disabled: false,
        };
        save_override(
            &pool,
            "p1",
            "Map001.json",
            "/events/1/pages/0/list/0",
            "h",
            Some(&choice),
        )
        .await
        .unwrap();
        let loaded = load_overrides(&pool, "p1").await.unwrap();
        let stored = &loaded["Map001.json"]["/events/1/pages/0/list/0"];
        assert_eq!(stored.choice, choice);
        assert_eq!(stored.source_hash, "h");
        save_override(
            &pool,
            "p1",
            "Map001.json",
            "/events/1/pages/0/list/0",
            "h",
            None,
        )
        .await
        .unwrap();
        assert!(load_overrides(&pool, "p1").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn wolf_defaults_shrink_and_fit_messages_only() {
        let (_file, pool) = pool().await;
        sqlx::query(
            "INSERT INTO projects (id, name, engine, game_path) VALUES ('w1', 'W', 'wolf', '/w')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let defaults = load_settings_for_engine(&pool, "w1", "wolf").await.unwrap();
        assert_eq!(defaults.overflow, Overflow::Shrink);
        assert_eq!(defaults.min_font_size, MIN_FONT_FLOOR);
        assert!(defaults.validate_wolf().is_ok());
        let settings = TextFitSettings {
            enabled_dialogue: true,
            enabled_descriptions: true,
            ..defaults
        };
        save_settings(&pool, "w1", &settings).await.unwrap();
        assert_eq!(enabled_kinds(&pool, "w1").await, (true, false));
        assert!(settings.validate_wolf().is_err());
        let new_page = TextFitSettings {
            enabled_descriptions: false,
            overflow: Overflow::NewPage,
            ..settings
        };
        assert!(new_page.validate_wolf().is_err());
    }

    #[test]
    fn validates_sizes() {
        let mut settings = TextFitSettings::default();
        assert!(settings.validate().is_ok());
        settings.min_font_size = 4;
        assert!(settings.validate().is_err());
        settings.min_font_size = 20;
        settings.dialogue_font_size = Some(200);
        assert!(settings.validate().is_err());
        settings.dialogue_font_size = None;
        settings.wolf_width_em = Some(2.0);
        assert!(settings.validate().is_err());
        settings.wolf_width_em = Some(22.0);
        settings.wolf_max_lines = Some(0);
        assert!(settings.validate().is_err());
    }
}
