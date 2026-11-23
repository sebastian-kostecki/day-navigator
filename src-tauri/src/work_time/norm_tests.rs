//! Norm and month-report tests on invented months in `Europe/Warsaw`.

use std::collections::BTreeSet;

use chrono::NaiveDate;
use chrono_tz::Europe::Warsaw;
use tempfile::TempDir;

use super::norm::{self, DAILY_NORM_MS, MonthPhase};
use super::report::{self, MonthReport};
use super::rules::resolve_local;
use super::*;
use crate::store::schema;

const MIN: i64 = 60_000;
const HOUR: i64 = 60 * MIN;

fn t(local: &str) -> i64 {
    resolve_local(local, &Warsaw).unwrap()
}

fn d(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

/// Monday 2026-11-23, noon local.
fn now() -> i64 {
    t("2026-11-23T12:00")
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
    add(store, from, to, t("2027-12-31T12:00"), &Warsaw).unwrap();
}

/// 8 h (08:00-16:00) on each given day.
fn work_days(store: &mut Store, days: &[&str]) {
    for day in days {
        add_entry(store, &format!("{day}T08:00"), &format!("{day}T16:00"));
    }
}

/// Fourteen November workdays before Monday 23rd (day off 11th not worked).
const NOV_DAYS: [&str; 14] = [
    "2026-11-02",
    "2026-11-03",
    "2026-11-04",
    "2026-11-05",
    "2026-11-06",
    "2026-11-09",
    "2026-11-10",
    "2026-11-12",
    "2026-11-13",
    "2026-11-16",
    "2026-11-17",
    "2026-11-18",
    "2026-11-19",
    "2026-11-20",
];

/// The worked example: day off 2026-11-11, 14 full days plus 4 h today = 116 h.
fn worked_example(store: &mut Store) {
    report::set_day_off(store, d("2026-11-11"), true, now()).unwrap();
    work_days(store, &NOV_DAYS);
    add_entry(store, "2026-11-23T08:00", "2026-11-23T12:00");
}

fn report_at(store: &Store, day: &str, now: i64) -> MonthReport {
    report::month_report(store, d(day), now, &Warsaw).unwrap()
}

fn day_off_rows(store: &Store) -> i64 {
    store
        .read::<_, WorkTimeError>(|c| {
            Ok(c.query_row("SELECT count(*) FROM days_off", [], |r| r.get(0))?)
        })
        .unwrap()
}

// ---- pure rules ----

#[test]
fn november_2026_workdays_with_and_without_a_day_off() {
    let (first, next) = norm::month_days(d("2026-11-17"));
    assert_eq!((first, next), (d("2026-11-01"), d("2026-12-01")));
    let none = BTreeSet::new();
    let w = norm::workdays(first, next, &none);
    assert_eq!(w.len(), 21);
    assert_eq!(DAILY_NORM_MS * 21, 168 * HOUR);
    let off = BTreeSet::from([d("2026-11-11")]);
    let w = norm::workdays(first, next, &off);
    assert_eq!(w.len(), 20);
    assert_eq!(DAILY_NORM_MS * 20, 160 * HOUR);
}

#[test]
fn month_days_wraps_the_year() {
    assert_eq!(
        norm::month_days(d("2026-12-31")),
        (d("2026-12-01"), d("2027-01-01"))
    );
}

#[test]
fn weekends_are_never_workdays_even_when_listed() {
    let (first, next) = norm::month_days(d("2026-11-01"));
    // 2026-11-01 is a Sunday, 2026-11-07 a Saturday.
    let listed = BTreeSet::from([d("2026-11-01"), d("2026-11-07")]);
    assert_eq!(norm::workdays(first, next, &listed).len(), 21);
    assert!(norm::is_weekend(d("2026-11-01")));
    assert!(norm::is_weekend(d("2026-11-07")));
    assert!(!norm::is_weekend(d("2026-11-09")));
}

#[test]
fn phase_is_relative_to_today() {
    let today = d("2026-11-23");
    assert_eq!(norm::phase(d("2026-10-01"), today), MonthPhase::Past);
    assert_eq!(norm::phase(d("2026-11-01"), today), MonthPhase::Current);
    assert_eq!(norm::phase(d("2026-12-01"), today), MonthPhase::Future);
    assert_eq!(norm::phase(d("2025-12-01"), today), MonthPhase::Past);
    assert_eq!(norm::phase(d("2027-01-01"), today), MonthPhase::Future);
}

#[test]
fn rounding_goes_up_to_the_minute() {
    assert_eq!(norm::ceil_to_minute(0), 0);
    assert_eq!(norm::ceil_to_minute(1), MIN);
    assert_eq!(norm::ceil_to_minute(MIN), MIN);
    assert_eq!(norm::ceil_to_minute(MIN + 1), 2 * MIN);
    // 43:59 over 5 days is 8:47.8 -> 8:48.
    let workdays: Vec<NaiveDate> = ["2026-11-23", "2026-11-24", "2026-11-25", "2026-11-26"]
        .into_iter()
        .chain(["2026-11-27", "2026-11-30"])
        .map(d)
        .collect();
    // Norm 6 days = 48 h; worked 4:01 -> remaining 43:59, 5 days after today.
    let f = norm::figures(
        &workdays,
        d("2026-11-23"),
        MonthPhase::Current,
        4 * HOUR + MIN,
    );
    assert_eq!(f.workdays_after_today, Some(5));
    assert_eq!(f.remaining_ms, Some(43 * HOUR + 59 * MIN));
    assert_eq!(f.needed_per_day_ms, Some(8 * HOUR + 48 * MIN));
}

// ---- store-backed report ----

#[test]
fn worked_example_gives_minus_four_hours_and_8_48_over_five_days() {
    let (_tmp, mut store) = open();
    worked_example(&mut store);
    let r = report_at(&store, "2026-11-23", now());
    assert_eq!(r.month, "2026-11");
    assert_eq!(r.phase, MonthPhase::Current);
    assert_eq!(r.as_of_ms, now());
    assert_eq!(r.worked_ms, 116 * HOUR);
    assert_eq!(r.figures.workdays, 20);
    assert_eq!(r.figures.norm_ms, 160 * HOUR);
    assert_eq!(r.figures.norm_to_date_ms, Some(120 * HOUR));
    assert_eq!(r.figures.deviation_ms, Some(-4 * HOUR));
    assert_eq!(r.figures.workdays_after_today, Some(5));
    assert_eq!(r.figures.remaining_ms, Some(44 * HOUR));
    assert_eq!(r.figures.needed_per_day_ms, Some(8 * HOUR + 48 * MIN));
    assert_eq!(r.days_off, vec!["2026-11-11".to_string()]);
    // Any day of the month gives the same report.
    assert_eq!(report_at(&store, "2026-11-02", now()), r);
}

#[test]
fn last_workday_shows_the_remaining_shortfall_and_no_needed_figure() {
    let (_tmp, mut store) = open();
    worked_example(&mut store);
    // Monday 2026-11-30 is the last workday: today counts in full, none after.
    let r = report_at(&store, "2026-11-30", t("2026-11-30T12:00"));
    assert_eq!(r.figures.workdays_after_today, Some(0));
    assert_eq!(r.figures.needed_per_day_ms, None);
    assert_eq!(r.figures.norm_to_date_ms, Some(160 * HOUR));
    assert_eq!(r.figures.remaining_ms, Some(44 * HOUR));
    assert_eq!(r.figures.deviation_ms, Some(-44 * HOUR));
}

#[test]
fn norm_met_has_zero_remaining_and_zero_needed() {
    let (_tmp, mut store) = open();
    report::set_day_off(&mut store, d("2026-11-11"), true, now()).unwrap();
    // Seven 23 h days = 161 h >= 160 h.
    for day in 2..=8 {
        add_entry(
            &mut store,
            &format!("2026-11-{day:02}T00:00"),
            &format!("2026-11-{day:02}T23:00"),
        );
    }
    let r = report_at(&store, "2026-11-23", now());
    assert_eq!(r.worked_ms, 161 * HOUR);
    assert_eq!(r.figures.remaining_ms, Some(0));
    assert_eq!(r.figures.needed_per_day_ms, Some(0));
    assert_eq!(r.figures.deviation_ms, Some(41 * HOUR));
}

#[test]
fn marking_today_off_drops_it_from_the_norm_to_date() {
    let (_tmp, mut store) = open();
    worked_example(&mut store);
    report::set_day_off(&mut store, d("2026-11-23"), true, now()).unwrap();
    let r = report_at(&store, "2026-11-23", now());
    assert_eq!(r.figures.workdays, 19);
    assert_eq!(r.figures.norm_to_date_ms, Some(112 * HOUR));
    assert_eq!(r.figures.deviation_ms, Some(4 * HOUR));
    assert_eq!(r.figures.workdays_after_today, Some(5));
}

#[test]
fn weekend_today_counts_only_the_days_before_it() {
    let (_tmp, store) = open();
    // Saturday 2026-11-28, no days off: 20 workdays up to Friday 27th.
    let r = report_at(&store, "2026-11-28", t("2026-11-28T12:00"));
    assert_eq!(r.figures.norm_to_date_ms, Some(160 * HOUR));
    assert_eq!(r.figures.workdays_after_today, Some(1));
    assert_eq!(r.figures.deviation_ms, Some(-160 * HOUR));
    assert_eq!(r.figures.needed_per_day_ms, Some(168 * HOUR));
}

#[test]
fn hours_on_a_day_off_and_on_a_saturday_still_count() {
    let (_tmp, mut store) = open();
    report::set_day_off(&mut store, d("2026-11-11"), true, now()).unwrap();
    work_days(&mut store, &["2026-11-11", "2026-11-07"]);
    let r = report_at(&store, "2026-11-23", now());
    assert_eq!(r.worked_ms, 16 * HOUR);
    assert_eq!(r.figures.deviation_ms, Some(16 * HOUR - 120 * HOUR));
}

#[test]
fn an_entry_across_the_month_boundary_counts_in_both_months() {
    let (_tmp, mut store) = open();
    add_entry(&mut store, "2026-10-31T22:00", "2026-11-01T02:00");
    assert_eq!(report_at(&store, "2026-11-10", now()).worked_ms, 2 * HOUR);
    assert_eq!(report_at(&store, "2026-10-10", now()).worked_ms, 2 * HOUR);
}

#[test]
fn dst_months_have_their_real_lengths() {
    let (_tmp, mut store) = open();
    // October 2026 ends DST (2026-10-25): 31 days + 1 h.
    add_entry(&mut store, "2026-10-01T00:00", "2026-11-01T00:00");
    let oct = report_at(&store, "2026-10-15", now());
    assert_eq!(oct.worked_ms, 31 * 24 * HOUR + HOUR);
    // March 2027 starts DST (2027-03-28): 31 days - 1 h.
    add_entry(&mut store, "2027-03-01T00:00", "2027-04-01T00:00");
    let mar = report_at(&store, "2027-03-15", t("2027-04-10T12:00"));
    assert_eq!(mar.worked_ms, 31 * 24 * HOUR - HOUR);
}

#[test]
fn a_running_entry_from_the_previous_month_counts_only_its_part_here() {
    let (_tmp, mut store) = open();
    start(&mut store, t("2026-10-31T22:00")).unwrap();
    let at = t("2026-11-02T06:00");
    let nov = report_at(&store, "2026-11-02", at);
    // 2026-11-01 00:00 to 2026-11-02 06:00.
    assert_eq!(nov.worked_ms, 30 * HOUR);
    let oct = report_at(&store, "2026-10-31", at);
    assert_eq!(oct.worked_ms, 2 * HOUR);
    // A month that has not begun yet gets nothing from it.
    assert_eq!(report_at(&store, "2026-12-01", at).worked_ms, 0);
}

#[test]
fn a_past_month_has_a_final_deviation_and_a_future_month_only_the_norm() {
    let (_tmp, mut store) = open();
    add_entry(&mut store, "2026-10-05T08:00", "2026-10-05T16:00");
    let oct = report_at(&store, "2026-10-20", now());
    assert_eq!(oct.phase, MonthPhase::Past);
    assert_eq!(oct.figures.workdays, 22);
    assert_eq!(oct.figures.norm_ms, 176 * HOUR);
    assert_eq!(oct.figures.deviation_ms, Some(8 * HOUR - 176 * HOUR));
    assert_eq!(oct.figures.workdays_after_today, None);
    assert_eq!(oct.figures.needed_per_day_ms, None);
    assert_eq!(oct.figures.remaining_ms, None);

    let dec = report_at(&store, "2026-12-20", now());
    assert_eq!(dec.phase, MonthPhase::Future);
    assert_eq!(dec.figures.workdays, 23);
    assert_eq!(dec.figures.norm_ms, 184 * HOUR);
    assert_eq!(dec.figures.norm_to_date_ms, None);
    assert_eq!(dec.figures.deviation_ms, None);
    assert_eq!(dec.figures.workdays_after_today, None);
    assert_eq!(dec.figures.needed_per_day_ms, None);
    assert_eq!(dec.figures.remaining_ms, None);
    assert_eq!(dec.worked_ms, 0);
}

#[test]
fn status_month_deviation_equals_the_report_deviation() {
    let (_tmp, mut store) = open();
    worked_example(&mut store);
    let s = status(&store, now(), &Warsaw).unwrap();
    let r = report_at(&store, "2026-11-23", now());
    assert_eq!(Some(s.month_deviation_ms), r.figures.deviation_ms);
    assert_eq!(s.month_deviation_ms, -4 * HOUR);
    assert_eq!(s.as_of_ms, now());

    // With a running entry the two still come from the same instant.
    start(&mut store, t("2026-11-23T13:00")).unwrap();
    let later = t("2026-11-23T15:30");
    let s = status(&store, later, &Warsaw).unwrap();
    let r = report_at(&store, "2026-11-23", later);
    assert_eq!(Some(s.month_deviation_ms), r.figures.deviation_ms);
    assert_eq!(s.month_deviation_ms, -4 * HOUR + 2 * HOUR + 30 * MIN);
}

#[test]
fn set_day_off_is_idempotent_and_refuses_a_weekend_without_a_row() {
    let (_tmp, mut store) = open();
    let day = d("2026-11-11");
    report::set_day_off(&mut store, day, true, now()).unwrap();
    report::set_day_off(&mut store, day, true, now() + HOUR).unwrap();
    assert_eq!(day_off_rows(&store), 1);
    // The first mark is kept.
    let created: i64 = store
        .read::<_, WorkTimeError>(|c| {
            Ok(c.query_row("SELECT created_at_ms FROM days_off", [], |r| r.get(0))?)
        })
        .unwrap();
    assert_eq!(created, now());
    assert!(!report::is_workday(&store, day).unwrap());
    report::set_day_off(&mut store, day, false, now()).unwrap();
    report::set_day_off(&mut store, day, false, now()).unwrap();
    assert_eq!(day_off_rows(&store), 0);
    assert!(report::is_workday(&store, day).unwrap());

    for weekend in ["2026-11-14", "2026-11-15"] {
        for off in [true, false] {
            let r = report::set_day_off(&mut store, d(weekend), off, now());
            assert!(matches!(r, Err(WorkTimeError::NotAWorkday)), "{weekend}");
        }
        assert!(!report::is_workday(&store, d(weekend)).unwrap());
    }
    assert_eq!(day_off_rows(&store), 0);
}

#[test]
fn parse_day_accepts_only_the_canonical_form() {
    assert_eq!(report::parse_day("2026-11-11").unwrap(), d("2026-11-11"));
    for bad in [
        "+10000-01-03",
        "10000-01-03",
        "2026-1-05",
        "2026-02-30",
        "2026-11-11 ",
        "",
    ] {
        let r = report::parse_day(bad);
        assert!(matches!(r, Err(WorkTimeError::InvalidLocalTime)), "{bad:?}");
    }
}

#[test]
fn a_day_off_key_the_check_refuses_fails_instead_of_being_ignored() {
    let (_tmp, mut store) = open();
    // Monday 10000-01-03 formats as `+10000-01-03`, which the GLOB CHECK refuses.
    let far = NaiveDate::from_ymd_opt(10000, 1, 3).unwrap();
    assert!(!norm::is_weekend(far));
    let r = report::set_day_off(&mut store, far, true, now());
    assert!(matches!(r, Err(WorkTimeError::Store(_))), "{r:?}");
    assert_eq!(day_off_rows(&store), 0);
}

#[test]
fn day_entries_report_all_three_day_kinds() {
    let (_tmp, mut store) = open();
    report::set_day_off(&mut store, d("2026-11-11"), true, now()).unwrap();
    let kind = |day: &str| {
        entries_for_day(&store, d(day), now(), &Warsaw)
            .unwrap()
            .day_kind
    };
    assert_eq!(kind("2026-11-10"), DayKind::Workday);
    assert_eq!(kind("2026-11-11"), DayKind::DayOff);
    assert_eq!(kind("2026-11-14"), DayKind::Weekend);
}
