use chrono::{Datelike, NaiveDate, Weekday};

use super::holidays::{HolidayCalendar, easter_sunday, polish_holidays};

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn name_on(date: NaiveDate) -> Option<&'static str> {
    HolidayCalendar::Pl.holiday(date)
}

#[test]
fn easter_matches_known_dates() {
    for (y, m, day) in [
        (2000, 4, 23),
        (2008, 3, 23),
        (2011, 4, 24),
        (2019, 4, 21),
        (2024, 3, 31),
        (2025, 4, 20),
        (2026, 4, 5),
        (2027, 3, 28),
        (2028, 4, 16),
        (2038, 4, 25),
    ] {
        assert_eq!(easter_sunday(y), Some(d(y, m, day)), "{y}");
    }
    assert_eq!(easter_sunday(1582), None);
}

#[test]
fn movable_holidays_match_known_dates() {
    for date in [
        d(2025, 4, 21),
        d(2026, 4, 6),
        d(2027, 3, 29),
        d(2028, 4, 17),
    ] {
        assert_eq!(name_on(date), Some("Poniedziałek Wielkanocny"));
    }
    for date in [
        d(2025, 6, 19),
        d(2026, 6, 4),
        d(2027, 5, 27),
        d(2028, 6, 15),
    ] {
        assert_eq!(name_on(date), Some("Boże Ciało"));
    }
}

#[test]
fn year_2026_has_fourteen_holidays_eight_on_weekdays() {
    let all = polish_holidays(2026);
    assert_eq!(all.len(), 14);
    let weekdays = all
        .iter()
        .filter(|(date, _)| !matches!(date.weekday(), Weekday::Sat | Weekday::Sun))
        .count();
    assert_eq!(weekdays, 8);
}

#[test]
fn year_gates() {
    assert_eq!(name_on(d(2024, 12, 24)), None);
    assert_eq!(name_on(d(2025, 12, 24)), Some("Wigilia"));
    assert_eq!(name_on(d(2010, 1, 6)), None);
    assert_eq!(name_on(d(2011, 1, 6)), Some("Święto Trzech Króli"));
}

#[test]
fn off_calendar_has_no_holidays() {
    assert_eq!(HolidayCalendar::Off.holiday(d(2026, 11, 11)), None);
    assert_eq!(name_on(d(2026, 11, 11)), Some("Święto Niepodległości"));
}

#[test]
fn easter_family_weekdays_and_uniqueness_1990_2100() {
    for year in 1990..=2100 {
        let easter = easter_sunday(year).unwrap();
        assert_eq!(easter.weekday(), Weekday::Sun, "{year}");
        let all = polish_holidays(year);
        let find = |name: &str| all.iter().find(|(_, n)| *n == name).unwrap().0;
        assert_eq!(find("Zielone Świątki").weekday(), Weekday::Sun, "{year}");
        assert_eq!(find("Boże Ciało").weekday(), Weekday::Thu, "{year}");
        let mut dates: Vec<_> = all.iter().map(|(date, _)| *date).collect();
        let n = dates.len();
        dates.dedup();
        assert_eq!(dates.len(), n, "{year}");
    }
}

#[test]
fn holiday_agrees_with_list_for_2025_to_2028() {
    let mut day = d(2025, 1, 1);
    while day <= d(2028, 12, 31) {
        let expected = polish_holidays(day.year())
            .into_iter()
            .find(|(date, _)| *date == day)
            .map(|(_, name)| name);
        assert_eq!(name_on(day), expected, "{day}");
        day = day.succ_opt().unwrap();
    }
}

#[test]
fn extreme_years_do_not_panic() {
    assert!(
        polish_holidays(1582)
            .iter()
            .all(|(date, _)| date.year() == 1582)
    );
    // Year 1: fixed dates only, no 6 January and no Christmas Eve.
    assert_eq!(polish_holidays(1).len(), 8);
    assert!(!polish_holidays(9999).is_empty());
    assert_eq!(name_on(d(1, 1, 1)), Some("Nowy Rok"));
}
