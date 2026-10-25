//! Session 3 (2026-10-25) — re-run of the 2026-09-05 probes on current main.
//! Observational: prints the CURRENT behaviour as `PROBE <id>: ...` lines.
//! Archive copy. To re-run: copy to `src-tauri/tests/zz_audit_probe_session3.rs`,
//! `cargo test --manifest-path src-tauri/Cargo.toml --test zz_audit_probe_session3
//! -- --nocapture --test-threads=1`, then delete the copy. Not a regression test:
//! it documents defects, it does not assert the desired behaviour.
use hoshi2star_lib::{
    core::qa,
    db,
    engines::mv_mz::{extractor, injector},
    llm::{
        pipeline,
        provider::{LlmError, LlmProvider, PromptContextPolicy, TranslationContext},
        tokenizer::{Engine, Tokenizer},
    },
    state::AppState,
};
use serde_json::json;
use tauri::{
    test::{mock_builder, mock_context, noop_assets},
    Manager,
};

fn context() -> TranslationContext {
    TranslationContext {
        source_lang: "ja".into(),
        target_lang: "fr".into(),
        terminology_hints: vec![],
        engine: "mv_mz".into(),
        batch_size: 2,
        batch_delay_ms: 0,
        prompt_context_policy: PromptContextPolicy::EngineOwned,
        segment_contexts: vec![],
    }
}

