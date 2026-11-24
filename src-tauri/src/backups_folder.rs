//! Opens `<app data dir>/backups/` in the default program (the file manager).
//! The path is fixed (no argument from the UI) and a missing folder is an
//! error: nothing is created here.
//!
//! Privacy: the path and system error text never reach a log or an error value.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::store::StoreHandle;

/// Why the backups folder could not be opened. Carries no path or system text.
#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum OpenBackupsError {
    #[error("the app data folder cannot be resolved")]
    DataDirUnresolved,
    #[error("the backups folder does not exist")]
    Missing,
    #[error("the backups folder could not be opened")]
    OpenFailed,
}

/// The path to open: `None` is an unresolved data dir, a non-folder is missing.
fn backups_folder_to_open(path: Option<PathBuf>) -> Result<PathBuf, OpenBackupsError> {
    let path = path.ok_or(OpenBackupsError::DataDirUnresolved)?;
    if path.is_dir() {
        Ok(path)
    } else {
        Err(OpenBackupsError::Missing)
    }
}

/// Opens the backups folder in the default program. Does not take the store
/// lock, so it needs no `spawn_blocking`.
#[tauri::command]
pub async fn open_backups_folder(app: AppHandle) -> Result<(), OpenBackupsError> {
    let path = backups_folder_to_open(app.state::<StoreHandle>().backups_dir())?;
    app.opener()
        .open_path(path.to_string_lossy().into_owned(), None::<&str>)
        .map_err(|_| {
            log::warn!("backups: open failed");
            OpenBackupsError::OpenFailed
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unresolved_data_dir_is_reported() {
        assert!(matches!(
            backups_folder_to_open(None),
            Err(OpenBackupsError::DataDirUnresolved)
        ));
    }

    #[test]
    fn a_missing_folder_is_reported_and_not_created() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("backups");
        assert!(matches!(
            backups_folder_to_open(Some(path.clone())),
            Err(OpenBackupsError::Missing)
        ));
        assert!(!path.exists());
    }

    #[test]
    fn an_existing_folder_is_returned() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("backups");
        std::fs::create_dir(&path).unwrap();
        assert_eq!(backups_folder_to_open(Some(path.clone())).unwrap(), path);
    }

    #[test]
    fn the_error_serializes_as_a_tagged_kind() {
        assert_eq!(
            serde_json::to_value(OpenBackupsError::Missing).unwrap(),
            serde_json::json!({"kind": "Missing"})
        );
    }
}
