//! Store-backed month report and day-off toggle.
//!
//! Days off live in the `days_off` table, one row per local date. The report
//! reads the days off and the month's entries in one `store.read`, so every
//! figure comes from one snapshot.

use std::collections::BTreeSet;

use chrono::{NaiveDate, TimeZone};
use rusqlite::Connection;
use serde::Serialize;

use super::error::WorkTimeError;
use super::in_day;
use super::norm::{self, MonthPhase, NormFigures};
use super::rules::{day_bounds, local_day, portion};
use crate::store::Store;

const DAY_FORMAT: &str = "%Y-%m-%d";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DayKind {
    Workday,
    DayOff,
    Weekend,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthReport {
    /// `YYYY-MM`.
    pub month: String,
    pub phase: MonthPhase,
    pub as_of_ms: i64,
    pub worked_ms: i64,
    #[serde(flatten)]
    pub figures: NormFigures,
    /// `YYYY-MM-DD`, sorted, this month only.
    pub days_off: Vec<String>,
}

fn days_off_between(
    conn: &Connection,
    first: NaiveDate,
    next_first: NaiveDate,
) -> Result<BTreeSet<NaiveDate>, WorkTimeError> {
    let mut stmt =
        conn.prepare("SELECT day FROM days_off WHERE day >= ?1 AND day < ?2 ORDER BY day")?;
    let rows = stmt.query_map(
        [
            first.format(DAY_FORMAT).to_string(),
            next_first.format(DAY_FORMAT).to_string(),
        ],
        |r| r.get::<_, String>(0),
    )?;
    let mut out = BTreeSet::new();
    for day in rows {
        // The CHECK constraint only guards the shape; a bad date is not a day off.
        if let Ok(d) = NaiveDate::parse_from_str(&day?, DAY_FORMAT) {
            out.insert(d);
        }
    }
    Ok(out)
}

fn is_day_off(conn: &Connection, day: NaiveDate) -> Result<bool, WorkTimeError> {
    let found = conn
        .prepare("SELECT 1 FROM days_off WHERE day = ?1")?
        .exists([day.format(DAY_FORMAT).to_string()])?;
    Ok(found)
}

pub(super) fn day_kind(conn: &Connection, day: NaiveDate) -> Result<DayKind, WorkTimeError> {
    Ok(if norm::is_weekend(day) {
        DayKind::Weekend
    } else if is_day_off(conn, day)? {
        DayKind::DayOff
    } else {
        DayKind::Workday
    })
}

/// The report of the month containing `day`, on an open connection.
pub(super) fn compute<Tz: TimeZone>(
    conn: &Connection,
    day: NaiveDate,
    now: i64,
    tz: &Tz,
) -> Result<MonthReport, WorkTimeError> {
    let (first, next_first) = norm::month_days(day);
    let (month_start, _) = day_bounds(first, tz)?;
    let (month_end, _) = day_bounds(next_first, tz)?;
    let today = local_day(now, tz)?;
    let phase = norm::phase(first, today);

    let days_off = days_off_between(conn, first, next_first)?;
    let workdays = norm::workdays(first, next_first, &days_off);
    let worked_ms = in_day(conn, month_start, month_end, now)?
        .iter()
        .map(|e| portion(e.start, e.end, now, month_start, month_end))
        .sum();

    Ok(MonthReport {
        month: first.format("%Y-%m").to_string(),
        phase,
        as_of_ms: now,
        worked_ms,
        figures: norm::figures(&workdays, today, phase, worked_ms),
        days_off: days_off
            .iter()
            .map(|d| d.format(DAY_FORMAT).to_string())
            .collect(),
    })
}

/// Report of the month containing `day`, as of `now`.
pub fn month_report<Tz: TimeZone>(
    store: &Store,
    day: NaiveDate,
    now: i64,
    tz: &Tz,
) -> Result<MonthReport, WorkTimeError> {
    store.read(|c| compute(c, day, now, tz))
}

/// Marks or unmarks a day off. Idempotent; a weekend is refused.
pub fn set_day_off(
    store: &mut Store,
    day: NaiveDate,
    off: bool,
    now: i64,
) -> Result<(), WorkTimeError> {
    if norm::is_weekend(day) {
        return Err(WorkTimeError::NotAWorkday);
    }
    let key = day.format(DAY_FORMAT).to_string();
    store.write(|tx| {
        if off {
            tx.execute(
                "INSERT OR IGNORE INTO days_off(day, created_at_ms) VALUES (?1, ?2)",
                (&key, now),
            )?;
        } else {
            tx.execute("DELETE FROM days_off WHERE day = ?1", [&key])?;
        }
        Ok(())
    })
}

/// Whether `day` is a Monday-to-Friday day that is not marked off.
pub fn is_workday(store: &Store, day: NaiveDate) -> Result<bool, WorkTimeError> {
    if norm::is_weekend(day) {
        return Ok(false);
    }
    store.read(|c| Ok(!is_day_off(c, day)?))
}
