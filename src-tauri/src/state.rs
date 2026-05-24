use sqlx::SqlitePool;

/// Global application state managed by Tauri.
///
/// Available in all `#[tauri::command]` functions via `tauri::State<'_, AppState>`.
/// Created once in `lib.rs::run()` inside `.setup()`.
pub struct AppState {
    pub db: SqlitePool,
}
