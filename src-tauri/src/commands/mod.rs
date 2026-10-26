// All #[tauri::command] functions are declared in sub-modules and
// registered via a single generate_handler![...] in lib.rs.

pub mod app;
pub mod coverage;
pub mod export;
pub mod pack;
mod pack_terminology;
pub mod pilot;
pub mod project;
pub mod qa;
pub mod resync;
pub mod search;
pub mod terminology;
pub mod text_fit;
pub mod translate;
