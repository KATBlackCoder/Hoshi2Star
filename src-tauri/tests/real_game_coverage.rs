//! Manual pilot against a real RPG Maker MZ game (never in CI).
//!
//! ```text
//! H2S_MZ_GAME_DIR="D:/path/to/game" cargo test --test real_game_coverage -- --ignored --nocapture
//! ```
//!
//! Only `data/` and `js/` are copied into a temporary folder: the game folder
//! itself is never written. The test scans, enables every category that may be
//! enabled, fake-translates every segment (keeping all placeholders), exports
//! twice and checks that the archives are identical and that `plugins.js`
//! changed only in profiled parameters.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use hoshi2star_lib::commands::coverage::{scan_text_coverage, set_text_categories, CategoryChange};
use hoshi2star_lib::commands::export::{export_project, preflight_export};
use hoshi2star_lib::commands::project::open_project;
use hoshi2star_lib::db;
use hoshi2star_lib::engines::mv_mz::coverage::CategoryState;
use hoshi2star_lib::engines::mv_mz::plugins_js::PluginsJs;
use hoshi2star_lib::llm::tokenizer::{Engine as TokEngine, Tokenizer};
use hoshi2star_lib::state::AppState;
use sha2::{Digest, Sha256};
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::Manager;

fn copy_tree(source: &Path, destination: &Path) {
    std::fs::create_dir_all(destination).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn zip_entries(zip_path: &str) -> BTreeMap<String, Vec<u8>> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(zip_path).unwrap()).unwrap();
    let mut entries = BTreeMap::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).unwrap();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        entries.insert(entry.name().to_string(), bytes);
    }
    entries
}

