//! Pure calendar and norm rules: workdays of a month, month phase and the
//! norm figures. No store, no clock, no time zone: dates and milliseconds only.

use std::collections::BTreeSet;

use chrono::{Datelike, Months, NaiveDate, Weekday};
use serde::Serialize;

use super::holidays::HolidayCalendar;

const MINUTE_MS: i64 = 60_000;

/// The default daily norm (8 h per workday), used when `settings.json` sets none.
pub const DEFAULT_DAILY_NORM_MS: i64 = 8 * 3_600_000;

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

/// Monday to Friday days in `[first, next_first)` that are not in `days_off`
/// and not a public holiday of `holidays`. A weekend is never a workday, even
/// when it is listed in `days_off`.
pub fn workdays(
    first: NaiveDate,
    next_first: NaiveDate,
    days_off: &BTreeSet<NaiveDate>,
    holidays: HolidayCalendar,
) -> Vec<NaiveDate> {
    first
        .iter_days()
        .take_while(|d| *d < next_first)
        .filter(|d| !is_weekend(*d) && !days_off.contains(d) && holidays.holiday(*d).is_none())
        .collect()
}

/// Norm of one day: `daily_norm_ms` on a Monday-to-Friday day that is not in
/// `days_off` and not a public holiday, otherwise 0. A weekend is 0 even when
/// listed in `days_off`.
pub fn day_norm_ms(
    day: NaiveDate,
    days_off: &BTreeSet<NaiveDate>,
    holidays: HolidayCalendar,
    daily_norm_ms: i64,
) -> i64 {
    if is_weekend(day) || days_off.contains(&day) || holidays.holiday(day).is_some() {
        0
    } else {
        daily_norm_ms
    }
}

/// Days of `[first, end_exclusive)` with a norm above 0 and that norm,
/// ascending. Works for any range (a month, a week): the one source of day
/// norms for the month figures and for week summaries.
pub fn day_norms(
    first: NaiveDate,
    end_exclusive: NaiveDate,
    days_off: &BTreeSet<NaiveDate>,
    holidays: HolidayCalendar,
    daily_norm_ms: i64,
) -> Vec<(NaiveDate, i64)> {
    first
        .iter_days()
        .take_while(|d| *d < end_exclusive)
        .map(|d| (d, day_norm_ms(d, days_off, holidays, daily_norm_ms)))
        .filter(|(_, norm)| *norm > 0)
        .collect()
}

/// Deviation of one day: `None` for a day after `today` and for a day with
/// neither norm nor hours; otherwise worked minus norm.
pub fn day_deviation_ms(
    day: NaiveDate,
    today: NaiveDate,
    norm_ms: i64,
    worked_ms: i64,
) -> Option<i64> {
    if day > today || (norm_ms == 0 && worked_ms == 0) {
        None
    } else {
        Some(worked_ms - norm_ms)
    }
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

/// Figures of a month with the given `day_norms` (days with a norm), the
/// `worked_ms` so far and the full-day `daily_norm_ms`, seen from `today`.
/// Today counts in full when it has a norm. "Needed per day" is the time per
/// full-norm day.
pub fn figures(
    day_norms: &[(NaiveDate, i64)],
    today: NaiveDate,
    phase: MonthPhase,
    worked_ms: i64,
    daily_norm_ms: i64,
) -> NormFigures {
    let count = day_norms.len() as u32;
    let norm_ms: i64 = day_norms.iter().map(|(_, n)| n).sum();
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
            let norm_to_date: i64 = day_norms
                .iter()
                .filter(|(d, _)| *d <= today)
                .map(|(_, n)| n)
                .sum();
            let after = day_norms.iter().filter(|(d, _)| *d > today).count() as u32;
            let norm_after = norm_ms - norm_to_date;
            let remaining = (norm_ms - worked_ms).max(0);
            f.norm_to_date_ms = Some(norm_to_date);
            f.deviation_ms = Some(worked_ms - norm_to_date);
            f.workdays_after_today = Some(after);
            f.remaining_ms = Some(remaining);
            if after > 0 && norm_after > 0 {
                // ceil(remaining * daily / norm_after), in i128 against overflow.
                let num = i128::from(remaining) * i128::from(daily_norm_ms);
                let den = i128::from(norm_after);
                let per_day = ((num + den - 1) / den) as i64;
                f.needed_per_day_ms = Some(ceil_to_minute(per_day));
            }
        }
    }
    f
}
