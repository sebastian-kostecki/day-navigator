//! Task notes: which fields are read and written, how a template becomes a
//! note, and the `Done` rule. Pure functions plus `Vault` calls; time comes in
//! as parameters. Nothing here imports `tauri`.
//!
//! Invariant: no error variant carries note content.

use chrono::{NaiveDate, NaiveDateTime};
use unicode_normalization::UnicodeNormalization;

use crate::settings::TaskSettings;
use crate::vault::edit::{self, FieldValue, check_plain};
use crate::vault::markdown::{self, BOM, SpanHash, find_field, span_hash};
use crate::vault::{EntryKind, Vault, VaultError, VaultPath};

/// NTFS limit for one file name, in UTF-16 code units.
const MAX_FILE_NAME: usize = 255;
/// Longest unsupported-placeholder text an error repeats back.
const MAX_TOKEN_LABEL: usize = 40;

#[derive(Debug, Clone, thiserror::Error, serde::Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum TaskError {
    #[error(transparent)]
    Vault(VaultError),
    #[error("tasks are not configured in settings")]
    TasksNotConfigured,
    #[error("tasks folder is missing: {path}")]
    FolderMissing { path: String },
    #[error("task template is missing: {path}")]
    TemplateMissing { path: String },
    #[error("template uses an unsupported placeholder: {placeholder}")]
    TemplateUnsupported { placeholder: String },
    #[error("template has no field {key}")]
    TemplateFieldMissing { key: String },
    #[error("invalid title: {reason}")]
    InvalidTitle { reason: String },
    #[error("unknown status: {status}")]
    UnknownStatus { status: String },
    #[error("unknown priority: {priority}")]
    UnknownPriority { priority: String },
    #[error("invalid due date: {value}")]
    InvalidDue { value: String },
    #[error("not a task note: {path}")]
    NotATask { path: String },
}

impl From<VaultError> for TaskError {
    fn from(e: VaultError) -> Self {
        TaskError::Vault(e)
    }
}

/// Every command resolves its config through this.
pub fn require_config(tasks: Option<&TaskSettings>) -> Result<&TaskSettings, TaskError> {
    tasks.ok_or(TaskError::TasksNotConfigured)
}

fn ser_path<S: serde::Serializer>(p: &VaultPath, s: S) -> Result<S::Ok, S::Error> {
    s.collect_str(p)
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRow {
    #[serde(serialize_with = "ser_path")]
    pub path: VaultPath,
    pub title: String,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub due: Option<String>,
    pub status_hash: Option<SpanHash>,
    pub done_hash: Option<SpanHash>,
    pub problem: Option<VaultError>,
    /// `Priority`/`Due` read errors. They never disable the status change.
    pub field_problems: Vec<VaultError>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewTask {
    pub title: String,
    pub priority: Option<String>,
    pub due: Option<String>,
}

/// Outcome of a status change. Every value except `NoWrite` means `Status`
/// was written; the name says what happened to `Done`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum DoneUpdate {
    /// `Done` set to today.
    Set,
    /// `Done` emptied.
    Cleared,
    /// `Done` left as it was: the move did not cross the done status.
    Unchanged,
    /// The move crossed the done status but the note has no `Done` field.
    Absent,
    /// The note already had this status; nothing was written.
    NoWrite,
}

// ---------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------

/// Scalar value of a field block (`key: value` plus any continuation lines):
/// trimmed, surrounding quotes stripped, empty becomes `None`. A list (block
/// continuation lines or an inline `[..]`) is `InvalidValue`.
fn scalar_value(raw_block: &str, key: &str) -> Result<Option<String>, VaultError> {
    let mut lines = raw_block.lines();
    let first = lines.next().unwrap_or("");
    let rest = first.strip_prefix(key).and_then(|r| r.strip_prefix(':'));
    let value = rest.unwrap_or("").trim();
    if lines.any(|l| !l.trim().is_empty()) || value.starts_with('[') {
        return Err(VaultError::InvalidValue {
            reason: format!("field {key} is a list"),
        });
    }
    let value = value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
        .unwrap_or(value)
        .trim();
    Ok((!value.is_empty()).then(|| value.to_string()))
}

fn file_stem(name: &str) -> String {
    match name.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem.to_string(),
        _ => name.to_string(),
    }
}

