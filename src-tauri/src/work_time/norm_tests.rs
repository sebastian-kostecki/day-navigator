//! Norm and month-report tests on invented months in `Europe/Warsaw`.

use std::collections::BTreeSet;

use chrono::NaiveDate;
use chrono_tz::Europe::Warsaw;
use tempfile::TempDir;

use super::holidays::HolidayCalendar;
use super::norm::{self, DAILY_NORM_MS, MonthPhase};
use super::report::{self, MonthReport};
use super::rules::resolve_local;
use super::*;
use crate::settings::WorkTimeSettings;
use crate::store::schema;

const MIN: i64 = 60_000;
const HOUR: i64 = 60 * MIN;

/// Settings with the holiday calendar off: public holidays are ordinary
/// weekdays, so a test can mark them as manual days off.
pub(super) fn manual_only() -> WorkTimeSettings {
    WorkTimeSettings {
        holidays: HolidayCalendar::Off,
    }
}

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
    add(store, from, to, "", t("2027-12-31T12:00"), &Warsaw).unwrap();
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
    report::set_day_off(store, d("2026-11-11"), true, now(), &manual_only()).unwrap();
    work_days(store, &NOV_DAYS);
    add_entry(store, "2026-11-23T08:00", "2026-11-23T12:00");
}

fn report_at(store: &Store, day: &str, now: i64) -> MonthReport {
    report::month_report(store, d(day), now, &Warsaw, &manual_only()).unwrap()
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
    let w = norm::workdays(first, next, &none, HolidayCalendar::Off);
    assert_eq!(w.len(), 21);
    assert_eq!(DAILY_NORM_MS * 21, 168 * HOUR);
    let off = BTreeSet::from([d("2026-11-11")]);
    let w = norm::workdays(first, next, &off, HolidayCalendar::Off);
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
    assert_eq!(
        norm::workdays(first, next, &listed, HolidayCalendar::Off).len(),
        21
    );
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
    report::set_day_off(&mut store, d("2026-11-11"), true, now(), &manual_only()).unwrap();
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
    report::set_day_off(&mut store, d("2026-11-23"), true, now(), &manual_only()).unwrap();
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
    report::set_day_off(&mut store, d("2026-11-11"), true, now(), &manual_only()).unwrap();
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
    start(&mut store, t("2026-10-31T22:00"), &Warsaw).unwrap();
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
    let s = status(&store, now(), &Warsaw, &manual_only()).unwrap();
    let r = report_at(&store, "2026-11-23", now());
    assert_eq!(Some(s.month_deviation_ms), r.figures.deviation_ms);
    assert_eq!(s.month_deviation_ms, -4 * HOUR);
    assert_eq!(s.as_of_ms, now());

    // With a running entry the two still come from the same instant.
    start(&mut store, t("2026-11-23T13:00"), &Warsaw).unwrap();
    let later = t("2026-11-23T15:30");
    let s = status(&store, later, &Warsaw, &manual_only()).unwrap();
    let r = report_at(&store, "2026-11-23", later);
    assert_eq!(Some(s.month_deviation_ms), r.figures.deviation_ms);
    assert_eq!(s.month_deviation_ms, -4 * HOUR + 2 * HOUR + 30 * MIN);
}

#[test]
fn set_day_off_is_idempotent_and_refuses_a_weekend_without_a_row() {
    let (_tmp, mut store) = open();
    let day = d("2026-11-11");
    report::set_day_off(&mut store, day, true, now(), &manual_only()).unwrap();
    report::set_day_off(&mut store, day, true, now() + HOUR, &manual_only()).unwrap();
    assert_eq!(day_off_rows(&store), 1);
    // The first mark is kept.
    let created: i64 = store
        .read::<_, WorkTimeError>(|c| {
            Ok(c.query_row("SELECT created_at_ms FROM days_off", [], |r| r.get(0))?)
        })
        .unwrap();
    assert_eq!(created, now());
    assert!(!report::is_workday(&store, day, &manual_only()).unwrap());
    report::set_day_off(&mut store, day, false, now(), &manual_only()).unwrap();
    report::set_day_off(&mut store, day, false, now(), &manual_only()).unwrap();
    assert_eq!(day_off_rows(&store), 0);
    assert!(report::is_workday(&store, day, &manual_only()).unwrap());

    for weekend in ["2026-11-14", "2026-11-15"] {
        for off in [true, false] {
            let r = report::set_day_off(&mut store, d(weekend), off, now(), &manual_only());
            assert!(matches!(r, Err(WorkTimeError::NotAWorkday)), "{weekend}");
        }
        assert!(!report::is_workday(&store, d(weekend), &manual_only()).unwrap());
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
    let r = report::set_day_off(&mut store, far, true, now(), &manual_only());
    assert!(matches!(r, Err(WorkTimeError::Store(_))), "{r:?}");
    assert_eq!(day_off_rows(&store), 0);
}

#[test]
fn day_entries_report_all_three_day_kinds() {
    let (_tmp, mut store) = open();
    report::set_day_off(&mut store, d("2026-11-11"), true, now(), &manual_only()).unwrap();
    let kind = |day: &str| {
        entries_for_day(&store, d(day), now(), &Warsaw, &manual_only())
            .unwrap()
            .day_kind
    };
    assert_eq!(kind("2026-11-10"), DayKind::Workday);
    assert_eq!(kind("2026-11-11"), DayKind::DayOff);
    assert_eq!(kind("2026-11-14"), DayKind::Weekend);
}

// ---- day rows ----

fn day_row<'a>(r: &'a MonthReport, day: &str) -> &'a report::DayReport {
    r.days.iter().find(|x| x.day == day).unwrap()
}

