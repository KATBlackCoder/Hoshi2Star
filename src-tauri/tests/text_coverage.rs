//! Optional text categories (MV/MZ): scan → decide → translate → export.
//!
//! Uses the synthetic `mz_coverage` fixture, which deliberately contains a
//! note tag that looks a skill type up by name, an enabled plugin parameter
//! that mentions an element name, a disabled plugin, a script comparing an
//! actor name and a map display name.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use hoshi2star_lib::commands::coverage::{
    scan_text_coverage, set_text_categories, CategoryChange, CoverageView,
};
use hoshi2star_lib::commands::export::{export_project, preflight_export};
use hoshi2star_lib::commands::project::{
    get_segments, get_source_files, open_project, update_segment,
};
use hoshi2star_lib::db;
use hoshi2star_lib::engines::mv_mz::coverage::CategoryState;
use hoshi2star_lib::engines::mv_mz::profiles::ProfileSet;
use hoshi2star_lib::state::AppState;
use serde_json::Value;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::Manager;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mv_mz/mz_coverage")
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

struct Opened {
    _temporary: tempfile::TempDir,
    app: tauri::App<tauri::test::MockRuntime>,
    project_id: String,
}

async fn opened() -> Opened {
    opened_with(None).await
}

/// Profile for the fixture's synthetic Keke stand-in (pinned to its test
/// version header), enabling the command-tag grammar.
const KEKE_TEST_PROFILE: &str = r#"
    [plugin]
    name = "keke/Keke_VariableActorCommand"
    title = "Variable actor commands (test stand-in)"
    verified_versions = ["h2s-test-1"]
    reference = "synthetic"
    note_tag_families = ["keke_command"]

    [[display]]
    path = "用語・自動戦闘"
    visible_in = "Battle auto-battle command"
"#;

async fn opened_with(profiles: Option<ProfileSet>) -> Opened {
    let temporary = tempfile::tempdir().unwrap();
    let game = temporary.path().join("game");
    copy_tree(&fixture(), &game);
    let db_path = temporary.path().join("h2s.db");
    let pool = db::pool::init(db_path.to_str().unwrap()).await.unwrap();
    let mut state = AppState::new(pool, temporary.path().join("app-data")).unwrap();
    if let Some(profiles) = profiles {
        state.plugin_profiles = std::sync::Arc::new(profiles);
    }
    let app = mock_builder()
        .manage(state)
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
    Opened {
        _temporary: temporary,
        app,
        project_id,
    }
}

fn state_of(view: &CoverageView, key: &str) -> CategoryState {
    view.categories
        .iter()
        .find(|c| c.key == key)
        .unwrap_or_else(|| panic!("missing category {key}"))
        .state
}

fn change(key: &str, enabled: bool) -> CategoryChange {
    CategoryChange {
        category_key: key.into(),
        enabled,
        accept_consequences: false,
    }
}

/// `(json_key, source, text_category)` of every segment, by file name.
async fn segments_by_file(
    opened: &Opened,
) -> BTreeMap<String, Vec<(String, String, String, Option<String>)>> {
    let mut out = BTreeMap::new();
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
        out.insert(
            file.file_name.clone(),
            page.items
                .into_iter()
                .map(|s| (s.id, s.json_key, s.source_text, s.text_category))
                .collect(),
        );
    }
    out
}

fn zip_json(zip_path: &str, entry: &str) -> Value {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(zip_path).unwrap()).unwrap();
    let mut bytes = Vec::new();
    archive
        .by_name(entry)
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn scan_classifies_categories_by_reference_safety() {
    let opened = opened().await;
    let view = scan_text_coverage(opened.project_id.clone(), opened.app.state())
        .await
        .unwrap();

    assert!(view.supported);
    assert!(!view.has_decisions);
    assert!(view.plugins_unreadable.is_none());
    // Looked up by name in a Weapons note tag.
    assert_eq!(
        state_of(&view, "system.skill_types"),
        CategoryState::Blocked
    );
    // Mentioned by an enabled plugin's parameter (the disabled plugin and the
    // editor-only comment are not references).
    assert_eq!(state_of(&view, "system.elements"), CategoryState::Blocked);
    let elements = view
        .categories
        .iter()
        .find(|c| c.key == "system.elements")
        .unwrap();
    assert_eq!(elements.reference_count, 1);
    assert_eq!(
        elements.references[0].location,
        "Sample/MemoPlugin::PictureList.0.Name"
    );
    assert_eq!(
        state_of(&view, "system.equip_types"),
        CategoryState::Verified
    );
    assert_eq!(
        state_of(&view, "system.armor_types"),
        CategoryState::Verified
    );
    assert_eq!(
        state_of(&view, "system.weapon_types"),
        CategoryState::Verified
    );
    assert_eq!(state_of(&view, "map.display_name"), CategoryState::Verified);
    // The actor name is compared in a script: a non-blocking warning.
    assert!(view.core_warnings.iter().any(|w| w.source == "恵"));
    // Scanning never inserts segments.
    let segments = segments_by_file(&opened).await;
    assert!(segments.values().flatten().all(|s| s.3.is_none()));
}

