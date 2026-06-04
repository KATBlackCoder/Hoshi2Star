// All #[tauri::command] functions are declared in sub-modules and
// registered via a single generate_handler![...] in lib.rs.

pub mod export;
pub mod glossary;
pub mod project;
pub mod qa;
pub mod translate;
