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

/// Text-fitting width check: measures `H2S_FONT_CORPUS` (JSON list of
/// strings) with the game's main font at its font size and writes
/// `[{text, width, missing}]` to `H2S_FONT_OUT`, for comparison with the
/// browser's `measureText` (per character, as normal play draws messages).
#[test]
#[ignore = "requires a private game: set H2S_MZ_GAME_DIR, H2S_FONT_CORPUS, H2S_FONT_OUT"]
fn real_game_font_widths() {
    use hoshi2star_lib::engines::mv_mz::layout::font::{GameFont, Metrics};
    use hoshi2star_lib::engines::mv_mz::layout::geometry::Geometry;

    let (Ok(game), Ok(corpus), Ok(out)) = (
        std::env::var("H2S_MZ_GAME_DIR"),
        std::env::var("H2S_FONT_CORPUS"),
        std::env::var("H2S_FONT_OUT"),
    ) else {
        eprintln!("H2S_MZ_GAME_DIR / H2S_FONT_CORPUS / H2S_FONT_OUT not set; skipping");
        return;
    };
    let game = PathBuf::from(game);
    let system: serde_json::Value =
        serde_json::from_slice(&std::fs::read(game.join("data/System.json")).unwrap()).unwrap();
    let geometry = Geometry::from_system(&system);
    let font_file = geometry.font_file.clone().expect("mainFontFilename");
    let metrics = Metrics::Font(std::sync::Arc::new(
        GameFont::from_bytes(&std::fs::read(game.join("fonts").join(font_file)).unwrap()).unwrap(),
    ));
    let corpus: Vec<String> = serde_json::from_slice(&std::fs::read(corpus).unwrap()).unwrap();
    let rows: Vec<serde_json::Value> = corpus
        .iter()
        .map(|text| {
            serde_json::json!({
                "text": text,
                "width": metrics.text_width(text, geometry.font_size),
                "missing": metrics.missing_chars(text).iter().collect::<String>(),
            })
        })
        .collect();
    std::fs::write(out, serde_json::to_vec_pretty(&rows).unwrap()).unwrap();
}

