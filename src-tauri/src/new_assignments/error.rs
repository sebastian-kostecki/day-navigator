//! Failures of the new-assignment store operation, serialized for logs.
//!
//! Invariant (same as `StoreError`): no variant carries ids or any other row
//! value.

use serde::Serialize;

use crate::store::StoreError;

#[derive(Debug, Clone, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum NewAssignmentError {
    #[error("app data store error: {0}")]
    Store(StoreError),
}

impl From<StoreError> for NewAssignmentError {
    fn from(e: StoreError) -> Self {
        NewAssignmentError::Store(e)
    }
}

impl From<rusqlite::Error> for NewAssignmentError {
    fn from(e: rusqlite::Error) -> Self {
        NewAssignmentError::Store(StoreError::from(e))
    }
}

impl NewAssignmentError {
    /// The variant name, for logs (carries no content).
    pub fn kind(&self) -> &'static str {
        match self {
            NewAssignmentError::Store(e) => e.kind(),
        }
    }
}
