//! Pure calendar and norm rules: workdays of a month, month phase and the
//! norm figures. No store, no clock, no time zone: dates and milliseconds only.

use std::collections::BTreeSet;

use chrono::{Datelike, Months, NaiveDate, Weekday};
use serde::Serialize;

const MINUTE_MS: i64 = 60_000;

/// The daily norm: a fixed 8 h per workday.
pub const DAILY_NORM_MS: i64 = 8 * 3_600_000;

/// First day of the month of `any_day` and the first day of the next month.
pub fn month_days(any_day: NaiveDate) -> (NaiveDate, NaiveDate) {
    let first = any_day.with_day(1).unwrap_or(any_day);
    let next_first = first
        .checked_add_months(Months::new(1))
        .unwrap_or(NaiveDate::MAX);
    (first, next_first)
}

/// Saturday or Sunday.
pub fn is_weekend(day: NaiveDate) -> bool {
    matches!(day.weekday(), Weekday::Sat | Weekday::Sun)
}

/// Monday to Friday days in `[first, next_first)` that are not in `days_off`.
/// A weekend is never a workday, even when it is listed in `days_off`.
pub fn workdays(
    first: NaiveDate,
    next_first: NaiveDate,
    days_off: &BTreeSet<NaiveDate>,
) -> Vec<NaiveDate> {
    first
        .iter_days()
        .take_while(|d| *d < next_first)
        .filter(|d| !is_weekend(*d) && !days_off.contains(d))
        .collect()
}

/// Where a month stands relative to today.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MonthPhase {
    Past,
    Current,
    Future,
}

/// Phase of the month starting at `first` relative to `today`'s month.
pub fn phase(first: NaiveDate, today: NaiveDate) -> MonthPhase {
    match (first.year(), first.month()).cmp(&(today.year(), today.month())) {
        std::cmp::Ordering::Less => MonthPhase::Past,
        std::cmp::Ordering::Equal => MonthPhase::Current,
        std::cmp::Ordering::Greater => MonthPhase::Future,
    }
}

/// Rounds up to a whole minute (for non-negative values).
pub fn ceil_to_minute(ms: i64) -> i64 {
    (ms + MINUTE_MS - 1).div_euclid(MINUTE_MS) * MINUTE_MS
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NormFigures {
    pub norm_ms: i64,
    pub norm_to_date_ms: Option<i64>,
    pub deviation_ms: Option<i64>,
    pub workdays: u32,
    pub workdays_after_today: Option<u32>,
    pub needed_per_day_ms: Option<i64>,
    pub remaining_ms: Option<i64>,
}

/// Figures of a month with the given `workdays`, `worked_ms` so far, seen from
/// `today`. Today counts in full when it is a workday.
pub fn figures(
    workdays: &[NaiveDate],
    today: NaiveDate,
    phase: MonthPhase,
    worked_ms: i64,
) -> NormFigures {
    let count = workdays.len() as u32;
    let norm_ms = DAILY_NORM_MS * i64::from(count);
    let mut f = NormFigures {
        norm_ms,
        norm_to_date_ms: None,
        deviation_ms: None,
        workdays: count,
        workdays_after_today: None,
        needed_per_day_ms: None,
        remaining_ms: None,
    };
    match phase {
        MonthPhase::Future => {}
        MonthPhase::Past => {
            f.norm_to_date_ms = Some(norm_ms);
            f.deviation_ms = Some(worked_ms - norm_ms);
        }
        MonthPhase::Current => {
            let to_date = workdays.iter().filter(|d| **d <= today).count() as u32;
            let after = count - to_date;
            let norm_to_date = DAILY_NORM_MS * i64::from(to_date);
            let remaining = (norm_ms - worked_ms).max(0);
            f.norm_to_date_ms = Some(norm_to_date);
            f.deviation_ms = Some(worked_ms - norm_to_date);
            f.workdays_after_today = Some(after);
            f.remaining_ms = Some(remaining);
            if after > 0 {
                let per_day = (remaining + i64::from(after) - 1) / i64::from(after);
                f.needed_per_day_ms = Some(ceil_to_minute(per_day));
            }
        }
    }
    f
}
