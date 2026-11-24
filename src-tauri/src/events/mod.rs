//! Today's event log (S-08): what happened to the user's list today.
//!
//! No `tauri` import and no clock: `today` and `now` are parameters. Only the
//! current day's rows are shown; every write first deletes the rows of past
//! days (a later day's rows stay, so a stale writer cannot touch them, and
//! the read shows only today's). A row holds
//! what the UI shows: the item's reference and title, the source key and
//! label, the error kind. No URL, hostname or message text.

pub mod commands;
pub mod error;

#[cfg(test)]
mod tests;

use chrono::{NaiveDate, NaiveDateTime};
use rusqlite::{Transaction, params};
use serde::Serialize;

use crate::gitlab::ItemKind;
use crate::health::{SourceId, Transition, short_reason};
use crate::store::{Store, StoreError};
pub use error::EventError;

/// Emitted (no payload) after events were stored.
pub const CHANGED_EVENT: &str = "events-changed";

const TS_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";
const DATE_FORMAT: &str = "%Y-%m-%d";
/// Titles are cut to this many characters (a character, not a byte).
const TITLE_MAX_CHARS: usize = 300;
/// The list shows at most this many events, newest first.
const LIST_LIMIT: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemSource {
    Own,
    Gitlab,
    Github,
}

impl ItemSource {
    fn text(self) -> &'static str {
        match self {
            ItemSource::Own => "own",
            ItemSource::Gitlab => "gitlab",
            ItemSource::Github => "github",
        }
    }

    fn from_text(text: &str) -> Option<Self> {
        match text {
            "own" => Some(ItemSource::Own),
            "gitlab" => Some(ItemSource::Gitlab),
            "github" => Some(ItemSource::Github),
            _ => None,
        }
    }
}

/// The item an event is about. `reference` is `None` for an own task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Subject {
    pub source: ItemSource,
    pub reference: Option<String>,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NewEvent {
    Assigned {
        item_kind: ItemKind,
        reference: String,
        title: String,
    },
    Ticked(Subject),
    Removed(Subject),
    ClosedInSource(Subject),
    WorkStarted,
    WorkStopped,
    SourceError {
        source: SourceId,
        label: String,
        error_kind: String,
    },
    SourceRecovered {
        source: SourceId,
        label: String,
    },
}

/// One stored event as the UI gets it: `id`, `at` and the event's own fields,
/// tagged on `kind`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventItem {
    pub id: i64,
    /// Local `YYYY-MM-DDTHH:MM:SS`.
    pub at: String,
    #[serde(flatten)]
    pub event: EventView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum EventView {
    Assigned {
        item_kind: ItemKind,
        reference: String,
        title: String,
    },
    Ticked(Subject),
    Removed(Subject),
    ClosedInSource(Subject),
    WorkStarted,
    WorkStopped,
    SourceError {
        source_id: SourceId,
        label: String,
        /// A fixed Polish phrase for the error kind.
        reason: String,
    },
    SourceRecovered {
        source_id: SourceId,
        label: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventLog {
    /// Newest first, at most 200.
    pub events: Vec<EventItem>,
    /// More events exist today than `events` holds.
    pub truncated: bool,
}

fn stamp(now: NaiveDateTime) -> String {
    now.format(TS_FORMAT).to_string()
}

fn day_text(day: NaiveDate) -> String {
    day.format(DATE_FORMAT).to_string()
}

fn kind_text(kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Issue => "issue",
        ItemKind::MergeRequest => "mergeRequest",
    }
}

fn kind_from_text(text: &str) -> Option<ItemKind> {
    match text {
        "issue" => Some(ItemKind::Issue),
        "mergeRequest" => Some(ItemKind::MergeRequest),
        _ => None,
    }
}

fn source_from_key(key: &str) -> Option<SourceId> {
    [
        SourceId::Settings,
        SourceId::AppData,
        SourceId::Gitlab,
        SourceId::Github,
        SourceId::Reminders,
    ]
    .into_iter()
    .find(|s| s.key() == key)
}

fn cut(title: &str) -> String {
    title.chars().take(TITLE_MAX_CHARS).collect()
}

/// The columns of one row after `id` and `at`.
#[derive(Default)]
struct Columns {
    kind: &'static str,
    item_source: Option<&'static str>,
    item_kind: Option<&'static str>,
    reference: Option<String>,
    title: Option<String>,
    source_id: Option<String>,
    source_label: Option<String>,
    error_kind: Option<String>,
}

fn subject_columns(kind: &'static str, s: &Subject) -> Columns {
    Columns {
        kind,
        item_source: Some(s.source.text()),
        reference: s.reference.clone(),
        title: Some(cut(&s.title)),
        ..Columns::default()
    }
}

fn columns_of(event: &NewEvent) -> Columns {
    match event {
        NewEvent::Assigned {
            item_kind,
            reference,
            title,
        } => Columns {
            kind: "assigned",
            item_source: Some("gitlab"),
            item_kind: Some(kind_text(*item_kind)),
            reference: Some(reference.clone()),
            title: Some(cut(title)),
            ..Columns::default()
        },
        NewEvent::Ticked(s) => subject_columns("ticked", s),
        NewEvent::Removed(s) => subject_columns("removed", s),
        NewEvent::ClosedInSource(s) => subject_columns("closedInSource", s),
        NewEvent::WorkStarted => Columns {
            kind: "workStarted",
            ..Columns::default()
        },
        NewEvent::WorkStopped => Columns {
            kind: "workStopped",
            ..Columns::default()
        },
        NewEvent::SourceError {
            source,
            label,
            error_kind,
        } => Columns {
            kind: "sourceError",
            source_id: Some(source.key()),
            source_label: Some(label.clone()),
            error_kind: Some(error_kind.clone()),
            ..Columns::default()
        },
        NewEvent::SourceRecovered { source, label } => Columns {
            kind: "sourceRecovered",
            source_id: Some(source.key()),
            source_label: Some(label.clone()),
            ..Columns::default()
        },
    }
}

/// Deletes the rows of past days, then inserts `event` at `now`. The
/// caller passes the transaction, so one write can hold several events.
pub fn record_in(
    tx: &Transaction<'_>,
    today: NaiveDate,
    now: NaiveDateTime,
    event: &NewEvent,
) -> Result<(), StoreError> {
    tx.execute(
        "DELETE FROM events WHERE substr(at, 1, 10) < ?1",
        [day_text(today)],
    )?;
    let c = columns_of(event);
    tx.execute(
        "INSERT INTO events
           (at, kind, item_source, item_kind, reference, title,
            source_id, source_label, error_kind)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            stamp(now),
            c.kind,
            c.item_source,
            c.item_kind,
            c.reference,
            c.title,
            c.source_id,
            c.source_label,
            c.error_kind
        ],
    )?;
    Ok(())
}

