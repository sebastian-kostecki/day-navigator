//! Store-backed month report and day-off toggle.
//!
//! Days off live in the `days_off` table, one row per local date. The report
//! reads the days off and the month's entries in one `store.read`, so every
//! figure comes from one snapshot.

use std::collections::BTreeSet;

use chrono::{Datelike, NaiveDate, TimeZone, Weekday};
use rusqlite::Connection;
use serde::Serialize;

use super::error::WorkTimeError;
use super::holidays::HolidayCalendar;
use super::norm::{self, MonthPhase, NormFigures};
use super::rules::{day_bounds, local_day, portion};
use super::{Entry, in_day};
use crate::settings::WorkTimeSettings;
use crate::store::Store;

const DAY_FORMAT: &str = "%Y-%m-%d";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DayKind {
    Workday,
    DayOff,
    Holiday,
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
    /// `YYYY-MM-DD`, sorted, this month only. Rows on a public holiday under
    /// the active calendar are left out: the holiday shadows them.
    pub days_off: Vec<String>,
    /// Every local day of the month, ascending.
    pub days: Vec<DayReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayReport {
    /// `YYYY-MM-DD`.
    pub day: String,
    pub kind: DayKind,
    /// Sum of the portions of entries in the day, a running one up to `now`.
    pub worked_ms: i64,
    pub norm_ms: i64,
    pub deviation_ms: Option<i64>,
    /// Entries with a non-zero portion in the day.
    pub entry_count: u32,
    /// Name of the public holiday under the active calendar, on any kind of day.
    pub holiday: Option<String>,
    /// True exactly when the holiday falls on a Saturday: a day off is owed.
    pub day_off_owed: bool,
}

pub(super) fn days_off_between(
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

/// The one classification rule: a weekend is `Weekend` (a Saturday or Sunday
/// holiday too), a weekday holiday is `Holiday` even when marked off, a marked
/// day is `DayOff`, anything else is `Workday`.
fn classify(day: NaiveDate, marked_off: bool, holidays: HolidayCalendar) -> DayKind {
    if norm::is_weekend(day) {
        DayKind::Weekend
    } else if holidays.holiday(day).is_some() {
        DayKind::Holiday
    } else if marked_off {
        DayKind::DayOff
    } else {
        DayKind::Workday
    }
}

pub(super) fn day_kind(
    conn: &Connection,
    day: NaiveDate,
    settings: &WorkTimeSettings,
) -> Result<DayKind, WorkTimeError> {
    Ok(classify(day, is_day_off(conn, day)?, settings.holidays))
}

/// The one place a day's worked time, norm, deviation and kind are computed.
/// `entries` and `days_off` must cover `d` (a wider range is fine).
pub(super) fn day_report<Tz: TimeZone>(
    d: NaiveDate,
    today: NaiveDate,
    now: i64,
    tz: &Tz,
    entries: &[Entry],
    days_off: &BTreeSet<NaiveDate>,
    settings: &WorkTimeSettings,
) -> Result<DayReport, WorkTimeError> {
    let holidays = settings.holidays;
    let (start, end) = day_bounds(d, tz)?;
    let mut day_worked = 0;
    let mut entry_count = 0;
    for e in entries {
        let p = portion(e.start, e.end, now, start, end);
        if p > 0 {
            day_worked += p;
            entry_count += 1;
        }
    }
    let norm_ms = norm::day_norm_ms(d, days_off, holidays);
    let holiday = holidays.holiday(d);
    Ok(DayReport {
        day: d.format(DAY_FORMAT).to_string(),
        kind: classify(d, days_off.contains(&d), holidays),
        worked_ms: day_worked,
        norm_ms,
        deviation_ms: norm::day_deviation_ms(d, today, norm_ms, day_worked),
        entry_count,
        holiday: holiday.map(str::to_string),
        day_off_owed: holiday.is_some() && d.weekday() == Weekday::Sat,
    })
}

/// The report of the month containing `day`, on an open connection.
pub(super) fn compute<Tz: TimeZone>(
    conn: &Connection,
    day: NaiveDate,
    now: i64,
    tz: &Tz,
    settings: &WorkTimeSettings,
) -> Result<MonthReport, WorkTimeError> {
    let holidays = settings.holidays;
    let (first, next_first) = norm::month_days(day);
    let (month_start, _) = day_bounds(first, tz)?;
    let (month_end, _) = day_bounds(next_first, tz)?;
    let today = local_day(now, tz)?;
    let phase = norm::phase(first, today);

    let days_off = days_off_between(conn, first, next_first)?;
    let workdays = norm::workdays(first, next_first, &days_off, holidays);
    let entries = in_day(conn, month_start, month_end, now)?;
    let worked_ms = entries
        .iter()
        .map(|e| portion(e.start, e.end, now, month_start, month_end))
        .sum();

    let mut days = Vec::new();
    for d in first.iter_days().take_while(|d| *d < next_first) {
        days.push(day_report(
            d, today, now, tz, &entries, &days_off, settings,
        )?);
    }

    Ok(MonthReport {
        month: first.format("%Y-%m").to_string(),
        phase,
        as_of_ms: now,
        worked_ms,
        figures: norm::figures(&workdays, today, phase, worked_ms),
        days_off: days_off
            .iter()
            .filter(|d| holidays.holiday(**d).is_none())
            .map(|d| d.format(DAY_FORMAT).to_string())
            .collect(),
        days,
    })
}

/// Report of the month containing `day`, as of `now`.
pub fn month_report<Tz: TimeZone>(
    store: &Store,
    day: NaiveDate,
    now: i64,
    tz: &Tz,
    settings: &WorkTimeSettings,
) -> Result<MonthReport, WorkTimeError> {
    store.read(|c| compute(c, day, now, tz, settings))
}

/// Parses a `YYYY-MM-DD` day from the UI. Only a four-digit year in the
/// canonical form is accepted (it must format back to the same string), so a
/// parsed day always yields a key the `days_off` CHECK allows: no signed or
/// five-digit years, no `2026-1-5`.
pub fn parse_day(day: &str) -> Result<NaiveDate, WorkTimeError> {
    NaiveDate::parse_from_str(day, DAY_FORMAT)
        .ok()
        .filter(|d| (1..=9999).contains(&d.year()))
        .filter(|d| d.format(DAY_FORMAT).to_string() == day)
        .ok_or(WorkTimeError::InvalidLocalTime)
}

/// Marks or unmarks a day off. Idempotent; a weekend is refused with
/// `NotAWorkday` and a weekday holiday with `Holiday`, both before any write
/// and in both directions.
pub fn set_day_off(
    store: &mut Store,
    day: NaiveDate,
    off: bool,
    now: i64,
    settings: &WorkTimeSettings,
) -> Result<(), WorkTimeError> {
    if norm::is_weekend(day) {
        return Err(WorkTimeError::NotAWorkday);
    }
    if settings.holidays.holiday(day).is_some() {
        return Err(WorkTimeError::Holiday);
    }
    let key = day.format(DAY_FORMAT).to_string();
    store.write(|tx| {
        if off {
            // Only the duplicate is ignored; a CHECK violation still fails the write.
            tx.execute(
                "INSERT INTO days_off(day, created_at_ms) VALUES (?1, ?2) \
                 ON CONFLICT(day) DO NOTHING",
                (&key, now),
            )?;
        } else {
            tx.execute("DELETE FROM days_off WHERE day = ?1", [&key])?;
        }
        Ok(())
    })
}

/// Whether `day` is a Monday-to-Friday day that is neither a holiday nor
/// marked off.
pub fn is_workday(
    store: &Store,
    day: NaiveDate,
    settings: &WorkTimeSettings,
) -> Result<bool, WorkTimeError> {
    store.read(|c| Ok(day_kind(c, day, settings)? == DayKind::Workday))
}
