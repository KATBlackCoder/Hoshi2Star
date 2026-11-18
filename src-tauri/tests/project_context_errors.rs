//! Characterisation of what the project commands answer for a project that
//! does not exist, and for a project whose engine this build does not know
//! (plan DD, L5-1). The messages are shown to the user, and the front reads
//! some of them (`src/lib/projectError.ts`): the shared project loading of L5
//! must leave every one of them unchanged.
//!
//! To write the expected output again (only on purpose):
//! `H2S_BLESS_CORPUS=1 cargo test --test project_context_errors`.

use std::path::{Path, PathBuf};

use hoshi2star_lib::commands::coverage::{
    get_text_coverage_status, scan_text_coverage, set_text_categories,
};
use hoshi2star_lib::commands::export::{
    export_debug_json, export_debug_project, export_project, export_qa_report, preflight_export,
    scan_font_status, strip_font_prefixes,
};
use hoshi2star_lib::commands::pack::{export_h2s_pack, preview_h2s_import};
use hoshi2star_lib::commands::qa::{get_qa_report, qa_check_segment};
use hoshi2star_lib::commands::resync::resync_project;
use hoshi2star_lib::commands::text_fit::{
    get_game_font, get_text_fit, preview_text_fit, scan_text_fit, set_text_fit,
};
use hoshi2star_lib::core::text_fit::TextFitSettings;
use hoshi2star_lib::db;
use hoshi2star_lib::state::AppState;
use serde_json::{json, Value};
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::Manager;

type App = tauri::App<tauri::test::MockRuntime>;

fn expected_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/project_context_errors.json")
}

/// `Ok` as its JSON (temporary paths replaced), `Err` as its message.
fn outcome<T: serde::Serialize>(result: Result<T, String>, tmp: &Path) -> Value {
    match result {
        Ok(value) => {
            let text = serde_json::to_string(&value)
                .unwrap()
                .replace(&*tmp.to_string_lossy(), "<tmp>");
            json!({ "ok": serde_json::from_str::<Value>(&text).unwrap() })
        }
        Err(error) => json!({ "err": error.replace(&*tmp.to_string_lossy(), "<tmp>") }),
    }
}

async fn answers(app: &App, project_id: &str, tmp: &Path) -> Value {
    let id = || project_id.to_string();
    let out = |name: &str| tmp.join(name).to_string_lossy().into_owned();
    let settings = TextFitSettings::default();
    json!({
        "get_text_fit": outcome(get_text_fit(id(), app.state()).await, tmp),
        "scan_text_fit": outcome(scan_text_fit(id(), None, app.state()).await, tmp),
        "set_text_fit": outcome(set_text_fit(id(), settings, app.state()).await, tmp),
        "preview_text_fit": outcome(
            preview_text_fit(id(), "no-segment".into(), None, app.state()).await,
            tmp
        ),
        "get_game_font": outcome(get_game_font(id(), app.state()).await.map(|_| "bytes"), tmp),
        "preflight_export": outcome(preflight_export(id(), app.state()).await, tmp),
        "export_project": outcome(export_project(id(), None, false, None, app.state()).await, tmp),
        "scan_font_status": outcome(scan_font_status(None, Some(id()), app.state()).await, tmp),
        "strip_font_prefixes": outcome(strip_font_prefixes(id(), app.state()).await, tmp),
        "export_debug_json": outcome(export_debug_json(id(), app.state()).await, tmp),
        "export_debug_project": outcome(export_debug_project(id(), app.state()).await, tmp),
        "export_qa_report": outcome(
            export_qa_report(id(), out("qa.html"), "en".into(), app.state()).await,
            tmp
        ),
        "get_text_coverage_status": outcome(get_text_coverage_status(id(), app.state()).await, tmp),
        "scan_text_coverage": outcome(scan_text_coverage(id(), app.state()).await, tmp),
        "set_text_categories": outcome(
            set_text_categories(id(), Vec::new(), app.state()).await,
            tmp
        ),
        "resync_project": outcome(
            resync_project(id(), false, app.handle().clone(), app.state()).await,
            tmp
        ),
        "export_h2s_pack": outcome(
            export_h2s_pack(id(), out("p.h2s"), false, "ja-en".into(), app.state()).await,
            tmp
        ),
        "preview_h2s_import": outcome(
            preview_h2s_import(id(), out("missing.h2s"), "ja-en".into(), app.state()).await,
            tmp
        ),
        "get_qa_report": outcome(get_qa_report(id(), app.state()).await, tmp),
        "qa_check_segment": outcome(
            qa_check_segment(
                "テスト".into(),
                "Test".into(),
                None,
                Some(id()),
                None,
                app.state()
            )
            .await,
            tmp
        ),
    })
}

#[tokio::test]
async fn project_commands_answer_unknown_projects_and_engines_the_same() {
    let temporary = tempfile::tempdir().unwrap();
    let tmp = temporary.path();
    let pool = db::pool::init(tmp.join("h2s.db").to_str().unwrap())
        .await
        .unwrap();
    let app = mock_builder()
        .manage(AppState::new(pool, tmp.join("app-data")).unwrap())
        .build(mock_context(noop_assets()))
        .unwrap();

    // A project of an engine this build does not know (a future engine, or a
    // database written by a newer version), with an empty game folder.
    let game = tmp.join("game");
    std::fs::create_dir_all(&game).unwrap();
    sqlx::query(
        "INSERT INTO projects (id, name, engine, game_path, source_language, target_language) \
         VALUES ('future', 'Future', 'future_engine', ?, 'ja', 'en')",
    )
    .bind(game.to_string_lossy().into_owned())
    .execute(&app.state::<AppState>().db)
    .await
    .unwrap();

    let actual = json!({
        "unknown_project": answers(&app, "no-such-project", tmp).await,
        "unknown_engine": answers(&app, "future", tmp).await,
    });
    let text = serde_json::to_string_pretty(&actual).unwrap() + "\n";
    let path = expected_path();
    if std::env::var_os("H2S_BLESS_CORPUS").is_some() {
        std::fs::write(&path, text).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e} (bless with H2S_BLESS_CORPUS=1)", path.display()));
    assert_eq!(
        serde_json::from_str::<Value>(&expected).unwrap(),
        actual,
        "the project commands answer differently (expected: {})",
        path.display()
    );
}