#[test]
fn day_norm_is_eight_hours_on_a_workday_and_zero_otherwise() {
    let off = BTreeSet::from([d("2026-11-11"), d("2026-11-14")]);
    assert_eq!(
        norm::day_norm_ms(d("2026-11-10"), &off, HolidayCalendar::Off),
        DAILY_NORM_MS
    );
    assert_eq!(
        norm::day_norm_ms(d("2026-11-11"), &off, HolidayCalendar::Off),
        0
    );
    assert_eq!(
        norm::day_norm_ms(d("2026-11-15"), &off, HolidayCalendar::Off),
        0
    );
    // A Saturday listed in days_off is still 0.
    assert_eq!(
        norm::day_norm_ms(d("2026-11-14"), &off, HolidayCalendar::Off),
        0
    );
}

#[test]
fn day_deviation_follows_the_month_rule() {
    let today = d("2026-11-23");
    let dev =
        |day: &str, norm: i64, worked: i64| norm::day_deviation_ms(d(day), today, norm, worked);
    assert_eq!(dev("2026-11-24", DAILY_NORM_MS, 0), None);
    assert_eq!(dev("2026-11-23", DAILY_NORM_MS, 4 * HOUR), Some(-4 * HOUR));
    assert_eq!(dev("2026-11-20", DAILY_NORM_MS, 0), Some(-8 * HOUR));
    assert_eq!(dev("2026-11-21", 0, 0), None);
    assert_eq!(dev("2026-11-21", 0, 2 * HOUR), Some(2 * HOUR));
    assert_eq!(dev("2026-11-11", 0, 0), None);
    assert_eq!(dev("2026-11-11", 0, 3 * HOUR), Some(3 * HOUR));
}

#[test]
fn worked_example_days_add_up_to_the_month() {
    let (_tmp, mut store) = open();
    worked_example(&mut store);
    let r = report_at(&store, "2026-11-23", now());
    assert_eq!(r.days.len(), 30);
    let off = day_row(&r, "2026-11-11");
    assert_eq!(off.kind, DayKind::DayOff);
    assert_eq!(off.norm_ms, 0);
    assert_eq!(off.deviation_ms, None);
    let today = day_row(&r, "2026-11-23");
    assert_eq!(today.worked_ms, 4 * HOUR);
    assert_eq!(today.deviation_ms, Some(-4 * HOUR));
    let next = day_row(&r, "2026-11-24");
    assert_eq!(next.deviation_ms, None);
    assert_eq!(next.worked_ms, 0);
    assert_eq!(r.days.iter().map(|x| x.worked_ms).sum::<i64>(), r.worked_ms);
    assert_eq!(
        r.days.iter().filter_map(|x| x.deviation_ms).sum::<i64>(),
        -4 * HOUR
    );
    assert_eq!(r.figures.deviation_ms, Some(-4 * HOUR));
    assert_eq!(
        r.days.iter().map(|x| x.norm_ms).sum::<i64>(),
        r.figures.norm_ms
    );
}

#[test]
fn an_entry_across_midnight_counts_in_both_days() {
    let (_tmp, mut store) = open();
    add_entry(&mut store, "2026-11-09T22:00", "2026-11-10T02:00");
    let r = report_at(&store, "2026-11-23", now());
    for day in ["2026-11-09", "2026-11-10"] {
        let row = day_row(&r, day);
        assert_eq!(row.worked_ms, 2 * HOUR, "{day}");
        assert_eq!(row.entry_count, 1, "{day}");
    }
    assert_eq!(day_row(&r, "2026-11-11").entry_count, 0);
}

