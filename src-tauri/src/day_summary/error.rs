//! Failures of the day summary read, serialized for the UI.
//!
//! Invariant: no variant carries a title, reference, time or path.

use serde::Serialize;

use crate::store::StoreError;
use crate::todo::TodoError;
use crate::work_time::WorkTimeError;

#[derive(Debug, Clone, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum DaySummaryError {
    #[error("app data store error: {0}")]
    Store(StoreError),
    #[error("local time is invalid or does not exist")]
    InvalidLocalTime,
    #[error("background task failed")]
    Background,
}

impl DaySummaryError {
    /// The variant name (the store's own kind for `Store`), for logs.
    pub fn kind(&self) -> &'static str {
        match self {
            DaySummaryError::Store(s) => s.kind(),
            DaySummaryError::InvalidLocalTime => "InvalidLocalTime",
            DaySummaryError::Background => "Background",
        }
    }
}

impl From<StoreError> for DaySummaryError {
    fn from(e: StoreError) -> Self {
        DaySummaryError::Store(e)
    }
}

impl From<crate::BackgroundFailed> for DaySummaryError {
    fn from(_: crate::BackgroundFailed) -> Self {
        DaySummaryError::Background
    }
}

impl From<WorkTimeError> for DaySummaryError {
    fn from(e: WorkTimeError) -> Self {
        match e {
            WorkTimeError::Store(e) => DaySummaryError::Store(e),
            WorkTimeError::InvalidLocalTime => DaySummaryError::InvalidLocalTime,
            WorkTimeError::Background => DaySummaryError::Background,
            _ => DaySummaryError::Store(StoreError::Sql {
                reason: "work-time state unreadable".into(),
            }),
        }
    }
}

impl From<TodoError> for DaySummaryError {
    fn from(e: TodoError) -> Self {
        match e {
            TodoError::Store(e) => DaySummaryError::Store(e),
            TodoError::Background => DaySummaryError::Background,
            _ => DaySummaryError::Store(StoreError::Sql {
                reason: "todo state unreadable".into(),
            }),
        }
    }
}
