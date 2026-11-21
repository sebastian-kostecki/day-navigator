pub mod settings;
pub mod tasks;
pub mod vault;

use chrono::Local;
use tauri::Manager;

use crate::settings::TaskSettings;
use crate::tasks::{DoneUpdate, NewTask, TaskError, TaskRow};
use crate::vault::markdown::SpanHash;
use crate::vault::{Space, Vault, VaultError, VaultPath, VaultRegistry};

/// Result of loading settings at startup, kept in Tauri managed state.
/// Later slices read it; a failure is stored, never fatal.
pub struct VaultState(pub Result<VaultRegistry, VaultError>);

/// Work-space task settings from the same startup load. A settings error is
/// stored and surfaces from every task command.
pub struct TasksState(pub Result<Option<TaskSettings>, VaultError>);

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSettingsView {
    pub statuses: Vec<String>,
    pub done_status: String,
    pub priorities: Vec<String>,
    pub folder: String,
}

fn task_cfg(state: &TasksState) -> Result<&TaskSettings, TaskError> {
    let tasks = state.0.as_ref().map_err(|e| TaskError::Vault(e.clone()))?;
    tasks::require_config(tasks.as_ref())
}

fn work_vault(state: &VaultState) -> Result<Vault, TaskError> {
    let registry = state.0.as_ref().map_err(|e| TaskError::Vault(e.clone()))?;
    Ok(registry.vault(Space::Work)?)
}

#[tauri::command]
fn get_work_task_settings(
    state: tauri::State<'_, TasksState>,
) -> Result<TaskSettingsView, TaskError> {
    let cfg = task_cfg(&state)?;
    Ok(TaskSettingsView {
        statuses: cfg.statuses.clone(),
        done_status: cfg.done_status.clone(),
        priorities: cfg.priorities.clone(),
        folder: cfg.folder.to_string(),
    })
}

#[tauri::command]
fn list_work_tasks(
    vault_state: tauri::State<'_, VaultState>,
    tasks_state: tauri::State<'_, TasksState>,
) -> Result<Vec<TaskRow>, TaskError> {
    let cfg = task_cfg(&tasks_state)?;
    let vault = work_vault(&vault_state)?;
    tasks::list_tasks(&vault, cfg)
}

#[tauri::command]
fn create_work_task(
    vault_state: tauri::State<'_, VaultState>,
    tasks_state: tauri::State<'_, TasksState>,
    new: NewTask,
) -> Result<String, TaskError> {
    let cfg = task_cfg(&tasks_state)?;
    let vault = work_vault(&vault_state)?;
    let path = tasks::create_task(&vault, cfg, &new, Local::now().naive_local())?;
    Ok(path.to_string())
}

#[tauri::command]
fn set_work_task_status(
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
    tasks::set_status(
        &vault,
        cfg,
        &path,
        &status,
        status_hash,
        done_hash,
        Local::now().date_naive(),
    )
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let loaded = match app.path().app_config_dir() {
                Ok(dir) => settings::load_settings(&dir.join("settings.json")),
                Err(e) => Err(VaultError::SettingsInvalid {
                    reason: format!("cannot resolve app config dir: {e}"),
                }),
            };
            let tasks = loaded
                .as_ref()
                .map(|s| s.work.as_ref().and_then(|w| w.tasks.clone()))
                .map_err(|e| e.clone());
            app.manage(VaultState(loaded.map(|s| VaultRegistry::from_settings(&s))));
            app.manage(TasksState(tasks));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_work_task_settings,
            list_work_tasks,
            create_work_task,
            set_work_task_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
