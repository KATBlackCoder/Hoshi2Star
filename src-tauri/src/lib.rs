// Hoshi2Star — entrée app (ADR-005 : lib.rs, pas main.rs)
// main.rs délègue à run() pour compatibilité mobile future.

pub mod commands;
pub mod core;
pub mod db;
pub mod engines;
pub mod llm;
pub mod state;
pub mod utils;

use commands::glossary::{
    add_glossary_term, delete_glossary_term, extract_glossary_terms, get_glossary,
    update_glossary_term,
};
use commands::project::{
    delete_project, export_debug_json, export_project, export_qa_report, export_tm,
    get_ollama_models, get_project_stats, get_qa_report, get_segments, get_source_files,
    get_tm_suggestions, list_projects, open_project, qa_check_segment, translate_all_segments,
    translate_segments, update_segment,
};
use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            let db_path = app.path().app_data_dir()?.join("hoshi2star.db");

            // Ensure the data directory exists before connecting
            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let path_str = db_path.to_str().ok_or("non-UTF-8 db path")?.to_string();

            let pool = tauri::async_runtime::block_on(db::pool::init(&path_str))
                .map_err(|e| e.to_string())?;

            app.manage(AppState { db: pool });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            open_project,
            get_source_files,
            get_segments,
            update_segment,
            export_project,
            translate_segments,
            get_tm_suggestions,
            get_qa_report,
            qa_check_segment,
            get_ollama_models,
            get_glossary,
            add_glossary_term,
            update_glossary_term,
            delete_glossary_term,
            extract_glossary_terms,
            export_tm,
            export_qa_report,
            export_debug_json,
            list_projects,
            delete_project,
            get_project_stats,
            translate_all_segments,
        ]);

    #[cfg(debug_assertions)]
    {
        builder = builder.plugin(tauri_plugin_mcp_bridge::init());
    }

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
