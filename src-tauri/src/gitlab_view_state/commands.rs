//! Tauri commands to read and save the remembered GitLab view state. The only
//! file in `gitlab_view_state` that imports `tauri`. Errors are logged by kind
//! only, never with the values.

use chrono::Local;
use tauri::{AppHandle, Manager};

use super::{GitlabViewState, GitlabViewStateError, load, save};
use crate::blocking;
use crate::store::StoreHandle;

fn log_failure(what: &str, e: &GitlabViewStateError) {
    let line = format!("gitlab_view_state: {what} failed: {}", e.kind());
    if e.kind() == "Busy" {
        log::debug!("{line}");
    } else {
        log::warn!("{line}");
    }
}

/// The remembered view state (the defaults when nothing was saved yet).
#[tauri::command]
pub async fn gitlab_view_state(app: AppHandle) -> Result<GitlabViewState, GitlabViewStateError> {
    let today = Local::now().date_naive();
    let result = blocking(move || app.state::<StoreHandle>().with_store(today, |s| load(s))).await;
    if let Err(e) = &result {
        log_failure("load", e);
    }
    result
}

/// Saves the view state (one upsert).
#[tauri::command]
pub async fn set_gitlab_view_state(
    app: AppHandle,
    view: GitlabViewState,
) -> Result<(), GitlabViewStateError> {
    let today = Local::now().date_naive();
    let result = blocking(move || {
        app.state::<StoreHandle>()
            .with_store(today, |s| save(s, view))
    })
    .await;
    if let Err(e) = &result {
        log_failure("save", e);
    }
    result
}
