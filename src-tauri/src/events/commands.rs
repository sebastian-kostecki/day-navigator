//! The read command of the event log, plus the change notification for the
//! places that write events. The only file in `events` that imports `tauri`.

use chrono::Local;
use tauri::{AppHandle, Emitter, Manager};

use super::{CHANGED_EVENT, EventError, EventLog, list_today};
use crate::blocking;
use crate::store::{StoreError, StoreHandle};

/// Today's events, newest first. Read-only. Errors are logged by kind only.
#[tauri::command]
pub async fn list_events_today(app: AppHandle) -> Result<EventLog, EventError> {
    let today = Local::now().date_naive();
    let result = blocking(move || {
        app.state::<StoreHandle>()
            .with_store(today, |s| list_today(s, today))
    })
    .await;
    if let Err(e) = &result {
        if matches!(e, EventError::Store(StoreError::Busy)) {
            log::debug!("events: list busy");
        } else {
            log::warn!("events: list failed: {}", e.kind());
        }
    }
    result
}

/// Tells the UI that events were stored.
pub fn notify(app: &AppHandle) {
    if let Err(e) = app.emit(CHANGED_EVENT, ()) {
        log::warn!("events: emit failed: {e}");
    }
}