#[test]
fn an_entry_across_the_month_boundary_counts_only_its_part_per_month() {
    let (_tmp, mut store) = open();
    add_entry(&mut store, "2026-11-30T22:00", "2026-12-01T02:00");
    let nov = report_at(&store, "2026-11-10", t("2026-12-10T12:00"));
    assert_eq!(day_row(&nov, "2026-11-30").worked_ms, 2 * HOUR);
    assert_eq!(nov.worked_ms, 2 * HOUR);
    let dec = report_at(&store, "2026-12-10", t("2026-12-10T12:00"));
    assert_eq!(dec.days.len(), 31);
    assert_eq!(day_row(&dec, "2026-12-01").worked_ms, 2 * HOUR);
    assert_eq!(dec.worked_ms, 2 * HOUR);
}

#[test]
fn dst_days_sum_their_real_lengths() {
    let (_tmp, mut store) = open();
    // 2026-10-25 is the fall-back day: 00:00-06:00 local lasts 7 h.
    add_entry(&mut store, "2026-10-25T00:00", "2026-10-25T06:00");
    let r = report_at(&store, "2026-10-15", now());
    assert_eq!(r.days.len(), 31);
    assert_eq!(day_row(&r, "2026-10-25").worked_ms, 7 * HOUR);
    assert_eq!(r.days.iter().map(|x| x.worked_ms).sum::<i64>(), r.worked_ms);
}

#[test]
fn a_running_entry_counts_up_to_now_today_and_its_part_yesterday() {
    let (_tmp, mut store) = open();
    start(&mut store, t("2026-11-22T22:00"), &Warsaw).unwrap();
    let at = t("2026-11-23T01:00");
    let r = report_at(&store, "2026-11-23", at);
    let yesterday = day_row(&r, "2026-11-22");
    let today = day_row(&r, "2026-11-23");
    assert_eq!((yesterday.worked_ms, yesterday.entry_count), (2 * HOUR, 1));
    assert_eq!((today.worked_ms, today.entry_count), (HOUR, 1));
}

#[test]
fn a_past_month_has_every_day_deviation_and_a_future_month_none() {
    let (_tmp, mut store) = open();
    // Saturday 2026-10-03, 2 h.
    add_entry(&mut store, "2026-10-03T10:00", "2026-10-03T12:00");
    let oct = report_at(&store, "2026-10-20", now());
    assert_eq!(oct.days.len(), 31);
    for row in &oct.days {
        assert_eq!(
            row.deviation_ms.is_some(),
            row.norm_ms > 0 || row.worked_ms > 0,
            "{}",
            row.day
        );
    }
    assert_eq!(day_row(&oct, "2026-10-03").deviation_ms, Some(2 * HOUR));
    assert_eq!(
        oct.days.iter().filter_map(|x| x.deviation_ms).sum::<i64>(),
        oct.figures.deviation_ms.unwrap()
    );

    let dec = report_at(&store, "2026-12-20", now());
    assert_eq!(dec.days.len(), 31);
    for row in &dec.days {
        assert_eq!(row.deviation_ms, None, "{}", row.day);
        assert_eq!(row.worked_ms, 0);
        let expected = if row.kind == DayKind::Workday {
            DAILY_NORM_MS
        } else {
            0
        };
        assert_eq!(row.norm_ms, expected, "{}", row.day);
    }
}

proptest::proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig::with_cases(24))]

    #[test]
    fn day_rows_add_up_to_the_month_figures(
        cuts in proptest::collection::vec(0i64..(22 * 24 * 60), 0..12),
        off_flags in proptest::collection::vec(proptest::prelude::any::<bool>(), 30),
        with_holidays in proptest::prelude::any::<bool>(),
    ) {
        let settings = if with_holidays {
            WorkTimeSettings::default()
        } else {
            manual_only()
        };
        let (_tmp, mut store) = open();
        let base = t("2026-11-01T00:00");
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
                        (s, e, now()),
                    )?;
                }
                Ok(())
            })
            .unwrap();
        for (i, flag) in off_flags.iter().enumerate() {
            let day = d("2026-11-01") + chrono::Days::new(i as u64);
            // Under the default calendar a holiday cannot be marked.
            if *flag && !norm::is_weekend(day) && settings.holidays.holiday(day).is_none() {
                report::set_day_off(&mut store, day, true, now(), &settings).unwrap();
            }
        }
        let r = report::month_report(&store, d("2026-11-23"), now(), &Warsaw, &settings).unwrap();
        proptest::prop_assert_eq!(r.days.len(), 30);
        proptest::prop_assert_eq!(r.days.iter().map(|x| x.worked_ms).sum::<i64>(), r.worked_ms);
        proptest::prop_assert_eq!(
            r.days.iter().map(|x| x.deviation_ms.unwrap_or(0)).sum::<i64>(),
            r.figures.deviation_ms.unwrap()
        );
        proptest::prop_assert_eq!(r.days.iter().map(|x| x.norm_ms).sum::<i64>(), r.figures.norm_ms);
    }
}

