//! Pure rule functions: local-time resolution, day bounds, day portions and
//! interval overlap. No store access, no clock.

use chrono::{NaiveDate, NaiveDateTime, TimeZone};

use super::error::WorkTimeError;

/// Format of the local date-time strings exchanged with the UI.
pub const LOCAL_FORMAT: &str = "%Y-%m-%dT%H:%M";

/// `"YYYY-MM-DDTHH:MM"` in `tz` to an epoch-millisecond instant. A time that
/// does not exist (DST gap) is refused; an ambiguous one resolves to the
/// earlier instant.
pub fn resolve_local<Tz: TimeZone>(local: &str, tz: &Tz) -> Result<i64, WorkTimeError> {
    let naive = NaiveDateTime::parse_from_str(local, LOCAL_FORMAT)
        .map_err(|_| WorkTimeError::InvalidLocalTime)?;
    tz.from_local_datetime(&naive)
        .earliest()
        .map(|d| d.timestamp_millis())
        .ok_or(WorkTimeError::InvalidLocalTime)
}

/// The instant as a local `"YYYY-MM-DDTHH:MM"` string (seconds dropped).
pub fn local_string<Tz: TimeZone>(ms: i64, tz: &Tz) -> Result<String, WorkTimeError> {
    let dt = tz
        .timestamp_millis_opt(ms)
        .earliest()
        .ok_or(WorkTimeError::InvalidLocalTime)?;
    Ok(dt.naive_local().format(LOCAL_FORMAT).to_string())
}

/// The local calendar day an instant falls on.
pub fn local_day<Tz: TimeZone>(ms: i64, tz: &Tz) -> Result<NaiveDate, WorkTimeError> {
    let dt = tz
        .timestamp_millis_opt(ms)
        .earliest()
        .ok_or(WorkTimeError::InvalidLocalTime)?;
    Ok(dt.naive_local().date())
}

/// `[start, end)` of a local day in epoch milliseconds (local midnight to the
/// next local midnight, so a DST day is 23 h or 25 h long).
pub fn day_bounds<Tz: TimeZone>(day: NaiveDate, tz: &Tz) -> Result<(i64, i64), WorkTimeError> {
    let midnight = |d: NaiveDate| -> Result<i64, WorkTimeError> {
        let naive = d
            .and_hms_opt(0, 0, 0)
            .ok_or(WorkTimeError::InvalidLocalTime)?;
        tz.from_local_datetime(&naive)
            .earliest()
            .map(|dt| dt.timestamp_millis())
            .ok_or(WorkTimeError::InvalidLocalTime)
    };
    let next = day.succ_opt().ok_or(WorkTimeError::InvalidLocalTime)?;
    Ok((midnight(day)?, midnight(next)?))
}

/// Milliseconds of `[start, end)` that fall in `[day_start, day_end)`. A
/// running entry (`end` is `None`) counts up to `now`.
pub fn portion(start: i64, end: Option<i64>, now: i64, day_start: i64, day_end: i64) -> i64 {
    let e = end.unwrap_or(now).min(day_end);
    let s = start.max(day_start);
    (e - s).max(0)
}