fn read_row(vault: &Vault, path: &VaultPath, title: String) -> TaskRow {
    let mut row = TaskRow {
        path: path.clone(),
        title,
        status: None,
        priority: None,
        due: None,
        status_hash: None,
        done_hash: None,
        problem: None,
        field_problems: Vec::new(),
    };
    let note = match vault.read_note(path) {
        Ok(n) => n,
        Err(e) => {
            row.problem = Some(e);
            return row;
        }
    };
    match note.field("Status") {
        Ok(f) => match scalar_value(&f.raw_block, "Status") {
            Ok(v) => {
                row.status = v;
                row.status_hash = Some(f.hash);
            }
            Err(e) => {
                row.problem = Some(e);
                return row;
            }
        },
        Err(e) => {
            row.problem = Some(e);
            return row;
        }
    }
    match note.field("Done") {
        Ok(f) => row.done_hash = Some(f.hash),
        Err(VaultError::FieldNotFound { .. }) => {}
        Err(e) => row.problem = Some(e),
    }
    let mut field_problems = Vec::new();
    let mut optional = |key: &str| match note
        .field(key)
        .and_then(|f| scalar_value(&f.raw_block, key))
    {
        Ok(v) => v,
        Err(VaultError::FieldNotFound { .. }) => None,
        Err(e) => {
            field_problems.push(e);
            None
        }
    };
    row.priority = optional("Priority");
    row.due = optional("Due");
    row.field_problems = field_problems;
    row
}

fn is_template(cfg: &TaskSettings, path: &VaultPath) -> bool {
    path.segments().len() == cfg.template.segments().len() && path.starts_with_ci(&cfg.template)
}

/// A task is a `.md` note directly inside the tasks folder, other than the template.
fn is_task_path(cfg: &TaskSettings, path: &VaultPath) -> bool {
    path.is_markdown()
        && path.segments().len() == cfg.folder.segments().len() + 1
        && path.starts_with_ci(&cfg.folder)
        && !is_template(cfg, path)
}

pub fn list_tasks(vault: &Vault, cfg: &TaskSettings) -> Result<Vec<TaskRow>, TaskError> {
    let entries = vault.list(Some(&cfg.folder)).map_err(|e| match e {
        VaultError::NotFound { .. } => TaskError::FolderMissing {
            path: cfg.folder.to_string(),
        },
        other => TaskError::Vault(other),
    })?;
    let mut rows = Vec::new();
    for entry in entries {
        if entry.kind != EntryKind::Note {
            continue;
        }
        if is_template(cfg, &entry.path) {
            continue;
        }
        let title = file_stem(&entry.name);
        rows.push(read_row(vault, &entry.path, title));
    }
    let rank = |r: &TaskRow| {
        r.status
            .as_ref()
            .and_then(|s| cfg.statuses.iter().position(|c| c == s))
            .unwrap_or(cfg.statuses.len())
    };
    rows.sort_by_cached_key(|r| {
        (
            rank(r),
            r.due.is_none(),
            r.due.clone(),
            r.title.to_lowercase(),
        )
    });
    Ok(rows)
}

// ---------------------------------------------------------------------------
// Template
// ---------------------------------------------------------------------------

/// An unsupported token as an error repeats it: first line only, capped, so
/// no stretch of template text rides along in the error.
fn token_label(token: &str) -> String {
    let line = token.lines().next().unwrap_or("");
    let mut label: String = line.chars().take(MAX_TOKEN_LABEL).collect();
    if label.len() < token.len() {
        label.push('…');
    }
    label
}

