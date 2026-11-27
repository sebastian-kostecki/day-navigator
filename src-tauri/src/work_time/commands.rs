//! Tauri commands for the work timer: thin async wrappers over the domain.
//! The only file in `work_time` that imports `tauri`.
//!
//! Each command runs on the blocking pool, reaches the store through an
//! `AppHandle` clone (`StoreHandle` is managed state, not `Clone`) and, after a
//! successful write, emits `work-time-changed` (no payload). Errors are logged
//! by kind only.

use std::path::{Path, PathBuf};

use chrono::{Local, Utc};
use tauri::{AppHandle, Emitter, Manager};

use super::report::{self, MonthReport};
use super::week::{self, WeekReport};
use super::{DayEntries, WorkTimeError, WorkTimerStatus};
use crate::blocking;
use crate::settings::{WorkTimeSettings, load_work_time_settings};
use crate::settings_file;
use crate::store::{Store, StoreHandle};

pub const EVENT: &str = "work-time-changed";

/// What a caller does with an invalid `workTime` block (or a settings file
/// that cannot be read at all).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Policy {
    /// Refuse with `WorkTimeError::SettingsInvalid`.
    Strict,
    /// Use the default calendar, so START/STOP and the reminders keep working.
    Lenient,
}

/// Reads the `workTime` settings for one call. `path` is `None` when the app
/// config dir cannot be resolved; the defaults apply then. Blocking file read:
/// call it off the main thread and before taking the store lock.
pub fn work_time_settings(
    path: Option<&Path>,
    policy: Policy,
) -> Result<WorkTimeSettings, WorkTimeError> {
    let Some(path) = path else {
        return Ok(WorkTimeSettings::default());
    };
    match load_work_time_settings(path) {
        Ok(settings) => Ok(settings),
        Err(_) => match policy {
            Policy::Strict => Err(WorkTimeError::SettingsInvalid),
            Policy::Lenient => {
                log::debug!("work_time: settings unavailable: SettingsInvalid");
                Ok(WorkTimeSettings::default())
            }
        },
    }
}

/// Runs `f(store, now_ms)` on the store, off the main thread.
async fn run<T: Send + 'static>(
    app: &AppHandle,
    name: &'static str,
    f: impl FnOnce(&mut Store, i64) -> Result<T, WorkTimeError> + Send + 'static,
) -> Result<T, WorkTimeError> {
    // No settings read: these commands do not use the holiday rule.
    execute(app, name, None, Policy::Lenient, move |store, now, _| {
        f(store, now)
    })
    .await
}

/// Like `run`, for the commands that use the holiday rule. The settings file
/// is read inside the blocking task but before `with_store`, so a file read
/// never lengthens the store lock.
async fn run_with_settings<T: Send + 'static>(
    app: &AppHandle,
    name: &'static str,
    policy: Policy,
    f: impl FnOnce(&mut Store, i64, &WorkTimeSettings) -> Result<T, WorkTimeError> + Send + 'static,
) -> Result<T, WorkTimeError> {
    execute(app, name, settings_file::settings_path(app), policy, f).await
}

async fn execute<T: Send + 'static>(
    app: &AppHandle,
    name: &'static str,
    path: Option<PathBuf>,
    policy: Policy,
    f: impl FnOnce(&mut Store, i64, &WorkTimeSettings) -> Result<T, WorkTimeError> + Send + 'static,
) -> Result<T, WorkTimeError> {
    let handle = app.clone();
    let today = Local::now().date_naive();
    let result = blocking(move || {
        let settings = work_time_settings(path.as_deref(), policy)?;
        handle.state::<StoreHandle>().with_store(today, |store| {
            f(store, Utc::now().timestamp_millis(), &settings)
        })
    })
    .await;
    match &result {
        // A store or background failure is worth a warning; a refusal of a
        // user action (overlap, future time…) is routine.
        Err(e @ (WorkTimeError::Store(_) | WorkTimeError::Background)) => {
            log::warn!("work_time: {name} failed: {}", e.kind());
        }
        Err(e) => log::debug!("work_time: {name} refused: {}", e.kind()),
        Ok(_) => {}
    }
    result
}

