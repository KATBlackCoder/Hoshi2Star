//! Decrypt a packed Wolf RPG game from the app (plan UWF, UWF-4).
//!
//! The work itself is in [`crate::engines::wolf::unpack`]; this layer allows
//! one run at a time, wires the cancel flag and throttles the progress events.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Runtime, State};

use crate::engines::wolf::unpack::{unpack_game, UnpackProgress, UnpackReport};
use crate::state::AppState;

/// Progress of the running decryption: one [`UnpackProgress`] per event.
pub const WOLF_UNPACK_PROGRESS_EVENT: &str = "h2s://wolf/unpack-progress";

/// At most one progress event per interval: an archive holds thousands of
/// small files.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

/// Stable marker of a second decryption started while one runs.
pub const WOLF_UNPACK_BUSY: &str = "wolf_unpack_busy";

/// Where `unwolf` keeps the archive keys found for Wolf RPG Pro games
/// (decision UWF-D8): an absolute path in the app's data folder.
pub fn unwolf_config_path(data_dir: &Path) -> PathBuf {
    data_dir.join("unwolf").join("UberWolfConfig.json")
}

/// Clears the running flag when the run ends, whatever its outcome.
struct RunningGuard(Arc<AtomicBool>);

impl Drop for RunningGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// Decrypt the Wolf RPG game at `game_path` (text archives extracted, Pro v3.5
/// protection removed, archives moved to `Hoshi2Star-archives/`). The report
/// is the return value; progress goes out as [`WOLF_UNPACK_PROGRESS_EVENT`].
#[tauri::command]
pub async fn unpack_wolf_game(
    game_path: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<UnpackReport, String> {
    unpack_wolf_game_with_app(game_path, app, &state).await
}

/// [`unpack_wolf_game`] for any runtime (tests use the mock runtime).
pub async fn unpack_wolf_game_with_app<R: Runtime>(
    game_path: String,
    app: AppHandle<R>,
    state: &AppState,
) -> Result<UnpackReport, String> {
    if state
        .wolf_unpack_running
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err(format!(
            "{WOLF_UNPACK_BUSY}: a Wolf RPG game is already being decrypted"
        ));
    }
    let running = RunningGuard(Arc::clone(&state.wolf_unpack_running));
    state.wolf_unpack_cancel.store(false, Ordering::SeqCst);
    let cancel = Arc::clone(&state.wolf_unpack_cancel);
    let config_path = unwolf_config_path(&state.data_dir);
    let game_dir = PathBuf::from(game_path);

    tokio::task::spawn_blocking(move || {
        let _running = running;
        let mut last_event: Option<Instant> = None;
        let mut on_progress = |progress: &UnpackProgress| {
            let now = Instant::now();
            if last_event.is_some_and(|last| now.duration_since(last) < PROGRESS_INTERVAL) {
                return;
            }
            last_event = Some(now);
            if let Err(error) = app.emit(WOLF_UNPACK_PROGRESS_EVENT, progress) {
                log::warn!("failed to emit the Wolf unpack progress: {error}");
            }
        };
        unpack_game(&game_dir, &config_path, &mut on_progress, &cancel)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("Wolf unpack worker failed: {error}"))?
}

/// Stop the running decryption at its next file; nothing is moved then.
#[tauri::command]
pub async fn cancel_wolf_unpack(state: State<'_, AppState>) -> Result<(), String> {
    state.wolf_unpack_cancel.store(true, Ordering::SeqCst);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::wolf::unpack::tests::split_game;
    use crate::engines::wolf::unpack::ARCHIVES_FOLDER;
    use std::sync::Mutex;
    use tauri::Listener;

    async fn mock_app(dir: &Path) -> tauri::App<tauri::test::MockRuntime> {
        let db = dir.join("h2s.db");
        let pool = crate::db::pool::init(db.to_str().unwrap()).await.unwrap();
        let state = AppState::new(pool, dir.join("data")).unwrap();
        tauri::test::mock_builder()
            .manage(state)
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap()
    }

    #[tokio::test]
    async fn unpacks_a_game_and_reports_progress() {
        use tauri::Manager;

        let work = tempfile::tempdir().unwrap();
        let app = mock_app(work.path()).await;
        let events = Arc::new(Mutex::new(Vec::<String>::new()));
        let sink = Arc::clone(&events);
        app.listen(WOLF_UNPACK_PROGRESS_EVENT, move |event| {
            sink.lock().unwrap().push(event.payload().to_string());
        });
        let game = split_game();

        let state = app.state::<AppState>();
        let report = unpack_wolf_game_with_app(
            game.path().to_string_lossy().into_owned(),
            app.handle().clone(),
            &state,
        )
        .await
        .unwrap();

        assert_eq!(report.moved, ["Data/MapData.wolf", "Data/BasicData.wolf"]);
        assert!(game.path().join(ARCHIVES_FOLDER).is_dir());
        assert!(!state.wolf_unpack_running.load(Ordering::SeqCst));
        let events = events.lock().unwrap();
        assert!(!events.is_empty(), "the first file is always reported");
        assert!(
            events[0].contains("\"archive\":\"Data/BasicData.wolf\""),
            "{}",
            events[0]
        );
    }

    #[tokio::test]
    async fn a_second_run_is_refused_while_one_runs() {
        use tauri::Manager;

        let work = tempfile::tempdir().unwrap();
        let app = mock_app(work.path()).await;
        let state = app.state::<AppState>();
        state.wolf_unpack_running.store(true, Ordering::SeqCst);

        let error = unpack_wolf_game_with_app(
            work.path().to_string_lossy().into_owned(),
            app.handle().clone(),
            &state,
        )
        .await
        .unwrap_err();
        assert!(error.starts_with(WOLF_UNPACK_BUSY), "{error}");
        assert!(
            state.wolf_unpack_running.load(Ordering::SeqCst),
            "the running flag of the other run is kept"
        );
    }

    #[tokio::test]
    async fn a_new_run_clears_an_old_cancel_request() {
        use tauri::Manager;

        let work = tempfile::tempdir().unwrap();
        let app = mock_app(work.path()).await;
        let state = app.state::<AppState>();
        state.wolf_unpack_cancel.store(true, Ordering::SeqCst);
        let game = split_game();

        let report = unpack_wolf_game_with_app(
            game.path().to_string_lossy().into_owned(),
            app.handle().clone(),
            &state,
        )
        .await
        .unwrap();
        assert_eq!(report.moved.len(), 2);
    }

    #[tokio::test]
    async fn cancel_sets_the_flag_of_the_running_decryption() {
        use tauri::Manager;

        let work = tempfile::tempdir().unwrap();
        let app = mock_app(work.path()).await;
        cancel_wolf_unpack(app.state()).await.unwrap();
        assert!(app
            .state::<AppState>()
            .wolf_unpack_cancel
            .load(Ordering::SeqCst));
    }

    #[test]
    fn config_path_is_absolute_in_the_data_folder() {
        let path = unwolf_config_path(Path::new("/data/app"));
        assert_eq!(path, Path::new("/data/app/unwolf/UberWolfConfig.json"));
    }
}
