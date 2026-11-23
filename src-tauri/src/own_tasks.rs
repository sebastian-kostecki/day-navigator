//! Own tasks: records, validation, ordering and the store operations.
//!
//! No `tauri` import; time is passed in. Errors name a rule and its limit,
//! never a title, note or due value, and nothing here logs a task field.

use std::cmp::Reverse;

use chrono::{NaiveDate, NaiveDateTime};
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};

use crate::space::Space;
use crate::store::{Store, StoreError};

pub const TITLE_MAX: usize = 200;
pub const NOTE_MAX: usize = 2000;

const TS_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";
const DATE_FORMAT: &str = "%Y-%m-%d";
const COLUMNS: &str = "id, space, title, note, due, priority, created_at, updated_at, done_at";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    High,
    Medium,
    Low,
}

impl Priority {
    fn as_str(self) -> &'static str {
        match self {
            Priority::High => "high",
            Priority::Medium => "medium",
            Priority::Low => "low",
        }
    }

    fn parse(s: &str) -> Option<Priority> {
        match s {
            "high" => Some(Priority::High),
            "medium" => Some(Priority::Medium),
            "low" => Some(Priority::Low),
            _ => None,
        }
    }

    fn rank(self) -> u8 {
        match self {
            Priority::High => 0,
            Priority::Medium => 1,
            Priority::Low => 2,
        }
    }
}

/// Add and edit share one shape; edit replaces every field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnTaskInput {
    pub space: Space,
    pub title: String,
    pub note: String,
    pub due: Option<String>,
    pub priority: Option<Priority>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnTask {
    pub id: i64,
    pub space: Space,
    pub title: String,
    pub note: String,
    pub due: Option<String>,
    pub priority: Option<Priority>,
    pub created_at: String,
    pub updated_at: String,
    pub done_at: Option<String>,
}

#[derive(Debug, Clone, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum OwnTaskError {
    #[error("{0}")]
    Store(StoreError),
    #[error("title is empty")]
    TitleEmpty,
    #[error("title is longer than {max} characters")]
    TitleTooLong { max: usize },
    #[error("title has more than one line")]
    TitleMultiline,
    #[error("note is longer than {max} characters")]
    NoteTooLong { max: usize },
    #[error("due date is not a valid YYYY-MM-DD date")]
    InvalidDue,
    #[error("task not found")]
    NotFound,
}

impl From<StoreError> for OwnTaskError {
    fn from(e: StoreError) -> Self {
        OwnTaskError::Store(e)
    }
}

impl From<rusqlite::Error> for OwnTaskError {
    fn from(e: rusqlite::Error) -> Self {
        OwnTaskError::Store(StoreError::from(e))
    }
}

/// Validated, normalized input, ready to bind.
struct Clean {
    space: Space,
    title: String,
    note: String,
    due: Option<String>,
    priority: Option<Priority>,
}

fn validate(input: &OwnTaskInput) -> Result<Clean, OwnTaskError> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err(OwnTaskError::TitleEmpty);
    }
    if title.contains(['\r', '\n']) {
        return Err(OwnTaskError::TitleMultiline);
    }
    if title.chars().count() > TITLE_MAX {
        return Err(OwnTaskError::TitleTooLong { max: TITLE_MAX });
    }

    let note = if input.note.trim().is_empty() {
        String::new()
    } else {
        input.note.replace("\r\n", "\n")
    };
    if note.chars().count() > NOTE_MAX {
        return Err(OwnTaskError::NoteTooLong { max: NOTE_MAX });
    }

    if let Some(due) = &input.due {
        let ok = NaiveDate::parse_from_str(due, DATE_FORMAT)
            .is_ok_and(|d| d.format(DATE_FORMAT).to_string() == *due);
        if !ok {
            return Err(OwnTaskError::InvalidDue);
        }
    }

    Ok(Clean {
        space: input.space,
        title: title.to_string(),
        note,
        due: input.due.clone(),
        priority: input.priority,
    })
}

/// Work while the work-time counter runs, otherwise Private.
pub fn default_space(counter_running: bool) -> Space {
    if counter_running {
        Space::Work
    } else {
        Space::Private
    }
}

