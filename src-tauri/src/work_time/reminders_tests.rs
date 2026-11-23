//! Reminder rule tests on invented data in `Europe/Warsaw`.

use std::time::Duration;

use chrono::{NaiveDate, NaiveTime};
use chrono_tz::Europe::Warsaw;

use super::DayKind;
use super::RunningEntry;
use super::reminders::{Reminder, ReminderFacts, ReminderMemory, due_reminders};
use super::rules::resolve_local;
use crate::settings::ReminderSettings;

fn t(local: &str) -> i64 {
    resolve_local(local, &Warsaw).unwrap()
}

fn d(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

fn settings() -> ReminderSettings {
    ReminderSettings::default()
}

fn running(day: &str, id: i64, start: &str) -> ReminderFacts {
    ReminderFacts {
        running: Some(RunningEntry {
            id,
            start_ms: t(start),
        }),
        today: d(day),
        day_kind: DayKind::Workday,
        worked_today: true,
    }
}

fn idle(day: &str, kind: DayKind, worked_today: bool) -> ReminderFacts {
    ReminderFacts {
        running: None,
        today: d(day),
        day_kind: kind,
        worked_today,
    }
}

/// One round: returns the reminders and carries the memory forward.
fn step(
    facts: &ReminderFacts,
    s: &ReminderSettings,
    at: &str,
    memory: &mut ReminderMemory,
) -> Vec<Reminder> {
    let (due, next) = due_reminders(facts, s, t(at), &Warsaw, memory);
    *memory = next;
    due
}

fn long_body(r: &Reminder) -> &str {
    assert_eq!(r.title, "Nawigator dnia — czy nadal pracujesz?");
    &r.body
}

#[test]
fn worked_example_hour_by_hour() {
    let s = settings();
    let mut m = ReminderMemory::default();
    // Monday 2026-11-23, 08:00-12:00 counted, 13:00 running.
    let monday = running("2026-11-23", 2, "2026-11-23T13:00");
    assert!(step(&monday, &s, "2026-11-23T19:00", &mut m).is_empty());
    let due = step(&monday, &s, "2026-11-23T23:00", &mut m);
    assert_eq!(due.len(), 1);
    assert_eq!(
        long_body(&due[0]),
        "Licznik czasu pracy biegnie od 13:00 (10 h). Zatrzymaj go albo popraw wpis w „Czas pracy”."
    );
    assert!(step(&monday, &s, "2026-11-23T23:40", &mut m).is_empty());
    let tuesday = running("2026-11-24", 2, "2026-11-23T13:00");
    let due = step(&tuesday, &s, "2026-11-24T00:00", &mut m);
    assert_eq!(due.len(), 1);
    assert_eq!(
        long_body(&due[0]),
        "Licznik czasu pracy biegnie od wczoraj 13:00 (11 h). Zatrzymaj go albo popraw wpis w „Czas pracy”."
    );
    // Stopped at 00:30: today already has counted work, so no "nie biegnie".
    let stopped = idle("2026-11-24", DayKind::Workday, true);
    assert!(step(&stopped, &s, "2026-11-24T09:30", &mut m).is_empty());
    assert!(step(&stopped, &s, "2026-11-24T10:30", &mut m).is_empty());
}

#[test]
fn threshold_boundary() {
    let s = settings();
    let f = running("2026-11-23", 1, "2026-11-23T08:00");
    let mut m = ReminderMemory::default();
    let just_before = t("2026-11-23T18:00") - 1_000;
    let (due, _) = due_reminders(&f, &s, just_before, &Warsaw, &m);
    assert!(due.is_empty());
    assert_eq!(step(&f, &s, "2026-11-23T18:00", &mut m).len(), 1);
}

#[test]
fn no_repeat_within_a_slot_and_hourly_after() {
    let s = settings();
    let f = running("2026-11-23", 1, "2026-11-23T08:00");
    let mut m = ReminderMemory::default();
    assert_eq!(step(&f, &s, "2026-11-23T18:00", &mut m).len(), 1);
    assert!(step(&f, &s, "2026-11-23T18:01", &mut m).is_empty());
    assert!(step(&f, &s, "2026-11-23T18:59", &mut m).is_empty());
    assert_eq!(step(&f, &s, "2026-11-23T19:00", &mut m).len(), 1);
    assert!(step(&f, &s, "2026-11-23T19:30", &mut m).is_empty());
}

#[test]
fn a_new_entry_starts_a_fresh_slot_sequence() {
    let s = settings();
    let mut m = ReminderMemory::default();
    let first = running("2026-11-23", 1, "2026-11-23T08:00");
    assert_eq!(step(&first, &s, "2026-11-23T18:00", &mut m).len(), 1);
    // Another entry, started 20:00 the same day, fires at its own 10 h.
    let second = running("2026-11-24", 2, "2026-11-23T20:00");
    assert!(step(&second, &s, "2026-11-24T05:59", &mut m).is_empty());
    assert_eq!(step(&second, &s, "2026-11-24T06:00", &mut m).len(), 1);
}

#[test]
fn start_text_covers_today_yesterday_and_older() {
    let s = settings();
    let today = running("2026-11-23", 1, "2026-11-23T07:30");
    let (due, _) = due_reminders(
        &today,
        &s,
        t("2026-11-23T17:30"),
        &Warsaw,
        &ReminderMemory::default(),
    );
    assert!(long_body(&due[0]).contains("od 07:30 (10 h)"));
    let old = running("2026-11-23", 1, "2026-11-20T22:15");
    let (due, _) = due_reminders(
        &old,
        &s,
        t("2026-11-23T12:00"),
        &Warsaw,
        &ReminderMemory::default(),
    );
    assert!(long_body(&due[0]).contains("od 20.11 22:15 (61 h)"));
}

#[test]
fn not_running_follows_the_hourly_slots() {
    let s = settings();
    let f = idle("2026-11-23", DayKind::Workday, false);
    let mut m = ReminderMemory::default();
    assert!(step(&f, &s, "2026-11-23T08:59", &mut m).is_empty());
    let due = step(&f, &s, "2026-11-23T09:00", &mut m);
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].title, "Nawigator dnia — licznik nie biegnie");
    assert_eq!(
        due[0].body,
        "Dziś jest dzień roboczy, a czas pracy nie jest liczony. Kliknij START albo oznacz dzień jako wolny."
    );
    assert!(step(&f, &s, "2026-11-23T09:30", &mut m).is_empty());
    assert_eq!(step(&f, &s, "2026-11-23T10:00", &mut m).len(), 1);
}

