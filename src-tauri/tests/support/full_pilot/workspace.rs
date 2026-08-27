use std::fs;
use std::path::{Path, PathBuf};

use hoshi2star_lib::commands::project::open_project;
use hoshi2star_lib::db;
use hoshi2star_lib::state::AppState;
use sha2::{Digest, Sha256};
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::Manager;

use super::PilotConfig;

pub struct PilotWorkspace {
    pub variant: String,
    pub root: PathBuf,
    pub project_id: String,
    pub app: tauri::App<tauri::test::MockRuntime>,
}

impl PilotWorkspace {
    pub fn state(&self) -> tauri::State<'_, AppState> {
        self.app.state::<AppState>()
    }

    pub fn metrics_path(&self) -> PathBuf {
        self.root.join("provider-metrics.jsonl")
    }

    pub async fn source_fingerprint(&self) -> Result<String, String> {
        let rows = sqlx::query_as::<_, (String, String, String)>(
            "SELECT file.file_name, segment.json_key, segment.source_text \
             FROM segments segment JOIN source_files file ON file.id = segment.source_file_id \
             WHERE file.project_id = ? ORDER BY file.file_name, segment.rowid",
        )
        .bind(&self.project_id)
        .fetch_all(&self.state().db)
        .await
        .map_err(|error| error.to_string())?;
        let mut digest = Sha256::new();
        for (file, key, source) in rows {
            for value in [file, key, source] {
                digest.update(value.as_bytes());
                digest.update([0x1f]);
            }
        }
        Ok(hex::encode(digest.finalize()))
    }
}

pub async fn prepare(config: &PilotConfig, variant: &str) -> Result<PilotWorkspace, String> {
    let root = config.root.join(variant);
    let game = root.join("game");
    fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    if !game.exists() {
        copy_tree(&config.source, &game)?;
    }
    let database = root.join("pilot.db");
    let database_text = database
        .to_str()
        .ok_or_else(|| "pilot database path is not UTF-8".to_string())?;
    let pool = db::pool::init(database_text)
        .await
        .map_err(|error| error.to_string())?;
    let app = mock_builder()
        .manage(AppState::new(pool).map_err(|error| error.to_string())?)
        .build(mock_context(noop_assets()))
        .map_err(|error| error.to_string())?;
    let opened = open_project(
        game.to_string_lossy().into_owned(),
        Some("ja".into()),
        Some("en".into()),
        app.state(),
    )
    .await?;
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM segments segment JOIN source_files file \
         ON file.id = segment.source_file_id WHERE file.project_id = ?",
    )
    .bind(&opened.project.id)
    .fetch_one(&app.state::<AppState>().db)
    .await
    .map_err(|error| error.to_string())?;
    if count != 1_191 {
        return Err(format!(
            "{variant} extracted {count} segments, expected 1191"
        ));
    }
    Ok(PilotWorkspace {
        variant: variant.to_string(),
        root,
        project_id: opened.project.id,
        app,
    })
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            copy_tree(&source_path, &destination_path)?;
        } else {
            fs::copy(source_path, destination_path).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}
