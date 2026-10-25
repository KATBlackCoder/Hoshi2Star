use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::core::terminology::service::TerminologyService;
use crate::core::terminology::Result;
use crate::engines::mv_mz::layout::font::FontCache;
use crate::engines::mv_mz::profiles::ProfileSet;

/// Global application state managed by Tauri.
///
/// Available in all `#[tauri::command]` functions via `tauri::State<'_, AppState>`.
/// Created once in `lib.rs::run()` inside `.setup()`.
pub struct AppState {
    pub db: SqlitePool,
    /// Set by `cancel_translation`; the running translation stops before its
    /// next batch. Reset when a new translation starts.
    pub translation_cancel: Arc<AtomicBool>,
    pub terminology: Arc<TerminologyService>,
    /// Per-installation data directory (holds the DB and pristine game-file
    /// snapshots). Injected explicitly so tests never touch the real app dir.
    pub data_dir: PathBuf,
    /// Verified MV/MZ plugin profiles (the embedded set; replaceable in tests).
    pub plugin_profiles: Arc<ProfileSet>,
    /// The last game font decoded for text fitting.
    pub game_fonts: FontCache,
}

impl AppState {
    pub fn new(db: SqlitePool, data_dir: PathBuf) -> Result<Self> {
        Ok(Self {
            db,
            translation_cancel: Arc::default(),
            terminology: TerminologyService::embedded_japanese()?,
            data_dir,
            plugin_profiles: Arc::new(ProfileSet::embedded().clone()),
            game_fonts: FontCache::default(),
        })
    }

    pub fn with_terminology(
        db: SqlitePool,
        terminology: Arc<TerminologyService>,
        data_dir: PathBuf,
    ) -> Self {
        Self {
            db,
            translation_cancel: Arc::default(),
            terminology,
            data_dir,
            plugin_profiles: Arc::new(ProfileSet::embedded().clone()),
            game_fonts: FontCache::default(),
        }
    }
}
