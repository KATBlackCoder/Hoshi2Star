//! App-level Tauri commands not tied to a specific domain layer.

/// Whether the current install can self-update via the Tauri updater.
///
/// The updater only replaces the **AppImage** on Linux and the NSIS `.exe` /
/// `.msi` on Windows. A `.deb` / `.rpm` install (plain binary, no `APPIMAGE`
/// env var) is managed by the system package manager and must not be offered an
/// in-app update; macOS builds are not produced. The frontend gates the whole
/// update UI on this so those users are never shown an update that would fail.
#[tauri::command]
pub fn updater_supported() -> bool {
    cfg!(target_os = "windows") || std::env::var_os("APPIMAGE").is_some()
}
