//! Tests for the month report as TSV, in `Europe/Warsaw` with invented data.
//! Expected cells are computed from the report fields, not typed in, except
//! for the literal example rows.

use chrono::NaiveDate;
use chrono_tz::Europe::Warsaw;
use tempfile::TempDir;

use super::month_text::{cell, hm, hours, month_report_text, signed_hours};
use super::report::{self, DayKind, DayReport, MonthReport};
use super::rules::resolve_local;
use super::*;
use crate::settings::WorkTimeSettings;
use crate::store::schema;

const MIN: i64 = 60_000;
const HOUR: i64 = 60 * MIN;
const SENTINEL: &str = "SENTINEL-invented-month-copy";

fn t(local: &str) -> i64 {
    resolve_local(local, &Warsaw).unwrap()
}

/// "Now" for every test: Monday 2026-11-23, noon local.
fn now() -> i64 {
    t("2026-11-23T12:00")
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

fn put(store: &mut Store, from: &str, to: &str, note: &str) {
    add(store, from, to, note, now(), &Warsaw).unwrap();
}

fn text(store: &Store, day: &str) -> String {
    month_report_text(store, d(day), now(), &Warsaw, &WorkTimeSettings::default()).unwrap()
}

fn month(store: &Store, day: &str) -> MonthReport {
    report::month_report(store, d(day), now(), &Warsaw, &WorkTimeSettings::default()).unwrap()
}

type Rows = Vec<Vec<String>>;

fn blocks(text: &str) -> Vec<Rows> {
    text.split("\n\n")
        .map(|b| {
            b.split('\n')
                .map(|l| l.split('\t').map(str::to_string).collect())
                .collect()
        })
        .collect()
}

/// The summary row with this label (columns 2 and 3).
fn summary<'a>(block: &'a Rows, label: &str) -> &'a [String] {
    &block.iter().find(|r| r[0] == label).unwrap()[1..]
}

fn assert_shape(text: &str) {
    assert!(!text.contains('\r'));
    assert!(!text.ends_with('\n'));
    let parts: Vec<&str> = text.split("\n\n").collect();
    assert_eq!(parts.len(), 3);
    for (block, tabs) in parts.iter().zip([2, 9, 5]) {
        for line in block.split('\n') {
            assert_eq!(line.matches('\t').count(), tabs, "{line:?}");
        }
    }
}

/// November 2026 with a closed entry (note with a tab), an entry across
/// midnight, a holiday entry, a marked day off and a running entry.
fn november() -> (TempDir, Store) {
    let (tmp, mut store) = open();
    put(
        &mut store,
        "2026-11-12T08:00",
        "2026-11-12T15:30",
        "plan\tsprint",
    );
    put(
        &mut store,
        "2026-11-19T22:00",
        "2026-11-20T02:00",
        "=SUMA(A1)",
    );
    put(&mut store, "2026-11-11T10:00", "2026-11-11T12:00", "");
    report::set_day_off(
        &mut store,
        d("2026-11-13"),
        true,
        now(),
        &WorkTimeSettings::default(),
    )
    .unwrap();
    start(&mut store, t("2026-11-23T09:00"), &Warsaw).unwrap();
    (tmp, store)
}

#[test]
fn cell_flattens_control_characters_and_trims() {
    assert_eq!(cell("a\tb"), "a b");
    assert_eq!(cell("a\u{7}b"), "a b");
    assert_eq!(cell("a\u{2028}b\u{2029}c"), "a b c");
    assert_eq!(cell("  padded \t"), "padded");
    assert_eq!(cell(""), "");
    assert_eq!(cell("plain text"), "plain text");
    assert_eq!(cell("say \"hi\""), "say \"hi\"");
}

#[test]
fn cell_guards_formula_starts() {
    for first in ["=", "+", "-", "@", "\"", "'", "＝", "＋", "－", "＠"] {
        let raw = format!("{first}x");
        assert_eq!(cell(&raw), format!("'{raw}"));
    }
    // The guard looks at the first character after the flattening and trim.
    assert_eq!(cell("\t=1"), "'=1");
}

#[test]
fn numbers_hm_floors_to_the_minute() {
    assert_eq!(hm(0), "0:00");
    assert_eq!(hm(59_999), "0:00");
    assert_eq!(hm(7 * HOUR + 30 * MIN), "7:30");
    assert_eq!(hm(40 * HOUR + 15 * MIN), "40:15");
    assert_eq!(hm(152 * HOUR + 5 * MIN), "152:05");
    assert_eq!(hm(-90 * MIN), "0:00");
}

#[test]
fn numbers_hours_use_a_decimal_comma() {
    assert_eq!(hours(7 * HOUR + 30 * MIN), "7,50");
    assert_eq!(hours(7 * HOUR + 20 * MIN), "7,33");
    assert_eq!(hours(MIN), "0,02");
    assert_eq!(hours(0), "0,00");
    assert_eq!(hours(-MIN), "0,00");
}

