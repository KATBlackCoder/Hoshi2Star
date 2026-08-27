use sqlx::SqlitePool;
use std::sync::Arc;

use crate::core::terminology::service::TerminologyService;
use crate::core::terminology::Result;

/// Global application state managed by Tauri.
///
/// Available in all `#[tauri::command]` functions via `tauri::State<'_, AppState>`.
/// Created once in `lib.rs::run()` inside `.setup()`.
pub struct AppState {
    pub db: SqlitePool,
    pub terminology: Arc<TerminologyService>,
}

impl AppState {
    pub fn new(db: SqlitePool) -> Result<Self> {
        Ok(Self {
            db,
            terminology: TerminologyService::embedded_japanese()?,
        })
    }

    pub fn with_terminology(db: SqlitePool, terminology: Arc<TerminologyService>) -> Self {
        Self { db, terminology }
    }
}
