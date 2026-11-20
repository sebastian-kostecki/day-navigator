pub mod health;
pub mod settings;
pub mod vault;

use std::sync::{Arc, RwLock};

use tauri::Manager;

use crate::vault::{VaultError, VaultRegistry};

/// Registry (or the error that stopped it) built from `settings.json`, kept in
/// Tauri managed state. Each health check replaces it, so fixing settings
/// takes effect without a restart. A failure is stored, never fatal.
pub struct VaultState(RwLock<Arc<Result<VaultRegistry, VaultError>>>);

impl VaultState {
    pub fn new(initial: Result<VaultRegistry, VaultError>) -> Self {
        VaultState(RwLock::new(Arc::new(initial)))
    }

    /// Cheap snapshot; the lock is not held after this returns.
    pub fn current(&self) -> Arc<Result<VaultRegistry, VaultError>> {
        Arc::clone(&self.0.read().unwrap_or_else(|p| p.into_inner()))
    }

    pub fn replace(&self, next: Result<VaultRegistry, VaultError>) {
        *self.0.write().unwrap_or_else(|p| p.into_inner()) = Arc::new(next);
    }
}

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
            app.manage(VaultState::new(state));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
