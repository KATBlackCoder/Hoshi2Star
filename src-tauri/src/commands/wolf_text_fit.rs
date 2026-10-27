//! Text fitting for Wolf RPG messages (plan WR): the measured window, the
//! scan behind the text fitting dialog, and the fitting the export applies.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;
use sqlx::SqlitePool;

use crate::commands::text_fit::{FitCounts, FitRow, FitScan, FontInfo, MissingChar};
use crate::core::text_fit::TextFitSettings;
use crate::engines::mv_mz::layout::fit::FitOutcome;
use crate::engines::mv_mz::layout::font::{FontCache, Metrics};
use crate::engines::wolf::layout::{
    find_game_font, fit_message, is_budget_key, is_fitted_key, measure_budget, WolfBudget,
    WolfFitSettings,
};

/// Rows listed for review in the dialog.
const MAX_ROWS: usize = 500;

/// The Wolf message window as fitting sees it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WolfFitInfo {
    /// Measured on the game's original map messages (`None`: too few).
    pub measured: Option<WolfBudget>,
    /// In use: the project's values where set, the measured ones otherwise
    /// (`None`: nothing to fit with).
    pub budget: Option<WolfBudget>,
    /// The game's default font size (`\f[n]` units), as set by the user.
    pub base_font_size: Option<u32>,
}

/// Everything needed to fit a Wolf project's messages.
pub struct WolfFitEnvironment {
    pub metrics: Metrics,
    pub font: FontInfo,
    pub info: WolfFitInfo,
    no_space_join: bool,
    space_before_marks: bool,
}

impl WolfFitEnvironment {
    /// What [`fit_message`] needs, or `None` without a budget.
    pub fn fit_settings(
        &self,
        settings: &TextFitSettings,
    ) -> Option<(WolfBudget, WolfFitSettings)> {
        let budget = self.info.budget?;
        Some((
            budget,
            WolfFitSettings {
                line_breaks: settings.line_breaks,
                overflow: settings.overflow,
                min_font_size: settings.min_font_size,
                base_font_size: settings.wolf_base_font_size,
                no_space_join: self.no_space_join,
                space_before_marks: self.space_before_marks,
            },
        ))
    }

    /// Fit one message segment (`key`); other segments are returned as is.
    pub fn fit(
        &self,
        fit: &(WolfBudget, WolfFitSettings),
        key: &str,
        target: &str,
        source: &str,
    ) -> Option<(String, FitOutcome)> {
        is_fitted_key(key).then(|| fit_message(target, source, &fit.0, &self.metrics, &fit.1))
    }
}

fn game_font(game_dir: &Path, fonts: &FontCache) -> (Metrics, FontInfo) {
    let path = match find_game_font(game_dir) {
        Ok(path) => path,
        Err(error) => {
            return (
                Metrics::Estimated,
                FontInfo {
                    file: None,
                    loaded: false,
                    error: Some(error),
                },
            )
        }
    };
    let file = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    match fonts.load(&path) {
        Ok(font) => (
            Metrics::Font(font),
            FontInfo {
                file,
                loaded: true,
                error: None,
            },
        ),
        Err(error) => (
            Metrics::Estimated,
            FontInfo {
                file,
                loaded: false,
                error: Some(error.to_string()),
            },
        ),
    }
}

/// Load the game font and measure the window on the original map messages;
/// `settings` override the measurement.
pub async fn wolf_environment(
    db: &SqlitePool,
    fonts: &FontCache,
    project_id: &str,
    settings: &TextFitSettings,
) -> Result<WolfFitEnvironment, String> {
    let (game_path, target_language): (String, String) = sqlx::query_as(
        "SELECT game_path, COALESCE(target_language, 'en') FROM projects WHERE id = ?",
    )
    .bind(project_id)
    .fetch_one(db)
    .await
    .map_err(|e| e.to_string())?;
    let (metrics, font) = game_font(Path::new(&game_path), fonts);
    let sources: Vec<(String, String)> = sqlx::query_as(
        "SELECT s.json_key, s.source_text FROM segments s \
         JOIN source_files f ON f.id = s.source_file_id WHERE f.project_id = ?",
    )
    .bind(project_id)
    .fetch_all(db)
    .await
    .map_err(|e| e.to_string())?;
    let measured = measure_budget(
        sources
            .iter()
            .filter(|(key, _)| is_budget_key(key))
            .map(|(_, source)| source.as_str()),
        &metrics,
    );
    let width = settings
        .wolf_width_em
        .or(measured.map(|budget| budget.width_em));
    let lines = settings
        .wolf_max_lines
        .map(|lines| lines as usize)
        .or(measured.map(|budget| budget.max_lines));
    let budget = width.zip(lines).map(|(width_em, max_lines)| WolfBudget {
        width_em,
        max_lines,
    });
    Ok(WolfFitEnvironment {
        metrics,
        font,
        info: WolfFitInfo {
            measured,
            budget,
            base_font_size: settings.wolf_base_font_size,
        },
        no_space_join: ["zh", "ja", "ko"]
            .iter()
            .any(|code| target_language.starts_with(code)),
        space_before_marks: target_language.starts_with("fr"),
    })
}