#[test]
fn not_running_is_silent_when_it_should_be() {
    let s = settings();
    let at = "2026-11-23T11:00";
    let silent = [
        idle("2026-11-23", DayKind::Weekend, false),
        idle("2026-11-23", DayKind::DayOff, false),
        idle("2026-11-23", DayKind::Workday, true),
    ];
    for f in silent {
        let mut m = ReminderMemory::default();
        assert!(step(&f, &s, at, &mut m).is_empty(), "{f:?}");
    }
    // While running only the long-run rule can speak (not yet due here).
    let f = running("2026-11-23", 1, "2026-11-23T10:00");
    assert!(step(&f, &s, at, &mut ReminderMemory::default()).is_empty());
}

#[test]
fn each_reminder_is_silent_when_switched_off() {
    let off_long = ReminderSettings {
        long_running: None,
        ..settings()
    };
    let f = running("2026-11-23", 1, "2026-11-23T06:00");
    let mut m = ReminderMemory::default();
    assert!(step(&f, &off_long, "2026-11-23T20:00", &mut m).is_empty());
    let off_idle = ReminderSettings {
        not_running_after: None,
        ..settings()
    };
    let f = idle("2026-11-23", DayKind::Workday, false);
    assert!(step(&f, &off_idle, "2026-11-23T12:00", &mut m).is_empty());
    // The other trigger still works with one switched off.
    assert_eq!(step(&f, &off_long, "2026-11-23T12:00", &mut m).len(), 1);
}

