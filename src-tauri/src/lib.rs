pub mod cli;
pub mod gitlab;
pub mod gitlab_refresh;
pub mod health;
pub mod runtime;
pub mod settings;
pub mod store;
pub mod tasks;
pub mod vault;
pub mod work_time;

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use chrono::Local;
use tauri::Manager;

use crate::settings::TaskSettings;
use crate::tasks::{DoneUpdate, NewTask, TaskError, TaskRow};
use crate::vault::markdown::SpanHash;
use crate::vault::{Space, Vault, VaultError, VaultPath, VaultRegistry};

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

/// Where `settings.json` lives (`None` if the config dir could not be
/// resolved). Task commands re-read the Work `tasks` block on every call, so a
/// fixed settings file takes effect without a restart, like `VaultState`.
pub struct TasksState(Option<PathBuf>);

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSettingsView {
    pub statuses: Vec<String>,
    pub done_status: String,
    pub priorities: Vec<String>,
    pub folder: String,
}

fn task_cfg(state: &TasksState) -> Result<TaskSettings, TaskError> {
    let Some(path) = &state.0 else {
        return Err(match health::unresolved_config_dir().0 {
            Err(e) => TaskError::Vault(e),
            Ok(_) => TaskError::TasksNotConfigured,
        });
    };
    let settings = settings::load_settings(path)?;
    let tasks = settings.work.and_then(|w| w.tasks);
    tasks::require_config(tasks.as_ref()).cloned()
}

fn work_vault(state: &VaultState) -> Result<Vault, TaskError> {
    let current = state.current();
    let registry = (*current)
        .as_ref()
        .map_err(|e| TaskError::Vault(e.clone()))?;
    Ok(registry.vault(Space::Work)?)
}

/// The blocking task could not run to completion (join error text). Each
/// command error type maps it to its own variant.
pub struct BackgroundFailed(pub String);

impl From<BackgroundFailed> for TaskError {
    fn from(e: BackgroundFailed) -> Self {
        TaskError::Vault(VaultError::Io {
            path: String::new(),
            kind: format!("background task failed: {}", e.0),
        })
    }
}

/// Runs vault or store I/O off the main thread, so a slow or cloud-synced
/// vault never freezes the window. `Vault::apply`'s re-check keeps concurrent
/// writes safe.
pub(crate) async fn blocking<T, E>(
    work: impl FnOnce() -> Result<T, E> + Send + 'static,
) -> Result<T, E>
where
    T: Send + 'static,
    E: From<BackgroundFailed> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| E::from(BackgroundFailed(e.to_string())))?
}

#[tauri::command]
fn get_work_task_settings(
    state: tauri::State<'_, TasksState>,
) -> Result<TaskSettingsView, TaskError> {
    let cfg = task_cfg(&state)?;
    Ok(TaskSettingsView {
        folder: cfg.folder.to_string(),
        statuses: cfg.statuses,
        done_status: cfg.done_status,
        priorities: cfg.priorities,
    })
}

#[tauri::command]
async fn list_work_tasks(
    vault_state: tauri::State<'_, VaultState>,
    tasks_state: tauri::State<'_, TasksState>,
) -> Result<Vec<TaskRow>, TaskError> {
    let cfg = task_cfg(&tasks_state)?;
    let vault = work_vault(&vault_state)?;
    blocking(move || tasks::list_tasks(&vault, &cfg)).await
}

#[tauri::command]
async fn create_work_task(
    vault_state: tauri::State<'_, VaultState>,
    tasks_state: tauri::State<'_, TasksState>,
    new: NewTask,
) -> Result<String, TaskError> {
    let cfg = task_cfg(&tasks_state)?;
    let vault = work_vault(&vault_state)?;
    let now = Local::now().naive_local();
    blocking(move || {
        let path = tasks::create_task(&vault, &cfg, &new, now)?;
        Ok(path.to_string())
    })
    .await
}

#[tauri::command]
async fn set_work_task_status(
    vault_state: tauri::State<'_, VaultState>,
    tasks_state: tauri::State<'_, TasksState>,
    path: String,
    status: String,
    status_hash: String,
    done_hash: Option<String>,
) -> Result<DoneUpdate, TaskError> {
    let cfg = task_cfg(&tasks_state)?;
    let vault = work_vault(&vault_state)?;
    let path = VaultPath::parse(&path)?;
    let status_hash = SpanHash::from_hex(&status_hash)?;
    let done_hash = done_hash.as_deref().map(SpanHash::from_hex).transpose()?;
    let today = Local::now().date_naive();
    blocking(move || tasks::set_status(&vault, &cfg, &path, &status, status_hash, done_hash, today))
        .await
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
                None => health::unresolved_config_dir().0,
            };
            app.manage(VaultState::new(state));
            app.manage(TasksState(settings_path.clone()));
            let data_dir = app.path().app_data_dir().ok();
            // Not opened here: the first health round opens it on the check
            // thread, so `quick_check`, backup and migrations never delay the
            // main thread.
            app.manage(store::StoreHandle::new(data_dir));
            // Managed before the health thread starts: its first round reads it.
            let gitlab_rx = gitlab_refresh::manage(app);
            let wake_rx = runtime::build_tray(app)?;
            runtime::start(app.handle(), settings_path.clone(), wake_rx);
            // After `runtime::start`: the refresh thread wakes `HealthState`.
            gitlab_refresh::start(app.handle(), settings_path, gitlab_rx);
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
        .invoke_handler(tauri::generate_handler![
            runtime::health_status,
            runtime::check_health_now,
            gitlab_refresh::gitlab_snapshot,
            gitlab_refresh::refresh_gitlab,
            get_work_task_settings,
            list_work_tasks,
            create_work_task,
            set_work_task_status,
            work_time::commands::work_timer_status,
            work_time::commands::list_work_entries,
            work_time::commands::start_work,
            work_time::commands::stop_work,
            work_time::commands::add_work_entry,
            work_time::commands::update_work_entry,
            work_time::commands::remove_work_entry
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            // Closing the window hides it; only "Zakończ" (`app.exit(0)`, code
            // `Some`) really quits.
            if let tauri::RunEvent::ExitRequested {
                code: None, api, ..
            } = event
            {
                api.prevent_exit();
            }
        });
}
