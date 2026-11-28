//! Read-only loader for the hand-written `settings.json`. Nothing here imports `tauri`.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::NaiveTime;
use serde::Serialize;
use serde_json::Value;

use crate::cli::{GithubConfig, GithubRepo, GitlabConfig, GitlabHost, GitlabUser};
use crate::github::GithubError;
use crate::gitlab::GitlabError;
use crate::gitlab::filter::{DimensionRule, GitlabFilter};
use crate::todo::{DEFAULT_TODO_LIMIT, TodoError};

mod autostart;
mod field_error;
mod notices;
#[cfg(test)]
mod validator_tests;
mod work_time;
pub use autostart::{AutostartSettings, AutostartSettingsInvalid, load_autostart_settings};
pub use field_error::FieldError;
use field_error::{checked, object_with_keys, present};
pub use notices::{NoticeSettings, load_notice_settings, validate_notices};
pub use work_time::{WorkTimeSettings, WorkTimeSettingsInvalid, load_work_time_settings};
const SUPPORTED_VERSION: u64 = 1;

/// Why `settings.json` as a whole cannot be used. `Io::path` is the file name
/// only; `NotConfigured::settings_path` is the app's own settings file.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, serde::Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum SettingsError {
    #[error("settings file not found: {}", settings_path.display())]
    NotConfigured { settings_path: PathBuf },
    #[error("settings are invalid: {reason}")]
    SettingsInvalid { reason: String },
    #[error("I/O error on {path}: {kind}")]
    Io { path: String, kind: String },
}

impl SettingsError {
    /// Variant name, for logs and the health snapshot.
    pub fn kind(&self) -> &'static str {
        match self {
            SettingsError::NotConfigured { .. } => "NotConfigured",
            SettingsError::SettingsInvalid { .. } => "SettingsInvalid",
            SettingsError::Io { .. } => "Io",
        }
    }
}

fn invalid(reason: impl Into<String>) -> SettingsError {
    SettingsError::SettingsInvalid {
        reason: reason.into(),
    }
}

/// Checks the file as a whole: it exists and reads, is UTF-8 (a BOM is
/// allowed), parses as JSON and has `version` 1. Each block is read by its own
/// loader below; any other top-level key, an old `spaces` block included, is
/// ignored whatever its shape.
pub fn check_settings(path: &Path) -> Result<(), SettingsError> {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(SettingsError::NotConfigured {
                settings_path: path.to_path_buf(),
            });
        }
        Err(e) => {
            // File name only: error paths never carry absolute locations.
            return Err(SettingsError::Io {
                path: path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                kind: format!("{:?}", e.kind()),
            });
        }
    };
    let bytes = bytes
        .strip_prefix(&[0xEF, 0xBB, 0xBF][..])
        .unwrap_or(&bytes);
    let text =
        std::str::from_utf8(bytes).map_err(|_| invalid("settings file is not valid UTF-8"))?;
    check_settings_text(text)
}

fn check_settings_text(text: &str) -> Result<(), SettingsError> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|e| invalid(format!("malformed JSON: {e}")))?;
    match value.get("version").and_then(|v| v.as_u64()) {
        Some(SUPPORTED_VERSION) => Ok(()),
        Some(other) => Err(invalid(format!("unsupported version {other}"))),
        None => Err(invalid("missing or non-numeric version")),
    }
}

/// Reads only the top-level `gitlab` block, on its own: a typo there never
/// breaks the other blocks and a typo elsewhere never hides the GitLab list. A
/// missing file, an absent block or `null` is `Ok(None)`. Every failure is
/// `SettingsInvalid` with no detail: serde errors can quote the value.
pub fn load_gitlab_settings(path: &Path) -> Result<Option<GitlabConfig>, GitlabError> {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(GitlabError::SettingsInvalid),
    };
    let bytes = bytes
        .strip_prefix(&[0xEF, 0xBB, 0xBF][..])
        .unwrap_or(&bytes);
    let text = std::str::from_utf8(bytes).map_err(|_| GitlabError::SettingsInvalid)?;
    parse_gitlab_settings(text)
}

fn parse_gitlab_settings(text: &str) -> Result<Option<GitlabConfig>, GitlabError> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|_| GitlabError::SettingsInvalid)?;
    if value.get("version").and_then(|v| v.as_u64()) != Some(SUPPORTED_VERSION) {
        return Err(GitlabError::SettingsInvalid);
    }
    let Some(block) = value.get("gitlab") else {
        return Ok(None);
    };
    validate_gitlab(block).map_err(|_| GitlabError::SettingsInvalid)
}

/// Checks a `gitlab` block, reporting every bad field; `null` is `Ok(None)`.
fn validate_gitlab(block: &Value) -> Result<Option<GitlabConfig>, Vec<FieldError>> {
    if block.is_null() {
        return Ok(None);
    }
    let map = object_with_keys(block, "gitlab", &["host", "user"]).map_err(|e| vec![e])?;
    let mut errors = Vec::new();
    let host = map
        .get("host")
        .and_then(Value::as_str)
        .and_then(|h| GitlabHost::parse(h).ok());
    if host.is_none() {
        errors.push(FieldError::new(
            "gitlab.host",
            "Podaj samą nazwę hosta, bez protokołu, portu i ścieżki.",
        ));
    }
    let user = present(map, "user").and_then(|v| {
        let user = v.as_str().and_then(|u| GitlabUser::parse(u).ok());
        if user.is_none() {
            errors.push(FieldError::new(
                "gitlab.user",
                "To nie jest poprawny login GitLaba.",
            ));
        }
        user
    });
    match host {
        Some(host) if errors.is_empty() => Ok(Some(GitlabConfig { host, user })),
        _ => Err(errors),
    }
}

const GITHUB_REPOS_MAX: usize = 50;

/// Reads only the top-level `github` block, on its own, like the GitLab block.
/// A missing file, an absent or `null` block or an empty `repos` list is
/// `Ok(None)` (not configured). Every failure, including more than 50 repos,
/// is `SettingsInvalid` with no detail: serde errors can quote the value.
pub fn load_github_settings(path: &Path) -> Result<Option<GithubConfig>, GithubError> {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(GithubError::SettingsInvalid),
    };
    let bytes = bytes
        .strip_prefix(&[0xEF, 0xBB, 0xBF][..])
        .unwrap_or(&bytes);
    let text = std::str::from_utf8(bytes).map_err(|_| GithubError::SettingsInvalid)?;
    parse_github_settings(text)
}

fn parse_github_settings(text: &str) -> Result<Option<GithubConfig>, GithubError> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|_| GithubError::SettingsInvalid)?;
    if value.get("version").and_then(|v| v.as_u64()) != Some(SUPPORTED_VERSION) {
        return Err(GithubError::SettingsInvalid);
    }
    let Some(block) = value.get("github") else {
        return Ok(None);
    };
    validate_github(block).map_err(|_| GithubError::SettingsInvalid)
}

/// Checks a `github` block, reporting every bad entry; `null` and an empty
/// `repos` list are `Ok(None)`.
fn validate_github(block: &Value) -> Result<Option<GithubConfig>, Vec<FieldError>> {
    const FIELD: &str = "github.repos";
    if block.is_null() {
        return Ok(None);
    }
    let map = object_with_keys(block, "github", &["repos"]).map_err(|e| vec![e])?;
    let entries = match map.get("repos") {
        None => &[][..],
        Some(Value::Array(entries)) => entries.as_slice(),
        Some(_) => return Err(vec![FieldError::new(FIELD, "Oczekiwano listy.")]),
    };
    let mut errors = Vec::new();
    let mut repos: Vec<GithubRepo> = Vec::with_capacity(entries.len());
    let mut seen = std::collections::HashSet::new();
    for (index, entry) in entries.iter().enumerate() {
        match entry.as_str().and_then(|e| GithubRepo::parse(e).ok()) {
            Some(repo) => {
                if seen.insert(repo.key()) {
                    repos.push(repo);
                }
            }
            None => errors.push(FieldError::at(
                FIELD,
                index,
                "Repozytorium podaj jako właściciel/nazwa.",
            )),
        }
    }
    if repos.len() > GITHUB_REPOS_MAX {
        errors.push(FieldError::new(FIELD, "Najwyżej 50 repozytoriów."));
    }
    let config = (!repos.is_empty()).then_some(GithubConfig { repos });
    checked(config, errors)
}

const TODO_LIMIT_MAX: u64 = 20;

/// Reads only the top-level `todo` block, on its own, like the GitLab block.
/// A missing file, an absent or `null` block or an absent `limit` is the
/// default (5). Anything else that is not an integer 1..=20 is
/// `SettingsInvalid`, with no detail: serde errors can quote the value.
pub fn load_todo_limit(path: &Path) -> Result<u32, TodoError> {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(DEFAULT_TODO_LIMIT),
        Err(_) => return Err(TodoError::SettingsInvalid),
    };
    let bytes = bytes
        .strip_prefix(&[0xEF, 0xBB, 0xBF][..])
        .unwrap_or(&bytes);
    let text = std::str::from_utf8(bytes).map_err(|_| TodoError::SettingsInvalid)?;
    parse_todo_limit(text)
}

