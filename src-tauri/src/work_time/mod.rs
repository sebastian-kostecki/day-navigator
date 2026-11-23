//! Work-time domain: entries stored as absolute instants (UTC epoch
//! milliseconds), with the rules for start, stop, add, edit, remove and day
//! totals split at local midnight.
//!
//! No `tauri` import and no clock: `now` and the time zone are parameters
//! (production passes `chrono::Local`, tests pass a fixed zone). Every
//! mutation is one `Store::write` closure that reads, validates and writes, so
//! a refusal changes nothing. Removed rows (`removed_at_ms` set) are kept but
//! ignored by every rule and total.

pub mod commands;
pub mod error;
pub mod rules;

#[cfg(test)]
mod tests;

use chrono::{NaiveDate, TimeZone};
use rusqlite::Connection;
use serde::Serialize;

use crate::store::Store;
pub use error::WorkTimeError;
use rules::{day_bounds, local_day, local_string, overlaps, portion, resolve_local};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningEntry {
    pub id: i64,
    pub start_ms: i64,
}

/// Header state. Live total = `today_closed_ms + (now - running_from_ms)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkTimerStatus {
    pub running: Option<RunningEntry>,
    pub today_closed_ms: i64,
    /// `max(start, day_start)` of the running entry.
    pub running_from_ms: Option<i64>,
    pub day_start_ms: i64,
    pub day_end_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryRow {
    pub id: i64,
    pub start_ms: i64,
    pub end_ms: Option<i64>,
    pub start_local: String,
    pub end_local: Option<String>,
    pub day_portion_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayEntries {
    /// `YYYY-MM-DD`.
    pub day: String,
    pub entries: Vec<EntryRow>,
    /// Sum of the portions of closed entries.
    pub closed_ms: i64,
    /// Set only while the running entry counts in this day and the day has
    /// not ended yet.
    pub running_from_ms: Option<i64>,
    pub day_end_ms: i64,
}

struct Entry {
    id: i64,
    start: i64,
    end: Option<i64>,
}

fn row_to_entry(r: &rusqlite::Row<'_>) -> rusqlite::Result<Entry> {
    Ok(Entry {
        id: r.get(0)?,
        start: r.get(1)?,
        end: r.get(2)?,
    })
}

/// Counted (not removed) entries, ordered by start, optionally without one id.
fn counted(conn: &Connection, exclude: Option<i64>) -> Result<Vec<Entry>, WorkTimeError> {
    let mut stmt = conn.prepare(
        "SELECT id, start_ms, end_ms FROM work_entries \
         WHERE removed_at_ms IS NULL AND id IS NOT ?1 ORDER BY start_ms, id",
    )?;
    let rows = stmt.query_map([exclude], row_to_entry)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn running_entry(conn: &Connection) -> Result<Option<Entry>, WorkTimeError> {
    let mut stmt = conn.prepare(
        "SELECT id, start_ms, end_ms FROM work_entries \
         WHERE removed_at_ms IS NULL AND end_ms IS NULL",
    )?;
    let mut rows = stmt.query_map([], row_to_entry)?;
    Ok(rows.next().transpose()?)
}

/// Counted entries that overlap `[day_start, day_end)`, sorted by start.
fn in_day(
    conn: &Connection,
    day_start: i64,
    day_end: i64,
    now: i64,
) -> Result<Vec<Entry>, WorkTimeError> {
    let mut stmt = conn.prepare(
        "SELECT id, start_ms, end_ms FROM work_entries \
         WHERE removed_at_ms IS NULL AND start_ms < ?2 AND (end_ms IS NULL OR end_ms > ?1) \
         ORDER BY start_ms, id",
    )?;
    let rows = stmt.query_map([day_start, day_end], row_to_entry)?;
    let mut out = Vec::new();
    for e in rows {
        let e = e?;
        // A running entry does not count in a day that has not begun yet.
        if e.end.is_none() && now <= day_start {
            continue;
        }
        out.push(e);
    }
    Ok(out)
}

/// First counted entry that overlaps `[start, end)` (`None` = unbounded).
fn find_overlap(
    conn: &Connection,
    exclude: Option<i64>,
    start: i64,
    end: Option<i64>,
) -> Result<(), WorkTimeError> {
    match counted(conn, exclude)?
        .into_iter()
        .find(|o| overlaps(start, end, o.start, o.end))
    {
        Some(o) => Err(WorkTimeError::Overlap { entry_id: o.id }),
        None => Ok(()),
    }
}

/// Today's state for the header.
pub fn status<Tz: TimeZone>(
    store: &Store,
    now: i64,
    tz: &Tz,
) -> Result<WorkTimerStatus, WorkTimeError> {
    let (day_start, day_end) = day_bounds(local_day(now, tz)?, tz)?;
    store.read(|c| {
        let mut closed = 0;
        let mut running = None;
        for e in in_day(c, day_start, day_end, now)? {
            match e.end {
                Some(_) => closed += portion(e.start, e.end, now, day_start, day_end),
                None => running = Some(e),
            }
        }
        // The running entry may have started before today.
        let running = match running {
            Some(e) => Some(e),
            None => running_entry(c)?,
        };
        Ok(WorkTimerStatus {
            running_from_ms: running.as_ref().map(|e| e.start.max(day_start)),
            running: running.map(|e| RunningEntry {
                id: e.id,
                start_ms: e.start,
            }),
            today_closed_ms: closed,
            day_start_ms: day_start,
            day_end_ms: day_end,
        })
    })
}

/// A chosen day's entries (those overlapping the day, running one included).
pub fn entries_for_day<Tz: TimeZone>(
    store: &Store,
    day: NaiveDate,
    now: i64,
    tz: &Tz,
) -> Result<DayEntries, WorkTimeError> {
    let (day_start, day_end) = day_bounds(day, tz)?;
    store.read(|c| {
        let mut entries = Vec::new();
        let mut closed_ms = 0;
        let mut running_from_ms = None;
        for e in in_day(c, day_start, day_end, now)? {
            let p = portion(e.start, e.end, now, day_start, day_end);
            if e.end.is_some() {
                closed_ms += p;
            } else if now < day_end {
                running_from_ms = Some(e.start.max(day_start));
            }
            entries.push(EntryRow {
                id: e.id,
                start_ms: e.start,
                end_ms: e.end,
                start_local: local_string(e.start, tz)?,
                end_local: e.end.map(|v| local_string(v, tz)).transpose()?,
                day_portion_ms: p,
            });
        }
        Ok(DayEntries {
            day: day.format("%Y-%m-%d").to_string(),
            entries,
            closed_ms,
            running_from_ms,
            day_end_ms: day_end,
        })
    })
}

/// Whether the counter runs (for S-06).
pub fn is_running(store: &Store) -> Result<bool, WorkTimeError> {
    store.read(|c| Ok(running_entry(c)?.is_some()))
}

/// Starts a running entry at `now`; returns its id.
pub fn start(store: &mut Store, now: i64) -> Result<i64, WorkTimeError> {
    store.write(|tx| {
        if running_entry(tx)?.is_some() {
            return Err(WorkTimeError::AlreadyRunning);
        }
        if let Some(e) = counted(tx, None)?
            .into_iter()
            .find(|e| e.end.is_some_and(|end| end > now))
        {
            return Err(WorkTimeError::Overlap { entry_id: e.id });
        }
        tx.execute(
            "INSERT INTO work_entries(start_ms, end_ms, created_at_ms, updated_at_ms) \
             VALUES (?1, NULL, ?2, ?2)",
            (now, now),
        )?;
        Ok(tx.last_insert_rowid())
    })
}

/// Closes the running entry at `now`.
pub fn stop(store: &mut Store, now: i64) -> Result<(), WorkTimeError> {
    store.write(|tx| {
        let running = running_entry(tx)?.ok_or(WorkTimeError::NotRunning)?;
        if now <= running.start {
            return Err(WorkTimeError::EndNotAfterStart);
        }
        tx.execute(
            "UPDATE work_entries SET end_ms = ?1, updated_at_ms = ?2 WHERE id = ?3",
            (now, now, running.id),
        )?;
        Ok(())
    })
}

/// Adds a closed entry from local `YYYY-MM-DDTHH:MM` strings; returns its id.
pub fn add<Tz: TimeZone>(
    store: &mut Store,
    start_local: &str,
    end_local: &str,
    now: i64,
    tz: &Tz,
) -> Result<i64, WorkTimeError> {
    let start = resolve_local(start_local, tz)?;
    let end = resolve_local(end_local, tz)?;
    if end <= start {
        return Err(WorkTimeError::EndNotAfterStart);
    }
    if start > now || end > now {
        return Err(WorkTimeError::InFuture);
    }
    store.write(|tx| {
        find_overlap(tx, None, start, Some(end))?;
        tx.execute(
            "INSERT INTO work_entries(start_ms, end_ms, created_at_ms, updated_at_ms) \
             VALUES (?1, ?2, ?3, ?3)",
            (start, end, now),
        )?;
        Ok(tx.last_insert_rowid())
    })
}

/// A submitted local time keeps the stored instant when it equals the stored
/// instant floored to the minute; otherwise it is resolved.
fn resolve_or_keep<Tz: TimeZone>(
    submitted: &str,
    stored: i64,
    tz: &Tz,
) -> Result<i64, WorkTimeError> {
    if local_string(stored, tz)? == submitted {
        Ok(stored)
    } else {
        resolve_local(submitted, tz)
    }
}

/// Edits an entry. `end_local` is required for a closed entry and must be
/// `None` for the running one (`RunningEndEdit`).
pub fn update<Tz: TimeZone>(
    store: &mut Store,
    id: i64,
    start_local: &str,
    end_local: Option<&str>,
    now: i64,
    tz: &Tz,
) -> Result<(), WorkTimeError> {
    store.write(|tx| {
        let current = counted(tx, None)?
            .into_iter()
            .find(|e| e.id == id)
            .ok_or(WorkTimeError::NotFound)?;
        let start = resolve_or_keep(start_local, current.start, tz)?;
        let end = match (current.end, end_local) {
            (None, Some(_)) => return Err(WorkTimeError::RunningEndEdit),
            (None, None) => None,
            // A closed entry needs an end; a missing one is not a valid time.
            (Some(_), None) => return Err(WorkTimeError::InvalidLocalTime),
            (Some(stored), Some(local)) => Some(resolve_or_keep(local, stored, tz)?),
        };
        if end.is_some_and(|e| e <= start) {
            return Err(WorkTimeError::EndNotAfterStart);
        }
        if start > now || end.is_some_and(|e| e > now) {
            return Err(WorkTimeError::InFuture);
        }
        find_overlap(tx, Some(id), start, end)?;
        tx.execute(
            "UPDATE work_entries SET start_ms = ?1, end_ms = ?2, updated_at_ms = ?3 WHERE id = ?4",
            (start, end, now, id),
        )?;
        Ok(())
    })
}

/// Soft removal: the row stays, but stops counting.
pub fn remove(store: &mut Store, id: i64, now: i64) -> Result<(), WorkTimeError> {
    store.write(|tx| {
        let changed = tx.execute(
            "UPDATE work_entries SET removed_at_ms = ?1, updated_at_ms = ?1 \
             WHERE id = ?2 AND removed_at_ms IS NULL",
            (now, id),
        )?;
        if changed == 0 {
            return Err(WorkTimeError::NotFound);
        }
        Ok(())
    })
}
