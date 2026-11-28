//! Reading the whole settings view and saving one section, and the two commands
//! that expose this to the dialog. This is the only code that writes
//! `settings.json`.
//!
//! Privacy: errors carry block names and field paths only, never a path, value
//! or system text; logs name only the section and the error kind.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use super::FieldError;
use super::document::{Document, ReadError, fingerprint};
use super::sections::check_block;
pub use super::sections::{
    AutostartForm, DimensionForm, FilterForm, GithubForm, GitlabForm, NoticesForm, RemindersForm,
    SectionInput, SectionView, SectionViews, TodoForm, WorkTimeForm,
};
use crate::health::SourceStatus;
use crate::settings_file::settings_path;
use crate::{BackgroundFailed, blocking};

/// Held across a whole save, so two saves never interleave their
/// read-modify-write.
#[derive(Default)]
pub struct SettingsWriteLock(Mutex<()>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FileState {
    Present,
    /// The next save creates it.
    Missing,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    pub file: FileState,
    pub sections: SectionViews,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveRequest {
    pub section: SectionInput,
    /// Per block key, as the view the form was loaded from gave them.
    pub fingerprints: BTreeMap<String, String>,
    /// The user agreed to replace a block that is invalid on disk.
    pub replace_invalid: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveResult {
    pub view: SettingsView,
    /// The outcome of the autostart sync, for an `autostart` save only.
    pub autostart: Option<SourceStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum SettingsViewError {
    #[error("the app config folder cannot be resolved")]
    ConfigDirUnresolved,
    #[error("the settings file is not valid")]
    FileInvalid,
    #[error("the settings file could not be read")]
    Io,
}

impl SettingsViewError {
    /// Variant name, for logs.
    fn kind(&self) -> &'static str {
        match self {
            SettingsViewError::ConfigDirUnresolved => "ConfigDirUnresolved",
            SettingsViewError::FileInvalid => "FileInvalid",
            SettingsViewError::Io => "Io",
        }
    }
}

impl From<ReadError> for SettingsViewError {
    fn from(e: ReadError) -> Self {
        match e {
            ReadError::Invalid => SettingsViewError::FileInvalid,
            ReadError::Io => SettingsViewError::Io,
        }
    }
}

impl From<BackgroundFailed> for SettingsViewError {
    fn from(_: BackgroundFailed) -> Self {
        SettingsViewError::Io
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum SaveSettingsError {
    #[error("the app config folder cannot be resolved")]
    ConfigDirUnresolved,
    #[error("the app config folder does not exist")]
    ConfigDirMissing,
    #[error("the settings file is not valid")]
    FileInvalid,
    #[error("the settings changed on disk")]
    Conflict { blocks: Vec<String> },
    #[error("a block on disk is invalid")]
    InvalidOnDisk { blocks: Vec<String> },
    #[error("some fields are invalid")]
    Invalid { fields: Vec<FieldError> },
    #[error("the settings file could not be read or written")]
    Io,
}

impl SaveSettingsError {
    /// Variant name, for logs.
    fn kind(&self) -> &'static str {
        match self {
            SaveSettingsError::ConfigDirUnresolved => "ConfigDirUnresolved",
            SaveSettingsError::ConfigDirMissing => "ConfigDirMissing",
            SaveSettingsError::FileInvalid => "FileInvalid",
            SaveSettingsError::Conflict { .. } => "Conflict",
            SaveSettingsError::InvalidOnDisk { .. } => "InvalidOnDisk",
            SaveSettingsError::Invalid { .. } => "Invalid",
            SaveSettingsError::Io => "Io",
        }
    }
}

impl From<ReadError> for SaveSettingsError {
    fn from(e: ReadError) -> Self {
        match e {
            ReadError::Invalid => SaveSettingsError::FileInvalid,
            ReadError::Io => SaveSettingsError::Io,
        }
    }
}

impl From<BackgroundFailed> for SaveSettingsError {
    fn from(_: BackgroundFailed) -> Self {
        SaveSettingsError::Io
    }
}

fn view_of(document: &Document, file: FileState) -> SettingsView {
    SettingsView {
        file,
        sections: SectionViews::read(&document.root),
    }
}

/// Everything the dialog renders, read from the file at `path`.
pub fn read_view(path: &Path) -> Result<SettingsView, SettingsViewError> {
    let document = Document::read(path)?;
    let file = if document.exists {
        FileState::Present
    } else {
        FileState::Missing
    };
    Ok(view_of(&document, file))
}

/// Writes the blocks of one section into the file at `path`. Nothing is written
/// on any error.
pub fn save_section(
    path: &Path,
    lock: &SettingsWriteLock,
    request: &SaveRequest,
) -> Result<SaveResult, SaveSettingsError> {
    // The lock guards no data, so a poisoned one is still good.
    let _guard = lock.0.lock().unwrap_or_else(|p| p.into_inner());
    let mut document = Document::read(path)?;
    // The folder is never created here.
    if !document.exists && !path.parent().is_some_and(Path::is_dir) {
        return Err(SaveSettingsError::ConfigDirMissing);
    }
    let keys = request.section.block_keys();

    let changed: Vec<String> = keys
        .iter()
        .filter(|key| {
            let current = fingerprint(document.root.get(**key));
            request.fingerprints.get(**key) != Some(&current)
        })
        .map(|key| key.to_string())
        .collect();
    if !changed.is_empty() {
        return Err(SaveSettingsError::Conflict { blocks: changed });
    }

    if !request.replace_invalid {
        let invalid: Vec<String> = keys
            .iter()
            .filter(|key| {
                let block = document.root.get(**key);
                block.is_some_and(|b| check_block(key, b).is_err())
            })
            .map(|key| key.to_string())
            .collect();
        if !invalid.is_empty() {
            return Err(SaveSettingsError::InvalidOnDisk { blocks: invalid });
        }
    }

    let blocks = request
        .section
        .build()
        .map_err(|fields| SaveSettingsError::Invalid { fields })?;
    for (key, block) in blocks {
        document.splice(key, block);
    }
    document.write(path).map_err(|_| SaveSettingsError::Io)?;

    Ok(SaveResult {
        view: view_of(&document, FileState::Present),
        autostart: None,
    })
}

/// The settings as the dialog shows them.
#[tauri::command]
pub async fn settings_view(app: AppHandle) -> Result<SettingsView, SettingsViewError> {
    let result = blocking(move || {
        let path = settings_path(&app).ok_or(SettingsViewError::ConfigDirUnresolved)?;
        read_view(&path)
    })
    .await;
    if let Err(e) = &result {
        log::warn!("settings: view failed ({})", e.kind());
    }
    result
}

/// Saves one section and returns the fresh view.
#[tauri::command]
pub async fn save_settings_section(
    app: AppHandle,
    request: SaveRequest,
) -> Result<SaveResult, SaveSettingsError> {
    let section = request.section.name();
    let result = blocking(move || {
        let path = settings_path(&app).ok_or(SaveSettingsError::ConfigDirUnresolved)?;
        save_section(&path, &app.state::<SettingsWriteLock>(), &request)
    })
    .await;
    match &result {
        Ok(_) => log::info!("settings: saved section {section}"),
        Err(e) => log::warn!("settings: saving section {section} failed ({})", e.kind()),
    }
    result
}
