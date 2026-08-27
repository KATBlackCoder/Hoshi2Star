use std::str::FromStr;

use hoshi2star_lib::db;
use sqlx::migrate::{Migrate, Migrator};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

async fn connect(path: &str) -> SqlitePool {
    let options = SqliteConnectOptions::from_str(&format!("sqlite://{path}"))
        .unwrap()
        .create_if_missing(true)
        .foreign_keys(true);
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap()
}

async fn migrate_through_0007(pool: &SqlitePool) {
    let mut connection = pool.acquire().await.unwrap();
    connection.ensure_migrations_table().await.unwrap();
    for migration in MIGRATOR.iter().filter(|migration| migration.version <= 7) {
        connection.apply(migration).await.unwrap();
    }
}

async fn seed_legacy_glossary(pool: &SqlitePool) {
    sqlx::query(
        "INSERT INTO projects \
         (id, name, engine, game_path, source_language, target_language) \
         VALUES ('p1', 'Legacy game', 'mv_mz', '/tmp/game', 'ja', 'en')",
    )
    .execute(pool)
    .await
    .unwrap();

    for (id, source, target, pair, domain, project_id, auto_generated) in [
        (
            "g-global-en",
            "勇者",
            "Hero",
            "ja-en",
            "character",
            None,
            false,
        ),
        (
            "g-project-en",
            "勇者",
            "Champion",
            "ja-en",
            "character",
            Some("p1"),
            true,
        ),
        (
            "g-global-fr",
            "勇者",
            "Héros",
            "ja-fr",
            "character",
            None,
            false,
        ),
        (
            "g-empty",
            "魔王",
            "",
            "ja-en",
            "character",
            Some("p1"),
            true,
        ),
    ] {
        sqlx::query(
            "INSERT INTO glossary_terms \
             (id, source_text, target_text, lang_pair, domain, project_id, auto_generated) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(source)
        .bind(target)
        .bind(pair)
        .bind(domain)
        .bind(project_id)
        .bind(auto_generated)
        .execute(pool)
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn legacy_glossary_migrates_without_losing_scope_or_languages() {
    let temporary = tempfile::NamedTempFile::new().unwrap();
    let path = temporary.path().to_str().unwrap().to_string();
    let pool = connect(&path).await;
    migrate_through_0007(&pool).await;
    seed_legacy_glossary(&pool).await;
    pool.close().await;

    let migrated = db::pool::init(&path).await.unwrap();

    let entries: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT source_language, canonical_text, normalized_text, semantic_type \
         FROM terminology_entries ORDER BY canonical_text",
    )
    .fetch_all(&migrated)
    .await
    .unwrap();
    assert_eq!(
        entries,
        vec![
            (
                "ja".to_string(),
                "勇者".to_string(),
                "勇者".to_string(),
                "character".to_string(),
            ),
            (
                "ja".to_string(),
                "魔王".to_string(),
                "魔王".to_string(),
                "character".to_string(),
            ),
        ]
    );

    let translations: Vec<(String, Option<String>, String, String, String, String)> =
        sqlx::query_as(
            "SELECT tt.target_language, tt.project_id, tt.target_text, \
                    tt.review_status, tt.enforcement, te.canonical_text \
             FROM terminology_translations tt \
             JOIN terminology_entries te ON te.id = tt.entry_id \
             ORDER BY tt.target_language, tt.project_id IS NOT NULL, tt.project_id",
        )
        .fetch_all(&migrated)
        .await
        .unwrap();
    assert_eq!(
        translations,
        vec![
            (
                "en".to_string(),
                None,
                "Hero".to_string(),
                "approved".to_string(),
                "required".to_string(),
                "勇者".to_string(),
            ),
            (
                "en".to_string(),
                Some("p1".to_string()),
                "Champion".to_string(),
                "proposed".to_string(),
                "preferred".to_string(),
                "勇者".to_string(),
            ),
            (
                "fr".to_string(),
                None,
                "Héros".to_string(),
                "approved".to_string(),
                "required".to_string(),
                "勇者".to_string(),
            ),
        ]
    );

    let empty_translation_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM terminology_translations WHERE trim(target_text) = ''",
    )
    .fetch_one(&migrated)
    .await
    .unwrap();
    assert_eq!(empty_translation_count, 0);

    sqlx::query("DELETE FROM projects WHERE id = 'p1'")
        .execute(&migrated)
        .await
        .unwrap();

    let entry_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM terminology_entries")
        .fetch_one(&migrated)
        .await
        .unwrap();
    let local_translation_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM terminology_translations WHERE project_id IS NOT NULL",
    )
    .fetch_one(&migrated)
    .await
    .unwrap();
    assert_eq!(
        entry_count, 2,
        "global source entries must survive game removal"
    );
    assert_eq!(local_translation_count, 0);
}

#[tokio::test]
async fn fresh_schema_and_repeated_startup_are_idempotent() {
    let temporary = tempfile::NamedTempFile::new().unwrap();
    let path = temporary.path().to_str().unwrap().to_string();

    db::pool::init(&path).await.unwrap().close().await;
    let reopened = db::pool::init(&path).await.unwrap();

    let versions: Vec<i64> =
        sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
            .fetch_all(&reopened)
            .await
            .unwrap();
    assert_eq!(versions, (1..=8).collect::<Vec<_>>());

    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name LIKE 'terminology_%' \
         ORDER BY name",
    )
    .fetch_all(&reopened)
    .await
    .unwrap();
    assert_eq!(tables.len(), 7);
}
