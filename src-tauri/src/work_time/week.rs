//! Store-backed report of one Monday-to-Sunday week.
//!
//! No `tauri` import and no clock: `now` and the time zone are parameters (like
//! `today.rs`). Every day row comes from `report::day_report`, the month's own
//! per-day code, over one store snapshot, so a week row equals the month
//! report's row for the same day. The week totals are sums of those rows.

use chrono::{Datelike, Days, NaiveDate, TimeZone};
use serde::Serialize;

use super::error::WorkTimeError;
use super::in_day;
use super::report::{self, DayReport};
use super::rules::{day_bounds, local_day};
use crate::settings::WorkTimeSettings;
use crate::store::Store;

/// The Monday on or before `any_day` and the next Monday (half-open range).
pub fn week_days(any_day: NaiveDate) -> (NaiveDate, NaiveDate) {
    let monday = any_day
        .checked_sub_days(Days::new(u64::from(
            any_day.weekday().num_days_from_monday(),
        )))
        .unwrap_or(NaiveDate::MIN);
    let next_monday = monday
        .checked_add_days(Days::new(7))
        .unwrap_or(NaiveDate::MAX);
    (monday, next_monday)
}

/// Where a week lies relative to today.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WeekPhase {
    Past,
    Current,
    Future,
}

/// Phase of the week starting on `monday` as seen from `today`.
pub fn week_phase(monday: NaiveDate, today: NaiveDate) -> WeekPhase {
    let this_monday = week_days(today).0;
    if monday < this_monday {
        WeekPhase::Past
    } else if monday == this_monday {
        WeekPhase::Current
    } else {
        WeekPhase::Future
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekReport {
    /// `YYYY-MM-DD`, first day row.
    pub monday: String,
    /// `YYYY-MM-DD`, last day row.
    pub sunday: String,
    pub phase: WeekPhase,
    pub as_of_ms: i64,
    pub worked_ms: i64,
    /// Norm of the whole week.
    pub norm_ms: i64,
    /// Norm of the days up to and including today (all of a past week);
    /// `None` for a future week.
    pub norm_to_date_ms: Option<i64>,
    /// `worked_ms - norm_to_date_ms`; `None` for a future week.
    pub deviation_ms: Option<i64>,
    /// `max(norm_ms - worked_ms, 0)` for the current week only.
    pub remaining_ms: Option<i64>,
    /// Exactly seven rows, Monday first.
    pub days: Vec<DayReport>,
}

/// Report of the week containing `day`, as of `now`, from one store read.
pub fn week_report<Tz: TimeZone>(
    store: &Store,
    day: NaiveDate,
    now: i64,
    tz: &Tz,
    settings: &WorkTimeSettings,
) -> Result<WeekReport, WorkTimeError> {
    let (monday, next_monday) = week_days(day);
    let (week_start, _) = day_bounds(monday, tz)?;
    let (week_end, _) = day_bounds(next_monday, tz)?;
    let today = local_day(now, tz)?;
    let phase = week_phase(monday, today);

    store.read(|c| {
        let days_off = report::days_off_between(c, monday, next_monday)?;
        let entries = in_day(c, week_start, week_end, now)?;

        let mut days = Vec::with_capacity(7);
        let mut norm_to_date = 0;
        for d in monday.iter_days().take_while(|d| *d < next_monday) {
            let row = report::day_report(d, today, now, tz, &entries, &days_off, settings)?;
            if d <= today {
                norm_to_date += row.norm_ms;
            }
            days.push(row);
        }

        let worked_ms: i64 = days.iter().map(|r| r.worked_ms).sum();
        let norm_ms: i64 = days.iter().map(|r| r.norm_ms).sum();
        let norm_to_date_ms = match phase {
            WeekPhase::Past => Some(norm_ms),
            WeekPhase::Current => Some(norm_to_date),
            WeekPhase::Future => None,
        };
        Ok(WeekReport {
            monday: days.first().map(|r| r.day.clone()).unwrap_or_default(),
            sunday: days.last().map(|r| r.day.clone()).unwrap_or_default(),
            phase,
            as_of_ms: now,
            worked_ms,
            norm_ms,
            norm_to_date_ms,
            deviation_ms: norm_to_date_ms.map(|n| worked_ms - n),
            remaining_ms: (phase == WeekPhase::Current).then(|| (norm_ms - worked_ms).max(0)),
            days,
        })
    })
}