#[tokio::test]
async fn blocked_categories_cannot_be_enabled() {
    let opened = opened().await;
    let error = set_text_categories(
        opened.project_id.clone(),
        vec![change("system.skill_types", true)],
        opened.app.state(),
    )
    .await
    .unwrap_err();
    assert!(error.contains("cannot be enabled"), "{error}");
    let segments = segments_by_file(&opened).await;
    assert!(segments.values().flatten().all(|s| s.3.is_none()));
}

#[tokio::test]
async fn locale_switch_is_previewed_and_applied_only_on_request() {
    let opened = opened().await;
    let option = preflight_export(opened.project_id.clone(), opened.app.state())
        .await
        .unwrap()
        .locale
        .expect("an MZ game with a language setting");
    assert_eq!(
        (option.current.as_str(), option.target.as_str()),
        ("ja_JP", "en_US")
    );
    let locations: Vec<_> = option.uses.iter().map(|u| u.location.as_str()).collect();
    assert_eq!(locations, vec!["js/rmmz_objects.js:7"]);
    assert!(option.consequences.iter().any(|c| c.contains("half-width")));

    // Off by default: nothing is translated, so System.json is not exported.
    let zip = export_project(
        opened.project_id.clone(),
        None,
        true,
        None,
        opened.app.state(),
    )
    .await
    .unwrap();
    let archive = zip::ZipArchive::new(std::fs::File::open(&zip).unwrap()).unwrap();
    assert!(archive.index_for_name("data/System.json").is_none());

    // On request: exactly one value changes, even with no translation at all.
    let zip = export_project(
        opened.project_id.clone(),
        None,
        true,
        Some(true),
        opened.app.state(),
    )
    .await
    .unwrap();
    let mut expected: Value =
        serde_json::from_slice(&std::fs::read(fixture().join("data/System.json")).unwrap())
            .unwrap();
    expected["locale"] = Value::String("en_US".into());
    assert_eq!(zip_json(&zip, "data/System.json"), expected);
}

#[tokio::test]
async fn enabled_categories_translate_export_and_park_round_trip() {
    let opened = opened().await;
    let view = set_text_categories(
        opened.project_id.clone(),
        vec![
            change("system.equip_types", true),
            change("map.display_name", true),
        ],
        opened.app.state(),
    )
    .await
    .unwrap();
    assert!(view.has_decisions);
    assert!(
        view.categories
            .iter()
            .find(|c| c.key == "map.display_name")
            .unwrap()
            .enabled
    );

    let segments = segments_by_file(&opened).await;
    let system: Vec<_> = segments["System.json"]
        .iter()
        .filter(|s| s.3.as_deref() == Some("system.equip_types"))
        .collect();
    assert_eq!(
        system.iter().map(|s| s.1.as_str()).collect::<Vec<_>>(),
        vec!["/equipTypes/1", "/equipTypes/2"]
    );
    assert!(segments["Map001.json"]
        .iter()
        .any(|s| s.1 == "/displayName"));

    // Translate everything that the export audit will see.
    for (id, _, source, _) in segments.values().flatten() {
        let target = match source.as_str() {
            "武器" => "Weapon".to_string(),
            "衣装" => "Outfit".to_string(),
            "始まりの村" => "Starting Village".to_string(),
            "%1は%2のダメージを受けた！" => "%1 took %2 damage!".to_string(),
            other => format!("EN[{}]", other.chars().count()),
        };
        update_segment(id.clone(), target, opened.app.state())
            .await
            .unwrap();
    }

    let zip = export_project(
        opened.project_id.clone(),
        Some(22),
        true,
        None,
        opened.app.state(),
    )
    .await
    .expect("export");
    let system_json = zip_json(&zip, "data/System.json");
    assert_eq!(
        system_json["equipTypes"],
        serde_json::json!(["", "Weapon", "Outfit"])
    );
    // Untouched lists stay exactly as they were.
    assert_eq!(
        system_json["skillTypes"],
        serde_json::json!(["", "魔術", "鞭"])
    );
    let map = zip_json(&zip, "data/Map001.json");
    // The map name popup draws plain text: never font-prefixed.
    assert_eq!(map["displayName"], "Starting Village");
    assert!(map["events"][1]["pages"][0]["list"][1]["parameters"][0]
        .as_str()
        .unwrap()
        .starts_with(r"\FS[22]"));

    // Disabling parks the translation; re-enabling restores it.
    set_text_categories(
        opened.project_id.clone(),
        vec![change("map.display_name", false)],
        opened.app.state(),
    )
    .await
    .unwrap();
    let after_disable = segments_by_file(&opened).await;
    assert!(after_disable["Map001.json"]
        .iter()
        .all(|s| s.1 != "/displayName"));
    set_text_categories(
        opened.project_id.clone(),
        vec![change("map.display_name", true)],
        opened.app.state(),
    )
    .await
    .unwrap();
    let restored = get_segments(
        opened.project_id.clone(),
        get_source_files(opened.project_id.clone(), opened.app.state())
            .await
            .unwrap()
            .into_iter()
            .find(|f| f.file_name == "Map001.json")
            .unwrap()
            .id,
        0,
        100,
        opened.app.state(),
    )
    .await
    .unwrap()
    .items
    .into_iter()
    .find(|s| s.json_key == "/displayName")
    .unwrap();
    assert_eq!(restored.target_text, "Starting Village");
    assert_eq!(restored.text_category.as_deref(), Some("map.display_name"));
}

