//! Summary of the day after STOP: today's work time, the month's deviation,
//! and today's "Do zrobienia" list split into done and carrying over.
//!
//! Pure rules, no `tauri` import (only `commands.rs` has one): `now` and the
//! time zone are parameters. Data comes from `work_time::status` and
//! `todo::list` only; the deviation is passed through, never computed here, so
//! the daily norm stays in `work_time::norm`.

pub mod commands;
pub mod error;

#[cfg(test)]
mod tests;

use chrono::TimeZone;
use serde::Serialize;

use crate::settings::WorkTimeSettings;
use crate::store::Store;
use crate::todo::{self, TodoItem, TodoList};
use crate::work_time::{self, WorkTimerStatus};
pub use error::DaySummaryError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DaySummary {
    /// The list's local day, `YYYY-MM-DD`.
    pub day: String,
    pub today_ms: i64,
    pub month_deviation_ms: i64,
    /// Ticked entries, in the order `todo::list` returned them.
    pub done: Vec<TodoItem>,
    /// Open entries (they carry over to tomorrow on their own).
    pub carry_over: Vec<TodoItem>,
}

/// The header's live total at `status.as_of_ms`.
pub fn today_ms(status: &WorkTimerStatus) -> i64 {
    status.today_closed_ms
        + status
            .running_from_ms
            .map_or(0, |from| (status.as_of_ms - from).max(0))
}

pub fn compose(status: &WorkTimerStatus, list: TodoList) -> DaySummary {
    let (done, carry_over) = list.items.into_iter().partition(|i| i.ticked);
    DaySummary {
        day: list.day,
        today_ms: today_ms(status),
        month_deviation_ms: status.month_deviation_ms,
        done,
        carry_over,
    }
}

/// Reads the status and then the list inside the caller's one store call, both
/// for the same local day. `todo::list` is a write (roll-over): keep this on
/// the blocking pool.
pub fn read<Tz: TimeZone>(
    store: &mut Store,
    now_ms: i64,
    tz: &Tz,
    settings: &WorkTimeSettings,
) -> Result<DaySummary, DaySummaryError> {
    let local = tz
        .timestamp_millis_opt(now_ms)
        .earliest()
        .ok_or(DaySummaryError::InvalidLocalTime)?
        .naive_local();
    let status = work_time::status(store, now_ms, tz, settings)?;
    let list = todo::list(store, local.date(), local, Ok(todo::DEFAULT_TODO_LIMIT))?;
    Ok(compose(&status, list))
}