/// Open tasks first (due ascending, none last; priority high to low, none
/// last; creation time; id), then done tasks, newest done first.
pub fn sort_for_display(tasks: &mut [OwnTask]) {
    tasks.sort_by(|a, b| match (&a.done_at, &b.done_at) {
        (Some(x), Some(y)) => (Reverse(x), a.id).cmp(&(Reverse(y), b.id)),
        _ => display_key(a).cmp(&display_key(b)),
    });
}

/// Borrowed sort key, so comparing never allocates.
fn display_key(t: &OwnTask) -> (bool, bool, Option<&str>, u8, &str, i64) {
    (
        t.done_at.is_some(),
        t.due.is_none(),
        t.due.as_deref(),
        t.priority.map_or(u8::MAX, Priority::rank),
        t.created_at.as_str(),
        t.id,
    )
}

fn stamp(now: NaiveDateTime) -> String {
    now.format(TS_FORMAT).to_string()
}

fn corrupt() -> rusqlite::Error {
    // A stored `space` or `priority` outside the known set (the CHECKs should
    // make this impossible). `InvalidQuery` is only a carrier: it surfaces as a
    // value-free `StoreError::Sql`, so the stored value is never formatted.
    rusqlite::Error::InvalidQuery
}

fn from_row(r: &Row<'_>) -> rusqlite::Result<OwnTask> {
    let space: String = r.get(1)?;
    let space = match space.as_str() {
        "work" => Space::Work,
        "private" => Space::Private,
        _ => return Err(corrupt()),
    };
    let priority: Option<String> = r.get(5)?;
    let priority = match priority {
        None => None,
        Some(p) => Some(Priority::parse(&p).ok_or_else(corrupt)?),
    };
    Ok(OwnTask {
        id: r.get(0)?,
        space,
        title: r.get(2)?,
        note: r.get(3)?,
        due: r.get(4)?,
        priority,
        created_at: r.get(6)?,
        updated_at: r.get(7)?,
        done_at: r.get(8)?,
    })
}

fn fetch(conn: &Connection, id: i64) -> Result<OwnTask, OwnTaskError> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM own_tasks WHERE id = ?1"),
        [id],
        from_row,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => OwnTaskError::NotFound,
        other => other.into(),
    })
}

/// All tasks in display order.
pub fn list(store: &Store) -> Result<Vec<OwnTask>, OwnTaskError> {
    store.read(|conn| {
        let mut stmt = conn.prepare(&format!("SELECT {COLUMNS} FROM own_tasks"))?;
        let mut tasks = stmt
            .query_map([], from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        sort_for_display(&mut tasks);
        Ok(tasks)
    })
}

pub fn add(
    store: &mut Store,
    input: &OwnTaskInput,
    now: NaiveDateTime,
) -> Result<OwnTask, OwnTaskError> {
    store.write(|tx| {
        let c = validate(input)?;
        let ts = stamp(now);
        tx.execute(
            "INSERT INTO own_tasks (space, title, note, due, priority, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            (
                c.space.to_string(),
                &c.title,
                &c.note,
                &c.due,
                c.priority.map(Priority::as_str),
                &ts,
            ),
        )?;
        fetch(tx, tx.last_insert_rowid())
    })
}

pub fn edit(
    store: &mut Store,
    id: i64,
    input: &OwnTaskInput,
    now: NaiveDateTime,
) -> Result<OwnTask, OwnTaskError> {
    store.write(|tx| {
        let c = validate(input)?;
        let changed = tx.execute(
            "UPDATE own_tasks SET space = ?1, title = ?2, note = ?3, due = ?4, \
             priority = ?5, updated_at = ?6 WHERE id = ?7",
            (
                c.space.to_string(),
                &c.title,
                &c.note,
                &c.due,
                c.priority.map(Priority::as_str),
                stamp(now),
                id,
            ),
        )?;
        if changed == 0 {
            return Err(OwnTaskError::NotFound);
        }
        fetch(tx, id)
    })
}