async fn opened_with_keke() -> Opened {
    opened_with(Some(
        ProfileSet::parse_all(&[("keke.toml", KEKE_TEST_PROFILE)]).unwrap(),
    ))
    .await
}

/// Translate every segment: `targets` for known sources, a short placeholder
/// otherwise (same source → same target, so QA consistency holds).
async fn translate_all(opened: &Opened, targets: &[(&str, &str)]) {
    for (id, _, source, _) in segments_by_file(opened).await.values().flatten() {
        let target = targets
            .iter()
            .find(|(from, _)| from == source)
            .map(|(_, to)| to.to_string())
            .unwrap_or_else(|| match source.as_str() {
                "%1は%2のダメージを受けた！" => "%1 took %2 damage!".to_string(),
                other => format!("EN[{}]", other.chars().count()),
            });
        update_segment(id.clone(), target, opened.app.state())
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn keke_tags_make_skill_types_linked_and_labels_translatable() {
    let opened = opened_with_keke().await;
    let view = scan_text_coverage(opened.project_id.clone(), opened.app.state())
        .await
        .unwrap();

    // The only references to skill-type names are command-tag lookups.
    assert_eq!(state_of(&view, "system.skill_types"), CategoryState::Linked);
    let skills = view
        .categories
        .iter()
        .find(|c| c.key == "system.skill_types")
        .unwrap();
    assert_eq!(skills.references[0].location, "Weapons.json /1/note");
    assert!(skills.consequences.iter().any(|c| c.contains("rewrites")));
    let labels = view
        .categories
        .iter()
        .find(|c| c.key == "notetag.keke_command.labels")
        .expect("label category");
    assert_eq!(labels.samples, vec!["鞭攻撃"]);
    assert_eq!(
        state_of(&view, "plugin.keke/Keke_VariableActorCommand"),
        CategoryState::Verified
    );

    // Linked categories need explicit acceptance.
    let error = set_text_categories(
        opened.project_id.clone(),
        vec![change("system.skill_types", true)],
        opened.app.state(),
    )
    .await
    .unwrap_err();
    assert!(error.contains("consequences"), "{error}");
}

#[tokio::test]
async fn linked_skill_types_rewrite_every_tag_and_pass_the_gate() {
    let opened = opened_with_keke().await;
    set_text_categories(
        opened.project_id.clone(),
        vec![
            CategoryChange {
                category_key: "system.skill_types".into(),
                enabled: true,
                accept_consequences: true,
            },
            change("notetag.keke_command.labels", true),
        ],
        opened.app.state(),
    )
    .await
    .unwrap();
    translate_all(
        &opened,
        &[("魔術", "Magic"), ("鞭", "Whip"), ("鞭攻撃", "WhipAttack")],
    )
    .await;

    let zip = export_project(
        opened.project_id.clone(),
        None,
        true,
        None,
        opened.app.state(),
    )
    .await
    .expect("linked export passes QA and the Keke resolution check");
    let system = zip_json(&zip, "data/System.json");
    assert_eq!(
        system["skillTypes"],
        serde_json::json!(["", "Magic", "Whip"])
    );
    let weapons = zip_json(&zip, "data/Weapons.json");
    assert_eq!(
        weapons[1]["note"],
        "<コマンド:スキル/Whip,48,WhipAttack,76>"
    );
}

#[tokio::test]
async fn invalid_or_duplicate_skill_type_names_block_the_export() {
    for (magic, whip, expected_rule) in [
        ("Dark Arts", "Whip", "keke_skill_type"),
        ("Whip", "Whip", "keke_skill_type_unique"),
    ] {
        let opened = opened_with_keke().await;
        set_text_categories(
            opened.project_id.clone(),
            vec![CategoryChange {
                category_key: "system.skill_types".into(),
                enabled: true,
                accept_consequences: true,
            }],
            opened.app.state(),
        )
        .await
        .unwrap();
        translate_all(&opened, &[("魔術", magic), ("鞭", whip)]).await;
        let error = export_project(
            opened.project_id.clone(),
            None,
            true,
            None,
            opened.app.state(),
        )
        .await
        .unwrap_err();
        assert!(error.contains("QA"), "{expected_rule}: {error}");
        let report = hoshi2star_lib::core::report::preview_project(
            &opened.app.state::<AppState>().db,
            &opened.project_id,
        )
        .await
        .unwrap()
        .0;
        assert!(
            report.errors_by_type.contains_key("constraint_violation"),
            "{expected_rule}: {:?}",
            report.errors_by_type
        );
    }
}
