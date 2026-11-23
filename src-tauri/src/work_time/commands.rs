//! Tauri commands for the work timer: thin async wrappers over the domain.
//! The only file in `work_time` that imports `tauri`.
//!
//! Each command runs on the blocking pool, reaches the store through an
//! `AppHandle` clone (`StoreHandle` is managed state, not `Clone`) and, after a
//! successful write, emits `work-time-changed` (no payload). Errors are logged
//! by kind only.

use chrono::{Local, NaiveDate, Utc};
use tauri::{AppHandle, Emitter, Manager};

use super::{DayEntries, WorkTimeError, WorkTimerStatus};
use crate::blocking;
use crate::store::{Store, StoreHandle};

pub const EVENT: &str = "work-time-changed";

fn kind(e: &WorkTimeError) -> &'static str {
    match e {
        WorkTimeError::Store(s) => s.kind(),
        WorkTimeError::AlreadyRunning => "AlreadyRunning",
        WorkTimeError::NotRunning => "NotRunning",
        WorkTimeError::NotFound => "NotFound",
        WorkTimeError::EndNotAfterStart => "EndNotAfterStart",
        WorkTimeError::InFuture => "InFuture",
        WorkTimeError::Overlap { .. } => "Overlap",
        WorkTimeError::InvalidLocalTime => "InvalidLocalTime",
        WorkTimeError::RunningEndEdit => "RunningEndEdit",
        WorkTimeError::Background => "Background",
    }
}

/// Runs `f(store, now_ms)` on the store, off the main thread.
async fn run<T: Send + 'static>(
    app: &AppHandle,
    name: &'static str,
    f: impl FnOnce(&mut Store, i64) -> Result<T, WorkTimeError> + Send + 'static,
) -> Result<T, WorkTimeError> {
    let handle = app.clone();
    let today = Local::now().date_naive();
    let result = blocking(move || {
        handle
            .state::<StoreHandle>()
            .with_store(today, |store| f(store, Utc::now().timestamp_millis()))
    })
    .await;
    if let Err(e) = &result {
        log::warn!("work_time: {name} failed: {}", kind(e));
    }
    result
}

fn notify(app: &AppHandle) {
    if let Err(e) = app.emit(EVENT, ()) {
        log::warn!("work_time: emit failed: {e}");
    }
}

#[tauri::command]
pub async fn work_timer_status(app: AppHandle) -> Result<WorkTimerStatus, WorkTimeError> {
    run(&app, "status", |store, now| {
        super::status(store, now, &Local)
    })
    .await
}

#[tauri::command]
pub async fn list_work_entries(app: AppHandle, day: String) -> Result<DayEntries, WorkTimeError> {
    let day =
        NaiveDate::parse_from_str(&day, "%Y-%m-%d").map_err(|_| WorkTimeError::InvalidLocalTime)?;
    run(&app, "list", move |store, now| {
        super::entries_for_day(store, day, now, &Local)
    })
    .await
}

#[tauri::command]
pub async fn start_work(app: AppHandle) -> Result<WorkTimerStatus, WorkTimeError> {
    let status = run(&app, "start", |store, now| {
        super::start(store, now)?;
        super::status(store, now, &Local)
    })
    .await?;
    notify(&app);
    Ok(status)
}

#[tauri::command]
pub async fn stop_work(app: AppHandle) -> Result<WorkTimerStatus, WorkTimeError> {
    let status = run(&app, "stop", |store, now| {
        super::stop(store, now)?;
        super::status(store, now, &Local)
    })
    .await?;
    notify(&app);
    Ok(status)
}

#[tauri::command]
pub async fn add_work_entry(
    app: AppHandle,
    start: String,
    end: String,
) -> Result<(), WorkTimeError> {
    run(&app, "add", move |store, now| {
        super::add(store, &start, &end, now, &Local).map(|_| ())
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
) -> Result<(), WorkTimeError> {
    run(&app, "update", move |store, now| {
        super::update(store, id, &start, end.as_deref(), now, &Local)
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
