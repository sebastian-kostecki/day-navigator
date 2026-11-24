//! Tauri commands for the "To do" list: thin async wrappers over the domain.
//! The only file in `todo` that imports `tauri`.
//!
//! Each command runs on the blocking pool, reaches the store through an
//! `AppHandle` clone, reads the limit from `settings.json` on every call (an
//! edit takes effect without a restart) and returns the fresh list. A
//! successful mutation emits `CHANGED_EVENT`; `list_todo` never does. Failures
//! are logged by kind only.

use std::path::PathBuf;

use chrono::Local;
use tauri::{AppHandle, Emitter, Manager};

use super::{CHANGED_EVENT, TodoError, TodoList};
use crate::blocking;
use crate::github::{GithubIssue, GithubSnapshot};
use crate::github_refresh::GithubRefreshState;
use crate::gitlab::{AssignedItems, GitlabItem, ItemKind};
use crate::gitlab_refresh::GitlabRefreshState;
use crate::settings::load_todo_limit;
use crate::store::{Store, StoreHandle};

/// `settings.json` in the app config dir, or `None` when the dir is unresolved.
fn settings_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join("settings.json"))
}

fn limit_from(path: Option<&PathBuf>) -> Result<u32, TodoError> {
    match path {
        Some(p) => load_todo_limit(p),
        None => Err(TodoError::ConfigDirUnresolved),
    }
}

/// The picked item as the live snapshot knows it, never from UI-supplied text.
fn find_gitlab_item(
    items: Option<&AssignedItems>,
    kind: ItemKind,
    reference: &str,
) -> Option<GitlabItem> {
    let items = items?;
    let pool = match kind {
        ItemKind::MergeRequest => &items.merge_requests,
        ItemKind::Issue => &items.issues,
    };
    pool.iter()
        .find(|i| i.kind == kind && i.reference == reference)
        .cloned()
}

/// The picked issue as the live snapshot knows it (by lowercase reference),
/// never from UI-supplied text.
fn find_github_issue(snapshot: &GithubSnapshot, reference: &str) -> Option<GithubIssue> {
    let reference = reference.to_lowercase();
    snapshot
        .repos
        .iter()
        .flat_map(|r| r.issues.iter())
        .find(|i| i.reference == reference)
        .cloned()
}

/// Whether a command changes the list. Only a mutation emits `CHANGED_EVENT`,
/// so a listener that reloads through `list_todo` cannot loop.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Call {
    Read,
    Mutation,
}

/// Runs `f(store, today, now, limit)` off the main thread, then returns the
/// fresh list from the same blocking closure. After a mutation whose `f`
/// succeeded, emits `CHANGED_EVENT` even when the list read then failed: `f`
/// committed in its own write.
async fn run(
    app: &AppHandle,
    name: &'static str,
    call: Call,
    f: impl FnOnce(
        &mut Store,
        chrono::NaiveDate,
        chrono::NaiveDateTime,
        &Result<u32, TodoError>,
    ) -> Result<(), TodoError>
    + Send
    + 'static,
) -> Result<TodoList, TodoError> {
    let handle = app.clone();
    let path = settings_path(app);
    let now = Local::now().naive_local();
    let today = now.date();
    // Outer `Ok`: `f` succeeded; the inner result is the list read after it.
    let outcome = blocking(move || {
        let limit = limit_from(path.as_ref());
        handle.state::<StoreHandle>().with_store(today, |store| {
            f(store, today, now, &limit)?;
            Ok::<_, TodoError>(super::list(store, today, now, limit))
        })
    })
    .await;
    if call == Call::Mutation
        && outcome.is_ok()
        && let Err(e) = app.emit(CHANGED_EVENT, ())
    {
        log::warn!("todo: emit failed: {e}");
    }
    let result = outcome.and_then(|list| list);
    if let Err(e) = &result {
        match e {
            TodoError::Store(_) | TodoError::Background => {
                log::warn!("todo: {name} failed: {}", e.kind());
            }
            _ => log::debug!("todo: {name} refused: {}", e.kind()),
        }
    }
    result
}

#[tauri::command]
pub async fn list_todo(app: AppHandle) -> Result<TodoList, TodoError> {
    run(&app, "list", Call::Read, |_, _, _, _| Ok(())).await
}

#[tauri::command]
pub async fn add_todo_own_task(app: AppHandle, task_id: i64) -> Result<TodoList, TodoError> {
    run(
        &app,
        "add own",
        Call::Mutation,
        move |store, today, now, limit| {
            super::add_own(store, task_id, today, now, limit.clone()).map(|_| ())
        },
    )
    .await
}

