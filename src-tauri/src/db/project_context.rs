//! A project as the commands read it (plan DD, L5): one query instead of a
//! `SELECT … FROM projects WHERE id = ?` per command, and the engine read
//! once into [`Engine`].

use sqlx::SqlitePool;

use crate::domain::types::SourceFile;
use crate::engines::detector::Engine;

/// One project's row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectContext {
    pub id: String,
    pub name: String,
    /// `projects.engine` as stored.
    pub engine_name: String,
    /// `None`: an engine this build does not know.
    pub engine: Option<Engine>,
    pub game_path: String,
    pub source_language: String,
    pub target_language: String,
}

impl ProjectContext {
    /// The project `project_id`. A missing project is the error of any
    /// single-row query, as the commands reported it before.
    pub async fn load(db: &SqlitePool, project_id: &str) -> Result<Self, String> {
        let (id, name, engine_name, game_path, source_language, target_language): (
            String,
            String,
            String,
            String,
            String,
            String,
        ) = sqlx::query_as(
            "SELECT id, name, engine, game_path, source_language, target_language \
             FROM projects WHERE id = ?",
        )
        .bind(project_id)
        .fetch_one(db)
        .await
        .map_err(|error| error.to_string())?;
        Ok(Self {
            id,
            name,
            engine: Engine::from_db_str(&engine_name),
            engine_name,
            game_path,
            source_language,
            target_language,
        })
    }

    /// The project's engine is `engine`.
    pub fn is(&self, engine: Engine) -> bool {
        self.engine == Some(engine)
    }

    /// Every source file of the project (no counts).
    pub async fn source_files(&self, db: &SqlitePool) -> Result<Vec<SourceFile>, String> {
        sqlx::query_as::<_, SourceFile>(
            "SELECT id, project_id, file_name, file_path, file_type, translation_secs \
             FROM source_files WHERE project_id = ?",
        )
        .bind(&self.id)
        .fetch_all(db)
        .await
        .map_err(|error| error.to_string())
    }
}