#[test]
fn numbers_signed_hours_truncate_toward_zero_without_a_plus() {
    assert_eq!(signed_hours(-90 * MIN), "-1,50");
    assert_eq!(signed_hours(-59_999), "0,00");
    assert_eq!(signed_hours(-61_000), "-0,02");
    assert_eq!(signed_hours(61_000), "0,02");
    assert_eq!(signed_hours(0), "0,00");
    for ms in [-90 * MIN, -61_000, 61_000, 7 * HOUR + 20 * MIN] {
        for out in [signed_hours(ms), hours(ms), hm(ms)] {
            assert!(!out.contains('.'));
            assert!(!out.contains('\u{2212}'));
            assert!(!out.starts_with('+'));
        }
    }
}

#[test]
fn current_month_matches_the_report() {
    let (_tmp, store) = november();
    let out = text(&store, "2026-11-23");
    let rep = month(&store, "2026-11-23");
    assert_shape(&out);
    let b = blocks(&out);

    // Block 1: the fixed 13 rows, equal to the report fields.
    assert_eq!(b[0].len(), 13);
    assert_eq!(summary(&b[0], "Miesiąc")[0], rep.month);
    assert_eq!(summary(&b[0], "Stan")[0], "bieżący");
    assert_eq!(summary(&b[0], "Stan na")[0], "2026-11-23 12:00");
    let f = &rep.figures;
    assert_eq!(
        summary(&b[0], "Przepracowano"),
        [hm(rep.worked_ms), hours(rep.worked_ms)]
    );
    assert_eq!(
        summary(&b[0], "Norma miesiąca"),
        [hm(f.norm_ms), hours(f.norm_ms)]
    );
    let norm_to_date = f.norm_to_date_ms.unwrap();
    assert_eq!(
        summary(&b[0], "Norma do dziś"),
        [hm(norm_to_date), hours(norm_to_date)]
    );
    assert_eq!(
        summary(&b[0], "Odchylenie"),
        ["".to_string(), signed_hours(f.deviation_ms.unwrap())]
    );
    let remaining = f.remaining_ms.unwrap();
    assert_eq!(
        summary(&b[0], "Pozostało"),
        [hm(remaining), hours(remaining)]
    );
    let per_day = f.needed_per_day_ms.unwrap();
    assert_eq!(
        summary(&b[0], "Średnio / dzień"),
        [hm(per_day), hours(per_day)]
    );
    assert_eq!(summary(&b[0], "Dni robocze")[0], f.workdays.to_string());
    assert_eq!(
        summary(&b[0], "Dni robocze po dziś")[0],
        f.workdays_after_today.unwrap().to_string()
    );
    assert_eq!(rep.days_off.len(), 1);
    assert_eq!(summary(&b[0], "Dni wolne")[0], "1");

    // Block 2: one row per day, each cell equal to the report field.
    assert_eq!(b[1].len(), 31);
    // 2026-11-01 is a Sunday.
    let weekdays = ["nd", "pn", "wt", "śr", "cz", "pt", "sb"];
    for (i, (row, day)) in b[1][1..].iter().zip(&rep.days).enumerate() {
        assert_eq!(row[1], weekdays[i % 7]);
        let DayReport {
            day: date,
            kind,
            worked_ms,
            norm_ms,
            deviation_ms,
            entry_count,
            holiday,
            day_off_owed,
        } = day;
        assert_eq!(&row[0], date);
        assert_eq!(
            row[2],
            ["roboczy", "wolny", "święto", "weekend"][*kind as usize]
        );
        assert_eq!(row[3], holiday.clone().unwrap_or_default());
        assert!(!day_off_owed);
        assert_eq!(row[4], entry_count.to_string());
        if *worked_ms == 0 && deviation_ms.is_none() {
            assert_eq!((row[5].as_str(), row[6].as_str()), ("", ""));
        } else {
            assert_eq!(
                (row[5].clone(), row[6].clone()),
                (hm(*worked_ms), hours(*worked_ms))
            );
        }
        assert_eq!(
            (row[7].clone(), row[8].clone()),
            (hm(*norm_ms), hours(*norm_ms))
        );
        assert_eq!(row[9], deviation_ms.map(signed_hours).unwrap_or_default());
    }
    let kind_of = |date: &str| {
        let row = b[1].iter().find(|r| r[0] == date).unwrap();
        row[2].clone()
    };
    assert_eq!(kind_of("2026-11-11"), "święto");
    assert_eq!(kind_of("2026-11-13"), "wolny");
    assert_eq!(kind_of("2026-11-14"), "weekend");
    assert_eq!(kind_of("2026-11-12"), "roboczy");
    assert_eq!(rep.days[10].kind, DayKind::Holiday);

    // Block 3: one row per entry part, as many as `entry_count` says.
    for day in &rep.days {
        let rows = b[2][1..].iter().filter(|r| r[0] == day.day).count();
        assert_eq!(rows, day.entry_count as usize, "{}", day.day);
    }
    let holiday = rep.days[10].holiday.clone().unwrap();
    let lines: Vec<&str> = out.split('\n').collect();
    for expected in [
        format!("2026-11-11\tśr\tświęto\t{holiday}\t1\t2:00\t2,00\t0:00\t0,00\t2,00"),
        "2026-11-12\t08:00\t15:30\t7:30\t7,50\tplan sprint".to_string(),
        "2026-11-19\t22:00\t2026-11-20 02:00\t2:00\t2,00\t'=SUMA(A1)".to_string(),
        "2026-11-20\t2026-11-19 22:00\t02:00\t2:00\t2,00\t'=SUMA(A1)".to_string(),
        "2026-11-23\t09:00\ttrwa\t3:00\t3,00\t".to_string(),
    ] {
        assert!(lines.contains(&expected.as_str()), "{expected:?}");
    }
}

