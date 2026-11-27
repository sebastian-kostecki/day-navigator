//! Week-report tests on invented data in `Europe/Warsaw`.
//!
//! Expected norms are `n * workday_norm` (the norm of an ordinary workday row
//! of the same report), never a literal, and every week is cross-checked row by
//! row against the month report of each day's month.

use chrono::NaiveDate;
use chrono_tz::Europe::Warsaw;
use tempfile::TempDir;

use super::norm_tests::manual_only;
use super::report::{self, DayKind, DayReport};
use super::rules::resolve_local;
use super::week::{WeekPhase, WeekReport, week_days, week_phase, week_report};
use super::*;
use crate::settings::WorkTimeSettings;
use crate::store::schema;

const MIN: i64 = 60_000;
const HOUR: i64 = 60 * MIN;

fn t(local: &str) -> i64 {
    resolve_local(local, &Warsaw).unwrap()
}

fn d(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

fn open() -> (TempDir, Store) {
    let tmp = tempfile::tempdir().unwrap();
    let store = Store::open(
        &tmp.path().join("data"),
        schema::MIGRATIONS,
        d("2026-11-23"),
    )
    .unwrap();
    (tmp, store)
}

/// Adds a closed entry; the "now" used for the future check is far ahead, so
/// the report's own `now` can be chosen freely.
fn add_entry(store: &mut Store, from: &str, to: &str) {
    add(store, from, to, "", t("2027-12-31T12:00"), &Warsaw).unwrap();
}

/// The only place the week tests mark a day off.
fn mark_off(
    store: &mut Store,
    day: &str,
    settings: &WorkTimeSettings,
) -> Result<(), WorkTimeError> {
    report::set_day_off(store, d(day), true, t("2026-11-23T12:00"), settings)
}

fn week_at(store: &Store, day: &str, now: i64, settings: &WorkTimeSettings) -> WeekReport {
    let w = week_report(store, d(day), now, &Warsaw, settings).unwrap();
    assert_eq!(w.days.len(), 7);
    assert_eq!(w.days[0].day, w.monday);
    assert_eq!(w.days[6].day, w.sunday);
    check_totals_and_months(store, &w, now, settings);
    w
}

/// The norm of an ordinary workday of this week.
fn workday_norm(w: &WeekReport) -> i64 {
    w.days
        .iter()
        .find(|r| r.kind == DayKind::Workday)
        .expect("the week has a workday")
        .norm_ms
}

fn row<'a>(w: &'a WeekReport, day: &str) -> &'a DayReport {
    w.days.iter().find(|r| r.day == day).unwrap()
}

fn workday_count(w: &WeekReport) -> i64 {
    w.days.iter().filter(|r| r.kind == DayKind::Workday).count() as i64
}

/// Sums add up and every row equals the month report's row for that day.
fn check_totals_and_months(store: &Store, w: &WeekReport, now: i64, settings: &WorkTimeSettings) {
    assert_eq!(w.days.iter().map(|r| r.worked_ms).sum::<i64>(), w.worked_ms);
    assert_eq!(w.days.iter().map(|r| r.norm_ms).sum::<i64>(), w.norm_ms);
    if w.phase != WeekPhase::Future {
        assert_eq!(
            w.days
                .iter()
                .map(|r| r.deviation_ms.unwrap_or(0))
                .sum::<i64>(),
            w.deviation_ms.unwrap()
        );
    }
    for r in &w.days {
        let month = report::month_report(store, d(&r.day), now, &Warsaw, settings).unwrap();
        let in_month = month.days.iter().find(|m| m.day == r.day).unwrap();
        assert_eq!(in_month, r, "{}", r.day);
    }
}

// ---- pure rules ----

#[test]
fn week_days_maps_every_weekday_to_its_monday() {
    for day in [
        "2026-11-23",
        "2026-11-24",
        "2026-11-25",
        "2026-11-26",
        "2026-11-27",
        "2026-11-28",
        "2026-11-29",
    ] {
        assert_eq!(
            week_days(d(day)),
            (d("2026-11-23"), d("2026-11-30")),
            "{day}"
        );
    }
}

#[test]
fn week_days_crosses_the_year() {
    assert_eq!(
        week_days(d("2027-01-01")),
        (d("2026-12-28"), d("2027-01-04"))
    );
}