// ---- public holidays (default calendar) ----

fn report_pl(store: &Store, day: &str) -> MonthReport {
    report::month_report(store, d(day), now(), &Warsaw, &WorkTimeSettings::default()).unwrap()
}

#[test]
fn pure_norm_counts_holidays_out() {
    let none = BTreeSet::new();
    let (first, next) = norm::month_days(d("2026-11-01"));
    assert_eq!(
        norm::workdays(first, next, &none, HolidayCalendar::Pl).len(),
        20
    );
    assert_eq!(
        norm::workdays(first, next, &none, HolidayCalendar::Off).len(),
        21
    );
    let (first, next) = norm::month_days(d("2026-12-01"));
    assert_eq!(
        norm::workdays(first, next, &none, HolidayCalendar::Pl).len(),
        21
    );
    assert_eq!(
        norm::day_norm_ms(d("2026-11-11"), &none, HolidayCalendar::Pl),
        0
    );
    assert_eq!(
        norm::day_norm_ms(d("2026-11-11"), &none, HolidayCalendar::Off),
        DAILY_NORM_MS
    );
}

#[test]
fn november_2026_has_a_160_hour_norm_and_december_168() {
    let (_tmp, store) = open();
    let nov = report_pl(&store, "2026-11-23");
    assert_eq!(nov.figures.workdays, 20);
    assert_eq!(nov.figures.norm_ms, 160 * HOUR);
    assert_eq!(day_row(&nov, "2026-11-11").kind, DayKind::Holiday);
    assert_eq!(day_row(&nov, "2026-11-11").norm_ms, 0);
    assert_eq!(day_row(&nov, "2026-11-01").kind, DayKind::Weekend);
    assert_eq!(day_row(&nov, "2026-11-02").kind, DayKind::Workday);

    let dec = report_pl(&store, "2026-12-01");
    assert_eq!(dec.figures.workdays, 21);
    assert_eq!(dec.figures.norm_ms, 168 * HOUR);
    assert_eq!(day_row(&dec, "2026-12-24").kind, DayKind::Holiday);
    assert_eq!(day_row(&dec, "2026-12-25").kind, DayKind::Holiday);
    assert_eq!(day_row(&dec, "2026-12-26").kind, DayKind::Weekend);
}

#[test]
fn easter_sunday_is_a_weekend_and_easter_monday_a_holiday() {
    let (_tmp, store) = open();
    let apr = report_pl(&store, "2026-04-15");
    assert_eq!(day_row(&apr, "2026-04-05").kind, DayKind::Weekend);
    assert_eq!(day_row(&apr, "2026-04-06").kind, DayKind::Holiday);
}

#[test]
fn set_day_off_refuses_a_holiday_both_ways_and_writes_nothing() {
    let (_tmp, mut store) = open();
    let pl = WorkTimeSettings::default();
    for off in [true, false] {
        let r = report::set_day_off(&mut store, d("2026-11-11"), off, now(), &pl);
        assert!(matches!(r, Err(WorkTimeError::Holiday)), "{off}");
    }
    assert_eq!(day_off_rows(&store), 0);
    // A weekend holiday is still refused as a weekend.
    let r = report::set_day_off(&mut store, d("2026-12-26"), true, now(), &pl);
    assert!(matches!(r, Err(WorkTimeError::NotAWorkday)));
    assert!(!report::is_workday(&store, d("2026-11-11"), &pl).unwrap());
    assert!(report::is_workday(&store, d("2026-11-10"), &pl).unwrap());
}

#[test]
fn a_row_shadowed_by_a_holiday_shows_again_when_the_calendar_is_off() {
    let (_tmp, mut store) = open();
    report::set_day_off(&mut store, d("2026-11-11"), true, now(), &manual_only()).unwrap();
    let pl = report_pl(&store, "2026-11-23");
    assert_eq!(day_row(&pl, "2026-11-11").kind, DayKind::Holiday);
    assert_eq!(pl.figures.norm_ms, 160 * HOUR);
    let off = report_at(&store, "2026-11-23", now());
    assert_eq!(day_row(&off, "2026-11-11").kind, DayKind::DayOff);
    assert_eq!(off.figures.norm_ms, 160 * HOUR);
}