/// `done = true` keeps an existing `done_at` (idempotent); `false` clears it.
pub fn set_done(
    store: &mut Store,
    id: i64,
    done: bool,
    now: NaiveDateTime,
) -> Result<OwnTask, OwnTaskError> {
    store.write(|tx| {
        let ts = stamp(now);
        let sql = if done {
            "UPDATE own_tasks SET done_at = coalesce(done_at, ?1), updated_at = ?1 WHERE id = ?2"
        } else {
            "UPDATE own_tasks SET done_at = NULL, updated_at = ?1 WHERE id = ?2"
        };
        let changed = tx.execute(sql, (&ts, id))?;
        if changed == 0 {
            return Err(OwnTaskError::NotFound);
        }
        fetch(tx, id)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(title: &str, note: &str, due: Option<&str>) -> OwnTaskInput {
        OwnTaskInput {
            space: Space::Private,
            title: title.into(),
            note: note.into(),
            due: due.map(String::from),
            priority: None,
        }
    }

    fn task(id: i64, due: Option<&str>, prio: Option<Priority>, created: &str) -> OwnTask {
        OwnTask {
            id,
            space: Space::Work,
            title: format!("t{id}"),
            note: String::new(),
            due: due.map(String::from),
            priority: prio,
            created_at: created.into(),
            updated_at: created.into(),
            done_at: None,
        }
    }

    fn ids(tasks: &[OwnTask]) -> Vec<i64> {
        tasks.iter().map(|t| t.id).collect()
    }

    #[test]
    fn title_length_boundaries_count_chars() {
        assert!(validate(&input(&"a".repeat(200), "", None)).is_ok());
        assert!(matches!(
            validate(&input(&"a".repeat(201), "", None)),
            Err(OwnTaskError::TitleTooLong { max: 200 })
        ));
        // 200 two-byte chars are 400 bytes but still within the limit.
        assert!(validate(&input(&"ż".repeat(200), "", None)).is_ok());
        assert!(validate(&input(&"ż".repeat(201), "", None)).is_err());
    }

    #[test]
    fn title_is_trimmed_and_must_be_single_line_and_non_empty() {
        let c = validate(&input("  Zadanie testowe  ", "", None)).unwrap();
        assert_eq!(c.title, "Zadanie testowe");
        assert!(matches!(
            validate(&input("   \t ", "", None)),
            Err(OwnTaskError::TitleEmpty)
        ));
        assert!(matches!(
            validate(&input("a\nb", "", None)),
            Err(OwnTaskError::TitleMultiline)
        ));
        assert!(matches!(
            validate(&input("a\rb", "", None)),
            Err(OwnTaskError::TitleMultiline)
        ));
    }

    #[test]
    fn note_boundaries_and_normalization() {
        assert!(validate(&input("t", &"n".repeat(2000), None)).is_ok());
        assert!(matches!(
            validate(&input("t", &"n".repeat(2001), None)),
            Err(OwnTaskError::NoteTooLong { max: 2000 })
        ));
        assert!(validate(&input("t", &"ą".repeat(2000), None)).is_ok());
        assert_eq!(
            validate(&input("t", "a\r\nb\nc", None)).unwrap().note,
            "a\nb\nc"
        );
        assert_eq!(validate(&input("t", " \r\n\t ", None)).unwrap().note, "");
        // Surrounding whitespace of a real note is kept.
        assert_eq!(validate(&input("t", " x ", None)).unwrap().note, " x ");
        // The limit applies after CRLF becomes LF.
        let note = "a\r\n".repeat(1000);
        assert!(validate(&input("t", &note, None)).is_ok());
    }

    #[test]
    fn due_must_be_canonical() {
        assert_eq!(
            validate(&input("t", "", Some("2026-02-01"))).unwrap().due,
            Some("2026-02-01".into())
        );
        assert_eq!(validate(&input("t", "", None)).unwrap().due, None);
        for bad in [
            "2026-2-1",
            "2026-02-30",
            "",
            "2026-02-01 ",
            "01-02-2026",
            "x",
        ] {
            assert!(
                matches!(
                    validate(&input("t", "", Some(bad))),
                    Err(OwnTaskError::InvalidDue)
                ),
                "should reject {bad:?}"
            );
        }
    }

    #[test]
    fn default_space_follows_counter() {
        assert_eq!(default_space(true), Space::Work);
        assert_eq!(default_space(false), Space::Private);
    }

    #[test]
    fn sort_none_due_last_then_priority_then_creation() {
        use Priority::*;
        let mut v = vec![
            task(1, None, Some(High), "2026-01-01T08:00:00"),
            task(2, Some("2026-03-02"), None, "2026-01-01T08:00:00"),
            task(3, Some("2026-03-01"), Some(Low), "2026-01-02T08:00:00"),
            task(4, Some("2026-03-01"), Some(High), "2026-01-03T08:00:00"),
            task(5, Some("2026-03-01"), None, "2026-01-01T08:00:00"),
            task(6, Some("2026-03-01"), Some(High), "2026-01-02T08:00:00"),
            task(7, Some("2026-03-01"), Some(Medium), "2026-01-01T08:00:00"),
        ];
        sort_for_display(&mut v);
        // Same due: High (older first), Medium, Low, none; then later due; none-due last.
        assert_eq!(ids(&v), vec![6, 4, 7, 3, 5, 2, 1]);
    }

    #[test]
    fn sort_equal_keys_fall_back_to_id() {
        let mut v = vec![
            task(2, None, None, "2026-01-01T08:00:00"),
            task(1, None, None, "2026-01-01T08:00:00"),
        ];
        sort_for_display(&mut v);
        assert_eq!(ids(&v), vec![1, 2]);
    }

    #[test]
    fn sort_done_after_open_newest_done_first() {
        let mut v = vec![
            task(1, None, None, "2026-01-01T08:00:00"),
            task(2, Some("2026-01-01"), None, "2026-01-01T08:00:00"),
            task(3, None, None, "2026-01-01T08:00:00"),
            task(4, None, None, "2026-01-01T08:00:00"),
        ];
        v[0].done_at = Some("2026-02-01T10:00:00".into());
        v[2].done_at = Some("2026-02-02T10:00:00".into());
        v[3].done_at = Some("2026-02-02T10:00:00".into());
        sort_for_display(&mut v);
        assert_eq!(ids(&v), vec![2, 3, 4, 1]);
    }

    #[test]
    fn error_json_shape() {
        let json = serde_json::to_string(&OwnTaskError::TitleTooLong { max: 200 }).unwrap();
        assert_eq!(json, r#"{"kind":"TitleTooLong","detail":{"max":200}}"#);
        let json = serde_json::to_string(&OwnTaskError::NotFound).unwrap();
        assert_eq!(json, r#"{"kind":"NotFound"}"#);
        let json = serde_json::to_string(&OwnTaskError::Store(StoreError::Busy)).unwrap();
        assert_eq!(json, r#"{"kind":"Store","detail":{"kind":"Busy"}}"#);
    }

    #[test]
    fn errors_never_contain_submitted_values() {
        let title = "Tajny-tytul-".to_string() + &"x".repeat(250);
        let note = "Tajna-notatka-".to_string() + &"y".repeat(2100);
        let cases = [
            validate(&input(&title, "", None)).err(),
            validate(&input("ok", &note, None)).err(),
            validate(&input("Tajny\ntytul", "", None)).err(),
            validate(&input("ok", "", Some("Tajna-data"))).err(),
        ];
        for e in cases {
            let e = e.expect("must fail");
            let shown = format!("{e} {e:?} {}", serde_json::to_string(&e).unwrap());
            for secret in ["Tajny", "Tajna", "tytul", "xxxx", "yyyy"] {
                assert!(!shown.contains(secret), "leaked {secret} in {shown}");
            }
        }
    }

    #[test]
    fn input_and_task_serialize_camel_case() {
        let t = task(
            1,
            Some("2026-03-01"),
            Some(Priority::High),
            "2026-01-01T08:00:00",
        );
        let v = serde_json::to_value(&t).unwrap();
        assert!(v.get("createdAt").is_some() && v.get("doneAt").is_some());
        assert_eq!(v["space"], "work");
        assert_eq!(v["priority"], "high");
    }
}