#[test]
fn week_phase_is_relative_to_the_week_of_today() {
    let mon = d("2026-11-23");
    assert_eq!(week_phase(mon, d("2026-11-23")), WeekPhase::Current);
    assert_eq!(week_phase(mon, d("2026-11-29")), WeekPhase::Current);
    assert_eq!(week_phase(mon, d("2026-11-30")), WeekPhase::Past);
    assert_eq!(
        week_phase(d("2026-11-30"), d("2026-11-29")),
        WeekPhase::Future
    );
    assert_eq!(
        week_phase(d("2026-11-16"), d("2026-11-23")),
        WeekPhase::Past
    );
}

// ---- the week figures ----

#[test]
fn current_week_has_norm_to_date_and_remaining() {
    let (_tmp, mut store) = open();
    for (day, from, to) in [
        ("2026-11-23", "08:00", "16:00"),
        ("2026-11-24", "08:00", "14:00"),
        ("2026-11-25", "08:00", "17:00"),
        ("2026-11-26", "08:00", "10:00"),
    ] {
        add_entry(&mut store, &format!("{day}T{from}"), &format!("{day}T{to}"));
    }
    let now = t("2026-11-26T12:00");
    let w = week_at(&store, "2026-11-25", now, &WorkTimeSettings::default());
    let wd = workday_norm(&w);
    assert_eq!(w.phase, WeekPhase::Current);
    assert_eq!(w.monday, "2026-11-23");
    assert_eq!(w.sunday, "2026-11-29");
    assert_eq!(w.as_of_ms, now);
    assert_eq!(w.worked_ms, 8 * HOUR + 6 * HOUR + 9 * HOUR + 2 * HOUR);
    assert_eq!(w.norm_ms, 5 * wd);
    assert_eq!(w.norm_to_date_ms, Some(4 * wd));
    assert_eq!(w.deviation_ms, Some(w.worked_ms - 4 * wd));
    assert_eq!(w.remaining_ms, Some((5 * wd - w.worked_ms).max(0)));
    assert_eq!(row(&w, "2026-11-26").deviation_ms, Some(2 * HOUR - wd));
    assert_eq!(row(&w, "2026-11-27").deviation_ms, None);
    assert_eq!(row(&w, "2026-11-28").kind, DayKind::Weekend);
}

#[test]
fn remaining_is_zero_once_the_week_norm_is_worked() {
    let (_tmp, mut store) = open();
    for day in [
        "2026-11-23",
        "2026-11-24",
        "2026-11-25",
        "2026-11-26",
        "2026-11-27",
    ] {
        add_entry(&mut store, &format!("{day}T06:00"), &format!("{day}T18:00"));
    }
    let w = week_at(
        &store,
        "2026-11-23",
        t("2026-11-27T20:00"),
        &WorkTimeSettings::default(),
    );
    assert!(w.worked_ms > w.norm_ms);
    assert_eq!(w.remaining_ms, Some(0));
}

#[test]
fn week_across_a_month_boundary_equals_both_month_reports() {
    let (_tmp, mut store) = open();
    add_entry(&mut store, "2026-11-30T08:00", "2026-11-30T12:00");
    add_entry(&mut store, "2026-12-01T09:00", "2026-12-01T10:30");
    let w = week_at(
        &store,
        "2026-12-03",
        t("2026-12-03T12:00"),
        &WorkTimeSettings::default(),
    );
    assert_eq!(w.monday, "2026-11-30");
    assert_eq!(w.sunday, "2026-12-06");
    assert_eq!(workday_count(&w), 5);
    assert_eq!(w.norm_ms, 5 * workday_norm(&w));
    assert_eq!(w.worked_ms, 4 * HOUR + 90 * MIN);
}

#[test]
fn christmas_week_has_holidays_and_an_owed_saturday() {
    let (_tmp, store) = open();
    let w = week_at(
        &store,
        "2026-12-21",
        t("2026-12-28T12:00"),
        &WorkTimeSettings::default(),
    );
    assert_eq!(row(&w, "2026-12-24").kind, DayKind::Holiday);
    assert_eq!(row(&w, "2026-12-24").norm_ms, 0);
    assert_eq!(row(&w, "2026-12-25").kind, DayKind::Holiday);
    assert_eq!(row(&w, "2026-12-25").norm_ms, 0);
    assert_eq!(row(&w, "2026-12-26").kind, DayKind::Weekend);
    assert!(row(&w, "2026-12-26").day_off_owed);
    assert_eq!(workday_count(&w), 3);
    assert_eq!(w.norm_ms, 3 * workday_norm(&w));
}