/// The earliest `<%` (Templater) or `{{..}}` other than title/date/time.
fn first_unsupported(template: &str) -> Option<String> {
    let templater = template.find("<%").map(|i| {
        let end = template[i..].find("%>").map_or(i + 2, |e| i + e + 2);
        (i, &template[i..end])
    });
    let mut braces = None;
    let mut pos = 0;
    while let Some(rel) = template[pos..].find("{{") {
        let open = pos + rel;
        let Some(close_rel) = template[open + 2..].find("}}") else {
            break;
        };
        let close = open + 2 + close_rel;
        if !matches!(&template[open + 2..close], "title" | "date" | "time") {
            braces = Some((open, &template[open..close + 2]));
            break;
        }
        pos = close + 2;
    }
    let first = match (templater, braces) {
        (Some(t), Some(b)) => Some(if t.0 < b.0 { t } else { b }),
        (t, b) => t.or(b),
    };
    first.map(|(_, token)| token_label(token))
}

/// Fills `{{title}}`, `{{date}}` and `{{time}}`. Any other `{{..}}` or `<%` is
/// refused, naming the earliest one. A title inside the frontmatter must be a
/// plain YAML scalar without `,` or `'`, so it also fits a flow list
/// (`[{{title}}]`) or a single-quoted value.
pub fn render_template(
    template: &str,
    title: &str,
    now: NaiveDateTime,
) -> Result<String, TaskError> {
    if let Some(placeholder) = first_unsupported(template) {
        return Err(TaskError::TemplateUnsupported { placeholder });
    }
    let fm = markdown::frontmatter_span(template.as_bytes());
    let date = now.format("%Y-%m-%d").to_string();
    let time = now.format("%H:%M").to_string();
    let mut out = String::with_capacity(template.len() + title.len());
    let mut pos = 0;
    while let Some(rel) = template[pos..].find("{{") {
        let open = pos + rel;
        let Some(close_rel) = template[open + 2..].find("}}") else {
            break;
        };
        let close = open + 2 + close_rel;
        out.push_str(&template[pos..open]);
        match &template[open + 2..close] {
            "title" => {
                if fm.is_some_and(|s| open >= s.start && open < s.end) {
                    check_plain(title).map_err(|e| TaskError::InvalidTitle {
                        reason: e.to_string(),
                    })?;
                    if title.contains([',', '\'']) {
                        return Err(invalid_title(
                            "a title in the template's frontmatter must not contain a comma or an apostrophe",
                        ));
                    }
                }
                out.push_str(title);
            }
            "date" => out.push_str(&date),
            "time" => out.push_str(&time),
            _ => {
                // `first_unsupported` refused every other token above.
                return Err(TaskError::TemplateUnsupported {
                    placeholder: token_label(&template[open..close + 2]),
                });
            }
        }
        pos = close + 2;
    }
    out.push_str(&template[pos..]);
    Ok(out)
}

// ---------------------------------------------------------------------------
// Create
// ---------------------------------------------------------------------------

fn invalid_title(reason: &str) -> TaskError {
    TaskError::InvalidTitle {
        reason: reason.to_string(),
    }
}

fn validate_title(cfg: &TaskSettings, title: &str) -> Result<(String, VaultPath), TaskError> {
    // NFC, as `VaultPath` compares: an NFD twin of an existing note would
    // otherwise become a second, visually identical file.
    let title: String = title.trim().nfc().collect();
    let title = title.as_str();
    if title.is_empty() {
        return Err(invalid_title("title is empty"));
    }
    if title.contains(['/', '\\', '[', ']', '#', '^', '|']) {
        return Err(invalid_title(
            "title contains a character not allowed in a task name",
        ));
    }
    if title.ends_with('.') {
        return Err(invalid_title("title ends with a dot"));
    }
    if title.encode_utf16().count() + ".md".len() > MAX_FILE_NAME {
        return Err(invalid_title("title is too long for a file name"));
    }
    let path = cfg
        .folder
        .join(&format!("{title}.md"))
        .map_err(|_| invalid_title("title is not a valid file name"))?;
    Ok((title.to_string(), path))
}

fn validate_due(value: &str) -> Result<(), TaskError> {
    let b = value.as_bytes();
    let shaped = b.len() == 10
        && b.iter().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                *c == b'-'
            } else {
                c.is_ascii_digit()
            }
        });
    if shaped && NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok() {
        Ok(())
    } else {
        Err(TaskError::InvalidDue {
            value: value.to_string(),
        })
    }
}