/// Stores `events` in one write; an empty list opens no transaction.
pub fn record(
    store: &mut Store,
    today: NaiveDate,
    now: NaiveDateTime,
    events: &[NewEvent],
) -> Result<usize, EventError> {
    if events.is_empty() {
        return Ok(0);
    }
    store.write::<_, EventError>(|tx| {
        for event in events {
            record_in(tx, today, now, event)?;
        }
        Ok(events.len())
    })
}

/// Health transitions as events: entering or changing an error is a
/// `SourceError`, recovering a `SourceRecovered`.
pub fn from_transitions(ts: &[Transition]) -> Vec<NewEvent> {
    ts.iter()
        .map(|t| match &t.to_kind {
            Some(kind) => NewEvent::SourceError {
                source: t.id,
                label: t.label.clone(),
                error_kind: kind.clone(),
            },
            None => NewEvent::SourceRecovered {
                source: t.id,
                label: t.label.clone(),
            },
        })
        .collect()
}

fn damaged() -> StoreError {
    StoreError::Damaged {
        reason: "event record is unreadable".into(),
    }
}

/// The columns of one stored row after `id`, `at` and `kind`.
struct Raw {
    item_source: Option<String>,
    item_kind: Option<String>,
    reference: Option<String>,
    title: Option<String>,
    source_id: Option<String>,
    source_label: Option<String>,
    error_kind: Option<String>,
}

fn subject_of(c: Raw) -> Result<Subject, StoreError> {
    let source = c
        .item_source
        .as_deref()
        .and_then(ItemSource::from_text)
        .ok_or_else(damaged)?;
    Ok(Subject {
        source,
        reference: c.reference,
        title: c.title.ok_or_else(damaged)?,
    })
}

fn source_of(c: &Raw) -> Result<SourceId, StoreError> {
    c.source_id
        .as_deref()
        .and_then(source_from_key)
        .ok_or_else(damaged)
}

fn view_of(kind: &str, c: Raw) -> Result<EventView, StoreError> {
    Ok(match kind {
        "assigned" => EventView::Assigned {
            item_kind: c
                .item_kind
                .as_deref()
                .and_then(kind_from_text)
                .ok_or_else(damaged)?,
            reference: c.reference.ok_or_else(damaged)?,
            title: c.title.ok_or_else(damaged)?,
        },
        "ticked" => EventView::Ticked(subject_of(c)?),
        "removed" => EventView::Removed(subject_of(c)?),
        "closedInSource" => EventView::ClosedInSource(subject_of(c)?),
        "workStarted" => EventView::WorkStarted,
        "workStopped" => EventView::WorkStopped,
        "sourceError" => EventView::SourceError {
            source_id: source_of(&c)?,
            label: c.source_label.ok_or_else(damaged)?,
            reason: short_reason(&c.error_kind.ok_or_else(damaged)?).to_string(),
        },
        "sourceRecovered" => EventView::SourceRecovered {
            source_id: source_of(&c)?,
            label: c.source_label.ok_or_else(damaged)?,
        },
        _ => return Err(damaged()),
    })
}

/// Today's events, newest first, at most 200 (`truncated` when more exist).
/// Rows of other days are never returned, even before the next write deletes
/// them.
pub fn list_today(store: &Store, today: NaiveDate) -> Result<EventLog, EventError> {
    let raw: Vec<(i64, String, String, Raw)> = store.read::<_, EventError>(|c| {
        let mut stmt = c.prepare(
            "SELECT id, at, kind, item_source, item_kind, reference, title,
                    source_id, source_label, error_kind
             FROM events WHERE substr(at, 1, 10) = ?1
             ORDER BY id DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![day_text(today), LIST_LIMIT as i64 + 1], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                Raw {
                    item_source: r.get(3)?,
                    item_kind: r.get(4)?,
                    reference: r.get(5)?,
                    title: r.get(6)?,
                    source_id: r.get(7)?,
                    source_label: r.get(8)?,
                    error_kind: r.get(9)?,
                },
            ))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    })?;
    let truncated = raw.len() > LIST_LIMIT;
    let events = raw
        .into_iter()
        .take(LIST_LIMIT)
        .map(|(id, at, kind, c)| {
            Ok(EventItem {
                id,
                at,
                event: view_of(&kind, c)?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(EventLog { events, truncated })
}