/// Notes for the dialog: what the measurement could not establish.
pub fn notes(env: &WolfFitEnvironment) -> Vec<String> {
    let mut notes = Vec::new();
    if env.info.measured.is_none() {
        notes.push(
            "Too few original map messages to measure the message window: \
             set the line width and the lines per window."
                .to_string(),
        );
    }
    if env.info.base_font_size.is_none() {
        notes.push(
            "The game's font size is not set: messages are fitted at the game's own size, \
             without shrinking, and a font size chosen at export turns fitting off."
                .to_string(),
        );
    }
    notes.push(
        "Choices and database texts are not laid out: Wolf RPG never wraps them, \
         check long choices in game."
            .to_string(),
    );
    notes
}

/// `(segment id, key, source, target, file id, file name)`.
type ScanRow = (String, String, String, String, String, String);

/// Fit every translated message with `settings` and summarise (the scan
/// behind the dialog and the export summary).
pub async fn scan(
    db: &SqlitePool,
    project_id: &str,
    env: WolfFitEnvironment,
    settings: &TextFitSettings,
) -> Result<FitScan, String> {
    let Some(fit) = env.fit_settings(settings) else {
        return Ok(fit_rows(&env, None, Vec::new()));
    };
    let segments: Vec<ScanRow> = sqlx::query_as(
        "SELECT s.id, s.json_key, s.source_text, s.target_text, f.id, f.file_name \
         FROM segments s JOIN source_files f ON f.id = s.source_file_id \
         WHERE f.project_id = ? AND s.target_text != '' ORDER BY f.file_name, s.rowid",
    )
    .bind(project_id)
    .fetch_all(db)
    .await
    .map_err(|e| e.to_string())?;
    // Measuring every message is CPU work: off the async runtime, as for MZ.
    tokio::task::spawn_blocking(move || fit_rows(&env, Some(&fit), segments))
        .await
        .map_err(|e| e.to_string())
}

fn fit_rows(
    env: &WolfFitEnvironment,
    fit: Option<&(WolfBudget, WolfFitSettings)>,
    segments: Vec<ScanRow>,
) -> FitScan {
    let mut dialogue = FitCounts::default();
    let mut rows = Vec::new();
    let mut missing: BTreeMap<char, usize> = BTreeMap::new();
    if let Some(fit) = fit {
        for (id, key, source, target, file_id, file_name) in segments {
            let Some((_, outcome)) = env.fit(fit, &key, &target, &source) else {
                continue;
            };
            dialogue.add(&outcome);
            for c in env.metrics.missing_chars(&target) {
                *missing.entry(c).or_default() += 1;
            }
            let notable = !matches!(outcome, FitOutcome::Fits | FitOutcome::Rewrapped);
            if notable && rows.len() < MAX_ROWS {
                rows.push(FitRow {
                    file_id,
                    file_name,
                    pointer: key,
                    description: false,
                    segment_id: Some(id),
                    outcome,
                    sample: target.chars().take(80).collect(),
                });
            }
        }
    }
    let mut missing_chars: Vec<MissingChar> = missing
        .into_iter()
        .map(|(c, count)| MissingChar {
            character: c.to_string(),
            count,
        })
        .collect();
    missing_chars.sort_by_key(|m| std::cmp::Reverse(m.count));
    FitScan {
        dialogue,
        descriptions: FitCounts::default(),
        rows,
        missing_chars,
    }
}

/// Fitting as the export applies it: on when the project enables dialogue
/// fitting and the window is known.
pub struct WolfExportFit {
    env: WolfFitEnvironment,
    fit: (WolfBudget, WolfFitSettings),
    counts: FitCounts,
    /// Messages left too long or not fitted (logged).
    notable: Vec<String>,
}

impl WolfExportFit {
    pub async fn load(
        db: &SqlitePool,
        fonts: &FontCache,
        project_id: &str,
    ) -> Result<Option<Self>, String> {
        let settings = crate::core::text_fit::load_settings_for_engine(db, project_id, "wolf")
            .await
            .map_err(|e| e.to_string())?;
        if !settings.enabled_dialogue {
            return Ok(None);
        }
        let env = wolf_environment(db, fonts, project_id, &settings).await?;
        let Some(fit) = env.fit_settings(&settings) else {
            log::warn!(
                "[h2s] Wolf text fitting is on but the message window is unknown: \
                 messages exported as translated"
            );
            return Ok(None);
        };
        Ok(Some(Self {
            env,
            fit,
            counts: FitCounts::default(),
            notable: Vec::new(),
        }))
    }

    /// The text to export for segment `key` (`target` with the export font
    /// prefix already applied).
    pub fn apply(&mut self, key: &str, target: String, source: &str) -> String {
        match self.env.fit(&self.fit, key, &target, source) {
            Some((text, outcome)) => {
                self.counts.add(&outcome);
                if matches!(
                    outcome,
                    FitOutcome::TooLong { .. } | FitOutcome::Skipped { .. }
                ) {
                    self.notable.push(format!("{key} ({outcome:?})"));
                }
                text
            }
            None => target,
        }
    }

    pub fn log_summary(&self) {
        let c = &self.counts;
        log::info!(
            "[h2s] Wolf text fitting: {} message(s): {} fit, {} rewrapped, {} shrunk, \
             {} still too long, {} not fitted",
            c.total,
            c.fits,
            c.rewrapped,
            c.shrunk,
            c.too_long,
            c.skipped
        );
        for line in self.notable.iter().take(50) {
            log::warn!("[h2s] Wolf text fitting: {line}");
        }
    }
}
