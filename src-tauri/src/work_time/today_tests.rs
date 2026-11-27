//! Tests for today's live total, in `Europe/Warsaw` with invented instants.

use chrono::NaiveDate;
use chrono_tz::Europe::Warsaw;
use tempfile::TempDir;

use super::rules::resolve_local;
use super::today::{TodayWork, today_work};
use super::*;
use crate::settings::WorkTimeSettings;
use crate::store::schema;

const HOUR: i64 = 3_600_000;

fn t(local: &str) -> i64 {
    resolve_local(local, &Warsaw).unwrap()
}

/// "Now" for every test: Monday 2026-11-23, noon local.
fn now() -> i64 {
    t("2026-11-23T12:00")
}

fn open() -> (TempDir, Store) {
    let tmp = tempfile::tempdir().unwrap();
    let today = NaiveDate::from_ymd_opt(2026, 11, 23).unwrap();
    let store = Store::open(&tmp.path().join("data"), schema::MIGRATIONS, today).unwrap();
    (tmp, store)
}

/// Checks the value and that it equals the header formula
/// (`src/worktime/total.ts`) computed from `status()`.
fn check(store: &Store, expected: TodayWork) {
    let got = today_work(store, now(), &Warsaw).unwrap();
    assert_eq!(got, expected);

    let s = status(store, now(), &Warsaw, &WorkTimeSettings::default()).unwrap();
    let running_span = s
        .running_from_ms
        .map_or(0, |from| (now().min(s.day_end_ms) - from).max(0));
    assert_eq!(got.today_ms, s.today_closed_ms + running_span);
    assert_eq!(got.running, s.running.is_some());
}

#[test]
fn no_entries_is_zero_and_stopped() {
    let (_tmp, store) = open();
    check(
        &store,
        TodayWork {
            running: false,
            today_ms: 0,
        },
    );
}

#[test]
fn closed_entries_are_summed() {
    let (_tmp, mut store) = open();
    add(
        &mut store,
        "2026-11-23T08:00",
        "2026-11-23T10:00",
        "",
        now(),
        &Warsaw,
    )
    .unwrap();
    add(
        &mut store,
        "2026-11-23T10:30",
        "2026-11-23T11:00",
        "",
        now(),
        &Warsaw,
    )
    .unwrap();
    check(
        &store,
        TodayWork {
            running: false,
            today_ms: 2 * HOUR + HOUR / 2,
        },
    );
}

#[test]
fn running_entry_adds_the_span_up_to_now() {
    let (_tmp, mut store) = open();
    add(
        &mut store,
        "2026-11-23T08:00",
        "2026-11-23T09:00",
        "",
        now(),
        &Warsaw,
    )
    .unwrap();
    start(&mut store, t("2026-11-23T10:00"), &Warsaw).unwrap();
    check(
        &store,
        TodayWork {
            running: true,
            today_ms: HOUR + 2 * HOUR,
        },
    );
}

#[test]
fn running_entry_from_yesterday_counts_from_midnight() {
    let (_tmp, mut store) = open();
    start(&mut store, t("2026-11-22T22:00"), &Warsaw).unwrap();
    check(
        &store,
        TodayWork {
            running: true,
            today_ms: 12 * HOUR,
        },
    );
}

#[test]
fn closed_entry_crossing_midnight_counts_only_today() {
    let (_tmp, mut store) = open();
    add(
        &mut store,
        "2026-11-22T23:00",
        "2026-11-23T01:30",
        "",
        now(),
        &Warsaw,
    )
    .unwrap();
    check(
        &store,
        TodayWork {
            running: false,
            today_ms: HOUR + HOUR / 2,
        },
    );
}

#[test]
fn removed_entry_is_not_counted() {
    let (_tmp, mut store) = open();
    let id = add(
        &mut store,
        "2026-11-23T08:00",
        "2026-11-23T10:00",
        "",
        now(),
        &Warsaw,
    )
    .unwrap();
    add(
        &mut store,
        "2026-11-23T10:00",
        "2026-11-23T11:00",
        "",
        now(),
        &Warsaw,
    )
    .unwrap();
    remove(&mut store, id, now()).unwrap();
    check(
        &store,
        TodayWork {
            running: false,
            today_ms: HOUR,
        },
    );
}

#[test]
fn running_entry_starting_after_now_contributes_zero() {
    let (_tmp, mut store) = open();
    // The clock was moved back after the entry started.
    start(&mut store, t("2026-11-23T13:00"), &Warsaw).unwrap();
    check(
        &store,
        TodayWork {
            running: true,
            today_ms: 0,
        },
    );
}

#[test]
fn stopped_entry_no_longer_runs() {
    let (_tmp, mut store) = open();
    start(&mut store, t("2026-11-23T09:00"), &Warsaw).unwrap();
    stop(&mut store, t("2026-11-23T11:00"), &Warsaw).unwrap();
    check(
        &store,
        TodayWork {
            running: false,
            today_ms: 2 * HOUR,
        },
    );
}
