//! SQLite pool initialisation + migration runner.
//!
//! Called once from `lib.rs::run()` inside `.setup()`.
//! Uses `sqlx::migrate!("./migrations")` to embed and run migrations at startup.

use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::str::FromStr;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// Repair the historical state where SQLx migration 0004 is recorded as
/// applied but `source_files.translation_secs` is absent.
///
/// SQLite has no portable `ADD COLUMN IF NOT EXISTS`, so the schema check and
/// conditional `ALTER TABLE` run in one transaction. Existing rows are not
/// rewritten; SQLite fills the new nullable column with `NULL`.
async fn repair_translation_secs(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    let repair = async {
        let column_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS( \
                 SELECT 1 FROM pragma_table_info('source_files') \
                 WHERE name = 'translation_secs' \
             )",
        )
        .fetch_one(&mut *tx)
        .await?;

        if !column_exists {
            sqlx::query("ALTER TABLE source_files ADD COLUMN translation_secs INTEGER")
                .execute(&mut *tx)
                .await?;
        }

        Ok::<(), sqlx::Error>(())
    }
    .await;

    match repair {
        Ok(()) => tx.commit().await.map_err(|source| {
            sqlx::Error::Protocol(format!(
                "failed to commit the source_files.translation_secs repair; \
                 the database was not reset. Back up the database file and retry. \
                 Cause: {source}"
            ))
        }),
        Err(source) => {
            let rollback = tx.rollback().await;
            let detail = match rollback {
                Ok(()) => format!(
                    "failed to repair source_files.translation_secs; the repair \
                     transaction was rolled back and existing data was preserved. \
                     Back up the database file and retry. Cause: {source}"
                ),
                Err(rollback_error) => format!(
                    "failed to repair source_files.translation_secs: {source}; \
                     rollback also failed: {rollback_error}. Back up the database \
                     file before retrying"
                ),
            };
            Err(sqlx::Error::Protocol(detail))
        }
    }
}

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

    // Embedded at compile time so packaged builds need no external SQL files.
    // A personal database may have been opened by a newer experimental build.
    // SQLx normally rejects those unknown versions. We preserve that database
    // and its migration history when (and only when) its Patch core schema is
    // still compatible with this build.
    let latest_embedded = MIGRATOR.iter().map(|migration| migration.version).max();
    let latest_applied = latest_applied_migration(&pool).await?;

    if latest_applied
        .zip(latest_embedded)
        .is_some_and(|(applied, embedded)| applied > embedded)
    {
        validate_patch_schema(&pool).await?;
        log::warn!(
            "database migration version {} is newer than embedded version {}; \
             preserving migration history and using the compatible Patch schema",
            latest_applied.unwrap_or_default(),
            latest_embedded.unwrap_or_default()
        );
    } else {
        MIGRATOR.run(&pool).await.map_err(|source| {
            sqlx::Error::Protocol(format!(
                "SQLite schema upgrade failed for '{db_path}'. The database was \
                 not reset; back up the file and retry. Cause: {source}"
            ))
        })?;
    }

    repair_translation_secs(&pool).await?;

    Ok(pool)
}

async fn latest_applied_migration(pool: &SqlitePool) -> Result<Option<i64>, sqlx::Error> {
    let has_migration_table: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master \
         WHERE type = 'table' AND name = '_sqlx_migrations'",
    )
    .fetch_one(pool)
    .await?;

    if has_migration_table == 0 {
        return Ok(None);
    }

    sqlx::query_scalar("SELECT MAX(version) FROM _sqlx_migrations WHERE success = TRUE")
        .fetch_one(pool)
        .await
}

