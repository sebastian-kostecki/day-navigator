//! Tauri commands to hide, unhide and dismiss an item: a GitLab issue or merge
//! request by its global `id`, a GitHub issue by its `reference`. The only file
//! in `hidden_issues` that imports `tauri`.
//!
//! The item data comes from the in-memory full list, never from the UI. Each
//! command runs on the blocking pool; `act_on_item` (GitLab) and `act_on_issue`
//! (GitHub) of the source's refresh state hold `hidden_gate` across the store op
//! and the snapshot store, and the
//! new snapshot is published after the gate is released. Errors are logged by
//! kind only.

use chrono::{Local, Utc};
use tauri::{AppHandle, Manager};

use super::HiddenIssueError;
use super::items::{
    HiddenItemKey, HiddenItemRecord, HiddenKind, HiddenSource, NewHiddenItem, dismiss_item,
    hide_item, list_items, unhide_item,
};
use super::merge_requests::{GitlabHidden, list_gitlab, mr_key, new_hidden_mr};
use super::rules::HideItem;
use crate::blocking;
use crate::cli::{GithubRepo, GitlabHost};
use crate::github::GithubIssue;
use crate::github_refresh::{GithubRefreshState, publish as github_publish};
use crate::gitlab::{GitlabItem, ItemKind};
use crate::gitlab_refresh::{GitlabRefreshState, publish};
use crate::store::{Store, StoreHandle};

/// Runs `op` on the store for the GitLab item `id` of `kind`, re-partitions the
/// kept list from the records of both kinds and publishes the new snapshot.
async fn run(
    app: &AppHandle,
    name: &'static str,
    kind: ItemKind,
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
        state.act_on_item(
            kind,
            id,
            |host, item| -> Result<GitlabHidden, HiddenIssueError> {
                store.with_store(today, |s| {
                    op(s, host, item, Utc::now().timestamp_millis())?;
                    list_gitlab(s, host)
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
    run(&app, "hide", ItemKind::Issue, id, super::hide).await
}

/// Shows a hidden issue again, unmarked.
#[tauri::command]
pub async fn unhide_gitlab_issue(app: AppHandle, id: u64) -> Result<(), HiddenIssueError> {
    run(
        &app,
        "unhide",
        ItemKind::Issue,
        id,
        |store, host, item, _| super::unhide(store, host, item.id),
    )
    .await
}

/// Dismisses the "changed" mark of a returned issue.
#[tauri::command]
pub async fn dismiss_gitlab_change(app: AppHandle, id: u64) -> Result<(), HiddenIssueError> {
    run(
        &app,
        "dismiss",
        ItemKind::Issue,
        id,
        |store, host, item, _| super::dismiss(store, host, item.id),
    )
    .await
}

/// Hides a merge request (assigned or awaiting the user's review) of the current
/// list with a fresh snapshot (also replaces a mark).
#[tauri::command]
pub async fn hide_gitlab_merge_request(app: AppHandle, id: u64) -> Result<(), HiddenIssueError> {
    run(
        &app,
        "hide-mr",
        ItemKind::MergeRequest,
        id,
        |store, host, item, now| hide_item(store, &new_hidden_mr(host, item), now),
    )
    .await
}

/// Shows a hidden merge request again, unmarked.
#[tauri::command]
pub async fn unhide_gitlab_merge_request(app: AppHandle, id: u64) -> Result<(), HiddenIssueError> {
    run(
        &app,
        "unhide-mr",
        ItemKind::MergeRequest,
        id,
        |store, host, item, _| unhide_item(store, &mr_key(host, item.id)),
    )
    .await
}

/// Dismisses the "changed" mark of a returned merge request.
#[tauri::command]
pub async fn dismiss_gitlab_merge_request_change(
    app: AppHandle,
    id: u64,
) -> Result<(), HiddenIssueError> {
    run(
        &app,
        "dismiss-mr",
        ItemKind::MergeRequest,
        id,
        |store, host, item, _| dismiss_item(store, &mr_key(host, item.id)),
    )
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
