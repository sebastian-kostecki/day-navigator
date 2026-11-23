//! Work-time reminders: the facts query and the pure slot rules.
//!
//! No `tauri` import and no clock: `now` and the time zone are parameters.
//! Repetition uses hourly slots counted from the trigger instant. A reminder
//! is keyed by the slot's start instant and fires only when that instant is
//! newer than the one last sent for the same entry or day, so a restart, a
//! long sleep or a clock moved backwards never produces a backlog or a repeat.

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone};

use super::error::WorkTimeError;
use super::report::{self, DayKind};
use super::rules::{day_bounds, local_day, portion};
use super::{RunningEntry, in_day, running_entry};
use crate::health::APP_NAME;
use crate::settings::ReminderSettings;
use crate::store::Store;

const HOUR_MS: i64 = 3_600_000;
/// Longest DST gap searched for when resolving a nonexistent local time.
const MAX_GAP_MINUTES: i64 = 180;

/// What the rules need from the store, read in one snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReminderFacts {
    pub running: Option<RunningEntry>,
    pub today: NaiveDate,
    pub day_kind: DayKind,
    /// Today has a counted closed entry with a positive portion in today.
    pub worked_today: bool,
}

/// One toast to send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reminder {
    pub title: String,
    pub body: String,
}

/// The slot start instant last sent per reminder kind. In-process only.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReminderMemory {
    long: Option<(i64, i64)>,
    idle: Option<(NaiveDate, i64)>,
}

pub fn reminder_facts<Tz: TimeZone>(
    store: &Store,
    now: i64,
    tz: &Tz,
) -> Result<ReminderFacts, WorkTimeError> {
    let today = local_day(now, tz)?;
    let (day_start, day_end) = day_bounds(today, tz)?;
    store.read(|c| {
        let running = running_entry(c)?.map(|e| RunningEntry {
            id: e.id,
            start_ms: e.start,
        });
        let worked_today = in_day(c, day_start, day_end, now)?
            .iter()
            .any(|e| e.end.is_some() && portion(e.start, e.end, now, day_start, day_end) > 0);
        Ok(ReminderFacts {
            running,
            today,
            day_kind: report::day_kind(c, today)?,
            worked_today,
        })
    })
}

/// Local `day` at `time` as an instant. A single instant is used as is, an
/// ambiguous one resolves to the earlier instant, a time inside a DST gap to
/// the first instant after the gap. Never refuses, unlike `resolve_local`.
fn anchor_instant<Tz: TimeZone>(day: NaiveDate, time: NaiveTime, tz: &Tz) -> Option<i64> {
    let start = NaiveDateTime::new(day, time);
    (0..=MAX_GAP_MINUTES).find_map(|minutes| {
        let naive = start + chrono::Duration::minutes(minutes);
        tz.from_local_datetime(&naive)
            .earliest()
            .map(|d| d.timestamp_millis())
    })
}

/// "HH:MM" today, "wczoraj HH:MM" yesterday, otherwise "DD.MM HH:MM".
fn start_text<Tz: TimeZone>(start_ms: i64, today: NaiveDate, tz: &Tz) -> String {
    let Some(dt) = tz.timestamp_millis_opt(start_ms).earliest() else {
        return String::new();
    };
    let local = dt.naive_local();
    let date = local.date();
    if date == today {
        local.format("%H:%M").to_string()
    } else if today.pred_opt() == Some(date) {
        local.format("wczoraj %H:%M").to_string()
    } else {
        local.format("%d.%m %H:%M").to_string()
    }
}

fn long_running_due<Tz: TimeZone>(
    facts: &ReminderFacts,
    threshold_ms: i64,
    now: i64,
    tz: &Tz,
    memory: &mut ReminderMemory,
) -> Option<Reminder> {
    let run = facts.running.as_ref()?;
    let elapsed = now - run.start_ms;
    if elapsed < threshold_ms {
        return None;
    }
    let slot = (elapsed - threshold_ms) / HOUR_MS;
    let slot_start = run.start_ms + threshold_ms + slot * HOUR_MS;
    let fresh = match memory.long {
        Some((id, sent)) if id == run.id => slot_start > sent,
        _ => true,
    };
    if !fresh {
        return None;
    }
    memory.long = Some((run.id, slot_start));
    Some(Reminder {
        title: format!("{APP_NAME} — czy nadal pracujesz?"),
        body: format!(
            "Licznik czasu pracy biegnie od {} ({} h). Zatrzymaj go albo popraw wpis w „Czas pracy”.",
            start_text(run.start_ms, facts.today, tz),
            elapsed / HOUR_MS
        ),
    })
}

fn not_running_due<Tz: TimeZone>(
    facts: &ReminderFacts,
    after: NaiveTime,
    now: i64,
    tz: &Tz,
    memory: &mut ReminderMemory,
) -> Option<Reminder> {
    if facts.running.is_some() || facts.day_kind != DayKind::Workday || facts.worked_today {
        return None;
    }
    let anchor = anchor_instant(facts.today, after, tz)?;
    if now < anchor {
        return None;
    }
    let slot_start = anchor + (now - anchor) / HOUR_MS * HOUR_MS;
    let fresh = match memory.idle {
        Some((day, sent)) if day == facts.today => slot_start > sent,
        _ => true,
    };
    if !fresh {
        return None;
    }
    memory.idle = Some((facts.today, slot_start));
    Some(Reminder {
        title: format!("{APP_NAME} — licznik nie biegnie"),
        body: "Dziś jest dzień roboczy, a czas pracy nie jest liczony. Kliknij START albo oznacz dzień jako wolny."
            .to_string(),
    })
}

/// The reminders due at `now` (at most one of each kind) and the updated
/// memory. The memory changes only for reminders returned.
pub fn due_reminders<Tz: TimeZone>(
    facts: &ReminderFacts,
    settings: &ReminderSettings,
    now: i64,
    tz: &Tz,
    memory: &ReminderMemory,
) -> (Vec<Reminder>, ReminderMemory) {
    let mut next = memory.clone();
    let mut due = Vec::new();
    if let Some(threshold) = settings.long_running {
        let threshold_ms = i64::try_from(threshold.as_millis()).unwrap_or(i64::MAX);
        due.extend(long_running_due(facts, threshold_ms, now, tz, &mut next));
    }
    if let Some(after) = settings.not_running_after {
        due.extend(not_running_due(facts, after, now, tz, &mut next));
    }
    (due, next)
}