#[tauri::command]
pub async fn add_todo_gitlab_item(
    app: AppHandle,
    item_kind: ItemKind,
    reference: String,
) -> Result<TodoList, TodoError> {
    // Items and account come from one snapshot, so a pick is never stamped
    // with an account its item did not come from.
    let snapshot = app.state::<GitlabRefreshState>().snapshot();
    let item = find_gitlab_item(snapshot.items.as_ref(), item_kind, &reference);
    let account = snapshot.account;
    run(
        &app,
        "add gitlab",
        Call::Mutation,
        move |store, today, now, limit| {
            let (item, account) = item.zip(account).ok_or(TodoError::NotInGitlabList)?;
            super::add_gitlab(store, &account, &item, today, now, limit.clone()).map(|_| ())
        },
    )
    .await
}

#[tauri::command]
pub async fn add_todo_github_item(
    app: AppHandle,
    reference: String,
) -> Result<TodoList, TodoError> {
    let issue = find_github_issue(&app.state::<GithubRefreshState>().snapshot(), &reference);
    run(
        &app,
        "add github",
        Call::Mutation,
        move |store, today, now, limit| {
            let issue = issue.ok_or(TodoError::NotInGithubList)?;
            super::add_github(store, &issue, today, now, limit.clone()).map(|_| ())
        },
    )
    .await
}

#[tauri::command]
pub async fn set_todo_ticked(app: AppHandle, id: i64, ticked: bool) -> Result<TodoList, TodoError> {
    run(&app, "tick", Call::Mutation, move |store, today, now, _| {
        super::set_ticked(store, id, ticked, today, now)
    })
    .await
}

#[tauri::command]
pub async fn remove_todo(app: AppHandle, id: i64) -> Result<TodoList, TodoError> {
    run(
        &app,
        "remove",
        Call::Mutation,
        move |store, today, now, _| super::remove(store, id, today, now),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(kind: ItemKind, reference: &str) -> GitlabItem {
        GitlabItem {
            kind,
            id: 1,
            project_id: 1,
            iid: 1,
            reference: reference.to_string(),
            title: "Invented".to_string(),
            web_url: None,
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            labels: vec![],
            draft: false,
            changed: None,
            milestone: None,
        }
    }

    #[test]
    fn lookup_matches_kind_and_reference_in_the_snapshot_only() {
        let items = AssignedItems {
            merge_requests: vec![item(ItemKind::MergeRequest, "g/p!1")],
            issues: vec![item(ItemKind::Issue, "g/p#1")],
            hidden_issues: vec![],
        };
        assert!(find_gitlab_item(Some(&items), ItemKind::MergeRequest, "g/p!1").is_some());
        assert!(find_gitlab_item(Some(&items), ItemKind::Issue, "g/p#1").is_some());
        assert!(find_gitlab_item(Some(&items), ItemKind::Issue, "g/p!1").is_none());
        assert!(find_gitlab_item(Some(&items), ItemKind::Issue, "g/p#2").is_none());
        assert!(find_gitlab_item(None, ItemKind::Issue, "g/p#1").is_none());
    }

    #[test]
    fn github_lookup_uses_the_snapshot_by_lowercase_reference_only() {
        use crate::github::RepoView;
        let issue = GithubIssue {
            repo: "Octo-Test/Alpha".to_string(),
            number: 1,
            reference: "octo-test/alpha#1".to_string(),
            title: "Invented".to_string(),
            web_url: None,
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            labels: vec![],
        };
        let snapshot = GithubSnapshot {
            repos: vec![RepoView {
                repo: "Octo-Test/Alpha".to_string(),
                issues: vec![issue],
                error: None,
            }],
            ..GithubSnapshot::starting()
        };
        assert!(find_github_issue(&snapshot, "octo-test/alpha#1").is_some());
        assert!(find_github_issue(&snapshot, "Octo-Test/Alpha#1").is_some());
        assert!(find_github_issue(&snapshot, "octo-test/alpha#2").is_none());
        assert!(find_github_issue(&GithubSnapshot::starting(), "octo-test/alpha#1").is_none());
    }

    #[test]
    fn unresolved_config_dir_is_the_limit_error() {
        assert!(matches!(
            limit_from(None),
            Err(TodoError::ConfigDirUnresolved)
        ));
    }
}