fn parse_todo_limit(text: &str) -> Result<u32, TodoError> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|_| TodoError::SettingsInvalid)?;
    if value.get("version").and_then(|v| v.as_u64()) != Some(SUPPORTED_VERSION) {
        return Err(TodoError::SettingsInvalid);
    }
    let Some(block) = value.get("todo") else {
        return Ok(DEFAULT_TODO_LIMIT);
    };
    validate_todo(block).map_err(|_| TodoError::SettingsInvalid)
}

/// Checks a `todo` block and gives the limit.
fn validate_todo(block: &Value) -> Result<u32, Vec<FieldError>> {
    if block.is_null() {
        return Ok(DEFAULT_TODO_LIMIT);
    }
    let map = object_with_keys(block, "todo", &["limit"]).map_err(|e| vec![e])?;
    match present(map, "limit") {
        None => Ok(DEFAULT_TODO_LIMIT),
        Some(v) => match v.as_u64() {
            Some(n) if (1..=TODO_LIMIT_MAX).contains(&n) => Ok(n as u32),
            _ => Err(vec![FieldError::new(
                "todo.limit",
                "Podaj liczbę całkowitą od 1 do 20.",
            )]),
        },
    }
}

/// Default `longRunningHours` of the `reminders` block.
pub const DEFAULT_LONG_RUNNING_HOURS: u64 = 10;
const LONG_RUNNING_HOURS_MAX: u64 = 24;

/// Work-time reminder triggers; `None` means that reminder is switched off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReminderSettings {
    pub long_running: Option<Duration>,
    pub not_running_after: Option<NaiveTime>,
}

impl Default for ReminderSettings {
    fn default() -> Self {
        ReminderSettings {
            long_running: Some(Duration::from_secs(DEFAULT_LONG_RUNNING_HOURS * 3600)),
            not_running_after: NaiveTime::from_hms_opt(9, 0, 0),
        }
    }
}

/// Which reminder fields `settings.json` set. A value or `false` counts as
/// set; an absent or `null` field, an absent or `null` block and a missing
/// file count as not set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReminderOrigin {
    pub long_running_from_file: bool,
    pub not_running_after_from_file: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum ReminderSettingsError {
    #[error("the reminders settings block is invalid")]
    SettingsInvalid,
}

/// Reads only the top-level `reminders` block, on its own, like the `todo`
/// block. Shape:
///
/// ```json
/// "reminders": { "longRunningHours": 10, "notRunningAfter": "09:00" }
/// ```
///
/// - `longRunningHours`: an integer 1..=24 or `false` (off). Default 10.
/// - `notRunningAfter`: a zero-padded 24-hour `"HH:MM"` or `false` (off).
///   Default `"09:00"`.
///
/// A missing file, an absent or `null` block and absent or `null` fields are
/// the defaults. Anything else is `SettingsInvalid`, with no detail: serde
/// errors can quote the value.
pub fn load_reminder_settings(path: &Path) -> Result<ReminderSettings, ReminderSettingsError> {
    load_reminder_settings_detailed(path).map(|(settings, _)| settings)
}

/// Same parse as `load_reminder_settings`, plus which fields the file set.
pub fn load_reminder_settings_detailed(
    path: &Path,
) -> Result<(ReminderSettings, ReminderOrigin), ReminderSettingsError> {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok((ReminderSettings::default(), ReminderOrigin::default()));
        }
        Err(_) => return Err(ReminderSettingsError::SettingsInvalid),
    };
    let bytes = bytes
        .strip_prefix(&[0xEF, 0xBB, 0xBF][..])
        .unwrap_or(&bytes);
    let text = std::str::from_utf8(bytes).map_err(|_| ReminderSettingsError::SettingsInvalid)?;
    parse_reminder_settings(text)
}

fn parse_hhmm(text: &str) -> Option<NaiveTime> {
    let b = text.as_bytes();
    if b.len() != 5 || b[2] != b':' {
        return None;
    }
    if ![0, 1, 3, 4].iter().all(|&i| b[i].is_ascii_digit()) {
        return None;
    }
    let hours: u32 = text[0..2].parse().ok()?;
    let minutes: u32 = text[3..5].parse().ok()?;
    NaiveTime::from_hms_opt(hours, minutes, 0)
}

fn parse_reminder_settings(
    text: &str,
) -> Result<(ReminderSettings, ReminderOrigin), ReminderSettingsError> {
    const INVALID: ReminderSettingsError = ReminderSettingsError::SettingsInvalid;
    let defaults = || (ReminderSettings::default(), ReminderOrigin::default());
    let value: serde_json::Value = serde_json::from_str(text).map_err(|_| INVALID)?;
    if value.get("version").and_then(|v| v.as_u64()) != Some(SUPPORTED_VERSION) {
        return Err(INVALID);
    }
    let Some(block) = value.get("reminders") else {
        return Ok(defaults());
    };
    validate_reminders(block).map_err(|_| INVALID)
}

/// Checks a `reminders` block, reporting every bad field, and says which fields
/// it set.
fn validate_reminders(
    block: &Value,
) -> Result<(ReminderSettings, ReminderOrigin), Vec<FieldError>> {
    let mut settings = ReminderSettings::default();
    let mut origin = ReminderOrigin::default();
    if block.is_null() {
        return Ok((settings, origin));
    }
    let keys = ["longRunningHours", "notRunningAfter"];
    let map = object_with_keys(block, "reminders", &keys).map_err(|e| vec![e])?;
    let mut errors = Vec::new();
    match present(map, "longRunningHours") {
        None => {}
        Some(Value::Bool(false)) => {
            settings.long_running = None;
            origin.long_running_from_file = true;
        }
        Some(v) => match v.as_u64() {
            Some(n) if (1..=LONG_RUNNING_HOURS_MAX).contains(&n) => {
                settings.long_running = Some(Duration::from_secs(n * 3600));
                origin.long_running_from_file = true;
            }
            _ => errors.push(FieldError::new(
                "reminders.longRunningHours",
                "Podaj liczbę całkowitą od 1 do 24 albo false.",
            )),
        },
    }
    if let Some(value) = present(map, "notRunningAfter") {
        let time = match value {
            Value::Bool(false) => Some(None),
            other => other.as_str().and_then(parse_hhmm).map(Some),
        };
        match time {
            Some(time) => {
                settings.not_running_after = time;
                origin.not_running_after_from_file = true;
            }
            None => errors.push(FieldError::new(
                "reminders.notRunningAfter",
                "Podaj godzinę w formacie GG:MM albo false.",
            )),
        }
    }
    checked((settings, origin), errors)
}

pub const DEFAULT_REFRESH_MINUTES: u32 = 10;
/// The wait starts after a round ends, so 15 would exceed the 15 minute limit;
/// 14 keeps a margin.
pub const REFRESH_MINUTES_MAX: u64 = 14;

/// A top-level refresh block (`gitlabRefresh` or `githubRefresh`) is invalid
/// (not an object, an unknown key or a value outside 1..=14). Carries no
/// detail: serde errors can quote the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefreshSettingInvalid;

/// Reads only the top-level `gitlabRefresh` block, on its own. A file-level
/// problem (unreadable, bad UTF-8 or JSON, wrong `version`) is `Ok(10)`: the
/// GitLab block already reports the file. Only a block-level problem is `Err`.
pub fn load_gitlab_refresh_minutes(path: &Path) -> Result<u32, RefreshSettingInvalid> {
    load_refresh_minutes(path, "gitlabRefresh")
}

/// Reads only the top-level `githubRefresh` block, on its own. A file-level
/// problem is `Ok(10)`: the `github` block already reports the file
/// (`load_github_settings`). Only a block-level problem is `Err`.
pub fn load_github_refresh_minutes(path: &Path) -> Result<u32, RefreshSettingInvalid> {
    load_refresh_minutes(path, "githubRefresh")
}

fn load_refresh_minutes(path: &Path, key: &str) -> Result<u32, RefreshSettingInvalid> {
    let Ok(bytes) = fs::read(path) else {
        return Ok(DEFAULT_REFRESH_MINUTES);
    };
    let bytes = bytes
        .strip_prefix(&[0xEF, 0xBB, 0xBF][..])
        .unwrap_or(&bytes);
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Ok(DEFAULT_REFRESH_MINUTES);
    };
    parse_refresh_minutes(text, key)
}

fn parse_refresh_minutes(text: &str, key: &str) -> Result<u32, RefreshSettingInvalid> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return Ok(DEFAULT_REFRESH_MINUTES);
    };
    if value.get("version").and_then(|v| v.as_u64()) != Some(SUPPORTED_VERSION) {
        return Ok(DEFAULT_REFRESH_MINUTES);
    }
    let Some(block) = value.get(key) else {
        return Ok(DEFAULT_REFRESH_MINUTES);
    };
    validate_refresh(block, key).map_err(|_| RefreshSettingInvalid)
}

