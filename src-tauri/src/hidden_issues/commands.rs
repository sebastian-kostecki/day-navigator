//! Tauri commands to hide, unhide and dismiss a GitLab issue by its `id`. The
//! only file in `hidden_issues` that imports `tauri`.
//!
//! The issue data comes from the in-memory full list, never from the UI. Each
//! command runs on the blocking pool; `GitlabRefreshState::act_on_issue` holds
//! `hidden_gate` across the store op and the snapshot store, and the new
//! snapshot is published after the gate is released. Errors are logged by kind
//! only.

use chrono::{Local, Utc};
use tauri::{AppHandle, Manager};

use super::{HiddenIssueError, HiddenRecord};
use crate::blocking;
use crate::cli::GitlabHost;
use crate::gitlab::GitlabItem;
use crate::gitlab_refresh::{GitlabRefreshState, publish};
use crate::store::{Store, StoreHandle};

/// Runs `op` on the store for issue `id`, re-partitions the kept list and
/// publishes the new snapshot.
async fn run(
    app: &AppHandle,
    name: &'static str,
    id: u64,
    op: impl FnOnce(&mut Store, &GitlabHost, &GitlabItem, i64) -> Result<(), HiddenIssueError>
    + Send
    + 'static,
) -> Result<(), HiddenIssueError> {
    let handle = app.clone();
    let today = Local::now().date_naive();
    let result = blocking(move || {
        let state = handle.state::<GitlabRefreshState>();
        let store = handle.state::<StoreHandle>();
        state.act_on_issue(
            id,
            |host, item| -> Result<Vec<HiddenRecord>, HiddenIssueError> {
                store.with_store(today, |s| {
                    op(s, host, item, Utc::now().timestamp_millis())?;
                    super::list(s, host)
                })
            },
        )
    })
    .await;
    match &result {
        Ok(()) => publish(app, &app.state::<GitlabRefreshState>()),
        Err(e @ HiddenIssueError::Store(_)) => {
            log::warn!("hidden_issues: {name} failed: {}", e.kind());
        }
        Err(e) => log::debug!("hidden_issues: {name} refused: {}", e.kind()),
    }
    result
}

/// Hides an issue of the current list with a fresh snapshot (also replaces a mark).
#[tauri::command]
pub async fn hide_gitlab_issue(app: AppHandle, id: u64) -> Result<(), HiddenIssueError> {
    run(&app, "hide", id, |store, host, item, now| {
        super::hide(store, host, item, now)
    })
    .await
}

/// Shows a hidden issue again, unmarked.
#[tauri::command]
pub async fn unhide_gitlab_issue(app: AppHandle, id: u64) -> Result<(), HiddenIssueError> {
    run(&app, "unhide", id, |store, host, item, _| {
        super::unhide(store, host, item.id)
    })
    .await
}

/// Dismisses the "changed" mark of a returned issue.
#[tauri::command]
pub async fn dismiss_gitlab_change(app: AppHandle, id: u64) -> Result<(), HiddenIssueError> {
    run(&app, "dismiss", id, |store, host, item, _| {
        super::dismiss(store, host, item.id)
    })
    .await
}