fn notify(app: &AppHandle) {
    if let Err(e) = app.emit(EVENT, ()) {
        log::warn!("work_time: emit failed: {e}");
    }
    // Every work-time write wakes the health round, the tray's only writer.
    crate::runtime::refresh_tray(app);
}

#[tauri::command]
pub async fn work_timer_status(app: AppHandle) -> Result<WorkTimerStatus, WorkTimeError> {
    run_with_settings(&app, "status", Policy::Lenient, |store, now, settings| {
        super::status(store, now, &Local, settings)
    })
    .await
}

#[tauri::command]
pub async fn list_work_entries(app: AppHandle, day: String) -> Result<DayEntries, WorkTimeError> {
    let day = report::parse_day(&day)?;
    run_with_settings(
        &app,
        "list",
        Policy::Lenient,
        move |store, now, settings| super::entries_for_day(store, day, now, &Local, settings),
    )
    .await
}

/// The one START path, for the header command and the tray: domain write, then
/// `work-time-changed`, then `events-changed`. Nothing is emitted on failure.
pub(crate) async fn start_timer(app: &AppHandle) -> Result<(), WorkTimeError> {
    run(app, "start", |store, now| {
        super::start(store, now, &Local).map(|_| ())
    })
    .await?;
    notify(app);
    crate::events::commands::notify(app);
    Ok(())
}

/// The one STOP path, for the header command and the tray. A later notice
/// after STOP hooks in here (for every STOP) or in the tray caller (tray STOP
/// only).
pub(crate) async fn stop_timer(app: &AppHandle) -> Result<(), WorkTimeError> {
    run(app, "stop", |store, now| super::stop(store, now, &Local)).await?;
    notify(app);
    crate::events::commands::notify(app);
    Ok(())
}

// START/STOP notify as soon as the write commits and only then read the new
// status, so a failed read can't hide a committed change from the listeners.
#[tauri::command]
pub async fn start_work(app: AppHandle) -> Result<WorkTimerStatus, WorkTimeError> {
    start_timer(&app).await?;
    work_timer_status(app).await
}

#[tauri::command]
pub async fn stop_work(app: AppHandle) -> Result<WorkTimerStatus, WorkTimeError> {
    stop_timer(&app).await?;
    work_timer_status(app).await
}

#[tauri::command]
pub async fn add_work_entry(
    app: AppHandle,
    start: String,
    end: String,
    note: String,
) -> Result<(), WorkTimeError> {
    run(&app, "add", move |store, now| {
        super::add(store, &start, &end, &note, now, &Local).map(|_| ())
    })
    .await?;
    notify(&app);
    Ok(())
}

#[tauri::command]
pub async fn update_work_entry(
    app: AppHandle,
    id: i64,
    start: String,
    end: Option<String>,
    note: String,
) -> Result<(), WorkTimeError> {
    run(&app, "update", move |store, now| {
        super::update(store, id, &start, end.as_deref(), &note, now, &Local)
    })
    .await?;
    notify(&app);
    Ok(())
}

#[tauri::command]
pub async fn remove_work_entry(app: AppHandle, id: i64) -> Result<(), WorkTimeError> {
    run(&app, "remove", move |store, now| {
        super::remove(store, id, now)
    })
    .await?;
    notify(&app);
    Ok(())
}

#[tauri::command]
pub async fn work_month_report(app: AppHandle, day: String) -> Result<MonthReport, WorkTimeError> {
    let day = report::parse_day(&day)?;
    run_with_settings(
        &app,
        "month_report",
        Policy::Strict,
        move |store, now, settings| report::month_report(store, day, now, &Local, settings),
    )
    .await
}

