pub mod health;
pub mod runtime;
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
    use tauri_plugin_log::{RotationStrategy, Target, TargetKind};

    let mut log_targets = vec![Target::new(TargetKind::LogDir { file_name: None })];
    #[cfg(debug_assertions)]
    log_targets.push(Target::new(TargetKind::Stdout));

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            runtime::show_main(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .clear_targets()
                .targets(log_targets)
                .max_file_size(1_000_000)
                .rotation_strategy(RotationStrategy::KeepSome(3))
                .level(log::LevelFilter::Warn)
                .level_for("day_navigator_lib", log::LevelFilter::Info)
                .build(),
        )
        .setup(|app| {
            let settings_path = app
                .path()
                .app_config_dir()
                .ok()
                .map(|dir| dir.join("settings.json"));
            let state = match &settings_path {
                Some(path) => {
                    settings::load_settings(path).map(|s| VaultRegistry::from_settings(&s))
                }
                None => Err(VaultError::SettingsInvalid {
                    reason: "cannot resolve app config dir".to_string(),
                }),
            };
            app.manage(VaultState::new(state));
            let wake_rx = runtime::build_tray(app)?;
            runtime::start(app.handle(), settings_path, wake_rx);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event
                && window.label() == "main"
            {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![greet])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            // Closing the window hides it; only "Zakończ" (`app.exit(0)`, code
            // `Some`) really quits.
            if let tauri::RunEvent::ExitRequested { code: None, api, .. } = event {
                api.prevent_exit();
            }
        });
}
