//! The `day_summary` command: a one-shot read the window calls after its own
//! STOP. The only file in `day_summary` that imports `tauri`.
//!
//! No emit (a read). Errors are logged by kind only, never a title or value.

use chrono::{Local, Utc};
use tauri::{AppHandle, Manager};

use super::{DaySummary, DaySummaryError};
use crate::blocking;
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