#[test]
fn week_across_the_year_has_a_new_year_holiday() {
    let (_tmp, store) = open();
    let w = week_at(
        &store,
        "2027-01-01",
        t("2027-01-04T12:00"),
        &WorkTimeSettings::default(),
    );
    assert_eq!(w.monday, "2026-12-28");
    assert_eq!(w.sunday, "2027-01-03");
    assert_eq!(row(&w, "2027-01-01").kind, DayKind::Holiday);
    assert_eq!(workday_count(&w), 4);
    assert_eq!(w.norm_ms, 4 * workday_norm(&w));
}

#[test]
fn easter_monday_is_a_holiday() {
    let (_tmp, store) = open();
    let w = week_at(
        &store,
        "2026-04-08",
        t("2026-04-13T12:00"),
        &WorkTimeSettings::default(),
    );
    assert_eq!(row(&w, "2026-04-06").kind, DayKind::Holiday);
    assert_eq!(workday_count(&w), 4);
    assert_eq!(w.norm_ms, 4 * workday_norm(&w));
}

#[test]
fn dst_weeks_count_real_lengths() {
    let (_tmp, mut store) = open();
    add_entry(&mut store, "2026-03-29T00:00", "2026-03-29T06:00");
    add_entry(&mut store, "2026-10-25T00:00", "2026-10-25T06:00");
    let spring = week_at(
        &store,
        "2026-03-25",
        t("2026-04-01T12:00"),
        &WorkTimeSettings::default(),
    );
    assert_eq!(workday_count(&spring), 5);
    assert_eq!(row(&spring, "2026-03-29").worked_ms, 5 * HOUR);
    assert_eq!(spring.worked_ms, 5 * HOUR);
    let autumn = week_at(
        &store,
        "2026-10-21",
        t("2026-10-28T12:00"),
        &WorkTimeSettings::default(),
    );
    assert_eq!(workday_count(&autumn), 5);
    assert_eq!(row(&autumn, "2026-10-25").worked_ms, 7 * HOUR);
    assert_eq!(autumn.worked_ms, 7 * HOUR);
}

#[test]
fn a_manual_day_off_lowers_the_week_norm() {
    let (_tmp, mut store) = open();
    let settings = manual_only();
    mark_off(&mut store, "2026-11-25", &settings).unwrap();
    let w = week_at(&store, "2026-11-23", t("2026-11-27T12:00"), &settings);
    let wd = workday_norm(&w);
    assert_eq!(row(&w, "2026-11-25").kind, DayKind::DayOff);
    assert_eq!(row(&w, "2026-11-25").norm_ms, 0);
    assert_eq!(w.norm_ms, 4 * wd);
}

#[test]
fn a_running_entry_counts_in_the_week_it_runs_in() {
    let (_tmp, mut store) = open();
    start(&mut store, t("2026-11-22T22:00"), &Warsaw).unwrap();
    let now = t("2026-11-23T01:00");
    let settings = WorkTimeSettings::default();
    let this = week_at(&store, "2026-11-23", now, &settings);
    assert_eq!(row(&this, "2026-11-23").worked_ms, HOUR);
    assert_eq!(this.worked_ms, HOUR);
    let before = week_at(&store, "2026-11-16", now, &settings);
    assert_eq!(row(&before, "2026-11-22").worked_ms, 2 * HOUR);
    assert_eq!(before.worked_ms, 2 * HOUR);
}

#[test]
fn an_entry_across_midnight_counts_in_both_days_and_weeks() {
    let (_tmp, mut store) = open();
    add_entry(&mut store, "2026-11-29T22:00", "2026-11-30T02:00");
    add_entry(&mut store, "2026-11-24T22:00", "2026-11-25T02:00");
    let now = t("2026-12-05T12:00");
    let settings = WorkTimeSettings::default();
    let first = week_at(&store, "2026-11-23", now, &settings);
    let second = week_at(&store, "2026-11-30", now, &settings);
    assert_eq!(row(&first, "2026-11-24").worked_ms, 2 * HOUR);
    assert_eq!(row(&first, "2026-11-25").worked_ms, 2 * HOUR);
    assert_eq!(row(&first, "2026-11-24").entry_count, 1);
    assert_eq!(row(&first, "2026-11-25").entry_count, 1);
    assert_eq!(row(&first, "2026-11-29").worked_ms, 2 * HOUR);
    assert_eq!(first.worked_ms, 6 * HOUR);
    assert_eq!(row(&second, "2026-11-30").worked_ms, 2 * HOUR);
    assert_eq!(second.worked_ms, 2 * HOUR);
}