#[test]
fn an_empty_memory_fires_the_current_slot_once() {
    let s = settings();
    let f = running("2026-11-23", 1, "2026-11-23T08:00");
    // Restart in the middle of slot 3.
    let mut m = ReminderMemory::default();
    assert_eq!(step(&f, &s, "2026-11-23T21:30", &mut m).len(), 1);
    assert!(step(&f, &s, "2026-11-23T21:45", &mut m).is_empty());
    let f = idle("2026-11-23", DayKind::Workday, false);
    let mut m = ReminderMemory::default();
    assert_eq!(step(&f, &s, "2026-11-23T13:20", &mut m).len(), 1);
    assert!(step(&f, &s, "2026-11-23T13:50", &mut m).is_empty());
}

#[test]
fn a_jump_of_several_hours_fires_once() {
    let s = settings();
    let f = running("2026-11-23", 1, "2026-11-23T01:00");
    let mut m = ReminderMemory::default();
    assert_eq!(step(&f, &s, "2026-11-23T11:00", &mut m).len(), 1);
    // The machine slept for five hours: one toast, no backlog.
    assert_eq!(step(&f, &s, "2026-11-23T16:10", &mut m).len(), 1);
    assert!(step(&f, &s, "2026-11-23T16:20", &mut m).is_empty());
    let f = idle("2026-11-23", DayKind::Workday, false);
    let mut m = ReminderMemory::default();
    assert_eq!(step(&f, &s, "2026-11-23T09:00", &mut m).len(), 1);
    assert_eq!(step(&f, &s, "2026-11-23T14:10", &mut m).len(), 1);
    assert!(step(&f, &s, "2026-11-23T14:20", &mut m).is_empty());
}

#[test]
fn a_clock_moved_backwards_does_not_repeat() {
    let s = settings();
    let f = running("2026-11-23", 1, "2026-11-23T08:00");
    let mut m = ReminderMemory::default();
    assert_eq!(step(&f, &s, "2026-11-23T19:00", &mut m).len(), 1);
    assert!(step(&f, &s, "2026-11-23T18:10", &mut m).is_empty());
    assert!(step(&f, &s, "2026-11-23T19:10", &mut m).is_empty());
    assert_eq!(step(&f, &s, "2026-11-23T20:00", &mut m).len(), 1);
    let f = idle("2026-11-23", DayKind::Workday, false);
    let mut m = ReminderMemory::default();
    assert_eq!(step(&f, &s, "2026-11-23T11:00", &mut m).len(), 1);
    assert!(step(&f, &s, "2026-11-23T10:00", &mut m).is_empty());
    assert!(step(&f, &s, "2026-11-23T11:20", &mut m).is_empty());
}

#[test]
fn a_start_edited_later_fires_at_the_new_threshold() {
    let s = settings();
    let mut m = ReminderMemory::default();
    let original = running("2026-11-23", 1, "2026-11-23T08:00");
    // Slots 0, 1, 2 at 18:00, 19:00, 20:00.
    for at in ["2026-11-23T18:00", "2026-11-23T19:00", "2026-11-23T20:00"] {
        assert_eq!(step(&original, &s, at, &mut m).len(), 1);
    }
    // The start is edited to 12:00 (same entry): new threshold is 22:00.
    let edited = running("2026-11-23", 1, "2026-11-23T12:00");
    assert!(step(&edited, &s, "2026-11-23T20:30", &mut m).is_empty());
    assert!(step(&edited, &s, "2026-11-23T21:59", &mut m).is_empty());
    assert_eq!(step(&edited, &s, "2026-11-23T22:00", &mut m).len(), 1);
}

#[test]
fn a_start_edited_earlier_sends_no_burst() {
    let s = settings();
    let mut m = ReminderMemory::default();
    let original = running("2026-11-23", 1, "2026-11-23T10:00");
    assert_eq!(step(&original, &s, "2026-11-23T20:00", &mut m).len(), 1);
    // Moved to 06:00: the current slot started 20:00, already sent.
    let edited = running("2026-11-23", 1, "2026-11-23T06:00");
    assert!(step(&edited, &s, "2026-11-23T20:10", &mut m).is_empty());
    assert_eq!(step(&edited, &s, "2026-11-23T21:00", &mut m).len(), 1);
}

