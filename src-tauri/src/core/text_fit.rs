//! Text fitting settings and per-text overrides (RPG Maker MZ), stored per
//! project (`project_text_fit`, `text_fit_overrides`).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::engines::mv_mz::layout::fit::{FitSettings, LineBreaks, Overflow};
use crate::engines::mv_mz::layout::plan::{FitOverride, StoredOverride};

/// Smallest font size the shrink strategy may use.
pub const MIN_FONT_FLOOR: u32 = 8;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextFitSettings {
    pub enabled_dialogue: bool,
    pub enabled_descriptions: bool,
    pub line_breaks: LineBreaks,
    pub overflow: Overflow,
    pub min_font_size: u32,
    /// Dialogue font size emitted at every page start (`None` = the game's).
    pub dialogue_font_size: Option<u32>,
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
        }
    }
}

impl TextFitSettings {
    pub fn enabled(&self) -> bool {
        self.enabled_dialogue || self.enabled_descriptions
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
    let row: Option<(i64, i64, String, String, i64, Option<i64>)> = sqlx::query_as(
        "SELECT enabled_dialogue, enabled_descriptions, line_breaks, overflow, min_font_size, \
                dialogue_font_size FROM project_text_fit WHERE project_id = ?",
    )
    .bind(project_id)
    .fetch_optional(db)
    .await?;
    Ok(match row {
        Some((dialogue, descriptions, line_breaks, overflow, min_font, dialogue_font)) => {
            TextFitSettings {
                enabled_dialogue: dialogue != 0,
                enabled_descriptions: descriptions != 0,
                line_breaks: parse_line_breaks(&line_breaks),
                overflow: parse_overflow(&overflow),
                min_font_size: u32::try_from(min_font).unwrap_or(20),
                dialogue_font_size: dialogue_font.and_then(|n| u32::try_from(n).ok()),
            }
        }
        None => TextFitSettings::default(),
    })
}

pub async fn save_settings(
    db: &SqlitePool,
    project_id: &str,
    settings: &TextFitSettings,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO project_text_fit (project_id, enabled_dialogue, enabled_descriptions, \
             line_breaks, overflow, min_font_size, dialogue_font_size, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now')) \
         ON CONFLICT(project_id) DO UPDATE SET \
             enabled_dialogue = excluded.enabled_dialogue, \
             enabled_descriptions = excluded.enabled_descriptions, \
             line_breaks = excluded.line_breaks, overflow = excluded.overflow, \
             min_font_size = excluded.min_font_size, \
             dialogue_font_size = excluded.dialogue_font_size, updated_at = datetime('now')",
    )
    .bind(project_id)
    .bind(i64::from(settings.enabled_dialogue))
    .bind(i64::from(settings.enabled_descriptions))
    .bind(line_breaks_text(settings.line_breaks))
    .bind(overflow_text(settings.overflow))
    .bind(i64::from(settings.min_font_size))
    .bind(settings.dialogue_font_size.map(i64::from))
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

/// Whether the project fits dialogue / descriptions (MV/MZ only; `false`
/// on any error so QA never fails because of this lookup). `set_text_fit`
/// only switches fitting on where export can fit.
pub async fn enabled_kinds(db: &SqlitePool, project_id: &str) -> (bool, bool) {
    let engine: Option<String> = sqlx::query_scalar("SELECT engine FROM projects WHERE id = ?")
        .bind(project_id)
        .fetch_optional(db)
        .await
        .ok()
        .flatten();
    if engine.as_deref() != Some("mv_mz") {
        return (false, false);
    }
    match load_settings(db, project_id).await {
        Ok(settings) => (settings.enabled_dialogue, settings.enabled_descriptions),
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

    #[test]
    fn validates_sizes() {
        let mut settings = TextFitSettings::default();
        assert!(settings.validate().is_ok());
        settings.min_font_size = 4;
        assert!(settings.validate().is_err());
        settings.min_font_size = 20;
        settings.dialogue_font_size = Some(200);
        assert!(settings.validate().is_err());
    }
}
