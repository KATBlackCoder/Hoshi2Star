//! Text fitting for Wolf RPG messages (plan WR): wraps translated messages
//! to the game's message window at export, measured in em on the game's own
//! original messages with its own font.
//!
//! Applied at export only: translations in the database are never rewritten.

pub mod fit;
pub mod text;

use std::path::{Path, PathBuf};

pub use fit::{fit_message, measure_budget, WolfBudget, WolfFitSettings};

/// A message segment (map or common event, not a choice): what fitting lays out.
pub fn is_fitted_key(key: &str) -> bool {
    (key.starts_with("MapData/") || key.starts_with("CommonEvents/")) && !key.contains("/choices/")
}

/// A map message: the originals the width budget is measured on (common
/// events hold the base system's own messages, which overflow on their own,
/// plan WR, K10).
pub fn is_budget_key(key: &str) -> bool {
    key.starts_with("MapData/") && !key.contains("/choices/")
}

/// The font file shipped next to `Game.exe` (Wolf loads `.ttf`/`.otf` files
/// from the game folder). `Err` when there is none, or several (which one
/// the game uses is not known): widths are then estimated.
pub fn find_game_font(game_dir: &Path) -> Result<PathBuf, String> {
    let entries = std::fs::read_dir(game_dir).map_err(|e| e.to_string())?;
    let mut fonts: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| {
                        ext.eq_ignore_ascii_case("ttf") || ext.eq_ignore_ascii_case("otf")
                    })
        })
        .collect();
    match fonts.len() {
        0 => Err("no font file in the game folder".into()),
        1 => Ok(fonts.remove(0)),
        n => Err(format!(
            "{n} font files in the game folder; the one the game uses is unknown"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fitted_and_budget_keys() {
        assert!(is_fitted_key("MapData/Map001/events/0/pages/0/12"));
        assert!(is_fitted_key("CommonEvents/Intro/3/7"));
        assert!(!is_fitted_key(
            "MapData/Map001/events/0/pages/0/12/choices/1"
        ));
        assert!(!is_fitted_key("Database/DataBase/0/1/name"));
        assert!(is_budget_key("MapData/Map001/events/0/pages/0/12"));
        assert!(!is_budget_key("CommonEvents/Intro/3/7"));
    }

    /// The measured window of a real game. With the Wolf 2.23 test game
    /// (`MapData/` extracted, `mplus-2c-medium.ttf`), it must match the
    /// in-game calibration (plan WR, WR-1): 22 em, 4 lines.
    #[test]
    #[ignore = "needs H2S_WOLF_FIT_MAPS (a MapData/ folder) and H2S_WOLF_FIT_FONT (its font)"]
    fn wolf_real_budget() {
        use std::sync::Arc;

        use crate::engines::detector::WolfVersion;
        use crate::engines::mv_mz::layout::font::{GameFont, Metrics};
        use crate::engines::wolf::extractor::extract_map_segments;

        let maps = std::env::var_os("H2S_WOLF_FIT_MAPS").expect("set H2S_WOLF_FIT_MAPS");
        let font = std::env::var_os("H2S_WOLF_FIT_FONT").expect("set H2S_WOLF_FIT_FONT");
        let font = GameFont::from_bytes(&std::fs::read(font).expect("read the font"))
            .expect("parse the font");
        let metrics = Metrics::Font(Arc::new(font));
        let version = WolfVersion { major: 2, minor: 0 };
        let mut sources = Vec::new();
        for entry in std::fs::read_dir(maps)
            .expect("H2S_WOLF_FIT_MAPS")
            .flatten()
        {
            let path = entry.path();
            if !path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("mps"))
            {
                continue;
            }
            let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
            let bytes = std::fs::read(&path).expect("read the map");
            for segment in extract_map_segments(&stem, &bytes, &version).expect("map") {
                if !segment.key.contains("/choices/") {
                    sources.push(segment.source_text);
                }
            }
        }
        let budget = measure_budget(sources.iter().map(String::as_str), &metrics)
            .expect("enough map messages");
        eprintln!("measured: {budget:?} on {} messages", sources.len());
        assert!((21.0..=22.5).contains(&budget.width_em), "{budget:?}");
        assert_eq!(budget.max_lines, 4);
    }

    #[test]
    fn the_single_font_of_the_game_folder_is_found() {
        let dir = tempfile::tempdir().unwrap();
        assert!(find_game_font(dir.path()).is_err());
        std::fs::write(dir.path().join("font.TTF"), b"x").unwrap();
        std::fs::write(dir.path().join("font0.ttf.wolfx"), b"x").unwrap();
        assert_eq!(
            find_game_font(dir.path()).unwrap(),
            dir.path().join("font.TTF")
        );
        std::fs::write(dir.path().join("other.otf"), b"x").unwrap();
        assert!(find_game_font(dir.path()).is_err());
    }
}
