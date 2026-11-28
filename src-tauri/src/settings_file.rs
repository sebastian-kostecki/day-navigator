//! Opens `settings.json` from the app config dir in the default program. The
//! path is fixed (no argument from the UI) and a missing file is an error:
//! opening never creates it. The app writes the file only on a save from the
//! settings dialog (`settings::commands`).
//!
//! Privacy: the path and system error text never reach a log or an error value.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

/// `<app config dir>/settings.json`; `None` when the config dir cannot be resolved.
pub fn settings_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join("settings.json"))
}

/// Why the settings file could not be opened. Carries no path or system text.
#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum OpenSettingsError {
    #[error("the app config folder cannot be resolved")]
    ConfigDirUnresolved,
    #[error("the settings file does not exist")]
    Missing,
    #[error("the settings file could not be opened")]
    OpenFailed,
}

/// The path to open: `None` is an unresolved config dir, a non-file is missing.
fn settings_file_to_open(path: Option<PathBuf>) -> Result<PathBuf, OpenSettingsError> {
    let path = path.ok_or(OpenSettingsError::ConfigDirUnresolved)?;
    if path.is_file() {
        Ok(path)
    } else {
        Err(OpenSettingsError::Missing)
    }
}

/// Opens `settings.json` in the default program.
#[tauri::command]
pub async fn open_settings_file(app: AppHandle) -> Result<(), OpenSettingsError> {
    let path = settings_file_to_open(settings_path(&app))?;
    app.opener()
        .open_path(path.to_string_lossy().into_owned(), None::<&str>)
        .map_err(|_| {
            log::warn!("settings: open failed");
            OpenSettingsError::OpenFailed
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unresolved_config_dir_is_reported() {
        assert!(matches!(
            settings_file_to_open(None),
            Err(OpenSettingsError::ConfigDirUnresolved)
        ));
    }

    #[test]
    fn a_missing_file_is_reported_and_not_created() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        assert!(matches!(
            settings_file_to_open(Some(path.clone())),
            Err(OpenSettingsError::Missing)
        ));
        assert!(!path.exists());
    }

    #[test]
    fn an_existing_file_is_returned() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, "{}").unwrap();
        assert_eq!(settings_file_to_open(Some(path.clone())).unwrap(), path);
    }

    #[test]
    fn the_error_serializes_as_a_tagged_kind() {
        assert_eq!(
            serde_json::to_value(OpenSettingsError::Missing).unwrap(),
            serde_json::json!({"kind": "Missing"})
        );
    }
}
