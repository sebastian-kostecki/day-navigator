//! Tauri commands for the "To do" list: thin async wrappers over the domain.
//! The only file in `todo` that imports `tauri`.
//!
//! Each command runs on the blocking pool, reaches the store through an
//! `AppHandle` clone, reads the limit from `settings.json` on every call (an
//! edit takes effect without a restart) and returns the fresh list. Failures
//! are logged by kind only.

use std::path::PathBuf;

use chrono::Local;
use tauri::{AppHandle, Manager};

use super::{TodoError, TodoList};
use crate::blocking;
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

/// Runs `f(store, today, now, limit)` off the main thread, then returns the
/// fresh list from the same blocking closure.
async fn run(
    app: &AppHandle,
    name: &'static str,
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
    let result = blocking(move || {
        let limit = limit_from(path.as_ref());
        handle.state::<StoreHandle>().with_store(today, |store| {
            f(store, today, now, &limit)?;
            super::list(store, today, now, limit)
        })
    })
    .await;
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
    run(&app, "list", |_, _, _, _| Ok(())).await
}

#[tauri::command]
pub async fn add_todo_own_task(app: AppHandle, task_id: i64) -> Result<TodoList, TodoError> {
    run(&app, "add own", move |store, today, now, limit| {
        super::add_own(store, task_id, today, now, limit.clone()).map(|_| ())
    })
    .await
}

#[tauri::command]
pub async fn add_todo_gitlab_item(
    app: AppHandle,
    item_kind: ItemKind,
    reference: String,
) -> Result<TodoList, TodoError> {
    let snapshot = app.state::<GitlabRefreshState>().snapshot();
    let item = find_gitlab_item(snapshot.items.as_ref(), item_kind, &reference);
    run(&app, "add gitlab", move |store, today, now, limit| {
        let item = item.ok_or(TodoError::NotInGitlabList)?;
        super::add_gitlab(store, &item, today, now, limit.clone()).map(|_| ())
    })
    .await
}

#[tauri::command]
pub async fn set_todo_ticked(app: AppHandle, id: i64, ticked: bool) -> Result<TodoList, TodoError> {
    run(&app, "tick", move |store, today, now, _| {
        super::set_ticked(store, id, ticked, today, now)
    })
    .await
}

#[tauri::command]
pub async fn remove_todo(app: AppHandle, id: i64) -> Result<TodoList, TodoError> {
    run(&app, "remove", move |store, today, now, _| {
        super::remove(store, id, today, now)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(kind: ItemKind, reference: &str) -> GitlabItem {
        GitlabItem {
            kind,
            reference: reference.to_string(),
            title: "Invented".to_string(),
            web_url: None,
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            labels: vec![],
            draft: false,
        }
    }

    #[test]
    fn lookup_matches_kind_and_reference_in_the_snapshot_only() {
        let items = AssignedItems {
            merge_requests: vec![item(ItemKind::MergeRequest, "g/p!1")],
            issues: vec![item(ItemKind::Issue, "g/p#1")],
        };
        assert!(find_gitlab_item(Some(&items), ItemKind::MergeRequest, "g/p!1").is_some());
        assert!(find_gitlab_item(Some(&items), ItemKind::Issue, "g/p#1").is_some());
        assert!(find_gitlab_item(Some(&items), ItemKind::Issue, "g/p!1").is_none());
        assert!(find_gitlab_item(Some(&items), ItemKind::Issue, "g/p#2").is_none());
        assert!(find_gitlab_item(None, ItemKind::Issue, "g/p#1").is_none());
    }

    #[test]
    fn unresolved_config_dir_is_the_limit_error() {
        assert!(matches!(
            limit_from(None),
            Err(TodoError::ConfigDirUnresolved)
        ));
    }
}
