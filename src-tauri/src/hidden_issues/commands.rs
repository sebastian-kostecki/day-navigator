//! Tauri commands to hide, unhide and dismiss an issue: a GitLab issue by its
//! `id`, a GitHub issue by its `reference`. The only file in `hidden_issues`
//! that imports `tauri`.
//!
//! The issue data comes from the in-memory full list, never from the UI. Each
//! command runs on the blocking pool; `act_on_issue` of the source's refresh
//! state holds `hidden_gate` across the store op and the snapshot store, and the
//! new snapshot is published after the gate is released. Errors are logged by
//! kind only.

use chrono::{Local, Utc};
use tauri::{AppHandle, Manager};

use super::items::{
    HiddenItemKey, HiddenItemRecord, HiddenKind, HiddenSource, NewHiddenItem, dismiss_item,
    hide_item, list_items, unhide_item,
};
use super::rules::HideItem;
use super::{HiddenIssueError, HiddenRecord};
use crate::blocking;
use crate::cli::{GithubRepo, GitlabHost};
use crate::github::GithubIssue;
use crate::github_refresh::{GithubRefreshState, publish as github_publish};
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

/// Runs `op` on the store for the GitHub issue `reference`, re-partitions the
/// kept list and publishes the new snapshot.
async fn run_github(
    app: &AppHandle,
    name: &'static str,
    reference: String,
    op: impl FnOnce(&mut Store, &GithubRepo, &GithubIssue, i64) -> Result<(), HiddenIssueError>
    + Send
    + 'static,
) -> Result<(), HiddenIssueError> {
    let handle = app.clone();
    let today = Local::now().date_naive();
    let result = blocking(move || {
        let state = handle.state::<GithubRefreshState>();
        let store = handle.state::<StoreHandle>();
        state.act_on_issue(
            &reference,
            |repo, issue| -> Result<Vec<HiddenItemRecord>, HiddenIssueError> {
                store.with_store(today, |s| {
                    op(s, repo, issue, Utc::now().timestamp_millis())?;
                    list_items(s, HiddenSource::Github, HiddenKind::Issue)
                })
            },
        )
    })
    .await;
    match &result {
        Ok(()) => github_publish(app, &app.state::<GithubRefreshState>()),
        Err(e @ HiddenIssueError::Store(_)) => {
            log::warn!("hidden_issues: github {name} failed: {}", e.kind());
        }
        Err(e) => log::debug!("hidden_issues: github {name} refused: {}", e.kind()),
    }
    result
}

fn github_key(repo: &GithubRepo, issue: &GithubIssue) -> HiddenItemKey {
    HiddenItemKey {
        source: HiddenSource::Github,
        scope: repo.key(),
        kind: HiddenKind::Issue,
        item_id: issue.number,
    }
}

/// Hides a GitHub issue of the current list with a fresh snapshot (also
/// replaces a mark).
#[tauri::command]
pub async fn hide_github_issue(app: AppHandle, reference: String) -> Result<(), HiddenIssueError> {
    run_github(&app, "hide", reference, |store, repo, issue, now| {
        let item = NewHiddenItem {
            source: HiddenSource::Github,
            scope: repo.key(),
            kind: HiddenKind::Issue,
            item_id: issue.number,
            project_id: None,
            iid: None,
            status_fingerprint: issue.status_fingerprint(),
            baseline_updated_at: issue.updated_at.clone(),
        };
        hide_item(store, &item, now)
    })
    .await
}

/// Shows a hidden GitHub issue again, unmarked.
#[tauri::command]
pub async fn unhide_github_issue(
    app: AppHandle,
    reference: String,
) -> Result<(), HiddenIssueError> {
    run_github(&app, "unhide", reference, |store, repo, issue, _| {
        unhide_item(store, &github_key(repo, issue))
    })
    .await
}

/// Dismisses the "changed" mark of a returned GitHub issue.
#[tauri::command]
pub async fn dismiss_github_change(
    app: AppHandle,
    reference: String,
) -> Result<(), HiddenIssueError> {
    run_github(&app, "dismiss", reference, |store, repo, issue, _| {
        dismiss_item(store, &github_key(repo, issue))
    })
    .await
}