/// Finds `key` in the freshly rendered bytes and returns its expected hash.
fn template_field(bytes: &[u8], key: &str) -> Result<SpanHash, TaskError> {
    match find_field(bytes, key) {
        Ok(span) => Ok(span_hash(bytes, span)),
        Err(VaultError::FieldNotFound { .. } | VaultError::NoFrontmatter) => {
            Err(TaskError::TemplateFieldMissing {
                key: key.to_string(),
            })
        }
        Err(e) => Err(e.into()),
    }
}

pub fn create_task(
    vault: &Vault,
    cfg: &TaskSettings,
    new: &NewTask,
    now: NaiveDateTime,
) -> Result<VaultPath, TaskError> {
    let (title, path) = validate_title(cfg, &new.title)?;
    if let Some(p) = &new.priority {
        if !cfg.priorities.contains(p) {
            return Err(TaskError::UnknownPriority {
                priority: p.clone(),
            });
        }
    }
    if let Some(d) = &new.due {
        validate_due(d)?;
    }
    let folder_missing = |e: VaultError| match e {
        VaultError::NotFound { .. } => TaskError::FolderMissing {
            path: cfg.folder.to_string(),
        },
        other => TaskError::Vault(other),
    };
    vault.list(Some(&cfg.folder)).map_err(folder_missing)?;
    let note = vault.read_note(&cfg.template).map_err(|e| match e {
        VaultError::NotFound { .. } => TaskError::TemplateMissing {
            path: cfg.template.to_string(),
        },
        other => TaskError::Vault(other),
    })?;
    let has_bom = note.bytes().starts_with(&BOM);
    let mut text = render_template(note.text(), &title, now)?;
    if has_bom {
        text.insert(0, '\u{FEFF}');
    }

    // Settings validation keeps `statuses` non-empty; a hand-built config may not be.
    let status = cfg.statuses.first().ok_or_else(|| {
        TaskError::Vault(VaultError::SettingsInvalid {
            reason: "tasks statuses are empty".to_string(),
        })
    })?;
    let bytes = text.into_bytes();
    let mut fields = vec![("Status", FieldValue::Scalar(status.clone()))];
    if let Some(p) = &new.priority {
        fields.push(("Priority", FieldValue::Scalar(p.clone())));
    }
    if let Some(d) = &new.due {
        fields.push(("Due", FieldValue::Scalar(d.clone())));
    }
    let mut edits = Vec::new();
    for (key, value) in fields {
        edits.push((key, value, template_field(&bytes, key)?));
    }
    let bytes = edit::replace_fields(&bytes, &edits)?;
    let content = String::from_utf8(bytes).map_err(|_| VaultError::NotUtf8 {
        path: path.to_string(),
    })?;
    // The folder was checked above. If it vanished since, refuse instead of
    // recreating a configured path.
    vault
        .create_note_in_existing_folder(&path, &content)
        .map_err(folder_missing)?;
    Ok(path)
}

// ---------------------------------------------------------------------------
// Status change
// ---------------------------------------------------------------------------