/// Text-fitting dump: fits every Show Text message of `H2S_MZ_GAME_DIR`
/// (original data) with the translations of the project database copy
/// `H2S_FIT_DB`, measured with the font file `H2S_FONT_FILE`, and writes
/// inputs, outputs and simulated pages to `H2S_FIT_OUT` (checked against the
/// engine's own code by a scratch harness).
#[tokio::test]
#[ignore = "requires a private game: set H2S_MZ_GAME_DIR, H2S_FIT_DB, H2S_FONT_FILE, H2S_FIT_OUT"]
async fn real_game_fit_dump() {
    use hoshi2star_lib::engines::mv_mz::layout::fit::{
        fit_message, FitContext, FitOutcome, FitSettings, LineBreaks, MessageInput, MessageLine,
        Overflow,
    };
    use hoshi2star_lib::engines::mv_mz::layout::font::{GameFont, Metrics};
    use hoshi2star_lib::engines::mv_mz::layout::geometry::Geometry;
    use hoshi2star_lib::engines::mv_mz::layout::message::messages_in;
    use hoshi2star_lib::engines::mv_mz::layout::text::Inserts;
    use std::collections::HashMap;
    use std::str::FromStr;

    let vars = [
        "H2S_MZ_GAME_DIR",
        "H2S_FIT_DB",
        "H2S_FONT_FILE",
        "H2S_FIT_OUT",
    ]
    .map(std::env::var);
    let [Ok(game), Ok(db_path), Ok(font_file), Ok(out)] = vars else {
        eprintln!("H2S_MZ_GAME_DIR / H2S_FIT_DB / H2S_FONT_FILE / H2S_FIT_OUT not set; skipping");
        return;
    };
    let data = PathBuf::from(game).join("data");
    let read = |name: &str| -> serde_json::Value {
        serde_json::from_slice(&std::fs::read(data.join(name)).unwrap()).unwrap()
    };
    let options =
        sqlx::sqlite::SqliteConnectOptions::from_str(&format!("sqlite://{db_path}?mode=ro"))
            .unwrap();
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect_with(options)
        .await
        .unwrap();
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT f.file_name, s.json_key, s.target_text FROM segments s \
         JOIN source_files f ON f.id = s.source_file_id WHERE s.target_text != ''",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let targets: HashMap<(String, String), String> =
        rows.into_iter().map(|(f, k, t)| ((f, k), t)).collect();
    let target_or = |file: &str, key: &str, fallback: &str| {
        targets
            .get(&(file.to_string(), key.to_string()))
            .cloned()
            .unwrap_or_else(|| fallback.to_string())
    };

    let system = read("System.json");
    let geometry = Geometry::from_system(&system);
    let metrics = Metrics::Font(std::sync::Arc::new(
        GameFont::from_bytes(&std::fs::read(font_file).unwrap()).unwrap(),
    ));
    let mut inserts = Inserts {
        currency: target_or(
            "System.json",
            "/currencyUnit",
            system["currencyUnit"].as_str().unwrap_or_default(),
        ),
        ..Inserts::default()
    };
    for (id, actor) in read("Actors.json").as_array().unwrap().iter().enumerate() {
        if let Some(name) = actor.get("name").and_then(|n| n.as_str()) {
            inserts.actor_names.insert(
                id as u32,
                target_or("Actors.json", &format!("/{id}/name"), name),
            );
        }
    }
    let ctx = FitContext {
        metrics: &metrics,
        geometry: &geometry,
        inserts: &inserts,
        no_space_join: false,
        plain_description_first_line: false,
    };
    // Optional: H2S_FIT_OVERFLOW=shrink|leave, H2S_FIT_FONT=<px>, H2S_FIT_KEEP=1.
    let settings = FitSettings {
        line_breaks: if std::env::var("H2S_FIT_KEEP").is_ok() {
            LineBreaks::Keep
        } else {
            LineBreaks::Reflow
        },
        overflow: match std::env::var("H2S_FIT_OVERFLOW").as_deref() {
            Ok("shrink") => Overflow::Shrink,
            Ok("leave") => Overflow::Leave,
            _ => Overflow::NewPage,
        },
        min_font_size: 20,
        dialogue_font_size: std::env::var("H2S_FIT_FONT")
            .ok()
            .and_then(|v| v.parse().ok()),
        strip_stored_font_prefix: false,
    };

    let mut files: Vec<String> = std::fs::read_dir(&data)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
        .filter(|n| {
            (n.starts_with("Map") && n != "MapInfos.json")
                || n == "CommonEvents.json"
                || n == "Troops.json"
        })
        .collect();
    files.sort();
    let mut records = Vec::new();
    let mut counts: std::collections::BTreeMap<String, usize> = Default::default();
    let mut chars: std::collections::BTreeSet<char> = Default::default();
    for file in &files {
        let json = read(file);
        for message in messages_in(file, &json) {
            let lines: Vec<MessageLine> = message
                .lines
                .iter()
                .map(|(index, original)| {
                    let key = message.line_pointer(*index);
                    match targets.get(&(file.clone(), key)) {
                        Some(target) => MessageLine {
                            text: target.clone(),
                            writable: true,
                        },
                        None => MessageLine {
                            text: original.clone(),
                            writable: false,
                        },
                    }
                })
                .collect();
            let input = MessageInput {
                lines: lines.clone(),
                face: message.face,
            };
            let fitted = fit_message(&input, &ctx, &settings);
            let kind = match &fitted.outcome {
                FitOutcome::NewPage { .. } => "newPage".to_string(),
                FitOutcome::Shrunk { .. } => "shrunk".to_string(),
                FitOutcome::TooLong { reason } => format!("tooLong: {reason}"),
                FitOutcome::Skipped { reason } => format!("skipped: {reason}"),
                other => format!("{other:?}"),
            };
            *counts.entry(kind).or_default() += 1;
            let output: Vec<String> = lines
                .iter()
                .zip(&fitted.values)
                .map(|(line, value)| value.clone().unwrap_or_else(|| line.text.clone()))
                .collect();
            chars.extend(output.iter().flat_map(|t| t.chars()));
            records.push(serde_json::json!({
                "file": file,
                "pointer": message.pointer(),
                "face": message.face,
                "input": lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>(),
                "output": output,
                "outcome": fitted.outcome,
                "pages": fitted.pages.iter().map(|page| page.lines.iter().map(|line| {
                    serde_json::json!({
                        "text": line.runs.iter().map(|r| r.text.as_str()).collect::<String>(),
                        "right": line.right,
                        "overflow": line.overflow,
                    })
                }).collect::<Vec<_>>()).collect::<Vec<_>>(),
            }));
        }
    }
    chars.extend(inserts.actor_names.values().flat_map(|n| n.chars()));
    chars.extend("0".chars());
    let widths: serde_json::Map<String, serde_json::Value> = chars
        .iter()
        .map(|c| {
            (
                c.to_string(),
                serde_json::json!(metrics.char_width(*c, 1.0).0),
            )
        })
        .collect();
    eprintln!("== fitted {} message(s): {counts:#?}", records.len());
    std::fs::write(
        out,
        serde_json::to_vec_pretty(&serde_json::json!({
            "geometry": geometry,
            "actors": inserts.actor_names,
            "currency": inserts.currency,
            "widths": widths,
            "messages": records,
        }))
        .unwrap(),
    )
    .unwrap();
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
        let syntax = TokEngine::for_segment("mv_mz", kind, context.as_deref(), None);
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

/// String leaves that differ between two JSON values: `(pointer, a, b)`.
fn differing_strings(
    a: &serde_json::Value,
    b: &serde_json::Value,
    pointer: &str,
    out: &mut Vec<String>,
) {
    use serde_json::Value;
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for (key, value) in x {
                if let Some(other) = y.get(key) {
                    differing_strings(value, other, &format!("{pointer}/{key}"), out);
                }
            }
        }
        (Value::Array(x), Value::Array(y)) => {
            for (index, (value, other)) in x.iter().zip(y).enumerate() {
                differing_strings(value, other, &format!("{pointer}/{index}"), out);
            }
        }
        (Value::String(x), Value::String(y)) if x != y => out.push(pointer.to_string()),
        _ => {}
    }
}