async fn validate_patch_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    const REQUIRED_COLUMNS: &[(&str, &[&str])] = &[
        (
            "projects",
            &[
                "id",
                "name",
                "engine",
                "game_path",
                "source_language",
                "target_language",
            ],
        ),
        (
            "source_files",
            &["id", "project_id", "file_name", "file_path", "file_type"],
        ),
        (
            "segments",
            &[
                "id",
                "source_file_id",
                "json_key",
                "source_text",
                "target_text",
                "status",
                "segment_kind",
                "scene_id",
                "sequence_index",
                "speaker",
                "branch_path",
                "context_json",
            ],
        ),
        (
            "tm_entries",
            &["source_hash", "source_text", "target_text", "lang_pair"],
        ),
        (
            "terminology_entries",
            &[
                "id",
                "source_language",
                "canonical_text",
                "normalized_text",
                "part_of_speech",
                "semantic_type",
                "status",
            ],
        ),
        (
            "terminology_translations",
            &[
                "id",
                "entry_id",
                "target_language",
                "project_id",
                "target_text",
                "enforcement",
            ],
        ),
        (
            "terminology_occurrences",
            &["entry_id", "project_id", "segment_id", "surface_text"],
        ),
    ];

    for (table, required) in REQUIRED_COLUMNS {
        let escaped_table = table.replace('"', "\"\"");
        let pragma = format!("PRAGMA table_info(\"{escaped_table}\")");
        let rows = sqlx::query(&pragma).fetch_all(pool).await?;
        if rows.is_empty() {
            return Err(sqlx::Error::Protocol(format!(
                "newer database is missing required Patch table `{table}`"
            )));
        }

        for column in *required {
            let present = rows.iter().any(|row| {
                row.try_get::<String, _>("name")
                    .is_ok_and(|name| name == *column)
            });
            if !present {
                return Err(sqlx::Error::Protocol(format!(
                    "newer database table `{table}` is missing required Patch column `{column}`"
                )));
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::migrate::Migrate;
    use sqlx::FromRow;
    use tempfile::NamedTempFile;

    async fn connect_pool(path: &str) -> SqlitePool {
        let opts = SqliteConnectOptions::from_str(&format!("sqlite://{path}"))
            .unwrap()
            .create_if_missing(true)
            .foreign_keys(true);
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await
            .unwrap()
    }

    async fn pre_0004_pool(path: &str) -> SqlitePool {
        let pool = connect_pool(path).await;
        sqlx::raw_sql(include_str!("../../tests/fixtures/sqlite_pre_0004.sql"))
            .execute(&pool)
            .await
            .unwrap();

        pool
    }

    async fn pre_0005_pool(path: &str) -> SqlitePool {
        let pool = pre_0004_pool(path).await;
        let mut conn = pool.acquire().await.unwrap();
        conn.apply(
            MIGRATOR
                .iter()
                .find(|migration| migration.version == 4)
                .unwrap(),
        )
        .await
        .unwrap();
        drop(conn);
        pool
    }

    async fn connect_read_only_pool(path: &str) -> SqlitePool {
        let opts = SqliteConnectOptions::from_str(&format!("sqlite://{path}"))
            .unwrap()
            .read_only(true)
            .foreign_keys(true);
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await
            .unwrap()
    }

    type LedgerEntry = (i64, String, String, i64, Vec<u8>, i64);

    async fn migration_ledger(pool: &SqlitePool) -> Vec<LedgerEntry> {
        sqlx::query_as(
            "SELECT version, description, CAST(installed_on AS TEXT), \
                    CAST(success AS INTEGER), checksum, execution_time \
             FROM _sqlx_migrations ORDER BY version",
        )
        .fetch_all(pool)
        .await
        .unwrap()
    }

    async fn schema_snapshot(pool: &SqlitePool) -> Vec<(String, String, String)> {
        sqlx::query_as(
            "SELECT type, name, sql FROM sqlite_master \
             WHERE sql IS NOT NULL ORDER BY type, name",
        )
        .fetch_all(pool)
        .await
        .unwrap()
    }

    async fn has_translation_secs(pool: &SqlitePool) -> bool {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS( \
                 SELECT 1 FROM pragma_table_info('source_files') \
                 WHERE name = 'translation_secs' \
             )",
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn translation_secs_column_count(pool: &SqlitePool) -> i64 {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('source_files') \
             WHERE name = 'translation_secs'",
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    async fn seed_representative_data(pool: &SqlitePool) {
        sqlx::query(
            "INSERT INTO projects (id, name, engine, game_path) \
             VALUES ('p1', 'Fixture', 'mv_mz', '/tmp/game')",
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
             VALUES ('f1', 'p1', 'Actors.json', '/tmp/game/www/data/Actors.json', 'actors')",
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO segments \
                 (id, source_file_id, json_key, source_text, target_text, status) \
             VALUES ('s1', 'f1', '/1/name', '勇者', 'Hero', 'translated')",
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO glossary_terms \
                 (id, source_text, target_text, lang_pair, project_id) \
             VALUES ('g1', '勇者', 'Hero', 'ja-en', 'p1')",
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO tm_entries \
                 (id, source_hash, source_text, target_text, engine, lang_pair) \
             VALUES ('tm1', 'hash', '勇者', 'Hero', 'mv_mz', 'ja-en')",
        )
        .execute(pool)
        .await
        .unwrap();
    }

    async fn assert_representative_data_is_preserved(pool: &SqlitePool) {
        let preserved: (String, String, String, String, String) = sqlx::query_as(
            "SELECT p.name, s.source_text, s.target_text, g.target_text, tm.target_text \
             FROM projects p \
             JOIN source_files sf ON sf.project_id = p.id \
             JOIN segments s ON s.source_file_id = sf.id \
             JOIN glossary_terms g ON g.project_id = p.id \
             JOIN tm_entries tm ON tm.source_text = s.source_text \
             WHERE p.id = 'p1'",
        )
        .fetch_one(pool)
        .await
        .unwrap();

        assert_eq!(
            preserved,
            (
                "Fixture".to_string(),
                "勇者".to_string(),
                "Hero".to_string(),
                "Hero".to_string(),
                "Hero".to_string(),
            )
        );
    }

    /// Reproduce a database whose SQLx ledger says migration 0004 ran while
    /// the corresponding column is absent.
    #[tokio::test]
    async fn repairs_partially_migrated_database_missing_translation_secs() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        let pool = init(&path).await.unwrap();

        seed_representative_data(&pool).await;
        sqlx::query("ALTER TABLE source_files DROP COLUMN translation_secs")
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;

        let repaired = init(&path)
            .await
            .expect("partial database should be repaired");
        assert!(has_translation_secs(&repaired).await);
        assert_representative_data_is_preserved(&repaired).await;
    }

    /// A real pre-0004 schema snapshot must follow the normal forward migration
    /// path and preserve all representative user records.
    #[tokio::test]
    async fn upgrades_pre_translation_secs_database() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        let pool = pre_0004_pool(&path).await;
        seed_representative_data(&pool).await;
        assert!(!has_translation_secs(&pool).await);
        pool.close().await;

        let upgraded = init(&path).await.expect("pre-0004 database should upgrade");
        assert!(has_translation_secs(&upgraded).await);
        assert_representative_data_is_preserved(&upgraded).await;

        let versions: Vec<i64> =
            sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
                .fetch_all(&upgraded)
                .await
                .unwrap();
        assert_eq!(versions, (1..=9).collect::<Vec<_>>());
    }

    /// A write failure inside the repair must roll back only that transaction,
    /// retain the SQLx cause, and leave both user data and migration history
    /// byte-for-byte unchanged.
    #[tokio::test]
    async fn repair_failure_is_actionable_and_rolls_back_everything() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        let pool = pre_0004_pool(&path).await;
        seed_representative_data(&pool).await;
        assert!(!has_translation_secs(&pool).await);
        let ledger_before = migration_ledger(&pool).await;
        let schema_before = schema_snapshot(&pool).await;
        pool.close().await;

        let read_only = connect_read_only_pool(&path).await;
        let error = repair_translation_secs(&read_only)
            .await
            .expect_err("ALTER TABLE must fail on a read-only database");
        let message = error.to_string();
        assert!(
            message.contains("repair transaction was rolled back"),
            "the repair error branch was not observed: {message}"
        );
        assert!(
            message.contains("attempt to write a readonly database"),
            "the underlying SQLx/SQLite cause was lost: {message}"
        );
        assert!(
            message.contains("Back up the database file and retry"),
            "recovery guidance is missing: {message}"
        );
        read_only.close().await;

        let verification = connect_pool(&path).await;
        assert!(!has_translation_secs(&verification).await);
        assert_representative_data_is_preserved(&verification).await;
        assert_eq!(migration_ledger(&verification).await, ledger_before);
        assert_eq!(schema_snapshot(&verification).await, schema_before);

        let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
            .fetch_one(&verification)
            .await
            .unwrap();
        let foreign_key_violations = sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&verification)
            .await
            .unwrap();
        assert_eq!(integrity, "ok");
        assert!(foreign_key_violations.is_empty());
    }

    /// Current databases are a no-op: startup neither duplicates the column nor
    /// overwrites its values, and a second startup is equally safe.
    #[tokio::test]
    async fn current_database_and_repeated_startup_are_unchanged() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        let pool = init(&path).await.unwrap();
        seed_representative_data(&pool).await;
        sqlx::query("UPDATE source_files SET translation_secs = 42 WHERE id = 'f1'")
            .execute(&pool)
            .await
            .unwrap();
        let schema_version_before: i64 = sqlx::query_scalar("PRAGMA schema_version")
            .fetch_one(&pool)
            .await
            .unwrap();
        pool.close().await;

        init(&path).await.unwrap().close().await;
        let reopened = init(&path).await.unwrap();

        let duration: Option<i64> =
            sqlx::query_scalar("SELECT translation_secs FROM source_files WHERE id = 'f1'")
                .fetch_one(&reopened)
                .await
                .unwrap();
        let schema_version_after: i64 = sqlx::query_scalar("PRAGMA schema_version")
            .fetch_one(&reopened)
            .await
            .unwrap();

        assert_eq!(duration, Some(42));
        assert_eq!(translation_secs_column_count(&reopened).await, 1);
        assert_eq!(schema_version_after, schema_version_before);
        assert_representative_data_is_preserved(&reopened).await;
    }

    /// A failing SQLx migration must return recovery guidance and roll back its
    /// script and bookkeeping instead of silently changing user data.
    #[tokio::test]
    async fn failed_migration_is_actionable_and_preserves_database() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        let pool = pre_0005_pool(&path).await;

        for (id, target) in [("tm1", "Hero"), ("tm2", "Champion")] {
            sqlx::query(
                "INSERT INTO tm_entries \
                     (id, source_hash, source_text, target_text, engine, lang_pair) \
                 VALUES (?, 'same-hash', '勇者', ?, 'mv_mz', 'ja-en')",
            )
            .bind(id)
            .bind(target)
            .execute(&pool)
            .await
            .unwrap();
        }
        sqlx::query(
            "CREATE TRIGGER block_tm_migration \
             BEFORE DELETE ON tm_entries \
             BEGIN SELECT RAISE(ABORT, 'fixture blocks migration'); END",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool.close().await;

        let error = init(&path).await.expect_err("migration 0005 should fail");
        let message = error.to_string();
        assert!(message.contains("SQLite schema upgrade failed"));
        assert!(message.contains("not reset; back up the file and retry"));

        let verification = connect_pool(&path).await;
        let entries: Vec<(String, String)> =
            sqlx::query_as("SELECT id, target_text FROM tm_entries ORDER BY id")
                .fetch_all(&verification)
                .await
                .unwrap();
        let version_five_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE version = 5")
                .fetch_one(&verification)
                .await
                .unwrap();

        assert_eq!(
            entries,
            vec![
                ("tm1".to_string(), "Hero".to_string()),
                ("tm2".to_string(), "Champion".to_string()),
            ]
        );
        assert_eq!(version_five_count, 0);
    }

    #[derive(Debug, FromRow, PartialEq, Eq)]
    struct StoredSegmentContext {
        segment_kind: String,
        scene_id: Option<String>,
        sequence_index: Option<i64>,
        speaker: Option<String>,
        branch_path: Option<String>,
        context_json: Option<String>,
    }

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
        assert!(has_translation_secs(&pool).await);

        let versions: Vec<i64> =
            sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(versions, (1..=9).collect::<Vec<_>>());
    }

    /// Existing inserts remain compatible and receive the historical language
    /// defaults until the user explicitly chooses another pair.
    #[tokio::test]
    async fn test_project_language_defaults() {
        let tmp = NamedTempFile::new().unwrap();
        let pool = init(tmp.path().to_str().unwrap()).await.unwrap();

        sqlx::query(
            "INSERT INTO projects (id, name, engine, game_path) VALUES ('p1','T','mv_mz','/tmp')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let pair: (String, String) =
            sqlx::query_as("SELECT source_language, target_language FROM projects WHERE id = 'p1'")
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(pair, ("ja".to_string(), "en".to_string()));
    }

    /// Databases created by a newer experimental build remain readable when
    /// they still expose the complete Patch core schema. Their SQLx history is
    /// never rewritten to impersonate this older build.
    #[tokio::test]
    async fn test_newer_compatible_database_is_preserved() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        let pool = init(&path).await.unwrap();

        sqlx::query(
            "INSERT INTO projects \
             (id, name, engine, game_path, source_language, target_language) \
             VALUES ('future-project', 'Future', 'mv_mz', '/tmp/future', 'ja', 'fr')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO _sqlx_migrations \
             (version, description, success, checksum, execution_time) \
             VALUES (10, 'future test migration', TRUE, X'010203', 1)",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool.close().await;

        let reopened = init(&path).await.expect("compatible future DB should open");
        let project_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM projects WHERE id = 'future-project'")
                .fetch_one(&reopened)
                .await
                .unwrap();
        let future_checksum: Vec<u8> =
            sqlx::query_scalar("SELECT checksum FROM _sqlx_migrations WHERE version = 10")
                .fetch_one(&reopened)
                .await
                .unwrap();

        assert_eq!(project_count, 1);
        assert_eq!(future_checksum, vec![1, 2, 3]);
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

    #[tokio::test]
    async fn test_segment_context_defaults_are_backward_compatible() {
        let tmp = NamedTempFile::new().unwrap();
        let pool = init(tmp.path().to_str().unwrap()).await.unwrap();

        sqlx::query(
            "INSERT INTO projects (id, name, engine, game_path) \
             VALUES ('p1','T','mv_mz','/tmp')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO source_files \
             (id, project_id, file_name, file_path, file_type) \
             VALUES ('f1','p1','A.json','/tmp/A.json','actors')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO segments (id, source_file_id, json_key, source_text) \
             VALUES ('s1','f1','/1/name','テスト')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let row: StoredSegmentContext = sqlx::query_as(
            "SELECT segment_kind, scene_id, sequence_index, speaker, \
                    branch_path, context_json \
             FROM segments WHERE id = 's1'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(
            row,
            StoredSegmentContext {
                segment_kind: "unknown".to_string(),
                scene_id: None,
                sequence_index: None,
                speaker: None,
                branch_path: None,
                context_json: None,
            }
        );
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
