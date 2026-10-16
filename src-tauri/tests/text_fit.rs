//! Text fitting (MZ): settings, scan, preview, per-text choices, QA.
//!
//! Uses the synthetic `mz_fit` fixture: a too-wide line, a portrait message
//! that needs a second page, a verbatim `……` line, a timed (`\|`) message,
//! a message followed by choices, and item descriptions. No font file ships
//! with it, so widths are estimated (13 px per Latin character at 26 px).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use hoshi2star_lib::commands::project::{get_segments, get_source_files, open_project};
use hoshi2star_lib::commands::text_fit::{
    get_text_fit, preview_text_fit, scan_text_fit, set_text_fit, set_text_fit_override,
};
use hoshi2star_lib::core::qa::QaError;
use hoshi2star_lib::core::report;
use hoshi2star_lib::core::text_fit::TextFitSettings;
use hoshi2star_lib::db;
use hoshi2star_lib::engines::mv_mz::layout::fit::FitOutcome;
use hoshi2star_lib::engines::mv_mz::layout::plan::FitOverride;
use hoshi2star_lib::state::AppState;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::Manager;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mv_mz/mz_fit")
}

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

const TRANSLATIONS: &[(&str, &str)] = &[
    (
        "長い台詞の一行目。",
        "This is a deliberately long first line of dialogue that will not fit in the box.",
    ),
    ("二行目。", "Second line."),
    ("一つ目の文。", "The first sentence of this rather talkative portrait message."),
    ("二つ目の文。", "The second sentence keeps going so that the box overflows."),
    ("三つ目の文。", "A third sentence follows, still long enough to need wrapping."),
    ("四つ目の文。", "And the fourth sentence finally ends this long message here."),
    ("長い前置き。", "A long introduction that certainly runs past the edge of the box, sadly."),
    ("そして。", "And then."),
    ("\\|時間の一。", "\\|The first timed line, which is long enough to be wrapped soon."),
    ("時間の二。", "The second timed line also carries plenty of words to wrap."),
    ("時間の三。", "The third timed line is here, making the message rather long."),
    (
        "時間の四。",
        "The fourth timed line ends it after a few more words than before.",
    ),
    ("どうする？", "What will you do?"),
    ("共通の台詞。", "Common line."),
    ("体力を少し回復する。", "Restores a little health."),
    (
        "体力を全回復する。\n戦闘中にも使える。",
        "Fully restores health and cures every ailment that the party might suffer from.\nCan also be used in battle.",
    ),
    ("若き魔術師。\n禁断の魔術を追う。", "A young magician.\nShe chases forbidden magic."),
];

struct Opened {
    _temporary: tempfile::TempDir,
    app: tauri::App<tauri::test::MockRuntime>,
    project_id: String,
}

/// `(segment id, file name, source)` of every segment.
async fn segments(opened: &Opened) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for file in get_source_files(opened.project_id.clone(), opened.app.state())
        .await
        .unwrap()
    {
        let page = get_segments(
            opened.project_id.clone(),
            file.id.clone(),
            0,
            100_000,
            opened.app.state(),
        )
        .await
        .unwrap();
        for segment in page.items {
            out.push((segment.id, file.file_name.clone(), segment.source_text));
        }
    }
    out
}

/// Open the fixture and translate every segment (known lines from
/// `TRANSLATIONS`, everything else with a short English stand-in).
async fn opened() -> Opened {
    let temporary = tempfile::tempdir().unwrap();
    let game = temporary.path().join("game");
    copy_tree(&fixture(), &game);
    let pool = db::pool::init(temporary.path().join("h2s.db").to_str().unwrap())
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
    .expect("open fixture")
    .project
    .id;
    let opened = Opened {
        _temporary: temporary,
        app,
        project_id,
    };
    let known: HashMap<&str, &str> = TRANSLATIONS.iter().copied().collect();
    let db = &opened.app.state::<AppState>().db;
    // Same source → same stand-in (repeated sources must stay consistent).
    let mut stand_ins: HashMap<String, String> = HashMap::new();
    for (id, _, source) in segments(&opened).await {
        let next = stand_ins.len();
        let target = match known.get(source.as_str()) {
            Some(text) => text.to_string(),
            None => stand_ins
                .entry(source.clone())
                .or_insert_with(|| format!("Text {next}"))
                .clone(),
        };
        sqlx::query("UPDATE segments SET target_text = ?, status = 'translated' WHERE id = ?")
            .bind(target)
            .bind(id)
            .execute(db)
            .await
            .unwrap();
    }
    opened
}

async fn segment_id(opened: &Opened, source: &str) -> String {
    segments(opened)
        .await
        .into_iter()
        .find(|(_, _, s)| s == source)
        .unwrap_or_else(|| panic!("no segment {source}"))
        .0
}

