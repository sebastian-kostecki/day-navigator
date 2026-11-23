//! Refusals and failures of the "To do" domain, serialized for the UI.
//!
//! Invariant: no variant carries a title, reference, URL or host.

use serde::Serialize;

use crate::own_tasks::OwnTaskError;
use crate::store::StoreError;

#[derive(Debug, Clone, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all_fields = "camelCase")]
pub enum TodoError {
    #[error("app data store error: {0}")]
    Store(StoreError),
    #[error("the list already has {limit} open items")]
    LimitReached { limit: u32 },
    #[error("the item is already on the list")]
    AlreadyOnList,
    #[error("the task is already done")]
    TaskDone,
    #[error("list entry or task not found")]
    NotFound,
    #[error("the item is not in the current gitlab list")]
    NotInGitlabList,
    #[error("the item is not in the current github list")]
    NotInGithubList,
    #[error("the todo settings block is invalid")]
    SettingsInvalid,
    #[error("the app config dir cannot be resolved")]
    ConfigDirUnresolved,
    #[error("background task failed")]
    Background,
}

impl TodoError {
    /// Variant name, or the store error's kind for `Store`, for logs (carries
    /// no content).
    pub fn kind(&self) -> &'static str {
        match self {
            // The store's own kind (`Busy`, `Damaged`, …) tells failures apart.
            TodoError::Store(s) => s.kind(),
            TodoError::LimitReached { .. } => "LimitReached",
            TodoError::AlreadyOnList => "AlreadyOnList",
            TodoError::TaskDone => "TaskDone",
            TodoError::NotFound => "NotFound",
            TodoError::NotInGitlabList => "NotInGitlabList",
            TodoError::NotInGithubList => "NotInGithubList",
            TodoError::SettingsInvalid => "SettingsInvalid",
            TodoError::ConfigDirUnresolved => "ConfigDirUnresolved",
            TodoError::Background => "Background",
        }
    }
}

impl From<crate::BackgroundFailed> for TodoError {
    fn from(_: crate::BackgroundFailed) -> Self {
        TodoError::Background
    }
}

impl From<StoreError> for TodoError {
    fn from(e: StoreError) -> Self {
        TodoError::Store(e)
    }
}

impl From<rusqlite::Error> for TodoError {
    fn from(e: rusqlite::Error) -> Self {
        TodoError::Store(StoreError::from(e))
    }
}

impl From<OwnTaskError> for TodoError {
    fn from(e: OwnTaskError) -> Self {
        match e {
            OwnTaskError::Store(e) => TodoError::Store(e),
            OwnTaskError::NotFound => TodoError::NotFound,
            // `set_done_in` only reports store errors and `NotFound`; anything
            // else stays value-free.
            _ => TodoError::Store(StoreError::Sql {
                reason: "own task state unreadable".to_string(),
            }),
        }
    }
}
