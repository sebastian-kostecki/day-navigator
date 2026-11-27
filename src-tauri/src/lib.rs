pub mod backups_folder;
pub mod cli;
pub mod day_summary;
pub mod events;
pub mod github;
pub mod github_refresh;
pub mod gitlab;
pub mod gitlab_refresh;
pub mod health;
pub mod hidden_issues;
pub mod new_assignments;
pub mod own_tasks;
pub mod runtime;
pub mod settings;
pub mod settings_file;
pub mod space;
pub mod store;
pub mod todo;
pub mod tray_view;
pub mod work_reminders;
pub mod work_time;

use chrono::{Local, NaiveDateTime};
use tauri::{Emitter, Manager};

use crate::own_tasks::{OwnTask, OwnTaskError, OwnTaskInput};
use crate::space::Space;
use crate::store::{Store, StoreError, StoreHandle};

/// Emitted (no payload) when the close button hides the main window.
const MAIN_HIDDEN_EVENT: &str = "main-hidden";

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
    use tauri_plugin_window_state::StateFlags;

    let mut log_targets = vec![Target::new(TargetKind::LogDir { file_name: None })];
    #[cfg(debug_assertions)]
    log_targets.push(Target::new(TargetKind::Stdout));

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            runtime::show_main(app);
        }))
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::all() & !StateFlags::VISIBLE)
                .build(),
        )
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
            let settings_path = settings_file::settings_path(app.handle());
            let data_dir = app.path().app_data_dir().ok();
            // Not opened here: the first health or reminder round opens it on
            // its own thread, so `quick_check`, backup and migrations never
            // delay the main thread.
            app.manage(store::StoreHandle::new(data_dir));
            // Managed before the health thread starts: its first round reads it.
            let gitlab_rx = gitlab_refresh::manage(app);
            let github_rx = github_refresh::manage(app);
            let wake_rx = runtime::build_tray(app)?;
            runtime::start(app.handle(), settings_path.clone(), wake_rx);
            work_reminders::start(app.handle(), settings_path.clone());
            // After `runtime::start`: the refresh threads wake `HealthState`.
            gitlab_refresh::start(app.handle(), settings_path.clone(), gitlab_rx);
            github_refresh::start(app.handle(), settings_path, github_rx);
            // The window starts hidden; show it once the saved size is restored.
            runtime::show_main(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event
                && window.label() == "main"
            {
                api.prevent_close();
                let _ = window.hide();
                // The view stays mounted while hidden: tell it the visit ended.
                if let Err(e) = window.emit(MAIN_HIDDEN_EVENT, ()) {
                    log::warn!("main window: emit failed: {e}");
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            events::commands::list_events_today,
            runtime::health_status,
            runtime::check_health_now,
            gitlab_refresh::gitlab_snapshot,
            gitlab_refresh::refresh_gitlab,
            gitlab_refresh::mark_gitlab_seen,
            settings_file::open_settings_file,
            backups_folder::open_backups_folder,
            github_refresh::github_snapshot,
            github_refresh::refresh_github,
            hidden_issues::commands::hide_gitlab_issue,
            hidden_issues::commands::unhide_gitlab_issue,
            hidden_issues::commands::dismiss_gitlab_change,
            list_own_tasks,
            add_own_task,
            edit_own_task,
            set_own_task_done,
            own_task_default_space,
            day_summary::commands::day_summary,
            todo::commands::list_todo,
            todo::commands::add_todo_own_task,
            todo::commands::add_todo_gitlab_item,
            todo::commands::add_todo_github_item,
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
            work_time::commands::work_week_report,
            work_time::commands::work_month_report_text,
            work_time::commands::set_day_off,
            work_reminders::reminder_settings
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