fn enabled() -> TextFitSettings {
    TextFitSettings {
        enabled_dialogue: true,
        enabled_descriptions: true,
        ..TextFitSettings::default()
    }
}

#[tokio::test]
async fn detects_the_box_and_scans_every_text() {
    let opened = opened().await;
    let view = get_text_fit(opened.project_id.clone(), opened.app.state())
        .await
        .unwrap();
    assert!(view.supported);
    assert_eq!(view.settings, TextFitSettings::default());
    let geometry = view.geometry.unwrap();
    assert_eq!(geometry.message.text_width(false), 776.0);
    assert_eq!(geometry.message.text_width(true), 616.0);
    let font = view.font.unwrap();
    assert!(!font.loaded, "no font file ships with the fixture");
    assert!(font.error.is_some());

    let scan = scan_text_fit(opened.project_id.clone(), None, opened.app.state())
        .await
        .unwrap();
    // Map001: A rewrapped, B new page, C rewrapped, D shrunk, E fits; plus
    // the common event (fits).
    assert_eq!(scan.dialogue.total, 6);
    assert_eq!(scan.dialogue.fits, 2);
    assert_eq!(scan.dialogue.rewrapped, 2);
    assert_eq!(scan.dialogue.new_page, 1);
    assert_eq!(scan.dialogue.shrunk, 1);
    assert_eq!(scan.descriptions.total, 3);
    assert_eq!(scan.descriptions.fits, 2);
    assert_eq!(scan.descriptions.rewrapped, 1);
    let outcomes: Vec<&FitOutcome> = scan.rows.iter().map(|row| &row.outcome).collect();
    assert!(outcomes
        .iter()
        .any(|o| matches!(o, FitOutcome::NewPage { pages: 2 })));
    assert!(outcomes
        .iter()
        .any(|o| matches!(o, FitOutcome::Shrunk { .. })));
    assert!(scan.rows.iter().all(|row| row.segment_id.is_some()));
}

#[tokio::test]
async fn settings_round_trip_and_preview_follows_drafts() {
    let opened = opened().await;
    set_text_fit(opened.project_id.clone(), enabled(), opened.app.state())
        .await
        .unwrap();
    let view = get_text_fit(opened.project_id.clone(), opened.app.state())
        .await
        .unwrap();
    assert_eq!(view.settings, enabled());
    let mut invalid = enabled();
    invalid.min_font_size = 2;
    assert!(
        set_text_fit(opened.project_id.clone(), invalid, opened.app.state())
            .await
            .is_err()
    );

    let line = segment_id(&opened, "長い台詞の一行目。").await;
    let preview = preview_text_fit(
        opened.project_id.clone(),
        line.clone(),
        None,
        opened.app.state(),
    )
    .await
    .unwrap()
    .expect("a message preview");
    assert!(preview.enabled);
    assert_eq!(preview.item.fitted.outcome, FitOutcome::Rewrapped);
    assert_eq!(preview.text_x, 4.0);
    assert_eq!(preview.limit, 780.0);
    assert_eq!(preview.item.fitted.pages.len(), 1);

    let short = preview_text_fit(
        opened.project_id.clone(),
        line,
        Some("Short now.".into()),
        opened.app.state(),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(short.item.fitted.outcome, FitOutcome::Fits);

    // Choices are not fitted: no preview.
    let choice = segment_id(&opened, "はい").await;
    assert!(
        preview_text_fit(opened.project_id.clone(), choice, None, opened.app.state())
            .await
            .unwrap()
            .is_none()
    );

    // Descriptions use the help box.
    let item = segment_id(&opened, "体力を全回復する。\n戦闘中にも使える。").await;
    let description = preview_text_fit(opened.project_id.clone(), item, None, opened.app.state())
        .await
        .unwrap()
        .unwrap();
    assert!(description.item.description);
    assert_eq!(description.text_x, 8.0);
    assert_eq!(description.limit, 776.0);
}

#[tokio::test]
async fn a_message_can_be_left_as_translated() {
    let opened = opened().await;
    let line = segment_id(&opened, "一つ目の文。").await;
    let disabled = FitOverride {
        disabled: true,
        ..FitOverride::default()
    };
    let preview = set_text_fit_override(
        opened.project_id.clone(),
        line.clone(),
        Some(disabled),
        opened.app.state(),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(matches!(
        preview.item.fitted.outcome,
        FitOutcome::Skipped { .. }
    ));
    assert!(preview.item.choice.as_ref().is_some_and(|c| c.disabled));
    let scan = scan_text_fit(opened.project_id.clone(), None, opened.app.state())
        .await
        .unwrap();
    assert_eq!(scan.dialogue.new_page, 0);
    assert_eq!(scan.dialogue.skipped, 1);

    let cleared = set_text_fit_override(opened.project_id.clone(), line, None, opened.app.state())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        cleared.item.fitted.outcome,
        FitOutcome::NewPage { pages: 2 }
    ));
    assert!(cleared.item.choice.is_none());
}

