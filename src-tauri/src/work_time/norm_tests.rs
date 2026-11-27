//! Norm and month-report tests on invented months in `Europe/Warsaw`.

use chrono::NaiveDate;
use chrono_tz::Europe::Warsaw;
use tempfile::TempDir;

use super::holidays::HolidayCalendar;
use super::norm::{self, DEFAULT_DAILY_NORM_MS, DayOff, DaysOff, MonthPhase};
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
        ..WorkTimeSettings::default()
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
    report::set_day_off(
        store,
        d("2026-11-11"),
        report::DayMark::DayOff,
        now(),
        &manual_only(),
    )
    .unwrap();
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
    let none = DaysOff::new();
    let w = norm::workdays(first, next, &none, HolidayCalendar::Off);
    assert_eq!(w.len(), 21);
    assert_eq!(DEFAULT_DAILY_NORM_MS * 21, 168 * HOUR);
    let off = DaysOff::from([(d("2026-11-11"), DayOff::Full)]);
    let w = norm::workdays(first, next, &off, HolidayCalendar::Off);
    assert_eq!(w.len(), 20);
    assert_eq!(DEFAULT_DAILY_NORM_MS * 20, 160 * HOUR);
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
    let listed = DaysOff::from([
        (d("2026-11-01"), DayOff::Full),
        (d("2026-11-07"), DayOff::Full),
    ]);
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
    let day_norms: Vec<_> = workdays
        .iter()
        .map(|day| (*day, DEFAULT_DAILY_NORM_MS))
        .collect();
    let f = norm::figures(
        &day_norms,
        d("2026-11-23"),
        MonthPhase::Current,
        4 * HOUR + MIN,
        DEFAULT_DAILY_NORM_MS,
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
    report::set_day_off(
        &mut store,
        d("2026-11-11"),
        report::DayMark::DayOff,
        now(),
        &manual_only(),
    )
    .unwrap();
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
    report::set_day_off(
        &mut store,
        d("2026-11-23"),
        report::DayMark::DayOff,
        now(),
        &manual_only(),
    )
    .unwrap();
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
    report::set_day_off(
        &mut store,
        d("2026-11-11"),
        report::DayMark::DayOff,
        now(),
        &manual_only(),
    )
    .unwrap();
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
    report::set_day_off(
        &mut store,
        day,
        report::DayMark::DayOff,
        now(),
        &manual_only(),
    )
    .unwrap();
    report::set_day_off(
        &mut store,
        day,
        report::DayMark::DayOff,
        now() + HOUR,
        &manual_only(),
    )
    .unwrap();
    assert_eq!(day_off_rows(&store), 1);
    // The first mark is kept.
    let created: i64 = store
        .read::<_, WorkTimeError>(|c| {
            Ok(c.query_row("SELECT created_at_ms FROM days_off", [], |r| r.get(0))?)
        })
        .unwrap();
    assert_eq!(created, now());
    assert!(!report::is_workday(&store, day, &manual_only()).unwrap());
    report::set_day_off(
        &mut store,
        day,
        report::DayMark::Workday,
        now(),
        &manual_only(),
    )
    .unwrap();
    report::set_day_off(
        &mut store,
        day,
        report::DayMark::Workday,
        now(),
        &manual_only(),
    )
    .unwrap();
    assert_eq!(day_off_rows(&store), 0);
    assert!(report::is_workday(&store, day, &manual_only()).unwrap());

    for weekend in ["2026-11-14", "2026-11-15"] {
        for off in [
            report::DayMark::DayOff,
            report::DayMark::HalfDayOff,
            report::DayMark::Workday,
        ] {
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
    let r = report::set_day_off(
        &mut store,
        far,
        report::DayMark::DayOff,
        now(),
        &manual_only(),
    );
    assert!(matches!(r, Err(WorkTimeError::Store(_))), "{r:?}");
    assert_eq!(day_off_rows(&store), 0);
}

#[test]
fn day_entries_report_all_three_day_kinds() {
    let (_tmp, mut store) = open();
    report::set_day_off(
        &mut store,
        d("2026-11-11"),
        report::DayMark::DayOff,
        now(),
        &manual_only(),
    )
    .unwrap();
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
    let off = DaysOff::from([
        (d("2026-11-11"), DayOff::Full),
        (d("2026-11-14"), DayOff::Full),
    ]);
    assert_eq!(
        norm::day_norm_ms(
            d("2026-11-10"),
            &off,
            HolidayCalendar::Off,
            DEFAULT_DAILY_NORM_MS
        ),
        DEFAULT_DAILY_NORM_MS
    );
    assert_eq!(
        norm::day_norm_ms(
            d("2026-11-11"),
            &off,
            HolidayCalendar::Off,
            DEFAULT_DAILY_NORM_MS
        ),
        0
    );
    assert_eq!(
        norm::day_norm_ms(
            d("2026-11-15"),
            &off,
            HolidayCalendar::Off,
            DEFAULT_DAILY_NORM_MS
        ),
        0
    );
    // A Saturday listed in days_off is still 0.
    assert_eq!(
        norm::day_norm_ms(
            d("2026-11-14"),
            &off,
            HolidayCalendar::Off,
            DEFAULT_DAILY_NORM_MS
        ),
        0
    );
}

#[test]
fn day_deviation_follows_the_month_rule() {
    let today = d("2026-11-23");
    let dev =
        |day: &str, norm: i64, worked: i64| norm::day_deviation_ms(d(day), today, norm, worked);
    assert_eq!(dev("2026-11-24", DEFAULT_DAILY_NORM_MS, 0), None);
    assert_eq!(
        dev("2026-11-23", DEFAULT_DAILY_NORM_MS, 4 * HOUR),
        Some(-4 * HOUR)
    );
    assert_eq!(dev("2026-11-20", DEFAULT_DAILY_NORM_MS, 0), Some(-8 * HOUR));
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
            DEFAULT_DAILY_NORM_MS
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
        off_flags in proptest::collection::vec(0u8..3, 30),
        with_holidays in proptest::prelude::any::<bool>(),
        norm_minutes in 60i64..=600,
    ) {
        let settings = WorkTimeSettings {
            daily_norm_ms: norm_minutes * MIN,
            ..if with_holidays { WorkTimeSettings::default() } else { manual_only() }
        };
        let daily = settings.daily_norm_ms;
        let mut marked = DaysOff::new();
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
            if *flag != 0 && !norm::is_weekend(day) && settings.holidays.holiday(day).is_none() {
                let (mark, kind) = if *flag == 1 {
                    (report::DayMark::HalfDayOff, DayOff::Half)
                } else {
                    (report::DayMark::DayOff, DayOff::Full)
                };
                report::set_day_off(&mut store, day, mark, now(), &settings).unwrap();
                marked.insert(day, kind);
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
        // Independent oracle from the flags and the calendar.
        let norm_days: Vec<(NaiveDate, i64)> = (0..30)
            .map(|i| d("2026-11-01") + chrono::Days::new(i))
            .filter(|day| !norm::is_weekend(*day) && settings.holidays.holiday(*day).is_none())
            .filter_map(|day| match marked.get(&day) {
                None => Some((day, daily)),
                Some(DayOff::Half) => Some((day, daily / 2)),
                Some(DayOff::Full) => None,
            })
            .collect();
        let halves = marked.values().filter(|k| **k == DayOff::Half).count() as i64;
        let unmarked = norm_days.len() as i64 - halves;
                proptest::prop_assert_eq!(r.figures.norm_ms, daily * unmarked + daily / 2 * halves);
        let to_date: i64 = norm_days.iter().filter(|(day, _)| *day <= d("2026-11-23")).map(|(_, n)| n).sum();
        proptest::prop_assert_eq!(r.figures.norm_to_date_ms, Some(to_date));
        proptest::prop_assert_eq!(r.daily_norm_ms, daily);
        let after_days = norm_days.iter().filter(|(day, _)| *day > d("2026-11-23")).count() as u32;
        let half_after = norm_days.iter().filter(|(day, n)| *day > d("2026-11-23") && *n < daily).count() as u32;
        proptest::prop_assert_eq!(r.figures.workdays_after_today, Some(after_days));
        proptest::prop_assert_eq!(r.figures.half_days_after_today, Some(half_after));
        if after_days > 0 {
            let remaining = i128::from(r.figures.remaining_ms.unwrap());
            let needed = i128::from(r.figures.needed_per_day_ms.unwrap());
            let norm_after = i128::from(
                norm_days
                    .iter()
                    .filter(|(day, _)| *day > d("2026-11-23"))
                    .map(|(_, n)| n)
                    .sum::<i64>(),
            );
            let wanted = remaining * i128::from(daily);
            proptest::prop_assert!(needed * norm_after >= wanted);
            proptest::prop_assert!((needed - i128::from(MIN)) * norm_after < wanted);
            proptest::prop_assert_eq!(needed % i128::from(MIN), 0);
        }
    }
}

// ---- public holidays (default calendar) ----

fn report_pl(store: &Store, day: &str) -> MonthReport {
    report::month_report(store, d(day), now(), &Warsaw, &WorkTimeSettings::default()).unwrap()
}

#[test]
fn pure_norm_counts_holidays_out() {
    let none = DaysOff::new();
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
        norm::day_norm_ms(
            d("2026-11-11"),
            &none,
            HolidayCalendar::Pl,
            DEFAULT_DAILY_NORM_MS
        ),
        0
    );
    assert_eq!(
        norm::day_norm_ms(
            d("2026-11-11"),
            &none,
            HolidayCalendar::Off,
            DEFAULT_DAILY_NORM_MS
        ),
        DEFAULT_DAILY_NORM_MS
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
    for off in [
        report::DayMark::DayOff,
        report::DayMark::HalfDayOff,
        report::DayMark::Workday,
    ] {
        let r = report::set_day_off(&mut store, d("2026-11-11"), off, now(), &pl);
        assert!(matches!(r, Err(WorkTimeError::Holiday)), "{off:?}");
    }
    assert_eq!(day_off_rows(&store), 0);
    // A weekend holiday is still refused as a weekend.
    let r = report::set_day_off(
        &mut store,
        d("2026-12-26"),
        report::DayMark::DayOff,
        now(),
        &pl,
    );
    assert!(matches!(r, Err(WorkTimeError::NotAWorkday)));
    assert!(!report::is_workday(&store, d("2026-11-11"), &pl).unwrap());
    assert!(report::is_workday(&store, d("2026-11-10"), &pl).unwrap());
}

#[test]
fn a_row_shadowed_by_a_holiday_shows_again_when_the_calendar_is_off() {
    let (_tmp, mut store) = open();
    report::set_day_off(
        &mut store,
        d("2026-11-11"),
        report::DayMark::DayOff,
        now(),
        &manual_only(),
    )
    .unwrap();
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
    assert_eq!(s.month_deviation_ms, -15 * DEFAULT_DAILY_NORM_MS);
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
    report::set_day_off(
        &mut store,
        d("2026-11-11"),
        report::DayMark::DayOff,
        now(),
        &manual_only(),
    )
    .unwrap();
    report::set_day_off(
        &mut store,
        d("2026-11-12"),
        report::DayMark::DayOff,
        now(),
        &manual_only(),
    )
    .unwrap();
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

// ---- configurable daily norm ----

fn with_norm(minutes: i64, base: WorkTimeSettings) -> WorkTimeSettings {
    WorkTimeSettings {
        daily_norm_ms: minutes * MIN,
        ..base
    }
}

#[test]
fn a_seven_and_a_half_hour_norm_sets_every_workday_to_7_30() {
    let (_tmp, store) = open();
    let st = with_norm(450, WorkTimeSettings::default());
    let r = report::month_report(&store, d("2026-11-23"), now(), &Warsaw, &st).unwrap();
    assert_eq!(r.daily_norm_ms, 27_000_000);
    assert_eq!(r.figures.workdays, 20);
    assert_eq!(r.figures.norm_ms, 540_000_000);
    assert_eq!(day_row(&r, "2026-11-10").norm_ms, 450 * MIN);
    assert_eq!(day_row(&r, "2026-11-11").norm_ms, 0);
    assert_eq!(day_row(&r, "2026-11-14").norm_ms, 0);

    let st = with_norm(450, manual_only());
    let r = report::month_report(&store, d("2026-11-23"), now(), &Warsaw, &st).unwrap();
    assert_eq!(r.figures.workdays, 21);
    assert_eq!(r.figures.norm_ms, 157 * HOUR + 30 * MIN);
}

#[test]
fn needed_per_day_for_whole_days_is_the_plain_ceiling() {
    let (_tmp, mut store) = open();
    work_days(&mut store, &["2026-11-02", "2026-11-03"]);
    let st = with_norm(360, manual_only());
    let r = report::month_report(&store, d("2026-11-23"), now(), &Warsaw, &st).unwrap();
    let after = i64::from(r.figures.workdays_after_today.unwrap());
    let remaining = r.figures.remaining_ms.unwrap();
    assert_eq!(
        r.figures.needed_per_day_ms,
        Some(norm::ceil_to_minute((remaining + after - 1) / after))
    );
}

#[test]
fn day_norms_cover_any_range_with_the_norm_of_each_day() {
    let off = DaysOff::from([(d("2026-11-26"), DayOff::Full)]);
    // Mon 2026-11-23 to Sun 2026-11-29: Thursday off, weekend out.
    let got = norm::day_norms(
        d("2026-11-23"),
        d("2026-11-30"),
        &off,
        HolidayCalendar::Off,
        450 * MIN,
    );
    let days: Vec<NaiveDate> = got.iter().map(|(day, _)| *day).collect();
    assert_eq!(
        days,
        vec![
            d("2026-11-23"),
            d("2026-11-24"),
            d("2026-11-25"),
            d("2026-11-27")
        ]
    );
    assert!(got.iter().all(|(_, n)| *n == 450 * MIN));
    // A week across the month boundary.
    let got = norm::day_norms(
        d("2026-11-30"),
        d("2026-12-07"),
        &DaysOff::new(),
        HolidayCalendar::Off,
        DEFAULT_DAILY_NORM_MS,
    );
    assert_eq!(got.len(), 5);
}

#[test]
fn status_deviation_follows_a_seven_hour_norm() {
    let (_tmp, store) = open();
    let st = with_norm(420, manual_only());
    let s = status(&store, now(), &Warsaw, &st).unwrap();
    // 16 workdays up to and including Monday the 23rd, nothing worked.
    assert_eq!(s.month_deviation_ms, -16 * 7 * HOUR);
    let r = report::month_report(&store, d("2026-11-23"), now(), &Warsaw, &st).unwrap();
    assert_eq!(s.month_deviation_ms, r.figures.deviation_ms.unwrap());
}

// ---- half a day off ----

fn mark(store: &mut Store, day: &str, mark: report::DayMark, settings: &WorkTimeSettings) {
    report::set_day_off(store, d(day), mark, now(), settings).unwrap();
}

fn day_off_kind(store: &Store, day: &str) -> Option<String> {
    store
        .read(|c| {
            Ok::<_, WorkTimeError>(
                c.query_row("SELECT kind FROM days_off WHERE day = ?1", [day], |r| {
                    r.get::<_, String>(0)
                })
                .ok(),
            )
        })
        .unwrap()
}

#[test]
fn a_half_day_off_takes_half_the_norm_off_the_month() {
    // (norm in minutes, expected half-day norm in ms)
    for (minutes, half_ms) in [
        (480, 4 * HOUR),
        (450, 3 * HOUR + 45 * MIN),
        (421, 12_630_000),
    ] {
        let (_tmp, mut store) = open();
        let st = with_norm(minutes, manual_only());
        mark(&mut store, "2026-11-12", report::DayMark::HalfDayOff, &st);
        let r = report::month_report(&store, d("2026-11-23"), now(), &Warsaw, &st).unwrap();
        let row = day_row(&r, "2026-11-12");
        assert_eq!(row.kind, DayKind::HalfDayOff);
        assert_eq!(row.norm_ms, half_ms, "{minutes}");
        // 21 November workdays; the half day still counts as a workday.
        assert_eq!(r.figures.workdays, 21);
        assert_eq!(r.figures.norm_ms, 20 * minutes * MIN + half_ms, "{minutes}");
        assert_eq!(r.half_days_off, vec!["2026-11-12".to_string()]);
        assert!(r.days_off.is_empty());
    }
}

#[test]
fn a_half_day_off_before_today_halves_its_part_of_the_status_deviation() {
    let (_tmp, mut store) = open();
    worked_example(&mut store);
    mark(
        &mut store,
        "2026-11-12",
        report::DayMark::HalfDayOff,
        &manual_only(),
    );
    let s = status(&store, now(), &Warsaw, &manual_only()).unwrap();
    let r = report_at(&store, "2026-11-23", now());
    assert_eq!(Some(s.month_deviation_ms), r.figures.deviation_ms);
    // Same entries as `worked_example` but the 12th now has a 4 h norm, not 8 h.
    let plain = {
        let (_t2, mut s2) = open();
        worked_example(&mut s2);
        report_at(&s2, "2026-11-23", now())
            .figures
            .deviation_ms
            .unwrap()
    };
    assert_eq!(r.figures.deviation_ms, Some(plain + 4 * HOUR));
}

#[test]
fn day_marks_move_between_the_three_states_with_one_row() {
    let (_tmp, mut store) = open();
    let st = manual_only();
    let day = "2026-11-12";
    assert_eq!(day_off_kind(&store, day), None);
    mark(&mut store, day, report::DayMark::HalfDayOff, &st);
    assert_eq!(day_off_kind(&store, day).as_deref(), Some("half"));
    mark(&mut store, day, report::DayMark::HalfDayOff, &st);
    assert_eq!(day_off_rows(&store), 1);
    mark(&mut store, day, report::DayMark::DayOff, &st);
    assert_eq!(day_off_kind(&store, day).as_deref(), Some("full"));
    mark(&mut store, day, report::DayMark::HalfDayOff, &st);
    assert_eq!(day_off_kind(&store, day).as_deref(), Some("half"));
    assert_eq!(day_off_rows(&store), 1);
    mark(&mut store, day, report::DayMark::Workday, &st);
    assert_eq!(day_off_rows(&store), 0);
    mark(&mut store, day, report::DayMark::Workday, &st);
    assert_eq!(day_off_rows(&store), 0);
}

#[test]
fn changing_the_kind_keeps_the_first_marking_time() {
    let (_tmp, mut store) = open();
    let st = manual_only();
    let created = |store: &Store| -> i64 {
        store
            .read(|c| {
                Ok::<_, WorkTimeError>(
                    c.query_row("SELECT created_at_ms FROM days_off", [], |r| r.get(0))
                        .unwrap(),
                )
            })
            .unwrap()
    };
    mark(&mut store, "2026-11-12", report::DayMark::HalfDayOff, &st);
    let first = created(&store);
    report::set_day_off(
        &mut store,
        d("2026-11-12"),
        report::DayMark::DayOff,
        now() + HOUR,
        &st,
    )
    .unwrap();
    assert_eq!(created(&store), first);
}

#[test]
fn a_half_day_row_on_a_holiday_is_shadowed_not_deleted() {
    let (_tmp, mut store) = open();
    mark(
        &mut store,
        "2026-11-11",
        report::DayMark::HalfDayOff,
        &manual_only(),
    );
    let pl = report_pl(&store, "2026-11-23");
    let row = day_row(&pl, "2026-11-11");
    assert_eq!(row.kind, DayKind::Holiday);
    assert_eq!(row.norm_ms, 0);
    assert!(pl.half_days_off.is_empty());
    assert_eq!(day_off_rows(&store), 1);
    let off = report_at(&store, "2026-11-23", now());
    assert_eq!(off.half_days_off, vec!["2026-11-11".to_string()]);
    assert_eq!(day_row(&off, "2026-11-11").norm_ms, 4 * HOUR);
}

#[test]
fn needed_per_day_is_per_full_day_with_half_days_after_today() {
    // Today (full) + 2 full days + 1 half day after: 20 h of norm after today.
    let norms: Vec<(NaiveDate, i64)> = vec![
        (d("2026-11-23"), 8 * HOUR),
        (d("2026-11-24"), 8 * HOUR),
        (d("2026-11-25"), 8 * HOUR),
        (d("2026-11-26"), 4 * HOUR),
    ];
    let f = norm::figures(
        &norms,
        d("2026-11-23"),
        MonthPhase::Current,
        3 * HOUR,
        DEFAULT_DAILY_NORM_MS,
    );
    assert_eq!(f.remaining_ms, Some(25 * HOUR));
    assert_eq!(f.workdays_after_today, Some(3));
    assert_eq!(f.half_days_after_today, Some(1));
    // 10 + 10 + 5 = 25 h.
    assert_eq!(f.needed_per_day_ms, Some(10 * HOUR));
    // A past month has no "after today".
    let past = norm::figures(
        &norms,
        d("2026-12-23"),
        MonthPhase::Past,
        0,
        DEFAULT_DAILY_NORM_MS,
    );
    assert_eq!(past.half_days_after_today, None);
}

#[test]
fn day_norm_of_a_half_day_is_half_and_a_weekend_or_holiday_stays_zero() {
    let marks = DaysOff::from([
        (d("2026-11-12"), DayOff::Half),
        (d("2026-11-14"), DayOff::Half),
        (d("2026-11-11"), DayOff::Half),
    ]);
    let n = |day: &str, cal| norm::day_norm_ms(d(day), &marks, cal, DEFAULT_DAILY_NORM_MS);
    assert_eq!(n("2026-11-12", HolidayCalendar::Pl), 4 * HOUR);
    assert_eq!(n("2026-11-14", HolidayCalendar::Pl), 0);
    assert_eq!(n("2026-11-11", HolidayCalendar::Pl), 0);
    assert_eq!(n("2026-11-11", HolidayCalendar::Off), 4 * HOUR);
    let (first, next) = norm::month_days(d("2026-11-01"));
    // The half day (12th) and the shadowed 11th are workdays under `Off`.
    assert_eq!(
        norm::workdays(first, next, &marks, HolidayCalendar::Off).len(),
        21
    );
}

#[test]
fn half_day_off_serializes_as_the_ui_expects() {
    assert_eq!(
        serde_json::to_value(DayKind::HalfDayOff).unwrap(),
        serde_json::json!("halfDayOff")
    );
}