#[tauri::command]
pub async fn work_week_report(app: AppHandle, day: String) -> Result<WeekReport, WorkTimeError> {
    let day = report::parse_day(&day)?;
    run_with_settings(
        &app,
        "week_report",
        Policy::Strict,
        move |store, now, settings| week::week_report(store, day, now, &Local, settings),
    )
    .await
}

#[tauri::command]
pub async fn set_day_off(
    app: AppHandle,
    day: String,
    mark: report::DayMark,
) -> Result<(), WorkTimeError> {
    let day = report::parse_day(&day)?;
    run_with_settings(
        &app,
        "set_day_off",
        Policy::Strict,
        move |store, now, settings| report::set_day_off(store, day, mark, now, settings),
    )
    .await?;
    notify(&app);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::work_time::holidays::HolidayCalendar;

    fn file(content: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, content).unwrap();
        (dir, path)
    }

    const INVALID: &str = r#"{"version":1,"workTime":{"holidays":1}}"#;

    #[test]
    fn no_path_gives_the_defaults_under_both_policies() {
        for policy in [Policy::Strict, Policy::Lenient] {
            assert_eq!(
                work_time_settings(None, policy).unwrap(),
                WorkTimeSettings::default()
            );
        }
    }

    #[test]
    fn a_valid_block_is_read_under_both_policies() {
        let (_dir, path) = file(r#"{"version":1,"workTime":{"holidays":false}}"#);
        for policy in [Policy::Strict, Policy::Lenient] {
            let s = work_time_settings(Some(&path), policy).unwrap();
            assert_eq!(s.holidays, HolidayCalendar::Off);
        }
    }

    #[test]
    fn a_valid_norm_is_read_and_an_invalid_one_follows_the_policy() {
        let (_dir, path) = file(r#"{"version":1,"workTime":{"dailyNormHours":7.5}}"#);
        for policy in [Policy::Strict, Policy::Lenient] {
            let s = work_time_settings(Some(&path), policy).unwrap();
            assert_eq!(s.daily_norm_ms, 27_000_000);
        }
        let (_dir, bad) = file(r#"{"version":1,"workTime":{"dailyNormHours":11}}"#);
        assert!(matches!(
            work_time_settings(Some(&bad), Policy::Strict),
            Err(WorkTimeError::SettingsInvalid)
        ));
        assert_eq!(
            work_time_settings(Some(&bad), Policy::Lenient).unwrap(),
            WorkTimeSettings::default()
        );
    }

    #[test]
    fn strict_refuses_an_invalid_block() {
        let (_dir, path) = file(INVALID);
        assert!(matches!(
            work_time_settings(Some(&path), Policy::Strict),
            Err(WorkTimeError::SettingsInvalid)
        ));
    }

    #[test]
    fn strict_refuses_a_file_level_problem() {
        let (_dir, path) = file("{ not json");
        assert!(matches!(
            work_time_settings(Some(&path), Policy::Strict),
            Err(WorkTimeError::SettingsInvalid)
        ));
    }

    #[test]
    fn lenient_falls_back_to_the_defaults() {
        for content in [INVALID, "{ not json"] {
            let (_dir, path) = file(content);
            assert_eq!(
                work_time_settings(Some(&path), Policy::Lenient).unwrap(),
                WorkTimeSettings::default()
            );
        }
    }

    #[test]
    fn a_missing_file_gives_the_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        for policy in [Policy::Strict, Policy::Lenient] {
            assert_eq!(
                work_time_settings(Some(&path), policy).unwrap(),
                WorkTimeSettings::default()
            );
        }
    }

    #[test]
    fn settings_invalid_serializes_as_a_tagged_kind() {
        assert_eq!(
            serde_json::to_string(&WorkTimeError::SettingsInvalid).unwrap(),
            r#"{"kind":"SettingsInvalid"}"#
        );
    }
}
