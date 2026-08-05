# SQLite `translation_secs` Repair Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make startup repair older or partially migrated SQLite databases whose `source_files` table is missing `translation_secs`, without changing or losing user data.

**Architecture:** Keep the immutable SQLx migration history (`0001` through `0005`) intact. After SQLx applies every pending migration, inspect `pragma_table_info('source_files')`; if and only if `translation_secs` is absent, add it inside a dedicated transaction, explicitly rolling back on failure. Cover the historical pre-`0004` upgrade, an inconsistent SQLx ledger that records `0004` while the column is absent, an already-current database, repeated startup, representative data preservation, MV/MZ export, and a read-only repair failure.

**Tech Stack:** Rust 2021, SQLx 0.8, SQLite, Tokio tests, Tauri mock runtime, tempfile, zip

---

### Task 1: Reproduce Historical Schema Failures

**Files:**
- Modify: `src-tauri/src/db/pool.rs`

- [ ] **Step 1: Add schema and fixture helpers to the existing test module**

```rust
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
```

- [ ] **Step 2: Add a regression test for the inconsistent migration ledger**

```rust
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

    let repaired = init(&path).await.expect("partial database should be repaired");
    assert!(has_translation_secs(&repaired).await);
}
```

- [ ] **Step 3: Run the regression test and confirm the original failure**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml db::pool::tests::repairs_partially_migrated_database_missing_translation_secs -- --exact
```

Expected before the fix: FAIL because SQLx trusts migration version `4` in `_sqlx_migrations`, leaves the column absent, and the assertion detects the missing column.

### Task 2: Implement Transactional, Idempotent Repair

**Files:**
- Modify: `src-tauri/src/db/pool.rs`

- [ ] **Step 1: Add the schema inspection helper**

```rust
async fn translation_secs_exists(
    executor: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS( \
             SELECT 1 FROM pragma_table_info('source_files') \
             WHERE name = 'translation_secs' \
         )",
    )
    .fetch_one(executor)
    .await
}
```

- [ ] **Step 2: Add the repair transaction with explicit rollback**

```rust
async fn repair_translation_secs(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    if translation_secs_exists(pool).await? {
        return Ok(());
    }

    let mut tx = pool.begin().await?;
    let repair = sqlx::query(
        "ALTER TABLE source_files ADD COLUMN translation_secs INTEGER",
    )
    .execute(&mut *tx)
    .await;

    match repair {
        Ok(_) => tx.commit().await,
        Err(source) => {
            let rollback = tx.rollback().await;
            let detail = match rollback {
                Ok(()) => format!(
                    "failed to add source_files.translation_secs; \
                     repair was rolled back and the database was preserved: {source}"
                ),
                Err(rollback_error) => format!(
                    "failed to add source_files.translation_secs: {source}; \
                     rollback also failed: {rollback_error}"
                ),
            };
            Err(sqlx::Error::Protocol(detail))
        }
    }
}
```

- [ ] **Step 3: Run the repair after the embedded forward migrations**

```rust
sqlx::migrate!("./migrations")
    .run(&pool)
    .await
    .map_err(|source| {
        sqlx::Error::Protocol(format!(
            "SQLite schema upgrade failed for {db_path}; the database was not \
             reset. Back up the file and retry. Cause: {source}"
        ))
    })?;

repair_translation_secs(&pool).await?;
```

- [ ] **Step 4: Run the focused migration tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml db::pool::tests
```

Expected: all pool migration tests pass.

### Task 3: Cover Every Required Upgrade State and Preserve Data

**Files:**
- Modify: `src-tauri/src/db/pool.rs`

- [ ] **Step 1: Extend the fresh-database test to assert the current schema**

```rust
assert!(has_translation_secs(&pool).await);
let versions: Vec<i64> =
    sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
        .fetch_all(&pool)
        .await
        .unwrap();
assert_eq!(versions, vec![1, 2, 3, 4, 5]);
```

- [ ] **Step 2: Add a pre-`translation_secs` snapshot upgrade test**

Create a database by applying the unmodified `0001`, `0002`, and `0003` SQL snapshots and recording their migration rows, seed representative data, then call `init`. Assert migrations `4` and `5` are applied and the column exists.

- [ ] **Step 3: Assert representative data survives the partial repair**

