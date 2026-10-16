use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;

use crate::core::terminology::service::TerminologyService;
use crate::core::terminology::Result;
use crate::engines::mv_mz::profiles::ProfileSet;

/// Global application state managed by Tauri.
///
/// Available in all `#[tauri::command]` functions via `tauri::State<'_, AppState>`.
/// Created once in `lib.rs::run()` inside `.setup()`.
pub struct AppState {
    pub db: SqlitePool,
    pub terminology: Arc<TerminologyService>,
    /// Per-installation data directory (holds the DB and pristine game-file
    /// snapshots). Injected explicitly so tests never touch the real app dir.
    pub data_dir: PathBuf,
    /// Verified MV/MZ plugin profiles (the embedded set; replaceable in tests).
    pub plugin_profiles: Arc<ProfileSet>,
}

impl AppState {
    pub fn new(db: SqlitePool, data_dir: PathBuf) -> Result<Self> {
        Ok(Self {
            db,
            terminology: TerminologyService::embedded_japanese()?,
            data_dir,
            plugin_profiles: Arc::new(ProfileSet::embedded().clone()),
        })
    }

    pub fn with_terminology(
        db: SqlitePool,
        terminology: Arc<TerminologyService>,
        data_dir: PathBuf,
    ) -> Self {
        Self {
            db,
            terminology,
            data_dir,
            plugin_profiles: Arc::new(ProfileSet::embedded().clone()),
        }
    }
}