/// Text fitting at export on real data: the original game (`H2S_MZ_GAME_DIR`,
/// `data/` + `js/`), the translations of a project database copy
/// (`H2S_FIT_DB`) and the game font (`H2S_FONT_FILE`). Exports with fitting
/// on, checks the gate, idempotency and that only dialogue/description
/// values differ from an unfitted export; writes the exported messages to
/// `H2S_FIT_EXPORT_OUT` for the engine harness.
#[tokio::test]
#[ignore = "requires a private game: set H2S_MZ_GAME_DIR, H2S_FIT_DB, H2S_FONT_FILE, H2S_FIT_EXPORT_OUT"]
async fn real_game_fit_export() {
    use hoshi2star_lib::commands::text_fit::{scan_text_fit, set_text_fit};
    use hoshi2star_lib::core::text_fit::TextFitSettings;
    use hoshi2star_lib::engines::mv_mz::layout::message::messages_in;
    use std::collections::HashMap;
    use std::str::FromStr;

    let vars = [
        "H2S_MZ_GAME_DIR",
        "H2S_FIT_DB",
        "H2S_FONT_FILE",
        "H2S_FIT_EXPORT_OUT",
    ]
    .map(std::env::var);
    let [Ok(source_game), Ok(db_path), Ok(font_file), Ok(out)] = vars else {
        eprintln!(
            "H2S_MZ_GAME_DIR / H2S_FIT_DB / H2S_FONT_FILE / H2S_FIT_EXPORT_OUT not set; skipping"
        );
        return;
    };
    let source_game = PathBuf::from(source_game);
    let temporary = tempfile::tempdir().unwrap();
    let game = temporary.path().join("game");
    copy_tree(&source_game.join("data"), &game.join("data"));
    copy_tree(&source_game.join("js"), &game.join("js"));
    let system: serde_json::Value =
        serde_json::from_slice(&std::fs::read(game.join("data/System.json")).unwrap()).unwrap();
    let font_name = system["advanced"]["mainFontFilename"]
        .as_str()
        .unwrap()
        .to_string();
    std::fs::create_dir_all(game.join("fonts")).unwrap();
    std::fs::copy(&font_file, game.join("fonts").join(&font_name)).unwrap();

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

    // The real translations, matched by file and pointer.
    let options =
        sqlx::sqlite::SqliteConnectOptions::from_str(&format!("sqlite://{db_path}?mode=ro"))
            .unwrap();
    let copy = sqlx::sqlite::SqlitePoolOptions::new()
        .connect_with(options)
        .await
        .unwrap();
    let real: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT f.file_name, s.json_key, s.target_text FROM segments s \
         JOIN source_files f ON f.id = s.source_file_id WHERE s.target_text != ''",
    )
    .fetch_all(&copy)
    .await
    .unwrap();
    let real: HashMap<(String, String), String> =
        real.into_iter().map(|(f, k, t)| ((f, k), t)).collect();
    let db = &app.state::<AppState>().db;
    let ours: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT s.id, f.file_name, s.json_key FROM segments s \
         JOIN source_files f ON f.id = s.source_file_id WHERE f.project_id = ?",
    )
    .bind(&project_id)
    .fetch_all(db)
    .await
    .unwrap();
    let mut matched = 0;
    for (id, file, key) in &ours {
        if let Some(target) = real.get(&(file.clone(), key.clone())) {
            sqlx::query("UPDATE segments SET target_text = ?, status = 'translated' WHERE id = ?")
                .bind(target)
                .bind(id)
                .execute(db)
                .await
                .unwrap();
            matched += 1;
        }
    }
    eprintln!("== translations matched: {matched} of {}", ours.len());
    let (summary, _) = hoshi2star_lib::core::report::preview_project(db, &project_id)
        .await
        .unwrap();
    eprintln!(
        "== QA with fresh segment kinds: {} critical; by type {:?}",
        summary.critical_count, summary.errors_by_type
    );
    // Mirror the source project: created before segment kinds existed, its
    // core segments are `unknown` without context.
    sqlx::query(
        "UPDATE segments SET segment_kind = 'unknown', context_json = NULL \
         WHERE text_category IS NULL AND source_file_id IN \
           (SELECT id FROM source_files WHERE project_id = ?)",
    )
    .bind(&project_id)
    .execute(db)
    .await
    .unwrap();

    let plain = match export_project(project_id.clone(), None, true, None, app.state()).await {
        Ok(path) => zip_entries(&path),
        Err(error) => {
            eprintln!("== export blocked before fitting: {error}");
            return;
        }
    };
    set_text_fit(
        project_id.clone(),
        TextFitSettings {
            enabled_dialogue: true,
            enabled_descriptions: true,
            ..TextFitSettings::default()
        },
        app.state(),
    )
    .await
    .unwrap();
    let scan = scan_text_fit(project_id.clone(), None, app.state())
        .await
        .unwrap();
    eprintln!("== dialogue {:?}", scan.dialogue);
    eprintln!("== descriptions {:?}", scan.descriptions);
    eprintln!("== missing glyphs {:?}", scan.missing_chars);
    for row in scan.rows.iter().take(12) {
        eprintln!(
            "   {} {} {:?} {:?}",
            row.file_name, row.pointer, row.outcome, row.sample
        );
    }

    let first = export_project(project_id.clone(), None, true, None, app.state())
        .await
        .expect("the fitted export passes QA and the integrity gate");
    let fitted = zip_entries(&first);
    let second = export_project(project_id.clone(), None, true, None, app.state())
        .await
        .unwrap();
    assert_eq!(
        zip_entries(&second),
        fitted,
        "fitted exports must be identical"
    );

    // Only dialogue lines and descriptions differ from the unfitted export.
    let line = regex::Regex::new(r"/list/\d+/parameters/0$").unwrap();
    let description = regex::Regex::new(r"^/\d+/(description|profile)$").unwrap();
    let mut changed = 0;
    for (name, bytes) in &fitted {
        if !name.ends_with(".json") {
            assert_eq!(Some(bytes), plain.get(name), "{name}");
            continue;
        }
        let Some(before) = plain.get(name) else {
            continue;
        };
        let a: serde_json::Value = serde_json::from_slice(before).unwrap();
        let b: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        let mut pointers = Vec::new();
        differing_strings(&a, &b, "", &mut pointers);
        for pointer in &pointers {
            assert!(
                line.is_match(pointer) || description.is_match(pointer),
                "{name} {pointer} changed"
            );
        }
        changed += pointers.len();
    }
    eprintln!("== values laid out differently: {changed}");

    // Exported messages for the engine harness.
    let mut records = Vec::new();
    for (name, bytes) in &fitted {
        let file = name.rsplit('/').next().unwrap_or(name);
        let json: serde_json::Value =
            serde_json::from_slice(bytes).unwrap_or(serde_json::Value::Null);
        for message in messages_in(file, &json) {
            records.push(serde_json::json!({
                "file": file,
                "pointer": message.pointer(),
                "face": message.face,
                "output": message.lines.iter().map(|(_, text)| text.clone()).collect::<Vec<_>>(),
            }));
        }
    }
    std::fs::write(out, serde_json::to_vec_pretty(&records).unwrap()).unwrap();
}
