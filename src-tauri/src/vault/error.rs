//! Typed refusals and failures for vault access.
//!
//! Invariant: no variant carries note content. Paths are vault-relative strings,
//! except `RootMissing::root`, which is the user's own configured path.

use std::path::PathBuf;

use super::Space;

#[derive(Debug, thiserror::Error, serde::Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum VaultError {
    #[error("settings file not found: {}", settings_path.display())]
    NotConfigured { settings_path: PathBuf },
    #[error("settings are invalid: {reason}")]
    SettingsInvalid { reason: String },
    #[error("space {0} is not configured")]
    SpaceNotConfigured(Space),

    #[error("vault root for {space} is missing: {}", root.display())]
    RootMissing { space: Space, root: PathBuf },
    #[error("work and private vault roots overlap")]
    RootsOverlap,

    #[error("invalid path {path:?}: {reason}")]
    InvalidPath { path: String, reason: String },
    #[error("path is outside the vault: {path}")]
    OutsideVault { path: String },
    #[error("path is excluded: {path}")]
    Excluded { path: String },
    #[error("path is hidden: {path}")]
    Hidden { path: String },
    #[error("not a Markdown note: {path}")]
    NotMarkdown { path: String },

    #[error("not found: {path}")]
    NotFound { path: String },
    #[error("already exists: {path}")]
    AlreadyExists { path: String },
    #[error("note is not valid UTF-8: {path}")]
    NotUtf8 { path: String },

    #[error("note has no frontmatter")]
    NoFrontmatter,
    #[error("frontmatter field not found: {key}")]
    FieldNotFound { key: String },
    #[error("frontmatter field is ambiguous: {key}")]
    FieldAmbiguous { key: String },
    #[error("section not found: {heading}")]
    SectionNotFound { heading: String },
    #[error("section is ambiguous ({count} matches): {heading}")]
    SectionAmbiguous { heading: String, count: usize },

    #[error("invalid value: {reason}")]
    InvalidValue { reason: String },
    #[error("conflicting change: {path}")]
    Conflict { path: String },
    #[error("file is busy: {path}")]
    Busy { path: String },
    #[error("file is read-only: {path}")]
    ReadOnly { path: String },
    #[error("replace incomplete for {path}; temp file {temp}, backup file {backup}")]
    ReplaceIncomplete {
        path: String,
        temp: String,
        backup: String,
    },

    #[error("I/O error on {path}: {kind}")]
    Io { path: String, kind: String },
}
