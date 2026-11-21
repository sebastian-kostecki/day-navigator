pub mod settings;
pub mod tasks;
pub mod vault;

use tauri::Manager;

use crate::vault::{VaultError, VaultRegistry};

/// Result of loading settings at startup, kept in Tauri managed state.
/// Later slices read it; a failure is stored, never fatal.
pub struct VaultState(pub Result<VaultRegistry, VaultError>);

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let state = match app.path().app_config_dir() {
                Ok(dir) => settings::load_settings(&dir.join("settings.json"))
                    .map(|s| VaultRegistry::from_settings(&s)),
                Err(e) => Err(VaultError::SettingsInvalid {
                    reason: format!("cannot resolve app config dir: {e}"),
                }),
            };
            app.manage(VaultState(state));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
