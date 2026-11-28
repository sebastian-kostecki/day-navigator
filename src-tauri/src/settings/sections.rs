//! The seven sections of the settings dialog: which blocks each one owns, what
//! its form holds, how a form is read from the file and how it becomes blocks
//! again. Every block is checked by the validator its loader uses.
//!
//! Numbers travel as strings, so parsing rules live only here. GitLab and GitHub
//! form values are work data: their `Debug` is opaque.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::autostart::validate_autostart;
use super::document::fingerprint;
use super::work_time::validate_work_time;
use super::{
    DEFAULT_LONG_RUNNING_HOURS, DEFAULT_REFRESH_MINUTES, FieldError, ReminderSettings,
    validate_github, validate_gitlab, validate_gitlab_filter, validate_notices, validate_refresh,
    validate_reminders, validate_todo,
};
use crate::gitlab::filter::DimensionRule;
use crate::todo::DEFAULT_TODO_LIMIT;
use crate::work_time::holidays::HolidayCalendar;

const NOT_A_NUMBER: &str = "To nie jest liczba.";

macro_rules! opaque_debug {
    ($($name:ident),+) => {$(
        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!(stringify!($name), "(..)"))
            }
        }
    )+};
}

#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DimensionForm {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterForm {
    pub projects: DimensionForm,
    pub labels: DimensionForm,
    pub milestones: DimensionForm,
    /// Empty is off.
    pub closed_days: String,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitlabForm {
    /// Empty is "not configured".
    pub host: String,
    pub user: String,
    pub refresh_minutes: String,
    pub filter: FilterForm,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubForm {
    /// Empty is "not configured".
    pub repos: Vec<String>,
    pub refresh_minutes: String,
}

opaque_debug!(DimensionForm, FilterForm, GitlabForm, GithubForm);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoForm {
    pub limit: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkTimeForm {
    pub holidays: bool,
    pub daily_norm_hours: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemindersForm {
    pub long_running_on: bool,
    pub long_running_hours: String,
    pub not_running_after_on: bool,
    pub not_running_after: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoticesForm {
    pub new_assignments: bool,
    pub source_problems: bool,
    pub day_summary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutostartForm {
    pub enabled: bool,
}

/// One section to save, with its form.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "section", content = "form", rename_all = "camelCase")]
pub enum SectionInput {
    Gitlab(GitlabForm),
    Github(GithubForm),
    Todo(TodoForm),
    WorkTime(WorkTimeForm),
    Reminders(RemindersForm),
    Notices(NoticesForm),
    Autostart(AutostartForm),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionView<F> {
    /// Current values; the defaults where a block is absent or invalid.
    pub form: F,
    /// Keys of the blocks that fail their validator on disk.
    pub invalid_blocks: Vec<String>,
    /// One per block of the section.
    pub fingerprints: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionViews {
    pub gitlab: SectionView<GitlabForm>,
    pub github: SectionView<GithubForm>,
    pub todo: SectionView<TodoForm>,
    pub work_time: SectionView<WorkTimeForm>,
    pub reminders: SectionView<RemindersForm>,
    pub notices: SectionView<NoticesForm>,
    pub autostart: SectionView<AutostartForm>,
}

/// Collects the invalid blocks and fingerprints while one section is read.
struct Reader<'a> {
    root: &'a Map<String, Value>,
    invalid_blocks: Vec<String>,
    fingerprints: BTreeMap<String, String>,
}

impl<'a> Reader<'a> {
    fn new(root: &'a Map<String, Value>) -> Self {
        Reader {
            root,
            invalid_blocks: Vec::new(),
            fingerprints: BTreeMap::new(),
        }
    }

    /// The block's settings; an absent block validates as `null` (the
    /// defaults). `None` marks an invalid one.
    fn block<T>(
        &mut self,
        key: &str,
        validate: impl FnOnce(&Value) -> Result<T, Vec<FieldError>>,
    ) -> Option<T> {
        let block = self.root.get(key);
        self.fingerprints
            .insert(key.to_string(), fingerprint(block));
        let result = validate(block.unwrap_or(&Value::Null)).ok();
        if result.is_none() {
            self.invalid_blocks.push(key.to_string());
        }
        result
    }

    fn finish<F>(self, form: F) -> SectionView<F> {
        SectionView {
            form,
            invalid_blocks: self.invalid_blocks,
            fingerprints: self.fingerprints,
        }
    }
}

impl SectionViews {
    pub(super) fn read(root: &Map<String, Value>) -> SectionViews {
        SectionViews {
            gitlab: read_gitlab(root),
            github: read_github(root),
            todo: read_todo(root),
            work_time: read_work_time(root),
            reminders: read_reminders(root),
            notices: read_notices(root),
            autostart: read_autostart(root),
        }
    }
}

fn read_gitlab(root: &Map<String, Value>) -> SectionView<GitlabForm> {
    let mut reader = Reader::new(root);
    let config = reader.block("gitlab", validate_gitlab).flatten();
    let minutes = reader
        .block("gitlabRefresh", |b| validate_refresh(b, "gitlabRefresh"))
        .unwrap_or(DEFAULT_REFRESH_MINUTES);
    let filter = reader
        .block("gitlabFilter", validate_gitlab_filter)
        .unwrap_or_default();
    let dimension = |rule: DimensionRule| DimensionForm {
        include: rule.include,
        exclude: rule.exclude,
    };
    let (host, user) = match config {
        Some(config) => (
            config.host.as_str().to_string(),
            config.user.map(|u| u.as_str().to_string()),
        ),
        None => (String::new(), None),
    };
    reader.finish(GitlabForm {
        host,
        user: user.unwrap_or_default(),
        refresh_minutes: minutes.to_string(),
        filter: FilterForm {
            projects: dimension(filter.projects),
            labels: dimension(filter.labels),
            milestones: dimension(filter.milestones),
            closed_days: filter
                .closed_days
                .map(|d| d.to_string())
                .unwrap_or_default(),
        },
    })
}

fn read_github(root: &Map<String, Value>) -> SectionView<GithubForm> {
    let mut reader = Reader::new(root);
    let config = reader.block("github", validate_github).flatten();
    let minutes = reader
        .block("githubRefresh", |b| validate_refresh(b, "githubRefresh"))
        .unwrap_or(DEFAULT_REFRESH_MINUTES);
    let repos = config
        .map(|c| c.repos.iter().map(|r| r.as_str().to_string()).collect())
        .unwrap_or_default();
    reader.finish(GithubForm {
        repos,
        refresh_minutes: minutes.to_string(),
    })
}

fn read_todo(root: &Map<String, Value>) -> SectionView<TodoForm> {
    let mut reader = Reader::new(root);
    let limit = reader
        .block("todo", validate_todo)
        .unwrap_or(DEFAULT_TODO_LIMIT);
    reader.finish(TodoForm {
        limit: limit.to_string(),
    })
}

fn read_work_time(root: &Map<String, Value>) -> SectionView<WorkTimeForm> {
    let mut reader = Reader::new(root);
    let settings = reader
        .block("workTime", validate_work_time)
        .unwrap_or_default();
    // The norm is whole minutes; the shortest `f64` text parses back to it.
    let hours = (settings.daily_norm_ms / 60_000) as f64 / 60.0;
    reader.finish(WorkTimeForm {
        holidays: settings.holidays == HolidayCalendar::Pl,
        daily_norm_hours: hours.to_string(),
    })
}

fn read_reminders(root: &Map<String, Value>) -> SectionView<RemindersForm> {
    let mut reader = Reader::new(root);
    let settings = reader
        .block("reminders", validate_reminders)
        .map(|(settings, _)| settings)
        .unwrap_or_default();
    // A reminder that is off still shows the default, ready to switch on.
    let default_time = ReminderSettings::default().not_running_after;
    reader.finish(RemindersForm {
        long_running_on: settings.long_running.is_some(),
        long_running_hours: settings
            .long_running
            .map_or(DEFAULT_LONG_RUNNING_HOURS, |d| d.as_secs() / 3600)
            .to_string(),
        not_running_after_on: settings.not_running_after.is_some(),
        not_running_after: settings
            .not_running_after
            .or(default_time)
            .map(|t| t.format("%H:%M").to_string())
            .unwrap_or_default(),
    })
}

fn read_notices(root: &Map<String, Value>) -> SectionView<NoticesForm> {
    let mut reader = Reader::new(root);
    let settings = reader
        .block("notices", validate_notices)
        .unwrap_or_default();
    reader.finish(NoticesForm {
        new_assignments: settings.new_assignments,
        source_problems: settings.source_problems,
        day_summary: settings.day_summary,
    })
}

fn read_autostart(root: &Map<String, Value>) -> SectionView<AutostartForm> {
    let mut reader = Reader::new(root);
    let settings = reader
        .block("autostart", validate_autostart)
        .unwrap_or_default();
    reader.finish(AutostartForm {
        enabled: settings.enabled,
    })
}

/// Runs the validator of the block `key`, keeping only its errors.
pub(super) fn check_block(key: &str, block: &Value) -> Result<(), Vec<FieldError>> {
    match key {
        "gitlab" => validate_gitlab(block).map(drop),
        "github" => validate_github(block).map(drop),
        "gitlabRefresh" | "githubRefresh" => validate_refresh(block, key).map(drop),
        "gitlabFilter" => validate_gitlab_filter(block).map(drop),
        "todo" => validate_todo(block).map(drop),
        "workTime" => validate_work_time(block).map(drop),
        "reminders" => validate_reminders(block).map(drop),
        "notices" => validate_notices(block).map(drop),
        "autostart" => validate_autostart(block).map(drop),
        _ => Ok(()),
    }
}

/// A block to write; `None` leaves its key out of the file.
pub(super) type Block = (&'static str, Option<Value>);

/// An object in the order given; no keys at all is no block.
fn object<const N: usize>(entries: [(&str, Option<Value>); N]) -> Option<Value> {
    let map: Map<String, Value> = entries
        .into_iter()
        .filter_map(|(key, value)| value.map(|v| (key.to_string(), v)))
        .collect();
    (!map.is_empty()).then_some(Value::Object(map))
}

fn text(value: &str) -> Option<Value> {
    (!value.is_empty()).then(|| Value::from(value))
}

fn list(entries: &[String]) -> Option<Value> {
    (!entries.is_empty()).then(|| Value::from(entries.to_vec()))
}

fn dimension(form: &DimensionForm) -> Option<Value> {
    object([
        ("include", list(&form.include)),
        ("exclude", list(&form.exclude)),
    ])
}

/// A numeric input as a JSON number; empty is `None` (the default). Range and
/// integer rules stay with the validator.
fn number(input: &str, field: &str, errors: &mut Vec<FieldError>) -> Option<Value> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    if let Ok(n) = input.parse::<i64>() {
        return Some(Value::from(n));
    }
    match input.parse::<f64>() {
        Ok(n) if n.is_finite() => Some(Value::from(n)),
        _ => {
            errors.push(FieldError::new(field, NOT_A_NUMBER));
            None
        }
    }
}

impl SectionInput {
    /// The section's id, as the UI names it; safe to log.
    pub fn name(&self) -> &'static str {
        match self {
            SectionInput::Gitlab(_) => "gitlab",
            SectionInput::Github(_) => "github",
            SectionInput::Todo(_) => "todo",
            SectionInput::WorkTime(_) => "workTime",
            SectionInput::Reminders(_) => "reminders",
            SectionInput::Notices(_) => "notices",
            SectionInput::Autostart(_) => "autostart",
        }
    }

    /// The top-level keys this section owns.
    pub(super) fn block_keys(&self) -> &'static [&'static str] {
        match self {
            SectionInput::Gitlab(_) => &["gitlab", "gitlabRefresh", "gitlabFilter"],
            SectionInput::Github(_) => &["github", "githubRefresh"],
            SectionInput::Todo(_) => &["todo"],
            SectionInput::WorkTime(_) => &["workTime"],
            SectionInput::Reminders(_) => &["reminders"],
            SectionInput::Notices(_) => &["notices"],
            SectionInput::Autostart(_) => &["autostart"],
        }
    }

    /// The blocks the form stands for, all validated. The errors cover every
    /// bad field of every block.
    pub(super) fn build(&self) -> Result<Vec<Block>, Vec<FieldError>> {
        let mut errors = Vec::new();
        let blocks = self.blocks(&mut errors);
        for (key, block) in &blocks {
            if let Some(Err(found)) = block.as_ref().map(|b| check_block(key, b)) {
                errors.extend(found);
            }
        }
        if errors.is_empty() {
            Ok(blocks)
        } else {
            Err(errors)
        }
    }

    /// The blocks as typed; a number that does not parse is reported and left
    /// out.
    fn blocks(&self, errors: &mut Vec<FieldError>) -> Vec<Block> {
        match self {
            SectionInput::Gitlab(form) => {
                // No host is "not configured", whatever the user field says.
                let gitlab = if form.host.is_empty() {
                    None
                } else {
                    object([("host", text(&form.host)), ("user", text(&form.user))])
                };
                let minutes = number(&form.refresh_minutes, "gitlabRefresh.minutes", errors);
                let filter = &form.filter;
                let closed_days = number(&filter.closed_days, "gitlabFilter.closedDays", errors);
                let gitlab_filter = object([
                    ("projects", dimension(&filter.projects)),
                    ("labels", dimension(&filter.labels)),
                    ("milestones", dimension(&filter.milestones)),
                    ("closedDays", closed_days),
                ]);
                vec![
                    ("gitlab", gitlab),
                    ("gitlabRefresh", object([("minutes", minutes)])),
                    ("gitlabFilter", gitlab_filter),
                ]
            }
            SectionInput::Github(form) => {
                let minutes = number(&form.refresh_minutes, "githubRefresh.minutes", errors);
                vec![
                    ("github", object([("repos", list(&form.repos))])),
                    ("githubRefresh", object([("minutes", minutes)])),
                ]
            }
            SectionInput::Todo(form) => {
                let limit = number(&form.limit, "todo.limit", errors);
                vec![("todo", object([("limit", limit)]))]
            }
            SectionInput::WorkTime(form) => {
                // `true` is the default (`"pl"`), so only `false` is written.
                let holidays = (!form.holidays).then_some(Value::Bool(false));
                // A decimal comma is accepted for the hours.
                let hours = form.daily_norm_hours.replace(',', ".");
                let norm = number(&hours, "workTime.dailyNormHours", errors);
                let work_time = object([("holidays", holidays), ("dailyNormHours", norm)]);
                vec![("workTime", work_time)]
            }
            SectionInput::Reminders(form) => {
                let off = Some(Value::Bool(false));
                let hours = if form.long_running_on {
                    let field = "reminders.longRunningHours";
                    number(&form.long_running_hours, field, errors)
                } else {
                    off.clone()
                };
                let after = if form.not_running_after_on {
                    text(form.not_running_after.trim())
                } else {
                    off
                };
                let reminders = object([("longRunningHours", hours), ("notRunningAfter", after)]);
                vec![("reminders", reminders)]
            }
            SectionInput::Notices(form) => {
                let notices = object([
                    ("newAssignments", Some(Value::Bool(form.new_assignments))),
                    ("sourceProblems", Some(Value::Bool(form.source_problems))),
                    ("daySummary", Some(Value::Bool(form.day_summary))),
                ]);
                vec![("notices", notices)]
            }
            SectionInput::Autostart(form) => {
                let autostart = object([("enabled", Some(Value::Bool(form.enabled)))]);
                vec![("autostart", autostart)]
            }
        }
    }
}