#[tokio::test]
async fn line_width_qa_is_dropped_only_for_fitted_texts() {
    let opened = opened().await;
    let line = segment_id(&opened, "長い台詞の一行目。").await;
    let db = &opened.app.state::<AppState>().db;
    let long_line_errors = |details: &[report::QaSegmentDetail]| {
        details
            .iter()
            .find(|d| d.segment_id == line)
            .map_or(0, |d| {
                d.errors
                    .iter()
                    .filter(|e| matches!(e, QaError::LineTooLong { .. }))
                    .count()
            })
    };
    let (_, before) = report::preview_project(db, &opened.project_id)
        .await
        .unwrap();
    assert!(long_line_errors(&before) > 0, "the raw line is too wide");

    set_text_fit(opened.project_id.clone(), enabled(), opened.app.state())
        .await
        .unwrap();
    let (_, after) = report::preview_project(db, &opened.project_id)
        .await
        .unwrap();
    assert_eq!(long_line_errors(&after), 0, "fitting lays it out at export");
}

// ---------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------

fn zip_entries(zip_path: &str) -> std::collections::BTreeMap<String, Vec<u8>> {
    use std::io::Read;
    let mut archive = zip::ZipArchive::new(std::fs::File::open(zip_path).unwrap()).unwrap();
    let mut entries = std::collections::BTreeMap::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).unwrap();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        entries.insert(entry.name().to_string(), bytes);
    }
    entries
}

fn zip_json(zip_path: &str, entry: &str) -> serde_json::Value {
    serde_json::from_slice(&zip_entries(zip_path)[entry]).unwrap()
}

/// Text of command `index` of the fixture's only event list.
fn line(map: &serde_json::Value, index: usize) -> String {
    map["events"][1]["pages"][0]["list"][index]["parameters"][0]
        .as_str()
        .unwrap()
        .to_string()
}

fn words(text: &str) -> Vec<String> {
    text.replace('\u{000C}', " ")
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

fn translation(source: &str) -> &'static str {
    TRANSLATIONS
        .iter()
        .find(|(s, _)| *s == source)
        .map(|(_, t)| *t)
        .unwrap()
}

async fn export(opened: &Opened, font_size: Option<u32>) -> String {
    export_with(opened, font_size, true).await
}

async fn export_with(opened: &Opened, font_size: Option<u32>, replace_existing: bool) -> String {
    hoshi2star_lib::commands::export::export_project(
        opened.project_id.clone(),
        font_size,
        replace_existing,
        None,
        opened.app.state(),
    )
    .await
    .expect("export passes QA and the integrity gate")
}

#[tokio::test]
async fn export_lays_out_dialogue_and_descriptions_only_when_enabled() {
    let opened = opened().await;
    let plain = zip_json(&export(&opened, None).await, "data/Map001.json");
    assert_eq!(line(&plain, 1), translation("長い台詞の一行目。"));

    set_text_fit(opened.project_id.clone(), enabled(), opened.app.state())
        .await
        .unwrap();
    let zip = export(&opened, None).await;
    let map = zip_json(&zip, "data/Map001.json");
    // A: rewrapped inside its own two commands, words unchanged.
    let a = format!("{}\n{}", line(&map, 1), line(&map, 2));
    assert_ne!(line(&map, 1), translation("長い台詞の一行目。"));
    assert_eq!(
        words(&a),
        words(&format!(
            "{} {}",
            translation("長い台詞の一行目。"),
            translation("二行目。")
        ))
    );
    // B (portrait): continues on a second page.
    let b: Vec<String> = (4..=7).map(|i| line(&map, i)).collect();
    assert_eq!(b.join("\n").matches('\u{000C}').count(), 1);
    // C: the verbatim line stays as it is.
    assert_eq!(line(&map, 10), "……");
    // D (timed): shrunk, never paged.
    assert!(line(&map, 13).starts_with("\\FS["));
    let d: String = (13..=16).map(|i| line(&map, i)).collect();
    assert!(!d.contains('\u{000C}'));
    // E: fits as translated.
    assert_eq!(line(&map, 18), "What will you do?");
    // A description rewrapped into the 2-line help box.
    let items = zip_json(&zip, "data/Items.json");
    let description = items[2]["description"].as_str().unwrap();
    assert_eq!(description.lines().count(), 2);
    assert_eq!(
        words(description),
        words(translation("体力を全回復する。\n戦闘中にも使える。"))
    );
    // Exporting again gives the same archive.
    assert_eq!(zip_entries(&export(&opened, None).await), zip_entries(&zip));
}

