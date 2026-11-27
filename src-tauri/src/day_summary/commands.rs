//! The `day_summary` command: a one-shot read the window calls after its own
//! STOP. The only file in `day_summary` that imports `tauri`.
//!
//! No emit (a read). Errors are logged by kind only, never a title or value.

use chrono::{Local, Utc};
use tauri::{AppHandle, Manager};

use super::{DaySummary, DaySummaryError, notice};
use crate::blocking;
use crate::health::Notice;
use crate::settings_file;
use crate::store::StoreHandle;
use crate::work_time::commands::{Policy, work_time_settings};

#[tauri::command]
pub async fn day_summary(app: AppHandle) -> Result<DaySummary, DaySummaryError> {
    let path = settings_file::settings_path(&app);
    let handle = app.clone();
    let today = Local::now().date_naive();
    let result = blocking(move || {
        let settings = work_time_settings(path.as_deref(), Policy::Lenient)?;
        handle.state::<StoreHandle>().with_store(today, |store| {
            super::read(store, Utc::now().timestamp_millis(), &Local, &settings)
        })
    })
    .await;
    if let Err(e) = &result {
        log::warn!("day_summary: read failed: {}", e.kind());
    }
    result
}

/// The toast after a committed tray STOP: today's time and the month's
/// deviation from `work_time::status` only (no todo write). The store lock is
/// released before this returns. A failed read gives the figure-free text.
pub(crate) async fn stop_notice(app: &AppHandle) -> Notice {
    let path = settings_file::settings_path(app);
    let handle = app.clone();
    let today = Local::now().date_naive();
    let result = blocking(move || {
        let settings = work_time_settings(path.as_deref(), Policy::Lenient)?;
        handle.state::<StoreHandle>().with_store(today, |store| {
            crate::work_time::status(store, Utc::now().timestamp_millis(), &Local, &settings)
        })
    })
    .await;
    match result {
        Ok(status) => notice::stop_notice(super::today_ms(&status), status.month_deviation_ms),
        Err(e) => {
            let e = DaySummaryError::from(e);
            log::warn!("day_summary: stop figures failed: {}", e.kind());
            notice::stop_notice_unavailable()
        }
    }
}
