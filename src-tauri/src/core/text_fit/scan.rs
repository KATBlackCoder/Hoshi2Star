//! The scan behind the text fitting dialog and the export summary, shared
//! by every engine: outcomes counted by kind, the texts worth reviewing, and
//! the characters the game font lacks.

use std::collections::BTreeMap;

use serde::Serialize;

use super::font::Metrics;
use super::{FitCounts, FitOutcome};

/// Rows listed for review in the dialog.
pub const MAX_ROWS: usize = 500;

/// A text worth reviewing (new page, shrunk, too long, skipped).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FitRow {
    pub file_id: String,
    pub file_name: String,
    pub pointer: String,
    pub description: bool,
    /// First translated line (to open it in the editor).
    pub segment_id: Option<String>,
    pub outcome: FitOutcome,
    pub sample: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingChar {
    pub character: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FitScan {
    pub dialogue: FitCounts,
    pub descriptions: FitCounts,
    pub rows: Vec<FitRow>,
    /// Characters the game font lacks (drawn with a fallback font).
    pub missing_chars: Vec<MissingChar>,
}

/// A scan being summed up, one fitted text at a time.
#[derive(Debug, Default)]
pub struct ScanSummary {
    dialogue: FitCounts,
    descriptions: FitCounts,
    rows: Vec<FitRow>,
    missing: BTreeMap<char, usize>,
}

impl ScanSummary {
    /// Count one fitted text. A text worth reviewing (anything but `Fits`
    /// and `Rewrapped`) is listed, up to [`MAX_ROWS`]: `row` builds it.
    pub fn add(&mut self, description: bool, outcome: &FitOutcome, row: impl FnOnce() -> FitRow) {
        let counts = if description {
            &mut self.descriptions
        } else {
            &mut self.dialogue
        };
        counts.add(outcome);
        let notable = !matches!(outcome, FitOutcome::Fits | FitOutcome::Rewrapped);
        if notable && self.rows.len() < MAX_ROWS {
            self.rows.push(row());
        }
    }

    /// Count the characters of `text` the game font lacks.
    pub fn add_missing_chars(&mut self, metrics: &Metrics, text: &str) {
        for c in metrics.missing_chars(text) {
            *self.missing.entry(c).or_default() += 1;
        }
    }

    /// The scan, missing characters most frequent first.
    pub fn finish(self) -> FitScan {
        let mut missing_chars: Vec<MissingChar> = self
            .missing
            .into_iter()
            .map(|(c, count)| MissingChar {
                character: c.to_string(),
                count,
            })
            .collect();
        missing_chars.sort_by_key(|m| std::cmp::Reverse(m.count));
        FitScan {
            dialogue: self.dialogue,
            descriptions: self.descriptions,
            rows: self.rows,
            missing_chars,
        }
    }
}