```rust
let preserved: (String, String, String, String, String) = sqlx::query_as(
    "SELECT p.name, s.source_text, s.target_text, g.target_text, tm.target_text \
     FROM projects p \
     JOIN source_files sf ON sf.project_id = p.id \
     JOIN segments s ON s.source_file_id = sf.id \
     JOIN glossary_terms g ON g.project_id = p.id \
     JOIN tm_entries tm ON tm.source_text = s.source_text \
     WHERE p.id = 'p1'",
)
.fetch_one(&repaired)
.await
.unwrap();
assert_eq!(
    preserved,
    (
        "Fixture".into(),
        "勇者".into(),
        "Hero".into(),
        "Hero".into(),
        "Hero".into(),
    )
);
```

- [ ] **Step 4: Add current-schema and repeated-startup tests**

Set `translation_secs = 42` in a current database, close it, call `init` twice, and assert both calls succeed, the schema contains exactly one `translation_secs` column, and the value remains `42`.

- [ ] **Step 5: Add a read-only repair failure test**

Open an old-schema fixture through `SqliteConnectOptions::read_only(true)`, invoke the repair helper, assert the error tells the user the repair was rolled back, then reopen read-only and verify all representative rows are unchanged and the column is still absent.

- [ ] **Step 6: Run all pool tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml db::pool::tests
```

Expected: fresh, pre-`0004`, partial, current, twice-run, preservation, and failure tests all pass.

### Task 4: Verify MV/MZ Export After Upgrade

**Files:**
- Create: `src-tauri/tests/sqlite_migration.rs`

- [ ] **Step 1: Build a minimal MV/MZ fixture without touching parser code**

```rust
let game_dir = tmp.path().join("game");
let data_dir = game_dir.join("www/data");
std::fs::create_dir_all(&data_dir).unwrap();
let actors = data_dir.join("Actors.json");
std::fs::write(
    &actors,
    r#"[null,{"id":1,"name":"勇者","nickname":"","profile":""}]"#,
)
.unwrap();
```

- [ ] **Step 2: Seed an upgraded historical database and call the real export command**

Use `db::pool::init`, seed the MV/MZ project, source file, and translated segment, drop `translation_secs` while keeping SQLx version `4`, close, reopen through `init`, wrap the pool in a Tauri mock app, and call:

```rust
let zip_path = export_project("p1".into(), None, true, app.state())
    .await
    .expect("export should succeed after repair");
```

- [ ] **Step 3: Verify the exported JSON contains the translation**

Open `www/data/Actors.json` in the zip and assert `/1/name` equals `"Hero"`.

- [ ] **Step 4: Run the integration test**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test sqlite_migration
```

Expected: PASS, proving the query that previously raised `no column found for name: translation_secs` now reaches a successful MV/MZ export.

### Task 5: Document Supported Historical States

**Files:**
- Modify: `docs/architecture.md`

- [ ] **Step 1: Update the database architecture**

Document that `pool.rs` applies immutable migrations `0001`–`0005`, then performs a transactional, idempotent repair for `source_files.translation_secs`. Explicitly list supported states: fresh database, pre-`0004` database, SQLx ledger at/after `0004` with the column missing, and current database with the column present. State that repair adds no default and does not rewrite existing rows.

- [ ] **Step 2: Confirm no Wolf parser files changed**

Run:

```bash
git diff --name-only -- src-tauri/src/engines/wolf
```

Expected: no output.

### Task 6: Full Validation and Review

**Files:**
- No source changes

- [ ] **Step 1: Run frontend validation**

```bash
pnpm typecheck
pnpm test
```

Expected: both commands exit `0`.

- [ ] **Step 2: Run Rust formatting and lint validation**

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
```

Expected: both commands exit `0`.

- [ ] **Step 3: Run the complete Rust suite**

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: all unit and integration tests pass.

- [ ] **Step 4: Review only issue-specific changes**

```bash
git status --short --branch
git diff -- src-tauri/src/db/pool.rs src-tauri/tests/sqlite_migration.rs docs/architecture.md
git diff --check
```

Expected: the pre-existing `tasks/todo.md` and `.claude/skills/*` changes remain untouched; the issue-specific diff contains only migration repair, regression tests, and documentation.

- [ ] **Step 5: Present results and stop before publication**

Show the user the complete issue-specific diff summary and every command result. Do not push, open a pull request, or merge into `main` until the user explicitly approves publication.
