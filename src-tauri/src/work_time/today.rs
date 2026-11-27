//! Today's live work total for the tray: a cheap read without the month report.
//!
//! No `tauri` import and no clock: `now` and the time zone are parameters. The
//! value equals what the header shows (closed time plus the running span, the
//! span capped at the day end and clamped at zero).

use chrono::TimeZone;

use super::error::WorkTimeError;
use super::rules::{day_bounds, local_day, portion};
use super::{in_day, running_entry};
use crate::store::Store;

/// Whether the counter runs and today's total including the running span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TodayWork {
    pub running: bool,
    pub today_ms: i64,
}

/// One store read. `running` looks at any day; `today_ms` counts only the part
/// of each counted entry that falls in today.
pub fn today_work<Tz: TimeZone>(
    store: &Store,
    now: i64,
    tz: &Tz,
) -> Result<TodayWork, WorkTimeError> {
    let today = local_day(now, tz)?;
    let (day_start, day_end) = day_bounds(today, tz)?;
    store.read(|c| {
        let today_ms = in_day(c, day_start, day_end, now)?
            .iter()
            .map(|e| portion(e.start, e.end, now, day_start, day_end))
            .sum();
        Ok(TodayWork {
            running: running_entry(c)?.is_some(),
            today_ms,
        })
    })
}