#[tokio::test]
async fn export_font_size_is_emitted_once_per_page_for_fitted_dialogue() {
    let opened = opened().await;
    let mut settings = enabled();
    settings.enabled_descriptions = false;
    set_text_fit(opened.project_id.clone(), settings, opened.app.state())
        .await
        .unwrap();
    let map = zip_json(&export(&opened, Some(22)).await, "data/Map001.json");
    assert!(line(&map, 1).starts_with("\\FS[22]"));
    assert!(!line(&map, 2).starts_with("\\FS["), "not a page start");
    assert!(line(&map, 18).starts_with("\\FS[22]"));
    // Choices are not fitted: the per-line prefix applies as before.
    let choice = map["events"][1]["pages"][0]["list"][19]["parameters"][0][0]
        .as_str()
        .unwrap()
        .to_string();
    assert!(choice.starts_with("\\FS[22]"), "{choice}");
}

#[tokio::test]
async fn a_message_turned_off_exports_as_translated() {
    let opened = opened().await;
    set_text_fit(opened.project_id.clone(), enabled(), opened.app.state())
        .await
        .unwrap();
    let first = segment_id(&opened, "一つ目の文。").await;
    set_text_fit_override(
        opened.project_id.clone(),
        first,
        Some(FitOverride {
            disabled: true,
            ..FitOverride::default()
        }),
        opened.app.state(),
    )
    .await
    .unwrap();
    let map = zip_json(&export(&opened, None).await, "data/Map001.json");
    assert_eq!(line(&map, 4), translation("一つ目の文。"));
    assert_eq!(line(&map, 7), translation("四つ目の文。"));
}

#[tokio::test]
async fn a_message_left_as_translated_keeps_the_export_font_prefix() {
    let opened = opened().await;
    set_text_fit(opened.project_id.clone(), enabled(), opened.app.state())
        .await
        .unwrap();
    let first = segment_id(&opened, "一つ目の文。").await;
    set_text_fit_override(
        opened.project_id.clone(),
        first,
        Some(FitOverride {
            disabled: true,
            ..FitOverride::default()
        }),
        opened.app.state(),
    )
    .await
    .unwrap();
    // Not fitted → exported like any map line: one prefix per line.
    let map = zip_json(&export(&opened, Some(22)).await, "data/Map001.json");
    for (index, source) in [(4, "一つ目の文。"), (7, "四つ目の文。")] {
        assert_eq!(
            line(&map, index),
            format!("\\FS[22]{}", translation(source))
        );
    }
}

#[tokio::test]
async fn a_translators_font_code_survives_fitting() {
    let opened = opened().await;
    set_text_fit(opened.project_id.clone(), enabled(), opened.app.state())
        .await
        .unwrap();
    let second = segment_id(&opened, "二行目。").await;
    let db = &opened.app.state::<AppState>().db;
    sqlx::query("UPDATE segments SET target_text = ? WHERE id = ?")
        .bind("\\FS[20]Second line.")
        .bind(&second)
        .execute(db)
        .await
        .unwrap();
    let message = |map: &serde_json::Value| format!("{}\n{}", line(map, 1), line(map, 2));

    // Kept (and laid out) unless the export replaces stored font codes.
    let kept = zip_json(&export_with(&opened, None, false).await, "data/Map001.json");
    assert!(
        message(&kept).contains("\\FS[20]Second"),
        "{}",
        message(&kept)
    );
    let kept = zip_json(
        &export_with(&opened, Some(22), false).await,
        "data/Map001.json",
    );
    assert!(
        message(&kept).contains("\\FS[20]Second"),
        "{}",
        message(&kept)
    );
    // "Replace existing" treats it as an older export's prefix, as for any row.
    let replaced = zip_json(
        &export_with(&opened, Some(22), true).await,
        "data/Map001.json",
    );
    assert!(
        !message(&replaced).contains("\\FS[20]"),
        "{}",
        message(&replaced)
    );
    assert!(message(&replaced).starts_with("\\FS[22]"));
}

#[tokio::test]
async fn fitting_cannot_be_enabled_where_export_cannot_fit() {
    let opened = opened().await;
    // Without the MZ core script this looks like an MV game.
    std::fs::remove_file(opened._temporary.path().join("game/js/rmmz_core.js")).unwrap();
    let error = set_text_fit(opened.project_id.clone(), enabled(), opened.app.state())
        .await
        .unwrap_err();
    assert!(error.contains("MZ"), "{error}");
    // Turning it off is always possible.
    set_text_fit(
        opened.project_id.clone(),
        TextFitSettings::default(),
        opened.app.state(),
    )
    .await
    .unwrap();
}
