//! Typed refusals and failures for the app data store.
//!
//! Invariant: no variant carries row values or absolute paths. Reasons are
//! built from the SQLite error code and message only; never format bound
//! parameters or row data into an error.

use rusqlite::ErrorCode;
use serde::Serialize;

#[derive(Debug, Clone, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum StoreError {
    #[error("data folder unavailable: {kind}")]
    DirUnavailable { kind: String },
    #[error("data file is damaged: {reason}")]
    Damaged { reason: String },
    #[error("data file is missing although backups exist")]
    Missing,
    #[error("data file is from a newer app version (found {found}, supported {supported})")]
    NewerSchema { found: u32, supported: u32 },
    #[error("migration {version} failed: {reason}")]
    MigrationFailed { version: u32, reason: String },
    #[error("data file is busy")]
    Busy,
    #[error("data file I/O error: {kind}")]
    Io { kind: String },
    #[error("database error: {reason}")]
    Sql { reason: String },
}

/// Reason text from the SQLite error code and message only.
pub(crate) fn reason_of(e: &rusqlite::Error) -> String {
    match e {
        rusqlite::Error::SqliteFailure(err, msg) => match msg {
            Some(m) => format!("{:?}: {m}", err.code),
            None => format!("{:?}", err.code),
        },
        // Other variants may embed values (e.g. a failed type conversion), so
        // only a fixed label is kept.
        rusqlite::Error::QueryReturnedNoRows => "no rows returned".to_string(),
        _ => "sqlite error".to_string(),
    }
}

impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        if let rusqlite::Error::SqliteFailure(err, _) = &e {
            match err.code {
                ErrorCode::NotADatabase | ErrorCode::DatabaseCorrupt => {
                    return StoreError::Damaged {
                        reason: reason_of(&e),
                    };
                }
                ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked => return StoreError::Busy,
                ErrorCode::CannotOpen
                | ErrorCode::ReadOnly
                | ErrorCode::SystemIoFailure
                | ErrorCode::DiskFull => {
                    return StoreError::Io {
                        kind: format!("{:?}", err.code),
                    };
                }
                _ => {}
            }
        }
        StoreError::Sql {
            reason: reason_of(&e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::ffi;

    fn failure(code: i32) -> rusqlite::Error {
        rusqlite::Error::SqliteFailure(ffi::Error::new(code), None)
    }

    #[test]
    fn maps_sqlite_codes_to_variants() {
        assert!(matches!(
            StoreError::from(failure(ffi::SQLITE_NOTADB)),
            StoreError::Damaged { .. }
        ));
        assert!(matches!(
            StoreError::from(failure(ffi::SQLITE_CORRUPT)),
            StoreError::Damaged { .. }
        ));
        assert!(matches!(
            StoreError::from(failure(ffi::SQLITE_BUSY)),
            StoreError::Busy
        ));
        assert!(matches!(
            StoreError::from(failure(ffi::SQLITE_LOCKED)),
            StoreError::Busy
        ));
        for code in [
            ffi::SQLITE_CANTOPEN,
            ffi::SQLITE_READONLY,
            ffi::SQLITE_IOERR,
            ffi::SQLITE_FULL,
        ] {
            assert!(matches!(
                StoreError::from(failure(code)),
                StoreError::Io { .. }
            ));
        }
        assert!(matches!(
            StoreError::from(failure(ffi::SQLITE_CONSTRAINT)),
            StoreError::Sql { .. }
        ));
    }
}