#[test]
fn future_and_past_weeks_have_their_own_shape() {
    let (_tmp, mut store) = open();
    add_entry(&mut store, "2026-11-17T08:00", "2026-11-17T12:00");
    let now = t("2026-11-26T12:00");
    let settings = WorkTimeSettings::default();

    let future = week_at(&store, "2026-12-07", now, &settings);
    assert_eq!(future.phase, WeekPhase::Future);
    assert_eq!(future.worked_ms, 0);
    assert_eq!(future.norm_to_date_ms, None);
    assert_eq!(future.deviation_ms, None);
    assert_eq!(future.remaining_ms, None);
    assert!(future.days.iter().all(|r| r.deviation_ms.is_none()));

    let past = week_at(&store, "2026-11-16", now, &settings);
    assert_eq!(past.phase, WeekPhase::Past);
    assert_eq!(past.norm_to_date_ms, Some(past.norm_ms));
    assert_eq!(past.deviation_ms, Some(4 * HOUR - past.norm_ms));
    assert_eq!(past.remaining_ms, None);
}

#[test]
fn week_report_serializes_the_contract_shape() {
    let (_tmp, store) = open();
    let now = t("2026-11-26T12:00");
    let settings = WorkTimeSettings::default();

    let current = serde_json::to_value(week_at(&store, "2026-11-23", now, &settings)).unwrap();
    let mut keys: Vec<&str> = current
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    let mut expected = [
        "monday",
        "sunday",
        "phase",
        "asOfMs",
        "workedMs",
        "normMs",
        "normToDateMs",
        "deviationMs",
        "remainingMs",
        "days",
    ];
    expected.sort_unstable();
    assert_eq!(keys, expected);
    assert_eq!(current["phase"], "current");
    assert_eq!(current["days"].as_array().unwrap().len(), 7);

    let future = serde_json::to_value(week_at(&store, "2026-12-07", now, &settings)).unwrap();
    assert_eq!(future["phase"], "future");
    assert!(future["normToDateMs"].is_null());
    assert!(future["deviationMs"].is_null());
    assert!(future["remainingMs"].is_null());

    let past = serde_json::to_value(week_at(&store, "2026-11-16", now, &settings)).unwrap();
    assert_eq!(past["phase"], "past");
    assert!(past["remainingMs"].is_null());
}

proptest::proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig::with_cases(24))]

    #[test]
    fn week_rows_add_up_and_equal_the_month_rows(
        cuts in proptest::collection::vec(0i64..(12 * 24 * 60), 0..12),
        off_flags in proptest::collection::vec(proptest::prelude::any::<bool>(), 14),
        with_holidays in proptest::prelude::any::<bool>(),
    ) {
        let settings = if with_holidays {
            WorkTimeSettings::default()
        } else {
            manual_only()
        };
        let (_tmp, mut store) = open();
        let base = t("2026-11-23T00:00");
        let mut cuts = cuts;
        cuts.sort_unstable();
        cuts.dedup();
        let pairs: Vec<(i64, i64)> = cuts
            .windows(2).step_by(2)
            .map(|c| (base + c[0] * MIN, base + c[1] * MIN))
            .collect();
        store
            .write::<_, WorkTimeError>(|tx| {
                for (s, e) in &pairs {
                    tx.execute(
                        "INSERT INTO work_entries(start_ms, end_ms, created_at_ms, updated_at_ms) \
                         VALUES (?1, ?2, ?3, ?3)",
                        (s, e, base),
                    )?;
                }
                Ok(())
            })
            .unwrap();
        for (i, flag) in off_flags.iter().enumerate() {
            if *flag {
                let day = (d("2026-11-23") + chrono::Days::new(i as u64)).to_string();
                // A weekend or a holiday refuses; that day is simply not marked.
                mark_off(&mut store, &day, &settings).ok();
            }
        }
        let now = t("2026-12-05T12:00");
        // `week_at` checks the sums and every row against the month report.
        let first = week_at(&store, "2026-11-23", now, &settings);
        let second = week_at(&store, "2026-11-30", now, &settings);
        proptest::prop_assert_eq!(first.phase, WeekPhase::Past);
        proptest::prop_assert_eq!(second.phase, WeekPhase::Current);
    }
}
