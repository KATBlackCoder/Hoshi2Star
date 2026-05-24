// Hoshi2Star — entrée app (ADR-005 : lib.rs, pas main.rs)
// main.rs délègue à run() pour compatibilité mobile future.

pub mod commands;
pub mod engines;
pub mod llm;
pub mod state;

// Commande de démonstration — à supprimer en F1
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet]);

    #[cfg(debug_assertions)]
    {
        builder = builder.plugin(tauri_plugin_mcp_bridge::init());
    }

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
