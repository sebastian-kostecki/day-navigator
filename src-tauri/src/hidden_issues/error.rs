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

/// A blocking task that could not finish is reported as a store I/O failure,
/// like `on_store` does for the own-task commands.
impl From<crate::BackgroundFailed> for HiddenIssueError {
    fn from(_: crate::BackgroundFailed) -> Self {
        HiddenIssueError::Store(StoreError::Io {
            kind: "background task failed".to_string(),
        })
    }
}

impl HiddenIssueError {
    /// The variant name, for logs (carries no content).
    pub fn kind(&self) -> &'static str {
        match self {
            HiddenIssueError::Store(e) => e.kind(),
            HiddenIssueError::NotFound => "NotFound",
            HiddenIssueError::NotInList => "NotInList",
            HiddenIssueError::ListUnavailable => "ListUnavailable",
        }
    }

    /// Fixed Polish text for the snapshot when the hidden state could not be
    /// read or written. Never includes a value.
    pub fn message_pl(&self) -> String {
        match self {
            HiddenIssueError::Store(StoreError::DirUnavailable { .. }) => {
                "Dane aplikacji: folder danych jest niedostępny."
            }
            HiddenIssueError::Store(StoreError::Damaged { .. }) => {
                "Dane aplikacji: plik danych jest uszkodzony."
            }
            HiddenIssueError::Store(StoreError::Missing) => {
                "Dane aplikacji: brakuje pliku danych, chociaż istnieją kopie zapasowe."
            }
            HiddenIssueError::Store(StoreError::NewerSchema { .. }) => {
                "Dane aplikacji pochodzą z nowszej wersji aplikacji."
            }
            HiddenIssueError::Store(StoreError::MigrationFailed { .. }) => {
                "Dane aplikacji: nie udało się zaktualizować struktury danych."
            }
            HiddenIssueError::Store(StoreError::Busy) => {
                "Dane aplikacji są chwilowo zajęte — spróbuj ponownie."
            }
            HiddenIssueError::Store(StoreError::Io { .. }) => {
                "Dane aplikacji: błąd odczytu lub zapisu."
            }
            HiddenIssueError::Store(StoreError::Sql { .. }) => "Dane aplikacji: błąd bazy danych.",
            HiddenIssueError::NotFound => "Zgłoszenie nie jest już ukryte — odśwież listę.",
            HiddenIssueError::NotInList => "Zgłoszenia nie ma na liście — odśwież listę.",
            HiddenIssueError::ListUnavailable => "Lista GitLaba nie jest jeszcze dostępna.",
        }
        .to_string()
    }
}