pub fn set_status(
    vault: &Vault,
    cfg: &TaskSettings,
    path: &VaultPath,
    status: &str,
    status_hash: SpanHash,
    done_hash: Option<SpanHash>,
    today: NaiveDate,
) -> Result<DoneUpdate, TaskError> {
    // The webview names the note; only a listed task may be written.
    if !is_task_path(cfg, path) {
        return Err(TaskError::NotATask {
            path: path.to_string(),
        });
    }
    if !cfg.statuses.iter().any(|s| s == status) {
        return Err(TaskError::UnknownStatus {
            status: status.to_string(),
        });
    }
    let note = vault.read_note(path)?;
    let conflict = || {
        TaskError::Vault(VaultError::Conflict {
            path: path.to_string(),
        })
    };
    let status_field = note.field("Status")?;
    let previous = scalar_value(&status_field.raw_block, "Status")?;
    if previous.as_deref() == Some(status) {
        return Ok(DoneUpdate::NoWrite);
    }
    let fresh_done = match note.field("Done") {
        Ok(f) => Some(f),
        Err(VaultError::FieldNotFound { .. }) => None,
        Err(e) => return Err(e.into()),
    };
    match (&done_hash, &fresh_done) {
        (Some(_), None) | (None, Some(_)) => return Err(conflict()),
        (Some(h), Some(f)) if *h != f.hash => return Err(conflict()),
        _ => {}
    }
    let was_done = previous.as_deref() == Some(cfg.done_status.as_str());
    let is_done = status == cfg.done_status;
    let crosses = was_done != is_done;

    let mut edits = vec![(
        "Status",
        FieldValue::Scalar(status.to_string()),
        status_hash,
    )];
    let result = match (crosses, done_hash) {
        (true, Some(hash)) => {
            let value = if is_done {
                FieldValue::Scalar(today.format("%Y-%m-%d").to_string())
            } else {
                FieldValue::Empty
            };
            edits.push(("Done", value, hash));
            if is_done {
                DoneUpdate::Set
            } else {
                DoneUpdate::Cleared
            }
        }
        (true, None) => DoneUpdate::Absent,
        (false, _) => DoneUpdate::Unchanged,
    };
    vault.replace_fields(path, &edits)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 3, 4)
            .unwrap()
            .and_hms_opt(9, 5, 0)
            .unwrap()
    }

    #[test]
    fn placeholders_are_filled() {
        let t = "---\nStatus: Todo\n---\n# {{title}}\n{{date}} {{time}} {{title}}\n";
        let out = render_template(t, "Zadanie A", now()).unwrap();
        assert_eq!(
            out,
            "---\nStatus: Todo\n---\n# Zadanie A\n2026-03-04 09:05 Zadanie A\n"
        );
    }

    #[test]
    fn unsupported_tokens_are_refused_naming_the_token() {
        match render_template("# {{date:YYYY}}\n", "A", now()) {
            Err(TaskError::TemplateUnsupported { placeholder }) => {
                assert_eq!(placeholder, "{{date:YYYY}}")
            }
            other => panic!("{other:?}"),
        }
        match render_template("# <% tp.file.title %>\n", "A", now()) {
            Err(TaskError::TemplateUnsupported { placeholder }) => {
                assert_eq!(placeholder, "<% tp.file.title %>")
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn title_in_frontmatter_must_be_plain() {
        let t = "---\nName: {{title}}\n---\n# {{title}}\n";
        assert!(render_template(t, "Plain title", now()).is_ok());
        assert!(matches!(
            render_template(t, "A: b", now()),
            Err(TaskError::InvalidTitle { .. })
        ));
        // In the body the title is taken verbatim.
        assert_eq!(
            render_template("---\nA: 1\n---\n# {{title}}\n", "A: b", now()).unwrap(),
            "---\nA: 1\n---\n# A: b\n"
        );
    }

    #[test]
    fn unclosed_braces_stay_literal() {
        assert_eq!(render_template("a {{ b\n", "T", now()).unwrap(), "a {{ b\n");
    }

    #[test]
    fn scalar_values_are_parsed() {
        assert_eq!(
            scalar_value("Due: 2026-01-01\n", "Due").unwrap().as_deref(),
            Some("2026-01-01")
        );
        assert_eq!(
            scalar_value("Due: \"x y\"\r\n", "Due").unwrap().as_deref(),
            Some("x y")
        );
        assert_eq!(
            scalar_value("Due: 'x'\n", "Due").unwrap().as_deref(),
            Some("x")
        );
        assert_eq!(scalar_value("Due:\n", "Due").unwrap(), None);
        assert_eq!(scalar_value("Due:   \n", "Due").unwrap(), None);
        assert!(scalar_value("Status:\n  - A\n", "Status").is_err());
        assert!(scalar_value("Status: [A]\n", "Status").is_err());
    }

    #[test]
    fn due_must_be_a_real_strict_date() {
        assert!(validate_due("2026-02-28").is_ok());
        for bad in [
            "2026-02-30",
            "2026-2-3",
            "26-02-03",
            "2026/02/03",
            "",
            "2026-02-03 ",
        ] {
            assert!(validate_due(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_earliest_unsupported_token_is_named() {
        let t = "# {{foo}}\n<% tp.date %>\n";
        match render_template(t, "A", now()) {
            Err(TaskError::TemplateUnsupported { placeholder }) => {
                assert_eq!(placeholder, "{{foo}}")
            }
            other => panic!("{other:?}"),
        }
        let t = "<% a %>\n{{foo}}\n";
        match render_template(t, "A", now()) {
            Err(TaskError::TemplateUnsupported { placeholder }) => {
                assert_eq!(placeholder, "<% a %>")
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn unsupported_token_label_is_one_capped_line() {
        // A stray `{{` would otherwise pull body text into the error.
        let t = "# {{ stray\nsecret body line\n}} end\n";
        match render_template(t, "A", now()) {
            Err(TaskError::TemplateUnsupported { placeholder }) => {
                assert_eq!(placeholder, "{{ stray…");
            }
            other => panic!("{other:?}"),
        }
        let long = format!("{{{{{}}}}}", "x".repeat(100));
        match render_template(&long, "A", now()) {
            Err(TaskError::TemplateUnsupported { placeholder }) => {
                assert_eq!(placeholder.chars().count(), MAX_TOKEN_LABEL + 1);
                assert!(placeholder.ends_with('…'));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn title_in_frontmatter_refuses_comma_and_apostrophe() {
        let t = "---\naliases: [{{title}}]\n---\n# {{title}}\n";
        for bad in ["a, b", "it's"] {
            assert!(
                matches!(
                    render_template(t, bad, now()),
                    Err(TaskError::InvalidTitle { .. })
                ),
                "{bad}"
            );
        }
        // In the body both are fine.
        assert!(render_template("# {{title}}\n", "a, it's", now()).is_ok());
    }

    #[test]
    fn serialised_shapes_match_the_typescript_types() {
        use serde_json::json;
        let hash = "a".repeat(64);
        let row = TaskRow {
            path: VaultPath::parse("Tasks/A.md").unwrap(),
            title: "A".into(),
            status: Some("Todo".into()),
            priority: None,
            due: None,
            status_hash: Some(SpanHash::from_hex(&hash).unwrap()),
            done_hash: None,
            problem: Some(VaultError::NoFrontmatter),
            field_problems: vec![VaultError::FieldAmbiguous { key: "Due".into() }],
        };
        assert_eq!(
            serde_json::to_value(&row).unwrap(),
            json!({
                "path": "Tasks/A.md",
                "title": "A",
                "status": "Todo",
                "priority": null,
                "due": null,
                "statusHash": hash,
                "doneHash": null,
                "problem": {"kind": "NoFrontmatter"},
                "fieldProblems": [{"kind": "FieldAmbiguous", "detail": {"key": "Due"}}],
            })
        );
        let conflict = TaskError::Vault(VaultError::Conflict {
            path: "Tasks/A.md".into(),
        });
        assert_eq!(
            serde_json::to_value(&conflict).unwrap(),
            json!({"kind": "Vault", "detail": {"kind": "Conflict", "detail": {"path": "Tasks/A.md"}}})
        );
        assert_eq!(
            serde_json::to_value(TaskError::TasksNotConfigured).unwrap(),
            json!({"kind": "TasksNotConfigured"})
        );
        assert_eq!(
            serde_json::to_value(TaskError::NotATask {
                path: "x.md".into()
            })
            .unwrap(),
            json!({"kind": "NotATask", "detail": {"path": "x.md"}})
        );
        for (u, s) in [
            (DoneUpdate::Set, "Set"),
            (DoneUpdate::Cleared, "Cleared"),
            (DoneUpdate::Unchanged, "Unchanged"),
            (DoneUpdate::Absent, "Absent"),
            (DoneUpdate::NoWrite, "NoWrite"),
        ] {
            assert_eq!(serde_json::to_value(u).unwrap(), json!(s));
        }
    }

    #[test]
    fn require_config_refuses_none() {
        assert!(matches!(
            require_config(None),
            Err(TaskError::TasksNotConfigured)
        ));
    }
}
