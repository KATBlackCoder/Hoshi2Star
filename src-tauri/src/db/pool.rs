//! SQLite pool initialisation + migration runner.
//!
//! Called once from `lib.rs::run()` inside `.setup()`.
//! Uses `sqlx::migrate!("./migrations")` to embed and run migrations at startup.

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;

/// Initialise the SQLite connection pool and run pending migrations.
///
/// `db_path` must be an absolute file-system path (no `sqlite://` prefix).
/// The file is created if it does not exist.
pub async fn init(db_path: &str) -> Result<SqlitePool, sqlx::Error> {
    let opts = SqliteConnectOptions::from_str(&format!("sqlite://{db_path}"))?
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    /// Run migrations on a fresh temp DB and verify the three tables exist.
    #[tokio::test]
    async fn test_migrations_create_tables() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        let pool = init(&path).await.expect("pool init failed");

        // Verify tables exist via sqlite_master
        let tables: Vec<String> =
            sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
                .fetch_all(&pool)
                .await
                .unwrap();

        assert!(tables.contains(&"projects".to_string()));
        assert!(tables.contains(&"source_files".to_string()));
        assert!(tables.contains(&"segments".to_string()));
    }

    /// Verify the status CHECK constraint rejects invalid values.
    #[tokio::test]
    async fn test_segment_status_constraint() {
        let tmp = NamedTempFile::new().unwrap();
        let pool = init(tmp.path().to_str().unwrap()).await.unwrap();

        sqlx::query(
            "INSERT INTO projects (id, name, engine, game_path) VALUES ('p1','T','mv_mz','/tmp')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO source_files (id, project_id, file_name, file_path, file_type) VALUES ('f1','p1','A.json','/tmp/A.json','actors')")
            .execute(&pool).await.unwrap();

        // Valid status — must succeed
        sqlx::query("INSERT INTO segments (id, source_file_id, json_key, source_text, status) VALUES ('s1','f1','/1/name','テスト','untranslated')")
            .execute(&pool).await.unwrap();

        // Invalid status — must fail
        let err = sqlx::query("INSERT INTO segments (id, source_file_id, json_key, source_text, status) VALUES ('s2','f1','/2/name','NG','invalid_status')")
            .execute(&pool).await;
        assert!(err.is_err(), "invalid status should be rejected");
    }

    /// Verify ON DELETE CASCADE removes child rows when a project is deleted.
    #[tokio::test]
    async fn test_cascade_delete() {
        let tmp = NamedTempFile::new().unwrap();
        let pool = init(tmp.path().to_str().unwrap()).await.unwrap();

        sqlx::query(
            "INSERT INTO projects (id, name, engine, game_path) VALUES ('p1','T','mv_mz','/tmp')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO source_files (id, project_id, file_name, file_path, file_type) VALUES ('f1','p1','A.json','/tmp/A.json','actors')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO segments (id, source_file_id, json_key, source_text) VALUES ('s1','f1','/1/name','テスト')")
            .execute(&pool).await.unwrap();

        sqlx::query("DELETE FROM projects WHERE id = 'p1'")
            .execute(&pool)
            .await
            .unwrap();

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM segments")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0, "cascade delete should remove segments");
    }
}
