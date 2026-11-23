//! Refusals and failures of the hidden-issue domain, serialized for the UI.
//!
//! Invariant (same as `StoreError`): no variant carries ids, labels, the host
//! or any other row value.

use serde::Serialize;

use crate::store::StoreError;

#[derive(Debug, Clone, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum HiddenIssueError {
    #[error("app data store error: {0}")]
    Store(StoreError),
    #[error("hidden issue record not found")]
    NotFound,
    #[error("the issue is not in the current list")]
    NotInList,
    #[error("there is no GitLab list to act on yet")]
    ListUnavailable,
}

impl From<StoreError> for HiddenIssueError {
    fn from(e: StoreError) -> Self {
        HiddenIssueError::Store(e)
    }
}

impl From<rusqlite::Error> for HiddenIssueError {
    fn from(e: rusqlite::Error) -> Self {
        HiddenIssueError::Store(StoreError::from(e))
    }
}
