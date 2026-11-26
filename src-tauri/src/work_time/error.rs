//! Refusals and failures of the work-time domain, serialized for the UI.
//!
//! Invariant (same as `StoreError`): no variant carries times or row values.
//! An overlap names the other entry by id only.

use serde::Serialize;

use crate::store::StoreError;

#[derive(Debug, Clone, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all_fields = "camelCase")]
pub enum WorkTimeError {
    #[error("app data store error: {0}")]
    Store(StoreError),
    #[error("work time is already running")]
    AlreadyRunning,
    #[error("work time is not running")]
    NotRunning,
    #[error("work entry not found")]
    NotFound,
    #[error("end is not after start")]
    EndNotAfterStart,
    #[error("time is in the future")]
    InFuture,
    #[error("entry overlaps entry {entry_id}")]
    Overlap { entry_id: i64 },
    #[error("local time is invalid or does not exist")]
    InvalidLocalTime,
    #[error("a running entry has no end; use stop")]
    RunningEndEdit,
    #[error("a weekend is not a workday")]
    NotAWorkday,
    #[error("a public holiday is not a workday")]
    Holiday,
    #[error("background task failed")]
    Background,
}

impl WorkTimeError {
    /// The variant name, for logs (carries no content).
    pub fn kind(&self) -> &'static str {
        match self {
            WorkTimeError::Store(s) => s.kind(),
            WorkTimeError::AlreadyRunning => "AlreadyRunning",
            WorkTimeError::NotRunning => "NotRunning",
            WorkTimeError::NotFound => "NotFound",
            WorkTimeError::EndNotAfterStart => "EndNotAfterStart",
            WorkTimeError::InFuture => "InFuture",
            WorkTimeError::Overlap { .. } => "Overlap",
            WorkTimeError::InvalidLocalTime => "InvalidLocalTime",
            WorkTimeError::RunningEndEdit => "RunningEndEdit",
            WorkTimeError::NotAWorkday => "NotAWorkday",
            WorkTimeError::Holiday => "Holiday",
            WorkTimeError::Background => "Background",
        }
    }
}

impl From<crate::BackgroundFailed> for WorkTimeError {
    fn from(_: crate::BackgroundFailed) -> Self {
        WorkTimeError::Background
    }
}

impl From<StoreError> for WorkTimeError {
    fn from(e: StoreError) -> Self {
        WorkTimeError::Store(e)
    }
}

impl From<rusqlite::Error> for WorkTimeError {
    fn from(e: rusqlite::Error) -> Self {
        WorkTimeError::Store(StoreError::from(e))
    }
}