#[test]
fn status_and_day_entries_follow_the_holiday_calendar() {
    let (_tmp, store) = open();
    let pl = WorkTimeSettings::default();
    let at = t("2026-11-23T12:00");
    let s = status(&store, at, &Warsaw, &pl).unwrap();
    let r = report_pl(&store, "2026-11-23");
    assert_eq!(s.month_deviation_ms, r.figures.deviation_ms.unwrap());
    // 15 workdays up to the 23rd (the 11th is out), nothing worked.
    assert_eq!(s.month_deviation_ms, -15 * DAILY_NORM_MS);
    let kind = |day: &str, st: &WorkTimeSettings| {
        entries_for_day(&store, d(day), at, &Warsaw, st)
            .unwrap()
            .day_kind
    };
    assert_eq!(kind("2026-11-11", &pl), DayKind::Holiday);
    assert_eq!(kind("2026-11-11", &manual_only()), DayKind::Workday);
}

#[test]
fn day_kind_and_error_serialize_as_the_ui_expects() {
    assert_eq!(
        serde_json::to_value(DayKind::Holiday).unwrap(),
        serde_json::json!("holiday")
    );
    assert_eq!(
        serde_json::to_value(WorkTimeError::Holiday).unwrap(),
        serde_json::json!({ "kind": "Holiday" })
    );
}

// ---- holiday names, the Saturday hint and shadowed rows ----

fn holiday_of<'a>(r: &'a MonthReport, day: &str) -> (Option<&'a str>, bool) {
    let row = day_row(r, day);
    (row.holiday.as_deref(), row.day_off_owed)
}

#[test]
fn rows_carry_the_holiday_name_and_the_saturday_hint() {
    let (_tmp, store) = open();
    let nov = report_pl(&store, "2026-11-23");
    assert_eq!(
        holiday_of(&nov, "2026-11-11"),
        (Some("Święto Niepodległości"), false)
    );
    assert_eq!(
        holiday_of(&nov, "2026-11-01"),
        (Some("Wszystkich Świętych"), false)
    );
    assert_eq!(holiday_of(&nov, "2026-11-02"), (None, false));

    let dec = report_pl(&store, "2026-12-01");
    assert_eq!(holiday_of(&dec, "2026-12-24"), (Some("Wigilia"), false));
    assert_eq!(
        holiday_of(&dec, "2026-12-25"),
        (Some("Boże Narodzenie"), false)
    );
    assert_eq!(
        holiday_of(&dec, "2026-12-26"),
        (Some("Drugi dzień Bożego Narodzenia"), true)
    );

    let aug = report_pl(&store, "2026-08-10");
    assert!(day_row(&aug, "2026-08-15").day_off_owed);
    let apr = report_pl(&store, "2026-04-15");
    assert_eq!(holiday_of(&apr, "2026-04-05"), (Some("Wielkanoc"), false));
}

#[test]
fn a_shadowed_row_is_not_listed_as_a_manual_day_off() {
    let (_tmp, mut store) = open();
    report::set_day_off(&mut store, d("2026-11-11"), true, now(), &manual_only()).unwrap();
    report::set_day_off(&mut store, d("2026-11-12"), true, now(), &manual_only()).unwrap();
    let pl = report_pl(&store, "2026-11-23");
    assert_eq!(pl.days_off, vec!["2026-11-12".to_string()]);
    let off = report_at(&store, "2026-11-23", now());
    assert_eq!(
        off.days_off,
        vec!["2026-11-11".to_string(), "2026-11-12".to_string()]
    );
}

#[test]
fn with_the_calendar_off_no_row_has_a_holiday() {
    let (_tmp, store) = open();
    for month in ["2026-11-23", "2026-12-01", "2026-08-10", "2026-04-15"] {
        let r = report_at(&store, month, now());
        assert!(
            r.days
                .iter()
                .all(|x| x.holiday.is_none() && !x.day_off_owed)
        );
    }
}

#[test]
fn day_report_serializes_the_holiday_fields() {
    let (_tmp, store) = open();
    let nov = report_pl(&store, "2026-11-23");
    let hol = serde_json::to_value(day_row(&nov, "2026-11-11")).unwrap();
    assert_eq!(hol["kind"], "holiday");
    assert_eq!(hol["holiday"], "Święto Niepodległości");
    assert_eq!(hol["dayOffOwed"], false);
    let plain = serde_json::to_value(day_row(&nov, "2026-11-02")).unwrap();
    assert!(plain["holiday"].is_null());
    assert_eq!(plain["dayOffOwed"], false);
}
