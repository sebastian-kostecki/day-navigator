pub mod cli;
pub mod gitlab;
pub mod gitlab_refresh;
pub mod health;
pub mod hidden_issues;
pub mod new_assignments;
pub mod own_tasks;
pub mod runtime;
pub mod settings;
pub mod space;
pub mod store;
pub mod todo;
pub mod vault;
pub mod work_reminders;
pub mod work_time;

use std::sync::{Arc, RwLock};

use chrono::{Local, NaiveDateTime};
use tauri::Manager;

use crate::own_tasks::{OwnTask, OwnTaskError, OwnTaskInput};
use crate::space::Space;
use crate::store::{Store, StoreError, StoreHandle};
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

/// The blocking task could not run to completion (join error text). Each
/// command error type maps it to its own variant.
pub struct BackgroundFailed(pub String);

/// Runs blocking work off the main thread (used by the work-time commands).
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

/// Runs a store operation off the main thread with the current local time.
/// `tauri::State` is not `'static`, so the handle is fetched from the
/// `AppHandle` inside the blocking closure.
async fn on_store<T, E>(
    app: tauri::AppHandle,
    f: impl FnOnce(&mut Store, NaiveDateTime) -> Result<T, E> + Send + 'static,
) -> Result<T, E>
where
    T: Send + 'static,
    E: From<StoreError> + Send + 'static,
{
    let now = Local::now().naive_local();
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<StoreHandle>()
            .with_store(now.date(), |s| f(s, now))
    })
    .await
    .map_err(|_| {
        E::from(StoreError::Io {
            kind: "background task failed".to_string(),
        })
    })?
}

#[tauri::command]
async fn list_own_tasks(app: tauri::AppHandle) -> Result<Vec<OwnTask>, OwnTaskError> {
    on_store(app, |s, _| own_tasks::list(s)).await
}

#[tauri::command]
async fn add_own_task(app: tauri::AppHandle, input: OwnTaskInput) -> Result<OwnTask, OwnTaskError> {
    on_store(app, move |s, now| own_tasks::add(s, &input, now)).await
}

#[tauri::command]
async fn edit_own_task(
    app: tauri::AppHandle,
    id: i64,
    input: OwnTaskInput,
) -> Result<OwnTask, OwnTaskError> {
    on_store(app, move |s, now| own_tasks::edit(s, id, &input, now)).await
}

#[tauri::command]
async fn set_own_task_done(
    app: tauri::AppHandle,
    id: i64,
    done: bool,
) -> Result<OwnTask, OwnTaskError> {
    // One write: roll over first, so a task ticked on an earlier day and
    // reopened before today's list was read stays closed in "To do".
    on_store(app, move |s, now| {
        s.write(|tx| {
            todo::roll_over_in(tx, now.date())?;
            own_tasks::set_done_in(tx, id, done, now)
        })
    })
    .await
}

/// The add form's space: Work while the work-time counter runs, else Private.
#[tauri::command]
async fn own_task_default_space(app: tauri::AppHandle) -> Result<Space, OwnTaskError> {
    on_store(app, |s, _| {
        let running = work_time::is_running(s).map_err(|e| match e {
            work_time::WorkTimeError::Store(e) => OwnTaskError::Store(e),
            // `is_running` only reads, so only store errors are expected; any
            // other variant stays value-free.
            _ => OwnTaskError::Store(StoreError::Sql {
                reason: "work-time state unreadable".to_string(),
            }),
        })?;
        Ok(own_tasks::default_space(running))
    })
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
            let data_dir = app.path().app_data_dir().ok();
            // Not opened here: the first health or reminder round opens it on
            // its own thread, so `quick_check`, backup and migrations never
            // delay the main thread.
            app.manage(store::StoreHandle::new(data_dir));
            // Managed before the health thread starts: its first round reads it.
            let gitlab_rx = gitlab_refresh::manage(app);
            let wake_rx = runtime::build_tray(app)?;
            runtime::start(app.handle(), settings_path.clone(), wake_rx);
            work_reminders::start(app.handle(), settings_path.clone());
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
            hidden_issues::commands::hide_gitlab_issue,
            hidden_issues::commands::unhide_gitlab_issue,
            hidden_issues::commands::dismiss_gitlab_change,
            list_own_tasks,
            add_own_task,
            edit_own_task,
            set_own_task_done,
            own_task_default_space,
            todo::commands::list_todo,
            todo::commands::add_todo_own_task,
            todo::commands::add_todo_gitlab_item,
            todo::commands::set_todo_ticked,
            todo::commands::remove_todo,
            work_time::commands::work_timer_status,
            work_time::commands::list_work_entries,
            work_time::commands::start_work,
            work_time::commands::stop_work,
            work_time::commands::add_work_entry,
            work_time::commands::update_work_entry,
            work_time::commands::remove_work_entry,
            work_time::commands::work_month_report,
            work_time::commands::set_day_off
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
