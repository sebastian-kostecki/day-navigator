//! Failures of the GitHub seen-set operations, serialized for logs and the UI.
//!
//! Invariant (same as `StoreError`): no variant carries row values.

use serde::Serialize;

use crate::store::StoreError;

#[derive(Debug, Clone, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum GithubSeenError {
    #[error("app data store error: {0}")]
    Store(StoreError),
    /// The blocking task of a command could not finish (no content).
    #[error("background task failed")]
    Background,
}

impl From<crate::BackgroundFailed> for GithubSeenError {
    fn from(_: crate::BackgroundFailed) -> Self {
        GithubSeenError::Background
    }
}

impl From<StoreError> for GithubSeenError {
    fn from(e: StoreError) -> Self {
        GithubSeenError::Store(e)
    }
}

impl From<rusqlite::Error> for GithubSeenError {
    fn from(e: rusqlite::Error) -> Self {
        GithubSeenError::Store(StoreError::from(e))
    }
}

impl GithubSeenError {
    /// The variant name, for logs (carries no content).
    pub fn kind(&self) -> &'static str {
        match self {
            GithubSeenError::Store(e) => e.kind(),
            GithubSeenError::Background => "Background",
        }
    }
}
