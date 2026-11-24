//! Failures of the event-log operations, serialized for logs and the UI.
//!
//! Invariant (same as `StoreError`): no variant carries a row value.

use serde::Serialize;

use crate::store::StoreError;

#[derive(Debug, Clone, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum EventError {
    #[error("app data store error: {0}")]
    Store(StoreError),
    /// The blocking task of a command could not finish (no content).
    #[error("background task failed")]
    Background,
}

impl From<crate::BackgroundFailed> for EventError {
    fn from(_: crate::BackgroundFailed) -> Self {
        EventError::Background
    }
}

impl From<StoreError> for EventError {
    fn from(e: StoreError) -> Self {
        EventError::Store(e)
    }
}

impl From<rusqlite::Error> for EventError {
    fn from(e: rusqlite::Error) -> Self {
        EventError::Store(StoreError::from(e))
    }
}

impl EventError {
    /// The variant name, for logs (carries no content).
    pub fn kind(&self) -> &'static str {
        match self {
            EventError::Store(e) => e.kind(),
            EventError::Background => "Background",
        }
    }
}