#[test]
fn a_bare_system_markers() {
    let source = "%1 obtained %2!";
    for (kind, ctx) in [
        ("system_term", None),
        ("unknown", None),
        ("dialogue", None),
        ("plugin_message", Some(r#"{"placeholderSyntax":"format"}"#)),
    ] {
        let engine = Engine::for_segment("mv_mz", kind, ctx);
        let tokenized = Tokenizer::tokenize(source, engine);
        let qa = qa::check_segment(source, "Objet obtenu !", &[], "mv_mz", kind, ctx);
        println!(
            "PROBE A kind={kind} ctx={ctx:?}: engine={engine:?} protected={} qa_score={} critical={}",
            tokenized.map.len(),
            qa.score,
            qa.has_critical_errors()
        );
    }
    let legacy = qa::check(source, "Objet obtenu !", &[], "mv_mz");
    println!(
        "PROBE A qa::check (kind unknown): score={} critical={}",
        legacy.score,
        legacy.has_critical_errors()
    );
}

#[test]
fn c_visible_fields() {
    let map = json!({"displayName":"Village", "events":[null,{"pages":[{"list":[
        {"code":320,"parameters":[1,"Alice"]},
        {"code":324,"parameters":[1,"Mage"]},
        {"code":325,"parameters":[1,"A travelling mage."]},
        {"code":401,"parameters":["Hello!"]}
    ]}]}]});
    let segments = extractor::extract_map(&map);
    let sources: Vec<_> = segments.iter().map(|s| s.source.as_str()).collect();
    println!("PROBE C map: {} segments {:?}", segments.len(), sources);
    let system = json!({"gameTitle":"Demo", "equipTypes":["","Shield"], "skillTypes":["","Magic"],
        "weaponTypes":["","Sword"], "armorTypes":["","Light"]});
    let sys = extractor::extract_system(&system);
    let sources: Vec<_> = sys.iter().map(|s| s.source.as_str()).collect();
    println!("PROBE C system: {} segments {:?}", sys.len(), sources);
}

#[test]
fn d_shared_vocabulary() {
    let neighbors: Vec<String> = vec![];
    for (src_lang, tgt_lang, kind, text) in [
        ("fr", "en", "item_name", "Potion"),
        ("ja", "zh", "system_term", "魔法"),
    ] {
        let ctx = qa::QaSemanticContext {
            source_language: src_lang,
            target_language: tgt_lang,
            segment_kind: kind,
            context_json: None,
            neighbor_sources: &neighbors,
            neighbor_targets: &neighbors,
        };
        let result = qa::check_with_context(text, text, &[], "mv_mz", &ctx);
        println!(
            "PROBE D {src_lang}->{tgt_lang} {text}: score={} critical={} errors={:?}",
            result.score,
            result.has_critical_errors(),
            result.errors
        );
    }
}

#[test]
fn a2_unknown_tokens_and_reordered_codes() {
    let tokenized = Tokenizer::tokenize(r"\C[2]Bonjour\C[0]", Engine::MvMz);
    let restored = Tokenizer::restore("⟦ph_1⟧Hello⟦ph_0⟧ ⟦ph_99⟧", &tokenized.map);
    println!("PROBE A2 restore: {restored:?}");
    if let Ok(restored) = restored {
        let qa = qa::check(r"\C[2]Bonjour\C[0]", &restored, &[], "mv_mz");
        println!(
            "PROBE A2 qa: score={} critical={}",
            qa.score,
            qa.has_critical_errors()
        );
    }
}

#[test]
fn f_injector_does_not_compare_source() {
    let mut changed_game = json!([null,{"name":"New item after game update"}]);
    let result = injector::inject(&mut changed_game, &[("/1/name", "Old translated item")]);
    println!(
        "PROBE F inject: result_ok={} value={}",
        result.is_ok(),
        changed_game[1]["name"]
    );
}

struct FailingProvider;
impl LlmProvider for FailingProvider {
    async fn translate(
        &self,
        _: Vec<String>,
        _: TranslationContext,
    ) -> Result<Vec<String>, LlmError> {
        Err(LlmError::Unavailable {
            message: "HTTP 401".into(),
        })
    }
    async fn health_check(&self) -> Result<(), LlmError> {
        Ok(())
    }
    async fn chat(&self, _: &str, _: &str) -> Result<String, LlmError> {
        unreachable!()
    }
}

#[tokio::test]
async fn b_provider_failure() {
    let temp = tempfile::NamedTempFile::new().unwrap();
    let pool = db::pool::init(temp.path().to_str().unwrap()).await.unwrap();
    sqlx::query("INSERT INTO projects(id,name,engine,game_path) VALUES('p','Demo','mv_mz','/tmp')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO source_files(id,project_id,file_name,file_path,file_type) VALUES('f','p','Map001.json','/tmp/Map001.json','map')").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO segments(id,source_file_id,json_key,source_text,segment_kind) VALUES('s','f','/text','こんにちは','dialogue')").execute(&pool).await.unwrap();
    let result = pipeline::run_inner(
        vec![("s".into(), "こんにちは".into())],
        &FailingProvider,
        context(),
        &pool,
        None,
        None,
        None,
        |_, _| {},
    )
    .await;
    match &result {
        Ok(r) => println!(
            "PROBE B pipeline: Ok, needs_review={} target={:?}",
            r[0].needs_review, r[0].translated_text
        ),
        Err(e) => println!("PROBE B pipeline: Err({e})"),
    }
    let row: (String, String) =
        sqlx::query_as("SELECT status, COALESCE(target_text,'') FROM segments WHERE id='s'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let resumable: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM segments WHERE status = 'untranslated' OR target_text = ''",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    println!(
        "PROBE B db: status={} target={:?} resumable_by_translate_all={resumable}",
        row.0, row.1
    );
}

#[tokio::test]
async fn sql_variable_limit() {
    let temp = tempfile::NamedTempFile::new().unwrap();
    let pool = db::pool::init(temp.path().to_str().unwrap()).await.unwrap();
    let ids = (0..33000).map(|n| n.to_string()).collect::<Vec<_>>();
    let result = hoshi2star_lib::llm::context::build_for_segments(&pool, "mv_mz", &ids).await;
    match result {
        Ok(v) => println!("PROBE SQL33000: Ok len={}", v.len()),
        Err(e) => println!("PROBE SQL33000: Err({e})"),
    }
}

#[tokio::test]
async fn e_reopen_old_project() {
    use hoshi2star_lib::commands::project::open_project;
    let temp = tempfile::tempdir().unwrap();
    let game = temp.path().join("game");
    std::fs::create_dir_all(game.join("data")).unwrap();
    std::fs::write(game.join("data/System.json"), r#"{"gameTitle":"Demo"}"#).unwrap();
    std::fs::write(
        game.join("data/Map001.json"),
        r#"{"events":[null,{"pages":[{"list":[{"code":401,"parameters":["Hello"]}]}]}]}"#,
    )
    .unwrap();
    let pool = db::pool::init(temp.path().join("db.sqlite").to_str().unwrap())
        .await
        .unwrap();
    let app = mock_builder()
        .manage(AppState::new(pool.clone(), temp.path().join("app-data")).unwrap())
        .build(mock_context(noop_assets()))
        .unwrap();
    let path = game.to_string_lossy().into_owned();
    let first = open_project(
        path.clone(),
        Some("ja".into()),
        Some("fr".into()),
        app.state(),
    )
    .await
    .unwrap();
    sqlx::query("UPDATE segments SET segment_kind='unknown',scene_id=NULL,sequence_index=NULL,speaker=NULL,branch_path=NULL,context_json=NULL").execute(&pool).await.unwrap();
    let restored = open_project(path, Some("fr".into()), Some("en".into()), app.state())
        .await
        .unwrap();
    let populated: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM segments WHERE segment_kind != 'unknown'")
            .fetch_one(&pool)
            .await
            .unwrap();
    println!(
        "PROBE E reopen: restored={} same_id={} target_lang={} segments_with_kind={populated}",
        restored.was_restored,
        restored.project.id == first.project.id,
        restored.project.target_lang
    );
}

#[tokio::test]
async fn s_partial_export_backend() {
    // sportsfan30 lot A: does the backend export a partially translated project?
    use hoshi2star_lib::commands::{export::export_project, project::{open_project, update_segment}};
    let temp = tempfile::tempdir().unwrap();
    let game = temp.path().join("game");
    std::fs::create_dir_all(game.join("data")).unwrap();
    std::fs::write(game.join("data/System.json"), r#"{"gameTitle":"デモ"}"#).unwrap();
    std::fs::write(game.join("data/Map001.json"), r#"{"events":[null,{"pages":[{"list":[{"code":401,"parameters":["こんにちは"]},{"code":401,"parameters":["さようなら"]}]}]}]}"#).unwrap();
    let pool = db::pool::init(temp.path().join("db.sqlite").to_str().unwrap())
        .await
        .unwrap();
    let app = mock_builder()
        .manage(AppState::new(pool.clone(), temp.path().join("app-data")).unwrap())
        .build(mock_context(noop_assets()))
        .unwrap();
    let project = open_project(
        game.to_string_lossy().into_owned(),
        Some("ja".into()),
        Some("fr".into()),
        app.state(),
    )
    .await
    .unwrap()
    .project;
    let id: String = sqlx::query_scalar("SELECT id FROM segments WHERE source_text='こんにちは'")
        .fetch_one(&pool)
        .await
        .unwrap();
    update_segment(id, "Bonjour".into(), app.state()).await.unwrap();
    let result = export_project(project.id, None, false, Some(false), app.state()).await;
    println!("PROBE LotA partial export (1/3 translated): {:?}", result.as_ref().map(|_| "zip written"));
}