#[test]
fn not_running_after_moved_later_waits_for_the_next_full_hour() {
    let mut m = ReminderMemory::default();
    let f = idle("2026-11-23", DayKind::Workday, false);
    assert_eq!(step(&f, &settings(), "2026-11-23T11:00", &mut m).len(), 1);
    let later = ReminderSettings {
        not_running_after: NaiveTime::from_hms_opt(10, 30, 0),
        ..settings()
    };
    // Slots are now 10:30, 11:30, ...: the 11:00 toast covers nothing new.
    assert!(step(&f, &later, "2026-11-23T11:10", &mut m).is_empty());
    assert!(step(&f, &later, "2026-11-23T11:29", &mut m).is_empty());
    assert_eq!(step(&f, &later, "2026-11-23T11:30", &mut m).len(), 1);
}

#[test]
fn not_running_after_moved_earlier_sends_no_burst() {
    let mut m = ReminderMemory::default();
    let f = idle("2026-11-23", DayKind::Workday, false);
    assert_eq!(step(&f, &settings(), "2026-11-23T11:00", &mut m).len(), 1);
    let earlier = ReminderSettings {
        not_running_after: NaiveTime::from_hms_opt(8, 0, 0),
        ..settings()
    };
    assert!(step(&f, &earlier, "2026-11-23T11:05", &mut m).is_empty());
    assert!(step(&f, &earlier, "2026-11-23T11:55", &mut m).is_empty());
    assert_eq!(step(&f, &earlier, "2026-11-23T12:00", &mut m).len(), 1);
}

#[test]
fn a_new_day_resets_the_not_running_memory() {
    let s = settings();
    let mut m = ReminderMemory::default();
    let monday = idle("2026-11-23", DayKind::Workday, false);
    assert_eq!(step(&monday, &s, "2026-11-23T09:00", &mut m).len(), 1);
    let tuesday = idle("2026-11-24", DayKind::Workday, false);
    assert_eq!(step(&tuesday, &s, "2026-11-24T09:00", &mut m).len(), 1);
}

#[test]
fn dst_gap_anchors_at_the_first_instant_after_the_gap() {
    // 2026-03-29: 02:00 -> 03:00 in Warsaw, so 02:30 does not exist.
    let s = ReminderSettings {
        not_running_after: NaiveTime::from_hms_opt(2, 30, 0),
        ..settings()
    };
    let f = idle("2026-03-29", DayKind::Workday, false);
    let mut m = ReminderMemory::default();
    assert!(step(&f, &s, "2026-03-29T01:59", &mut m).is_empty());
    assert_eq!(step(&f, &s, "2026-03-29T03:00", &mut m).len(), 1);
    assert!(step(&f, &s, "2026-03-29T03:59", &mut m).is_empty());
    assert_eq!(step(&f, &s, "2026-03-29T04:00", &mut m).len(), 1);
}

#[test]
fn dst_fall_back_counts_the_extra_hour_in_the_elapsed_time() {
    // 2026-10-25: 03:00 -> 02:00, a 25 h day. 22:00 CEST to 07:00 CET is 10 h.
    let s = settings();
    let f = running("2026-10-25", 1, "2026-10-24T22:00");
    let mut m = ReminderMemory::default();
    assert!(step(&f, &s, "2026-10-25T06:59", &mut m).is_empty());
    let due = step(&f, &s, "2026-10-25T07:00", &mut m);
    assert_eq!(due.len(), 1);
    assert!(long_body(&due[0]).contains("od wczoraj 22:00 (10 h)"));
}

#[test]
fn reminders_use_the_default_thresholds() {
    let s = settings();
    assert_eq!(s.long_running, Some(Duration::from_secs(10 * 3600)));
    assert_eq!(s.not_running_after, NaiveTime::from_hms_opt(9, 0, 0));
}
