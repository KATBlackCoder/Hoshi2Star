//! Text fitting shared by every engine: settings, font metrics, layout
//! tokens, line breaking, and the outcome of fitting one text.
//!
//! Engines plug in their own rules (codes, window, verdict); this module
//! knows no engine (see `docs/adr/ADR-009.md`).

pub mod font;
pub mod settings;
pub mod token;
pub mod wrap;

use serde::{Deserialize, Serialize};

pub use settings::*;

/// How line breaks are chosen inside a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineBreaks {
    /// Join the lines of each block and rewrap them evenly.
    Reflow,
    /// Keep each translated line; split only the ones that are too wide.
    Keep,
}

/// What happens when a message needs more lines than the box holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Overflow {
    /// Continue on a new page at a sentence boundary.
    NewPage,
    /// Shrink the font (down to the minimum); a new page if still too long.
    Shrink,
    /// Leave the text as translated and report it.
    Leave,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FitOutcome {
    /// Already fits as translated.
    Fits,
    /// Line breaks changed; same number of pages.
    Rewrapped,
    /// Continued on extra pages.
    NewPage { pages: usize },
    /// Font reduced to `size` px.
    Shrunk { size: u32 },
    /// Still does not fit with these settings (left as translated).
    TooLong { reason: String },
    /// Not fitted at all (left as translated).
    Skipped { reason: String },
}

impl FitOutcome {
    pub fn changed_layout(&self) -> bool {
        matches!(
            self,
            FitOutcome::Rewrapped | FitOutcome::NewPage { .. } | FitOutcome::Shrunk { .. }
        )
    }
}

/// Outcomes counted by kind (scan and export summaries).
#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FitCounts {
    pub total: usize,
    pub fits: usize,
    pub rewrapped: usize,
    pub new_page: usize,
    pub shrunk: usize,
    pub too_long: usize,
    pub skipped: usize,
}

impl FitCounts {
    pub(crate) fn add(&mut self, outcome: &FitOutcome) {
        self.total += 1;
        match outcome {
            FitOutcome::Fits => self.fits += 1,
            FitOutcome::Rewrapped => self.rewrapped += 1,
            FitOutcome::NewPage { .. } => self.new_page += 1,
            FitOutcome::Shrunk { .. } => self.shrunk += 1,
            FitOutcome::TooLong { .. } => self.too_long += 1,
            FitOutcome::Skipped { .. } => self.skipped += 1,
        }
    }
}