/// Checks a `gitlabRefresh` or `githubRefresh` block (`key` names it) and
/// gives the minutes.
fn validate_refresh(block: &Value, key: &str) -> Result<u32, Vec<FieldError>> {
    if block.is_null() {
        return Ok(DEFAULT_REFRESH_MINUTES);
    }
    let map = object_with_keys(block, key, &["minutes"]).map_err(|e| vec![e])?;
    match present(map, "minutes") {
        None => Ok(DEFAULT_REFRESH_MINUTES),
        Some(v) => match v.as_u64() {
            Some(n) if (1..=REFRESH_MINUTES_MAX).contains(&n) => Ok(n as u32),
            _ => Err(vec![FieldError::new(
                format!("{key}.minutes"),
                "Podaj liczbę całkowitą od 1 do 14.",
            )]),
        },
    }
}

/// The top-level `gitlabFilter` block is invalid (not an object, an unknown
/// key, a bad list or entry, or `closedDays` outside 1..=30). Carries no
/// detail: serde errors can quote the value and the entries are work data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilterSettingInvalid;

const FILTER_ENTRIES_MAX: usize = 100;
const FILTER_ENTRY_CHARS_MAX: usize = 255;
const FILTER_CLOSED_DAYS_MAX: u64 = 30;

/// Reads only the top-level `gitlabFilter` block, on its own, and fails open.
/// Shape:
///
/// ```json
/// "gitlabFilter": {
///   "projects":   { "include": ["acme/backend"], "exclude": ["acme/backend/legacy"] },
///   "labels":     { "exclude": ["blocked"] },
///   "milestones": { "include": ["Sprint 42"] },
///   "closedDays": 7
/// }
/// ```
///
/// - Each dimension has optional `include` and `exclude` lists of strings (at
///   most 100 entries of at most 255 characters, no control characters,
///   trimmed and lowercased). An empty or absent list is no constraint.
/// - A `projects` entry is a project path or a group prefix, with no empty
///   segment (no leading or trailing `/`).
/// - `closedDays`: an integer 1..=30, or `null`/absent (off).
///
/// A file-level problem (missing, unreadable, bad UTF-8 or JSON, wrong
/// `version`) is `Ok(default)`: the `gitlab` block already reports the file.
/// So is an absent or `null` block. Everything else wrong in the block is
/// `Err(FilterSettingInvalid)`, with no detail.
pub fn load_gitlab_filter(path: &Path) -> Result<GitlabFilter, FilterSettingInvalid> {
    let Ok(bytes) = fs::read(path) else {
        return Ok(GitlabFilter::default());
    };
    let bytes = bytes
        .strip_prefix(&[0xEF, 0xBB, 0xBF][..])
        .unwrap_or(&bytes);
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Ok(GitlabFilter::default());
    };
    parse_gitlab_filter(text)
}

fn parse_gitlab_filter(text: &str) -> Result<GitlabFilter, FilterSettingInvalid> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return Ok(GitlabFilter::default());
    };
    if value.get("version").and_then(|v| v.as_u64()) != Some(SUPPORTED_VERSION) {
        return Ok(GitlabFilter::default());
    }
    let Some(block) = value.get("gitlabFilter") else {
        return Ok(GitlabFilter::default());
    };
    validate_gitlab_filter(block).map_err(|_| FilterSettingInvalid)
}

/// Checks a `gitlabFilter` block, reporting every bad field and list entry.
fn validate_gitlab_filter(block: &Value) -> Result<GitlabFilter, Vec<FieldError>> {
    if block.is_null() {
        return Ok(GitlabFilter::default());
    }
    let keys = ["projects", "labels", "milestones", "closedDays"];
    let map = object_with_keys(block, "gitlabFilter", &keys).map_err(|e| vec![e])?;
    let mut errors = Vec::new();
    let closed_days = present(map, "closedDays").and_then(|v| match v.as_u64() {
        Some(n) if (1..=FILTER_CLOSED_DAYS_MAX).contains(&n) => Some(n as u32),
        _ => {
            errors.push(FieldError::new(
                "gitlabFilter.closedDays",
                "Podaj liczbę całkowitą od 1 do 30.",
            ));
            None
        }
    });
    let mut dimension = |key: &str, project_paths: bool| {
        filter_dimension(
            present(map, key),
            &format!("gitlabFilter.{key}"),
            project_paths,
            &mut errors,
        )
    };
    let filter = GitlabFilter {
        projects: dimension("projects", true),
        labels: dimension("labels", false),
        milestones: dimension("milestones", false),
        closed_days,
    };
    checked(filter, errors)
}

/// One filter dimension; whatever is wrong with it goes into `errors`.
fn filter_dimension(
    value: Option<&Value>,
    field: &str,
    project_paths: bool,
    errors: &mut Vec<FieldError>,
) -> DimensionRule {
    let Some(value) = value else {
        return DimensionRule::default();
    };
    let map = match object_with_keys(value, field, &["include", "exclude"]) {
        Ok(map) => map,
        Err(e) => {
            errors.push(e);
            return DimensionRule::default();
        }
    };
    let mut list = |key: &str| {
        filter_list(
            present(map, key),
            &format!("{field}.{key}"),
            project_paths,
            errors,
        )
    };
    DimensionRule {
        include: list("include"),
        exclude: list("exclude"),
    }
}

/// The valid entries of one list, trimmed and lowercased.
fn filter_list(
    value: Option<&Value>,
    field: &str,
    project_paths: bool,
    errors: &mut Vec<FieldError>,
) -> Vec<String> {
    let Some(value) = value else {
        return Vec::new();
    };
    let Value::Array(entries) = value else {
        errors.push(FieldError::new(field, "Oczekiwano listy."));
        return Vec::new();
    };
    if entries.len() > FILTER_ENTRIES_MAX {
        errors.push(FieldError::new(field, "Najwyżej 100 pozycji."));
    }
    let mut out = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        match filter_entry(entry, project_paths) {
            Ok(text) => out.push(text),
            Err(message) => errors.push(FieldError::at(field, index, message)),
        }
    }
    out
}

