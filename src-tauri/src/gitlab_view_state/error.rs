//! Failures of the GitLab view state operations, serialized for logs and the UI.
//!
//! Invariant (same as `StoreError`): no variant carries row values.

use serde::Serialize;

use crate::store::StoreError;

#[derive(Debug, Clone, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum GitlabViewStateError {
    #[error("app data store error: {0}")]
    Store(StoreError),
    /// The blocking task of a command could not finish (no content).
    #[error("background task failed")]
    Background,
}

impl From<crate::BackgroundFailed> for GitlabViewStateError {
    fn from(_: crate::BackgroundFailed) -> Self {
        GitlabViewStateError::Background
    }
}

impl From<StoreError> for GitlabViewStateError {
    fn from(e: StoreError) -> Self {
        GitlabViewStateError::Store(e)
    }
}

impl From<rusqlite::Error> for GitlabViewStateError {
    fn from(e: rusqlite::Error) -> Self {
        GitlabViewStateError::Store(StoreError::from(e))
    }
}

impl GitlabViewStateError {
    /// The variant name, for logs (carries no content).
    pub fn kind(&self) -> &'static str {
        match self {
            GitlabViewStateError::Store(e) => e.kind(),
            GitlabViewStateError::Background => "Background",
        }
    }
}