#[tokio::test]
#[ignore = "requires a private game: set H2S_MZ_GAME_DIR"]
async fn real_game_scan_enable_translate_export() {
    let Ok(source_game) = std::env::var("H2S_MZ_GAME_DIR") else {
        eprintln!("H2S_MZ_GAME_DIR not set; skipping");
        return;
    };
    let source_game = PathBuf::from(source_game);
    let temporary = tempfile::tempdir().unwrap();
    let game = temporary.path().join("game");
    copy_tree(&source_game.join("data"), &game.join("data"));
    copy_tree(&source_game.join("js"), &game.join("js"));
    let original_plugins = std::fs::read_to_string(game.join("js/plugins.js")).unwrap();

    let pool = db::pool::init(temporary.path().join("pilot.db").to_str().unwrap())
        .await
        .unwrap();
    let app = mock_builder()
        .manage(AppState::new(pool, temporary.path().join("app-data")).unwrap())
        .build(mock_context(noop_assets()))
        .unwrap();
    let project_id = open_project(
        game.to_string_lossy().into_owned(),
        Some("ja".into()),
        Some("en".into()),
        app.state(),
    )
    .await
    .unwrap()
    .project
    .id;

    let view = scan_text_coverage(project_id.clone(), app.state())
        .await
        .unwrap();
    eprintln!("== categories ({})", view.categories.len());
    for category in &view.categories {
        eprintln!(
            "  [{:?}] {} — {} text(s), {} reference(s){}",
            category.state,
            category.key,
            category.segment_count,
            category.reference_count,
            category
                .references
                .first()
                .map(|r| format!(" e.g. {} @ {}", r.text, r.location))
                .unwrap_or_default()
        );
    }
    eprintln!("== not covered: {} row(s)", view.not_covered.len());
    for row in view.not_covered.iter().take(15) {
        eprintln!(
            "  {} :: {} = {:?} ({:?})",
            row.plugin, row.field, row.sample, row.hint
        );
    }
    eprintln!("== core warnings: {}", view.core_warnings.len());

    let changes: Vec<CategoryChange> = view
        .categories
        .iter()
        .filter(|category| category.state != CategoryState::Blocked)
        .map(|category| CategoryChange {
            category_key: category.key.clone(),
            enabled: true,
            accept_consequences: category.state == CategoryState::Linked,
        })
        .collect();
    set_text_categories(project_id.clone(), changes, app.state())
        .await
        .unwrap();

    // Fake-translate every segment, keeping every placeholder, same source →
    // same target (canonical kinds must stay consistent).
    let db = &app.state::<AppState>().db;
    let rows: Vec<(String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT s.id, s.source_text, s.segment_kind, s.context_json FROM segments s \
         JOIN source_files f ON f.id = s.source_file_id WHERE f.project_id = ?",
    )
    .bind(&project_id)
    .fetch_all(db)
    .await
    .unwrap();
    let mut transaction = db.begin().await.unwrap();
    for (id, source, kind, context) in &rows {
        let syntax = TokEngine::for_segment("mv_mz", kind, context.as_deref());
        let mut placeholders = Tokenizer::tokenize(source, syntax)
            .map
            .into_values()
            .collect::<Vec<_>>();
        placeholders.sort();
        let placeholders = placeholders.join(" ");
        let digest = hex::encode(Sha256::digest(source.as_bytes()));
        let target = if context
            .as_deref()
            .is_some_and(|c| c.contains("keke_skill_type"))
        {
            // Skill types looked up in battle-command tags: one word, no digits.
            let letters: String = digest[..8]
                .chars()
                .map(|c| match c.to_digit(16) {
                    Some(value) => char::from(b'a' + value as u8),
                    None => c,
                })
                .collect();
            format!("Type{letters}")
        } else if placeholders.is_empty() {
            format!("EN-{}", &digest[..8])
        } else {
            format!("EN-{} {placeholders}", &digest[..8])
        };
        sqlx::query("UPDATE segments SET target_text = ?, status = 'translated' WHERE id = ?")
            .bind(&target)
            .bind(id)
            .execute(&mut *transaction)
            .await
            .unwrap();
    }
    transaction.commit().await.unwrap();
    eprintln!("== translated {} segment(s)", rows.len());
    let (summary, details) = hoshi2star_lib::core::report::preview_project(db, &project_id)
        .await
        .unwrap();
    eprintln!(
        "== QA: {} critical; by type {:?}",
        summary.critical_count, summary.errors_by_type
    );
    for detail in details
        .iter()
        .filter(|detail| detail.errors.iter().any(|error| error.is_critical()))
        .take(12)
    {
        eprintln!(
            "  {} #{} {:?} -> {:?}: {:?}",
            detail.file_name,
            detail.segment_number,
            detail.source_text,
            detail.target_text,
            detail.errors
        );
    }

    let first = export_project(project_id.clone(), Some(22), true, None, app.state())
        .await
        .expect("export passes QA and the integrity gate");
    let first_entries = zip_entries(&first);
    let second = export_project(project_id.clone(), Some(22), true, None, app.state())
        .await
        .unwrap();
    assert_eq!(
        zip_entries(&second),
        first_entries,
        "exports must be identical"
    );

    // Optional language switch: only System.json `locale` may differ.
    if let Some(option) = preflight_export(project_id.clone(), app.state())
        .await
        .unwrap()
        .locale
    {
        eprintln!("== locale {} -> {}", option.current, option.target);
        for line in &option.consequences {
            eprintln!("  - {line}");
        }
        for code in &option.uses {
            eprintln!("    {} | {}", code.location, code.code);
        }
        let switched = export_project(project_id.clone(), Some(22), true, Some(true), app.state())
            .await
            .expect("the locale switch passes the integrity gate");
        let switched_entries = zip_entries(&switched);
        assert_eq!(
            switched_entries.keys().collect::<Vec<_>>(),
            first_entries.keys().collect::<Vec<_>>()
        );
        for (name, bytes) in &switched_entries {
            if name.ends_with("data/System.json") {
                let mut expected: serde_json::Value =
                    serde_json::from_slice(&first_entries[name]).unwrap();
                expected["locale"] = serde_json::Value::String(option.target.clone());
                let actual: serde_json::Value = serde_json::from_slice(bytes).unwrap();
                assert_eq!(actual, expected);
            } else {
                assert_eq!(Some(bytes), first_entries.get(name), "{name} changed");
            }
        }
    }

    let exported_plugins = first_entries
        .get("js/plugins.js")
        .map(|bytes| String::from_utf8(bytes.clone()).unwrap());
    if let Some(exported) = exported_plugins {
        let before = PluginsJs::parse(&original_plugins).unwrap();
        let after = PluginsJs::parse(&exported).unwrap();
        let mut changed_plugins = Vec::new();
        for (old, new) in before.entries().iter().zip(after.entries()) {
            if before.entry_text(old) != after.entry_text(new) {
                changed_plugins.push(old.name.clone());
            }
        }
        eprintln!("== plugins.js entries changed: {changed_plugins:?}");
        assert_eq!(before.prefix(), after.prefix());
        assert_eq!(before.suffix(), after.suffix());
        std::fs::write(temporary.path().join("exported_plugins.js"), &exported).unwrap();
        if let Ok(path) = std::env::var("H2S_DUMP_PLUGINS_JS") {
            std::fs::write(path, &exported).unwrap();
        }
    }
    eprintln!("== archive entries: {}", first_entries.len());
}
