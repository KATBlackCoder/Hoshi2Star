//! Regression coverage for upgrading an SQLite database before project export.

use std::io::Read;

use hoshi2star_lib::commands::export::export_project;
use hoshi2star_lib::db;
use hoshi2star_lib::state::AppState;
use serde_json::Value;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::Manager;

#[tokio::test]
async fn export_succeeds_after_repairing_translation_secs() {
    let tmp = tempfile::tempdir().unwrap();
    let game_dir = tmp.path().join("game");
    let data_dir = game_dir.join("www").join("data");
    std::fs::create_dir_all(&data_dir).unwrap();
    let actors_path = data_dir.join("Actors.json");
    std::fs::write(
        &actors_path,
        r#"[null,{"id":1,"name":"勇者","nickname":"","profile":""}]"#,
    )
    .unwrap();

    let db_path = tmp.path().join("hoshi2star.db");
    let db_path = db_path.to_str().unwrap().to_string();
    let pool = db::pool::init(&db_path).await.unwrap();

    sqlx::query(
        "INSERT INTO projects (id, name, engine, game_path) \
         VALUES ('p1', 'Historical project', 'mv_mz', ?)",
    )
    .bind(game_dir.to_str().unwrap())
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO source_files (id, project_id, file_name, file_path, file_type) \
         VALUES ('f1', 'p1', 'Actors.json', ?, 'actors')",
    )
    .bind(actors_path.to_str().unwrap())
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO segments \
             (id, source_file_id, json_key, source_text, target_text, status) \
         VALUES ('s1', 'f1', '/1/name', '勇者', 'Hero', 'translated')",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Simulate an inconsistent installed database: SQLx records migration 0004,
    // but the schema change itself is absent.
    sqlx::query("ALTER TABLE source_files DROP COLUMN translation_secs")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;

    let repaired = db::pool::init(&db_path)
        .await
        .expect("startup should repair the historical schema");
    let app = mock_builder()
        .manage(AppState { db: repaired })
        .build(mock_context(noop_assets()))
        .expect("mock app build");

    let zip_path = export_project("p1".to_string(), None, true, app.state())
        .await
        .expect("export should succeed after schema repair");

    let zip_file = std::fs::File::open(zip_path).unwrap();
    let mut archive = zip::ZipArchive::new(zip_file).unwrap();
    let mut actors_entry = archive.by_name("www/data/Actors.json").unwrap();
    let mut exported_json = String::new();
    actors_entry.read_to_string(&mut exported_json).unwrap();
    let exported: Value = serde_json::from_str(&exported_json).unwrap();

    assert_eq!(
        exported.pointer("/1/name").and_then(Value::as_str),
        Some("Hero")
    );
}