#[test]
fn past_month_with_dst_and_a_saturday_holiday() {
    let (_tmp, mut store) = open();
    // The clock goes back at 03:00 on 2026-10-25: 01:00 to 04:00 is four hours.
    put(&mut store, "2026-10-25T01:00", "2026-10-25T04:00", "");

    let out = text(&store, "2026-10-10");
    let rep = month(&store, "2026-10-10");
    assert_shape(&out);
    let b = blocks(&out);
    assert_eq!(b[1].len(), 32);
    let row = b[1].iter().find(|r| r[0] == "2026-10-25").unwrap();
    assert_eq!((row[4].as_str(), row[5].as_str()), ("1", "4:00"));
    assert_eq!(b[2].len(), 2);
    assert_eq!(b[2][1][..3], ["2026-10-25", "01:00", "04:00"]);
    assert_eq!(b[2][1][3], "4:00");

    assert_eq!(summary(&b[0], "Stan")[0], "zamknięty");
    assert_eq!(
        summary(&b[0], "Norma do dziś"),
        summary(&b[0], "Norma miesiąca")
    );
    for label in ["Pozostało", "Średnio / dzień", "Dni robocze po dziś"] {
        assert!(
            summary(&b[0], label).iter().all(String::is_empty),
            "{label}"
        );
    }
    assert_eq!(
        summary(&b[0], "Odchylenie")[1],
        signed_hours(rep.figures.deviation_ms.unwrap())
    );

    // 2026-08-15 is a Saturday holiday: a day off is owed.
    let aug = text(&store, "2026-08-20");
    assert_shape(&aug);
    let aug_rep = month(&store, "2026-08-20");
    let holiday = aug_rep.days[14].holiday.clone().unwrap();
    assert!(aug_rep.days[14].day_off_owed);
    let row = blocks(&aug)[1]
        .iter()
        .find(|r| r[0] == "2026-08-15")
        .unwrap()
        .clone();
    assert_eq!(row[2], "weekend");
    assert_eq!(row[3], format!("{holiday} · należy się inny dzień wolny"));
}

#[test]
fn future_month_is_empty_but_shaped() {
    let (_tmp, store) = open();
    let out = text(&store, "2026-12-05");
    assert_shape(&out);
    let b = blocks(&out);
    assert_eq!(summary(&b[0], "Stan")[0], "przyszły");
    for label in [
        "Odchylenie",
        "Norma do dziś",
        "Pozostało",
        "Średnio / dzień",
        "Dni robocze po dziś",
    ] {
        assert!(
            summary(&b[0], label).iter().all(String::is_empty),
            "{label}"
        );
    }
    assert!(!summary(&b[0], "Norma miesiąca")[0].is_empty());
    assert!(!summary(&b[0], "Dni robocze")[0].is_empty());
    assert_eq!(b[1].len(), 32);
    for row in &b[1][1..] {
        assert_eq!(
            (row[5].as_str(), row[6].as_str(), row[9].as_str()),
            ("", "", "")
        );
    }
    assert_eq!(b[2].len(), 1);
}

#[test]
fn shape_is_rectangular() {
    let (_tmp, store) = november();
    assert_shape(&text(&store, "2026-11-01"));
    let (_tmp, empty) = open();
    assert_shape(&text(&empty, "2026-11-01"));
}

#[test]
fn sentinel_note_reaches_only_the_text() {
    let (_tmp, mut store) = open();
    put(&mut store, "2026-11-10T08:00", "2026-11-10T09:00", SENTINEL);
    assert!(text(&store, "2026-11-10").contains(SENTINEL));
    for err in [
        WorkTimeError::SettingsInvalid,
        WorkTimeError::InvalidLocalTime,
    ] {
        assert!(!serde_json::to_string(&err).unwrap().contains(SENTINEL));
    }
}