fn filter_entry(entry: &Value, project_paths: bool) -> Result<String, &'static str> {
    let Value::String(text) = entry else {
        return Err("Pozycja musi być tekstem.");
    };
    if text.chars().any(char::is_control) {
        return Err("Pozycja zawiera niedozwolone znaki sterujące.");
    }
    let text = text.trim();
    if text.is_empty() {
        return Err("Pozycja nie może być pusta.");
    }
    if text.chars().count() > FILTER_ENTRY_CHARS_MAX {
        return Err("Pozycja może mieć najwyżej 255 znaków.");
    }
    if project_paths && text.split('/').any(str::is_empty) {
        return Err("Ścieżka projektu nie może zaczynać ani kończyć się „/” ani zawierać „//”.");
    }
    Ok(text.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &tempfile::TempDir, text: &str) -> PathBuf {
        let p = dir.path().join("settings.json");
        fs::write(&p, text).unwrap();
        p
    }

    fn json_path(dir: &tempfile::TempDir) -> String {
        dir.path().to_string_lossy().replace('\\', "\\\\")
    }

    #[test]
    fn settings_missing_file_is_not_configured() {
        let dir = tempfile::tempdir().unwrap();
        let err = check_settings(&dir.path().join("settings.json")).unwrap_err();
        assert!(matches!(err, SettingsError::NotConfigured { .. }));
    }

    #[test]
    fn settings_malformed_json_is_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let err = check_settings(&write(&dir, "{ not json")).unwrap_err();
        assert!(matches!(err, SettingsError::SettingsInvalid { .. }));
    }

    #[test]
    fn settings_wrong_or_missing_version_is_invalid() {
        let dir = tempfile::tempdir().unwrap();
        for text in [
            r#"{"version": 2}"#,
            r#"{"version": "1"}"#,
            r#"{"version": null}"#,
            r#"{}"#,
            "[1]",
        ] {
            let err = check_settings(&write(&dir, text)).unwrap_err();
            assert!(
                matches!(err, SettingsError::SettingsInvalid { .. }),
                "{text}"
            );
        }
        assert!(check_settings(&write(&dir, r#"{"version": 1}"#)).is_ok());
    }

    #[test]
    fn settings_bom_is_stripped_and_bad_utf8_is_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(br#"{"version":1}"#);
        fs::write(&p, bytes).unwrap();
        assert!(check_settings(&p).is_ok());
        fs::write(&p, [0xFF, 0xFE, 0x00]).unwrap();
        assert!(matches!(
            check_settings(&p),
            Err(SettingsError::SettingsInvalid { .. })
        ));
    }

    #[test]
    fn settings_unreadable_file_is_io_with_the_file_name_only() {
        let dir = tempfile::tempdir().unwrap();
        // A directory where the file should be: unreadable, not "missing".
        let p = dir.path().join("settings.json");
        fs::create_dir(&p).unwrap();
        match check_settings(&p).unwrap_err() {
            SettingsError::Io { path, kind } => {
                assert_eq!(path, "settings.json");
                assert!(!kind.is_empty());
            }
            other => panic!("expected Io, got {other:?}"),
        }
    }

    #[test]
    fn settings_error_serialises_with_kind_and_detail() {
        let v = serde_json::to_value(invalid("zły")).unwrap();
        assert_eq!(v["kind"], "SettingsInvalid");
        assert_eq!(v["detail"]["reason"], "zły");
    }

    /// An old hand-written `settings.json` still carries the retired vault
    /// block (`spaces`). It is ignored in any shape and never fails the file.
    #[test]
    fn settings_old_spaces_block_of_any_shape_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let root = json_path(&dir);
        let tasks = r#""tasks":{"folder":"Tasks","template":"Templates/Task.md","statuses":["Todo","Done"],"done_status":"Done"}"#;
        let spaces = [
            format!(
                r#"{{"work":{{"vault":"{root}","excluded":["Dostepy","../up","CON"],{tasks}}},"private":null}}"#
            ),
            r#"{"work":{"vault":"relative/vault","excluded":["/abs",".hidden"],"bogus":1}}"#
                .to_string(),
            r#"{"work":{"vault":"relative/vault","tasks":[1,2]},"other":{"x":1}}"#.to_string(),
            r#"{}"#.to_string(),
            r#""x""#.to_string(),
            "null".to_string(),
            "42".to_string(),
            "[1,2]".to_string(),
        ];
        for block in spaces {
            let text = format!(r#"{{"version":1,"spaces":{block}}}"#);
            assert!(check_settings(&write(&dir, &text)).is_ok(), "{block}");
        }
    }

    fn gitlab_text(block: &str) -> String {
        format!(r#"{{"version":1,"spaces":{{}},"gitlab":{block}}}"#)
    }

    #[test]
    fn gitlab_valid_block_parses_and_lowercases() {
        let dir = tempfile::tempdir().unwrap();
        let text = gitlab_text(r#"{"host":"GitLab.Example.Invalid"}"#);
        let cfg = load_gitlab_settings(&write(&dir, &text)).unwrap().unwrap();
        assert_eq!(cfg.host.as_str(), "gitlab.example.invalid");
        assert!(cfg.user.is_none());
    }

    #[test]
    fn gitlab_optional_user_parses_and_null_means_none() {
        let dir = tempfile::tempdir().unwrap();
        let text = gitlab_text(r#"{"host":"gitlab.example.invalid","user":"Invented.User"}"#);
        let cfg = load_gitlab_settings(&write(&dir, &text)).unwrap().unwrap();
        // The login keeps its case.
        assert_eq!(cfg.user.unwrap().as_str(), "Invented.User");
        let text = gitlab_text(r#"{"host":"gitlab.example.invalid","user":null}"#);
        let cfg = load_gitlab_settings(&write(&dir, &text)).unwrap().unwrap();
        assert!(cfg.user.is_none());
    }

    #[test]
    fn gitlab_invalid_user_is_refused_without_leaking_it() {
        const SECRET: &str = "secret-login";
        let dir = tempfile::tempdir().unwrap();
        let cases = [
            r#"{"host":"gitlab.example.invalid","user":""}"#.to_string(),
            r#"{"host":"gitlab.example.invalid","user":5}"#.to_string(),
            r#"{"host":"gitlab.example.invalid","user":["x"]}"#.to_string(),
            format!(r#"{{"host":"gitlab.example.invalid","user":"{SECRET}&x"}}"#),
            format!(r#"{{"host":"gitlab.example.invalid","user":"{SECRET}/x"}}"#),
            format!(r#"{{"host":"gitlab.example.invalid","user":"{SECRET} x"}}"#),
            format!(r#"{{"host":"gitlab.example.invalid","user":"-{SECRET}"}}"#),
            format!(r#"{{"host":"gitlab.example.invalid","user":".{SECRET}"}}"#),
            format!(r#"{{"host":"gitlab.example.invalid","user":"{SECRET}","extra":1}}"#),
        ];
        for block in cases {
            let err = load_gitlab_settings(&write(&dir, &gitlab_text(&block))).unwrap_err();
            assert_eq!(err, GitlabError::SettingsInvalid, "{block}");
            let shown = [
                format!("{err:?}"),
                err.to_string(),
                serde_json::to_string(&err).unwrap(),
                err.message_pl(),
            ];
            for text in shown {
                assert!(!text.contains(SECRET), "leak for {block}: {text}");
            }
        }
    }

    #[test]
    fn gitlab_bom_is_stripped() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(gitlab_text(r#"{"host":"gitlab.example.invalid"}"#).as_bytes());
        fs::write(&p, bytes).unwrap();
        assert!(load_gitlab_settings(&p).unwrap().is_some());
    }

    #[test]
    fn gitlab_missing_file_absent_and_null_block_are_not_configured() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            load_gitlab_settings(&dir.path().join("settings.json")),
            Ok(None)
        );
        let text = r#"{"version":1,"spaces":{}}"#;
        assert_eq!(load_gitlab_settings(&write(&dir, text)), Ok(None));
        let text = gitlab_text("null");
        assert_eq!(load_gitlab_settings(&write(&dir, &text)), Ok(None));
    }

    #[test]
    fn gitlab_invalid_blocks_are_refused_without_leaking_the_value() {
        const SECRET: &str = "secret.example.invalid";
        let dir = tempfile::tempdir().unwrap();
        let cases = [
            format!(r#"{{"host":"{SECRET}","extra":1}}"#),
            format!(r#""{SECRET}""#),
            format!(r#"[ "{SECRET}" ]"#),
            format!(r#"{{"host":5,"x":"{SECRET}"}}"#),
            r#"{}"#.to_string(),
            r#"{"host":""}"#.to_string(),
            format!(r#"{{"host":"https://{SECRET}"}}"#),
            format!(r#"{{"host":"{SECRET}:8443"}}"#),
            format!(r#"{{"host":"{SECRET}/path"}}"#),
            format!(r#"{{"host":" {SECRET}"}}"#),
            format!(r#"{{"host":"-{SECRET}"}}"#),
            format!(r#"{{"host":"{}"}}"#, SECRET.split('.').next().unwrap()),
        ];
        for block in cases {
            let err = load_gitlab_settings(&write(&dir, &gitlab_text(&block))).unwrap_err();
            assert_eq!(err, GitlabError::SettingsInvalid, "{block}");
            let shown = [
                format!("{err:?}"),
                err.to_string(),
                serde_json::to_string(&err).unwrap(),
                err.message_pl(),
            ];
            for text in shown {
                assert!(!text.contains("secret"), "leak for {block}: {text}");
            }
        }
    }

    #[test]
    fn gitlab_broken_file_is_settings_invalid() {
        let dir = tempfile::tempdir().unwrap();
        for text in [
            "{ not json",
            r#"{"gitlab":{"host":"gitlab.example.invalid"}}"#,
            r#"{"version":2,"gitlab":{"host":"gitlab.example.invalid"}}"#,
        ] {
            assert_eq!(
                load_gitlab_settings(&write(&dir, text)),
                Err(GitlabError::SettingsInvalid),
                "{text}"
            );
        }
        let p = dir.path().join("settings.json");
        fs::write(&p, [0xFF, 0xFE, 0x00]).unwrap();
        assert_eq!(load_gitlab_settings(&p), Err(GitlabError::SettingsInvalid));
        // A directory where the file should be: unreadable, not "missing".
        assert_eq!(
            load_gitlab_settings(dir.path()),
            Err(GitlabError::SettingsInvalid)
        );
    }

    #[test]
    fn settings_check_ignores_the_gitlab_block_valid_or_not() {
        let dir = tempfile::tempdir().unwrap();
        for block in [r#"{"host":"gitlab.example.invalid"}"#, r#""not-an-object""#] {
            let text = format!(r#"{{"version":1,"gitlab":{block}}}"#);
            assert!(check_settings(&write(&dir, &text)).is_ok(), "{block}");
        }
    }

    fn github_text(block: &str) -> String {
        format!(r#"{{"version":1,"spaces":{{}},"github":{block}}}"#)
    }

    fn repo_names(cfg: &GithubConfig) -> Vec<&str> {
        cfg.repos.iter().map(|r| r.as_str()).collect()
    }

    #[test]
    fn github_valid_block_parses_in_order_and_drops_case_insensitive_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        let text = github_text(
            r#"{"repos":["octo-test/alpha","Octo-Test/Beta","OCTO-TEST/ALPHA","octo-test/beta","octo-test/gamma"]}"#,
        );
        let cfg = load_github_settings(&write(&dir, &text)).unwrap().unwrap();
        assert_eq!(
            repo_names(&cfg),
            ["octo-test/alpha", "Octo-Test/Beta", "octo-test/gamma"]
        );
    }

    #[test]
    fn github_missing_file_absent_null_and_empty_are_not_configured() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            load_github_settings(&dir.path().join("settings.json")),
            Ok(None)
        );
        let text = r#"{"version":1,"spaces":{}}"#;
        assert_eq!(load_github_settings(&write(&dir, text)), Ok(None));
        for block in ["null", "{}", r#"{"repos":[]}"#] {
            let text = github_text(block);
            assert_eq!(
                load_github_settings(&write(&dir, &text)),
                Ok(None),
                "{block}"
            );
        }
    }

    #[test]
    fn github_fifty_repos_are_accepted_and_fifty_one_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let list = |n: usize| {
            let repos: Vec<String> = (0..n).map(|i| format!("\"octo-test/r{i}\"")).collect();
            github_text(&format!(r#"{{"repos":[{}]}}"#, repos.join(",")))
        };
        let cfg = load_github_settings(&write(&dir, &list(50)))
            .unwrap()
            .unwrap();
        assert_eq!(cfg.repos.len(), 50);
        assert_eq!(
            load_github_settings(&write(&dir, &list(51))),
            Err(GithubError::SettingsInvalid)
        );
    }

    #[test]
    fn github_invalid_blocks_are_refused_without_leaking_the_value() {
        const SECRET: &str = "secret-owner";
        let dir = tempfile::tempdir().unwrap();
        let cases = [
            format!(r#"{{"repos":["{SECRET}"]}}"#),
            format!(r#"{{"repos":["-{SECRET}/x"]}}"#),
            format!(r#"{{"repos":["{SECRET}/x y"]}}"#),
            format!(r#"{{"repos":["{SECRET}/a/b"]}}"#),
            format!(r#"{{"repos":["{SECRET}/.."]}}"#),
            format!(r#"{{"repos":[5,"{SECRET}/x"]}}"#),
            format!(r#"{{"repos":"{SECRET}/x"}}"#),
            format!(r#"{{"repos":["{SECRET}/x"],"extra":1}}"#),
            format!(r#""{SECRET}""#),
            format!(r#"["{SECRET}/x"]"#),
            r#"{"repos":null}"#.to_string(),
        ];
        for block in cases {
            let err = load_github_settings(&write(&dir, &github_text(&block))).unwrap_err();
            assert_eq!(err, GithubError::SettingsInvalid, "{block}");
            let shown = [
                format!("{err:?}"),
                err.to_string(),
                serde_json::to_string(&err).unwrap(),
                err.message_pl(),
            ];
            for text in shown {
                assert!(!text.contains("secret"), "leak for {block}: {text}");
            }
        }
    }

    #[test]
    fn github_bom_is_stripped_and_broken_files_are_settings_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(github_text(r#"{"repos":["octo-test/alpha"]}"#).as_bytes());
        fs::write(&p, bytes).unwrap();
        assert!(load_github_settings(&p).unwrap().is_some());
        for text in [
            "{ not json",
            r#"{"github":{"repos":["octo-test/alpha"]}}"#,
            r#"{"version":2,"github":{"repos":["octo-test/alpha"]}}"#,
        ] {
            assert_eq!(
                load_github_settings(&write(&dir, text)),
                Err(GithubError::SettingsInvalid),
                "{text}"
            );
        }
        fs::write(&p, [0xFF, 0xFE, 0x00]).unwrap();
        assert_eq!(load_github_settings(&p), Err(GithubError::SettingsInvalid));
        // A directory where the file should be: unreadable, not "missing".
        assert_eq!(
            load_github_settings(dir.path()),
            Err(GithubError::SettingsInvalid)
        );
    }

    #[test]
    fn github_block_is_independent_of_the_other_loaders() {
        let dir = tempfile::tempdir().unwrap();
        // A broken github block leaves the settings check, GitLab and todo
        // loaders green.
        for block in [r#""nope""#, r#"{"repos":["bad"]}"#, r#"{"bogus":1}"#] {
            let text = format!(
                r#"{{"version":1,"gitlab":{{"host":"gitlab.example.invalid"}},"todo":{{"limit":4}},"github":{block}}}"#
            );
            let p = write(&dir, &text);
            assert!(check_settings(&p).is_ok(), "{block}");
            assert!(load_gitlab_settings(&p).unwrap().is_some(), "{block}");
            assert_eq!(load_todo_limit(&p).unwrap(), 4, "{block}");
            assert!(load_github_settings(&p).is_err(), "{block}");
        }
        // And a broken sibling block never hides the repos.
        let text = r#"{"version":1,"spaces":{},"gitlab":"nope","todo":{"limit":0},"github":{"repos":["octo-test/alpha"]}}"#;
        let cfg = load_github_settings(&write(&dir, text)).unwrap().unwrap();
        assert_eq!(repo_names(&cfg), ["octo-test/alpha"]);
    }

    fn todo_text(block: &str) -> String {
        format!(r#"{{"version":1,"spaces":{{}},"todo":{block}}}"#)
    }

    #[test]
    fn todo_limit_defaults() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            load_todo_limit(&dir.path().join("settings.json")).unwrap(),
            DEFAULT_TODO_LIMIT
        );
        for text in [
            r#"{"version":1,"spaces":{}}"#.to_string(),
            todo_text("null"),
            todo_text("{}"),
            todo_text(r#"{"limit":null}"#),
        ] {
            assert_eq!(load_todo_limit(&write(&dir, &text)).unwrap(), 5, "{text}");
        }
    }

    #[test]
    fn todo_limit_range_edges() {
        let dir = tempfile::tempdir().unwrap();
        for n in [1u32, 7, 20] {
            let text = todo_text(&format!(r#"{{"limit":{n}}}"#));
            assert_eq!(load_todo_limit(&write(&dir, &text)).unwrap(), n);
        }
    }

    #[test]
    fn todo_limit_invalid_values_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let blocks = [
            r#"{"limit":0}"#,
            r#"{"limit":21}"#,
            r#"{"limit":-1}"#,
            r#"{"limit":"5"}"#,
            r#"{"limit":5.5}"#,
            r#"{"limit":true}"#,
            r#"{"limit":5,"extra":1}"#,
            r#"{"other":1}"#,
            r#""5""#,
            r#"[5]"#,
        ];
        for block in blocks {
            let err = load_todo_limit(&write(&dir, &todo_text(block))).unwrap_err();
            assert!(matches!(err, TodoError::SettingsInvalid), "{block}");
        }
        for text in [
            "{ not json",
            r#"{"todo":{"limit":5}}"#,
            r#"{"version":2,"todo":{"limit":5}}"#,
        ] {
            let err = load_todo_limit(&write(&dir, text)).unwrap_err();
            assert!(matches!(err, TodoError::SettingsInvalid), "{text}");
        }
        let p = dir.path().join("settings.json");
        fs::write(&p, [0xFF, 0xFE, 0x00]).unwrap();
        assert!(matches!(
            load_todo_limit(&p),
            Err(TodoError::SettingsInvalid)
        ));
    }

    #[test]
    fn todo_limit_bom_is_stripped() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(todo_text(r#"{"limit":3}"#).as_bytes());
        fs::write(&p, bytes).unwrap();
        assert_eq!(load_todo_limit(&p).unwrap(), 3);
    }

    #[test]
    fn broken_todo_block_leaves_settings_check_and_gitlab_loader_green() {
        let dir = tempfile::tempdir().unwrap();
        for block in [r#"{"limit":0}"#, r#""nope""#, r#"{"bogus":1}"#] {
            let text = format!(
                r#"{{"version":1,"gitlab":{{"host":"gitlab.example.invalid"}},"todo":{block}}}"#
            );
            let p = write(&dir, &text);
            assert!(check_settings(&p).is_ok(), "{block}");
            assert!(load_gitlab_settings(&p).unwrap().is_some(), "{block}");
            assert!(load_todo_limit(&p).is_err(), "{block}");
        }
    }

    #[test]
    fn todo_loader_ignores_a_broken_gitlab_block() {
        let dir = tempfile::tempdir().unwrap();
        let text = r#"{"version":1,"spaces":{},"gitlab":"nope","todo":{"limit":4}}"#;
        assert_eq!(load_todo_limit(&write(&dir, text)).unwrap(), 4);
    }

    fn reminders_text(block: &str) -> String {
        format!(r#"{{"version":1,"spaces":{{}},"reminders":{block}}}"#)
    }

    fn hours(h: u64) -> Option<Duration> {
        Some(Duration::from_secs(h * 3600))
    }

    fn hm(h: u32, m: u32) -> Option<NaiveTime> {
        NaiveTime::from_hms_opt(h, m, 0)
    }

    #[test]
    fn reminders_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let want = ReminderSettings {
            long_running: hours(10),
            not_running_after: hm(9, 0),
        };
        assert_eq!(
            load_reminder_settings(&dir.path().join("settings.json")).unwrap(),
            want
        );
        for text in [
            r#"{"version":1,"spaces":{}}"#.to_string(),
            reminders_text("null"),
            reminders_text("{}"),
            reminders_text(r#"{"longRunningHours":null,"notRunningAfter":null}"#),
        ] {
            assert_eq!(
                load_reminder_settings(&write(&dir, &text)).unwrap(),
                want,
                "{text}"
            );
        }
    }

    #[test]
    fn reminders_false_switches_each_off_separately() {
        let dir = tempfile::tempdir().unwrap();
        let text = reminders_text(r#"{"longRunningHours":false}"#);
        let s = load_reminder_settings(&write(&dir, &text)).unwrap();
        assert_eq!((s.long_running, s.not_running_after), (None, hm(9, 0)));
        let text = reminders_text(r#"{"notRunningAfter":false}"#);
        let s = load_reminder_settings(&write(&dir, &text)).unwrap();
        assert_eq!((s.long_running, s.not_running_after), (hours(10), None));
        let both = reminders_text(r#"{"longRunningHours":false,"notRunningAfter":false}"#);
        let s = load_reminder_settings(&write(&dir, &both)).unwrap();
        assert_eq!((s.long_running, s.not_running_after), (None, None));
    }

    #[test]
    fn reminders_range_edges_are_accepted() {
        let dir = tempfile::tempdir().unwrap();
        for (h, t) in [(1u64, "00:00"), (24, "23:59"), (7, "09:30")] {
            let text = reminders_text(&format!(
                r#"{{"longRunningHours":{h},"notRunningAfter":"{t}"}}"#
            ));
            let s = load_reminder_settings(&write(&dir, &text)).unwrap();
            assert_eq!(s.long_running, hours(h), "{t}");
            assert_eq!(
                s.not_running_after,
                Some(NaiveTime::parse_from_str(t, "%H:%M").unwrap())
            );
        }
    }

    #[test]
    fn reminders_invalid_values_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let blocks = [
            r#"{"longRunningHours":0}"#,
            r#"{"longRunningHours":25}"#,
            r#"{"longRunningHours":-1}"#,
            r#"{"longRunningHours":1.5}"#,
            r#"{"longRunningHours":true}"#,
            r#"{"longRunningHours":"10"}"#,
            r#"{"longRunningHours":[10]}"#,
            r#"{"notRunningAfter":true}"#,
            r#"{"notRunningAfter":9}"#,
            r#"{"notRunningAfter":"9:00"}"#,
            r#"{"notRunningAfter":"24:00"}"#,
            r#"{"notRunningAfter":"09:60"}"#,
            r#"{"notRunningAfter":"09:00:00"}"#,
            r#"{"notRunningAfter":"+9:00"}"#,
            r#"{"notRunningAfter":""}"#,
            r#"{"notRunningAfter":["09:00"]}"#,
            r#"{"extra":1}"#,
            r#"{"longRunningHours":5,"other":1}"#,
            r#""10""#,
            r#"[10]"#,
            r#"true"#,
        ];
        for block in blocks {
            let err = load_reminder_settings(&write(&dir, &reminders_text(block))).unwrap_err();
            assert_eq!(err, ReminderSettingsError::SettingsInvalid, "{block}");
        }
        for text in [
            "{ not json",
            r#"{"reminders":{"longRunningHours":5}}"#,
            r#"{"version":2,"reminders":{"longRunningHours":5}}"#,
        ] {
            let err = load_reminder_settings(&write(&dir, text)).unwrap_err();
            assert_eq!(err, ReminderSettingsError::SettingsInvalid, "{text}");
        }
        let p = dir.path().join("settings.json");
        fs::write(&p, [0xFF, 0xFE, 0x00]).unwrap();
        assert!(load_reminder_settings(&p).is_err());
    }

    #[test]
    fn reminders_bom_is_stripped() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(reminders_text(r#"{"longRunningHours":3}"#).as_bytes());
        fs::write(&p, bytes).unwrap();
        assert_eq!(load_reminder_settings(&p).unwrap().long_running, hours(3));
    }

    #[test]
    fn reminders_loader_ignores_broken_todo_and_gitlab_blocks() {
        let dir = tempfile::tempdir().unwrap();
        let text = r#"{"version":1,"spaces":{},"gitlab":"nope","todo":{"limit":0},"reminders":{"longRunningHours":4}}"#;
        let p = write(&dir, text);
        assert_eq!(load_reminder_settings(&p).unwrap().long_running, hours(4));
        assert!(load_todo_limit(&p).is_err());
        assert!(load_gitlab_settings(&p).is_err());
    }

    #[test]
    fn broken_reminders_block_leaves_other_loaders_green() {
        let dir = tempfile::tempdir().unwrap();
        for block in [r#"{"longRunningHours":0}"#, r#""nope""#, r#"{"bogus":1}"#] {
            let text = format!(
                r#"{{"version":1,"gitlab":{{"host":"gitlab.example.invalid"}},"todo":{{"limit":3}},"reminders":{block}}}"#
            );
            let p = write(&dir, &text);
            assert!(check_settings(&p).is_ok(), "{block}");
            assert!(load_gitlab_settings(&p).unwrap().is_some(), "{block}");
            assert_eq!(load_todo_limit(&p).unwrap(), 3, "{block}");
            assert!(load_reminder_settings(&p).is_err(), "{block}");
        }
    }

    fn origin(long: bool, not_running: bool) -> ReminderOrigin {
        ReminderOrigin {
            long_running_from_file: long,
            not_running_after_from_file: not_running,
        }
    }

    #[test]
    fn reminder_origin_missing_file_and_absent_block_are_not_set() {
        let dir = tempfile::tempdir().unwrap();
        let (s, o) = load_reminder_settings_detailed(&dir.path().join("settings.json")).unwrap();
        assert_eq!(s, ReminderSettings::default());
        assert_eq!(o, origin(false, false));
        for text in [
            r#"{"version":1,"spaces":{}}"#.to_string(),
            reminders_text("null"),
            reminders_text("{}"),
            reminders_text(r#"{"longRunningHours":null,"notRunningAfter":null}"#),
        ] {
            let (s, o) = load_reminder_settings_detailed(&write(&dir, &text)).unwrap();
            assert_eq!(s, ReminderSettings::default(), "{text}");
            assert_eq!(o, origin(false, false), "{text}");
        }
    }

    #[test]
    fn reminder_origin_follows_each_field() {
        let dir = tempfile::tempdir().unwrap();
        let cases = [
            (r#"{"longRunningHours":4}"#, origin(true, false)),
            (r#"{"notRunningAfter":"08:15"}"#, origin(false, true)),
            (
                r#"{"longRunningHours":4,"notRunningAfter":null}"#,
                origin(true, false),
            ),
            (
                r#"{"longRunningHours":null,"notRunningAfter":"08:15"}"#,
                origin(false, true),
            ),
            (
                r#"{"longRunningHours":4,"notRunningAfter":"08:15"}"#,
                origin(true, true),
            ),
        ];
        for (block, want) in cases {
            let (_, o) =
                load_reminder_settings_detailed(&write(&dir, &reminders_text(block))).unwrap();
            assert_eq!(o, want, "{block}");
        }
    }

    #[test]
    fn reminder_origin_false_counts_as_set_and_switches_off() {
        let dir = tempfile::tempdir().unwrap();
        let text = reminders_text(r#"{"longRunningHours":false}"#);
        let (s, o) = load_reminder_settings_detailed(&write(&dir, &text)).unwrap();
        assert_eq!((s.long_running, s.not_running_after), (None, hm(9, 0)));
        assert_eq!(o, origin(true, false));
        let text = reminders_text(r#"{"notRunningAfter":false}"#);
        let (s, o) = load_reminder_settings_detailed(&write(&dir, &text)).unwrap();
        assert_eq!((s.long_running, s.not_running_after), (hours(10), None));
        assert_eq!(o, origin(false, true));
    }

    #[test]
    fn reminder_detailed_loader_agrees_with_the_plain_one() {
        let dir = tempfile::tempdir().unwrap();
        let valid = [
            r#"null"#,
            r#"{}"#,
            r#"{"longRunningHours":null,"notRunningAfter":null}"#,
            r#"{"longRunningHours":false}"#,
            r#"{"notRunningAfter":false}"#,
            r#"{"longRunningHours":false,"notRunningAfter":false}"#,
            r#"{"longRunningHours":1,"notRunningAfter":"00:00"}"#,
            r#"{"longRunningHours":24,"notRunningAfter":"23:59"}"#,
            r#"{"longRunningHours":7,"notRunningAfter":"09:30"}"#,
        ];
        for block in valid {
            let p = write(&dir, &reminders_text(block));
            assert_eq!(
                load_reminder_settings_detailed(&p).unwrap().0,
                load_reminder_settings(&p).unwrap(),
                "{block}"
            );
        }
        let invalid_blocks = [
            r#"{"longRunningHours":0}"#,
            r#"{"longRunningHours":25}"#,
            r#"{"longRunningHours":true}"#,
            r#"{"notRunningAfter":true}"#,
            r#"{"notRunningAfter":"9:00"}"#,
            r#"{"notRunningAfter":"24:00"}"#,
            r#"{"extra":1}"#,
            r#""10""#,
        ];
        for block in invalid_blocks {
            let p = write(&dir, &reminders_text(block));
            assert_eq!(
                load_reminder_settings_detailed(&p),
                Err(ReminderSettingsError::SettingsInvalid),
                "{block}"
            );
            assert!(load_reminder_settings(&p).is_err(), "{block}");
        }
        for text in [
            "{ not json",
            r#"{"reminders":{"longRunningHours":5}}"#,
            r#"{"version":2,"reminders":{"longRunningHours":5}}"#,
        ] {
            let p = write(&dir, text);
            assert_eq!(
                load_reminder_settings_detailed(&p),
                Err(ReminderSettingsError::SettingsInvalid),
                "{text}"
            );
        }
        let p = dir.path().join("settings.json");
        fs::write(&p, [0xFF, 0xFE, 0x00]).unwrap();
        assert!(load_reminder_settings_detailed(&p).is_err());
    }

    #[test]
    fn reminder_detailed_loader_strips_the_bom() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(reminders_text(r#"{"longRunningHours":3}"#).as_bytes());
        fs::write(&p, bytes).unwrap();
        let (s, o) = load_reminder_settings_detailed(&p).unwrap();
        assert_eq!(s.long_running, hours(3));
        assert_eq!(o, origin(true, false));
    }

    #[test]
    fn reminder_error_serializes_as_a_tagged_kind() {
        let json = serde_json::to_string(&ReminderSettingsError::SettingsInvalid).unwrap();
        assert_eq!(json, r#"{"kind":"SettingsInvalid"}"#);
    }

    fn refresh_text(block: &str) -> String {
        format!(r#"{{"version":1,"spaces":{{}},"gitlabRefresh":{block}}}"#)
    }

    #[test]
    fn refresh_minutes_defaults_and_file_level_problems_give_ten() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            load_gitlab_refresh_minutes(&dir.path().join("settings.json")),
            Ok(DEFAULT_REFRESH_MINUTES)
        );
        for text in [
            r#"{"version":1,"spaces":{}}"#.to_string(),
            refresh_text("null"),
            refresh_text("{}"),
            refresh_text(r#"{"minutes":null}"#),
            "{ not json".to_string(),
            r#"{"version":2,"gitlabRefresh":{"minutes":0}}"#.to_string(),
            r#"{"gitlabRefresh":{"minutes":0}}"#.to_string(),
        ] {
            assert_eq!(
                load_gitlab_refresh_minutes(&write(&dir, &text)),
                Ok(10),
                "{text}"
            );
        }
        let p = dir.path().join("settings.json");
        fs::write(&p, [0xFF, 0xFE, 0x00]).unwrap();
        assert_eq!(load_gitlab_refresh_minutes(&p), Ok(10));
        // A directory where the file should be: unreadable, still the default.
        assert_eq!(load_gitlab_refresh_minutes(dir.path()), Ok(10));
    }

    #[test]
    fn refresh_minutes_range_edges() {
        let dir = tempfile::tempdir().unwrap();
        for n in [1u32, 7, 14] {
            let text = refresh_text(&format!(r#"{{"minutes":{n}}}"#));
            assert_eq!(load_gitlab_refresh_minutes(&write(&dir, &text)), Ok(n));
        }
    }

    #[test]
    fn refresh_minutes_invalid_blocks_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        for block in [
            r#"{"minutes":0}"#,
            r#"{"minutes":15}"#,
            r#"{"minutes":"10"}"#,
            r#"{"minutes":10.5}"#,
            r#"{"minutes":true}"#,
            r#"{"minutes":-1}"#,
            r#"{"minutes":5,"extra":1}"#,
            r#"{"other":1}"#,
            r#""10""#,
            r#"[10]"#,
        ] {
            assert_eq!(
                load_gitlab_refresh_minutes(&write(&dir, &refresh_text(block))),
                Err(RefreshSettingInvalid),
                "{block}"
            );
        }
    }

    #[test]
    fn refresh_minutes_bom_is_stripped() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(refresh_text(r#"{"minutes":3}"#).as_bytes());
        fs::write(&p, bytes).unwrap();
        assert_eq!(load_gitlab_refresh_minutes(&p), Ok(3));
    }

    #[test]
    fn broken_refresh_block_leaves_the_other_loaders_green() {
        let dir = tempfile::tempdir().unwrap();
        for block in [r#"{"minutes":0}"#, r#""nope""#, r#"{"bogus":1}"#] {
            let text = format!(
                r#"{{"version":1,"gitlab":{{"host":"gitlab.example.invalid"}},"todo":{{"limit":4}},"gitlabRefresh":{block}}}"#
            );
            let p = write(&dir, &text);
            assert!(check_settings(&p).is_ok(), "{block}");
            assert!(load_gitlab_settings(&p).unwrap().is_some(), "{block}");
            assert_eq!(load_todo_limit(&p).unwrap(), 4, "{block}");
            assert!(load_gitlab_refresh_minutes(&p).is_err(), "{block}");
        }
    }

    #[test]
    fn refresh_loader_ignores_a_broken_gitlab_block() {
        let dir = tempfile::tempdir().unwrap();
        let text = r#"{"version":1,"spaces":{},"gitlab":"nope","todo":{"limit":0},"gitlabRefresh":{"minutes":6}}"#;
        assert_eq!(load_gitlab_refresh_minutes(&write(&dir, text)), Ok(6));
    }

    fn github_refresh_text(block: &str) -> String {
        format!(r#"{{"version":1,"spaces":{{}},"githubRefresh":{block}}}"#)
    }

    #[test]
    fn github_refresh_minutes_defaults_and_file_level_problems_give_ten() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            load_github_refresh_minutes(&dir.path().join("settings.json")),
            Ok(DEFAULT_REFRESH_MINUTES)
        );
        for text in [
            r#"{"version":1,"spaces":{}}"#.to_string(),
            github_refresh_text("null"),
            github_refresh_text("{}"),
            github_refresh_text(r#"{"minutes":null}"#),
            "{ not json".to_string(),
            r#"{"version":2,"githubRefresh":{"minutes":0}}"#.to_string(),
            r#"{"githubRefresh":{"minutes":0}}"#.to_string(),
        ] {
            assert_eq!(
                load_github_refresh_minutes(&write(&dir, &text)),
                Ok(10),
                "{text}"
            );
        }
        let p = dir.path().join("settings.json");
        fs::write(&p, [0xFF, 0xFE, 0x00]).unwrap();
        assert_eq!(load_github_refresh_minutes(&p), Ok(10));
        assert_eq!(load_github_refresh_minutes(dir.path()), Ok(10));
    }

    #[test]
    fn github_refresh_minutes_range_edges() {
        let dir = tempfile::tempdir().unwrap();
        for n in [1u32, 7, 14] {
            let text = github_refresh_text(&format!(r#"{{"minutes":{n}}}"#));
            assert_eq!(load_github_refresh_minutes(&write(&dir, &text)), Ok(n));
        }
    }

    #[test]
    fn github_refresh_minutes_invalid_blocks_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        for block in [
            r#"{"minutes":0}"#,
            r#"{"minutes":15}"#,
            r#"{"minutes":"10"}"#,
            r#"{"minutes":10.5}"#,
            r#"{"minutes":true}"#,
            r#"{"minutes":-1}"#,
            r#"{"minutes":5,"extra":1}"#,
            r#"{"other":1}"#,
            r#""10""#,
            r#"[10]"#,
        ] {
            assert_eq!(
                load_github_refresh_minutes(&write(&dir, &github_refresh_text(block))),
                Err(RefreshSettingInvalid),
                "{block}"
            );
        }
    }

    #[test]
    fn github_refresh_minutes_bom_is_stripped() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(github_refresh_text(r#"{"minutes":3}"#).as_bytes());
        fs::write(&p, bytes).unwrap();
        assert_eq!(load_github_refresh_minutes(&p), Ok(3));
    }

    #[test]
    fn refresh_keys_are_independent() {
        let dir = tempfile::tempdir().unwrap();
        let both = |gl: &str, gh: &str| {
            format!(r#"{{"version":1,"spaces":{{}},"gitlabRefresh":{gl},"githubRefresh":{gh}}}"#)
        };
        let p = write(&dir, &both(r#"{"minutes":4}"#, r#"{"minutes":99}"#));
        assert_eq!(load_gitlab_refresh_minutes(&p), Ok(4));
        assert_eq!(load_github_refresh_minutes(&p), Err(RefreshSettingInvalid));
        let p = write(&dir, &both(r#"{"minutes":99}"#, r#"{"minutes":5}"#));
        assert_eq!(load_gitlab_refresh_minutes(&p), Err(RefreshSettingInvalid));
        assert_eq!(load_github_refresh_minutes(&p), Ok(5));
        let p = write(&dir, &refresh_text(r#"{"minutes":3}"#));
        assert_eq!(load_gitlab_refresh_minutes(&p), Ok(3));
        assert_eq!(load_github_refresh_minutes(&p), Ok(10));
        let p = write(&dir, &github_refresh_text(r#"{"minutes":3}"#));
        assert_eq!(load_github_refresh_minutes(&p), Ok(3));
        assert_eq!(load_gitlab_refresh_minutes(&p), Ok(10));
    }

    #[test]
    fn broken_github_refresh_block_leaves_the_other_loaders_green() {
        let dir = tempfile::tempdir().unwrap();
        for block in [r#"{"minutes":0}"#, r#""nope""#, r#"{"bogus":1}"#] {
            let text = format!(
                r#"{{"version":1,"gitlab":{{"host":"gitlab.example.invalid"}},"todo":{{"limit":4}},"githubRefresh":{block}}}"#
            );
            let p = write(&dir, &text);
            assert!(check_settings(&p).is_ok(), "{block}");
            assert!(load_github_settings(&p).is_ok(), "{block}");
            assert!(load_gitlab_refresh_minutes(&p).is_ok(), "{block}");
            assert_eq!(load_todo_limit(&p).unwrap(), 4, "{block}");
            assert!(load_github_refresh_minutes(&p).is_err(), "{block}");
        }
    }

    #[test]
    fn github_refresh_loader_ignores_a_broken_github_block() {
        let dir = tempfile::tempdir().unwrap();
        let text = r#"{"version":1,"spaces":{},"github":"nope","githubRefresh":{"minutes":6}}"#;
        assert_eq!(load_github_refresh_minutes(&write(&dir, text)), Ok(6));
    }

    fn filter_text(block: &str) -> String {
        format!(r#"{{"version":1,"spaces":{{}},"gitlabFilter":{block}}}"#)
    }

    fn load_filter(
        dir: &tempfile::TempDir,
        block: &str,
    ) -> Result<GitlabFilter, FilterSettingInvalid> {
        load_gitlab_filter(&write(dir, &filter_text(block)))
    }

    #[test]
    fn filter_absent_null_and_file_level_problems_give_the_default() {
        let dir = tempfile::tempdir().unwrap();
        let default = GitlabFilter::default();
        assert_eq!(
            load_gitlab_filter(&dir.path().join("settings.json")),
            Ok(default.clone())
        );
        for text in [
            r#"{"version":1,"spaces":{}}"#.to_string(),
            filter_text("null"),
            filter_text("{}"),
            filter_text(r#"{"projects":null,"labels":{"include":null}}"#),
            "{ not json".to_string(),
            r#"{"version":2,"gitlabFilter":{"bogus":1}}"#.to_string(),
            r#"{"gitlabFilter":{"bogus":1}}"#.to_string(),
        ] {
            assert_eq!(
                load_gitlab_filter(&write(&dir, &text)),
                Ok(default.clone()),
                "{text}"
            );
        }
        let p = dir.path().join("settings.json");
        fs::write(&p, [0xFF, 0xFE, 0x00]).unwrap();
        assert_eq!(load_gitlab_filter(&p), Ok(default.clone()));
        // A directory where the file should be: unreadable, still the default.
        assert_eq!(load_gitlab_filter(dir.path()), Ok(default));
    }

    #[test]
    fn filter_reads_every_field_trimmed_and_lowercased() {
        let dir = tempfile::tempdir().unwrap();
        let f = load_filter(
            &dir,
            r#"{
                "projects": {"include": [" Acme/Backend "], "exclude": ["acme/backend/legacy"]},
                "labels": {"exclude": ["Blocked", "WIP"]},
                "milestones": {"include": ["Sprint 42"]},
                "closedDays": 7
            }"#,
        )
        .unwrap();
        assert_eq!(f.projects.include, ["acme/backend"]);
        assert_eq!(f.projects.exclude, ["acme/backend/legacy"]);
        assert!(f.labels.include.is_empty());
        assert_eq!(f.labels.exclude, ["blocked", "wip"]);
        assert_eq!(f.milestones.include, ["sprint 42"]);
        assert!(f.milestones.exclude.is_empty());
        assert_eq!(f.closed_days, Some(7));
        assert!(f.is_active());
        // Empty lists and null/absent closedDays are no constraint.
        let f = load_filter(
            &dir,
            r#"{"projects":{"include":[]},"labels":{},"closedDays":null}"#,
        )
        .unwrap();
        assert_eq!(f, GitlabFilter::default());
    }

    #[test]
    fn filter_closed_days_bounds() {
        let dir = tempfile::tempdir().unwrap();
        for (n, ok) in [(0, false), (1, true), (30, true), (31, false)] {
            let got = load_filter(&dir, &format!(r#"{{"closedDays":{n}}}"#));
            if ok {
                assert_eq!(got.unwrap().closed_days, Some(n), "{n}");
            } else {
                assert_eq!(got, Err(FilterSettingInvalid), "{n}");
            }
        }
        for bad in ["-1", "7.5", "7.0", r#""7""#, "true", "[7]", "{}"] {
            assert_eq!(
                load_filter(&dir, &format!(r#"{{"closedDays":{bad}}}"#)),
                Err(FilterSettingInvalid),
                "{bad}"
            );
        }
    }

    #[test]
    fn filter_invalid_shapes_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        for block in [
            r#""nope""#,
            "[]",
            "5",
            "true",
            r#"{"projcts":{"include":["a"]}}"#,
            r#"{"projects":{"includ":["a"]}}"#,
            r#"{"projects":{"include":["a"],"extra":1}}"#,
            r#"{"closedDays":3,"extra":1}"#,
            r#"{"projects":["a"]}"#,
            r#"{"projects":"a"}"#,
            r#"{"labels":5}"#,
            r#"{"labels":{"include":"a"}}"#,
            r#"{"labels":{"include":{"a":1}}}"#,
            r#"{"labels":{"include":[1]}}"#,
            r#"{"labels":{"exclude":["a",null]}}"#,
            r#"{"milestones":{"include":[["a"]]}}"#,
            r#"{"labels":{"include":[""]}}"#,
            r#"{"labels":{"include":["   "]}}"#,
            r#"{"labels":{"include":["a\u0007b"]}}"#,
            r#"{"labels":{"include":["a\nb"]}}"#,
            r#"{"projects":{"include":["/acme"]}}"#,
            r#"{"projects":{"include":["acme/"]}}"#,
            r#"{"projects":{"exclude":["acme//api"]}}"#,
            r#"{"projects":{"include":["/"]}}"#,
        ] {
            assert_eq!(
                load_filter(&dir, block),
                Err(FilterSettingInvalid),
                "{block}"
            );
        }
    }

    #[test]
    fn filter_entry_limits() {
        let dir = tempfile::tempdir().unwrap();
        let entry = |n: usize| format!(r#"{{"labels":{{"include":["{}"]}}}}"#, "a".repeat(n));
        assert!(load_filter(&dir, &entry(255)).is_ok());
        assert_eq!(load_filter(&dir, &entry(256)), Err(FilterSettingInvalid));
        // Length is counted after trimming.
        let padded = format!(r#"{{"labels":{{"include":["  {}  "]}}}}"#, "a".repeat(255));
        assert!(load_filter(&dir, &padded).is_ok());
        // Characters, not bytes.
        let wide = format!(r#"{{"labels":{{"include":["{}"]}}}}"#, "ż".repeat(255));
        assert!(load_filter(&dir, &wide).is_ok());

        let list = |n: usize| {
            let items: Vec<String> = (0..n).map(|i| format!(r#""l{i}""#)).collect();
            format!(r#"{{"milestones":{{"exclude":[{}]}}}}"#, items.join(","))
        };
        assert_eq!(
            load_filter(&dir, &list(100))
                .unwrap()
                .milestones
                .exclude
                .len(),
            100
        );
        assert_eq!(load_filter(&dir, &list(101)), Err(FilterSettingInvalid));
    }

    #[test]
    fn filter_errors_carry_no_value() {
        const MARKER: &str = "leak-marker-7c1d";
        let dir = tempfile::tempdir().unwrap();
        for block in [
            format!(r#"{{"{MARKER}":1}}"#),
            format!(r#"{{"projects":{{"{MARKER}":["a"]}}}}"#),
            format!(r#"{{"labels":{{"include":"{MARKER}"}}}}"#),
            format!(r#"{{"labels":{{"include":[{{"{MARKER}":1}}]}}}}"#),
            format!(r#"{{"closedDays":"{MARKER}"}}"#),
            format!(r#"{{"projects":{{"include":["{MARKER}//x"]}}}}"#),
            format!(r#""{MARKER}""#),
        ] {
            let err = load_filter(&dir, &block).unwrap_err();
            assert!(!format!("{err:?}").contains(MARKER), "{block}");
        }
        // Debug of a loaded filter is opaque.
        let f = load_filter(&dir, &format!(r#"{{"labels":{{"include":["{MARKER}"]}}}}"#)).unwrap();
        assert!(!format!("{f:?}").contains(MARKER));
    }

    #[test]
    fn filter_bom_is_stripped() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(filter_text(r#"{"closedDays":3}"#).as_bytes());
        fs::write(&p, bytes).unwrap();
        assert_eq!(load_gitlab_filter(&p).unwrap().closed_days, Some(3));
    }

    #[test]
    fn broken_filter_block_leaves_the_other_loaders_green() {
        let dir = tempfile::tempdir().unwrap();
        for block in [r#"{"closedDays":0}"#, r#""nope""#, r#"{"projcts":{}}"#] {
            let text = format!(
                r#"{{"version":1,"gitlab":{{"host":"gitlab.example.invalid"}},"todo":{{"limit":4}},"gitlabRefresh":{{"minutes":6}},"gitlabFilter":{block}}}"#
            );
            let p = write(&dir, &text);
            assert!(check_settings(&p).is_ok(), "{block}");
            assert!(load_gitlab_settings(&p).unwrap().is_some(), "{block}");
            assert_eq!(load_todo_limit(&p).unwrap(), 4, "{block}");
            assert_eq!(load_gitlab_refresh_minutes(&p), Ok(6), "{block}");
            assert!(load_gitlab_filter(&p).is_err(), "{block}");
        }
    }

    #[test]
    fn filter_loader_ignores_broken_neighbouring_blocks() {
        let dir = tempfile::tempdir().unwrap();
        let text = r#"{"version":1,"spaces":{},"gitlab":"nope","todo":{"limit":0},"gitlabRefresh":{"minutes":0},"gitlabFilter":{"closedDays":5}}"#;
        assert_eq!(
            load_gitlab_filter(&write(&dir, text)).unwrap().closed_days,
            Some(5)
        );
    }
}
