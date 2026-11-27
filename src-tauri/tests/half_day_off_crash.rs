//! S-02 (configurable-daily-norm) crash proof: killing a process that marks
//! days as half or whole days off at any moment leaves the store consistent,
//! never loses a reported commit and never leaves a wrong kind.
//!
//! The writer marks distinct invented weekdays, one commit each (odd ones half
//! a day off, even ones a whole day off), and does not
//! pause inside a transaction, so kills land between commits (see
//! `tests/work_time_crash.rs`).

mod common;

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use chrono::{Datelike, NaiveDate, Weekday};
use common::crash::{KillAt, MAX_WRITER_COMMITS, WRITER_DIR_ENV, run_writer_until};
use common::test_store_dir;
use day_navigator_lib::store::{Store, schema};
use day_navigator_lib::work_time::{WorkTimeError, report};

const WRITER: &str = "writer_process";

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 23).unwrap()
}

/// Invented first day to mark (a Thursday).
fn first_day() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
}

fn days(store: &Store) -> Vec<(String, String)> {
    store
        .read::<_, WorkTimeError>(|c| {
            let mut stmt = c.prepare("SELECT day, kind FROM days_off ORDER BY day")?;
            let out =
                stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
            Ok(out.collect::<Result<_, _>>()?)
        })
        .unwrap()
}

/// Kind of the `n`-th marked day (1-based): odd is half a day, even whole.
fn kind_of(n: u64) -> (report::DayMark, &'static str) {
    if n % 2 == 1 {
        (report::DayMark::HalfDayOff, "half")
    } else {
        (report::DayMark::DayOff, "full")
    }
}

fn parse(day: &str) -> NaiveDate {
    NaiveDate::parse_from_str(day, "%Y-%m-%d").unwrap()
}

/// Child side: runs only when the parent test re-launches this binary.
#[test]
#[ignore = "writer process for crash_keeps_half_and_whole_days_off_consistent"]
fn writer_process() {
    let Some(dir) = std::env::var_os(WRITER_DIR_ENV) else {
        return;
    };
    let mut store = Store::open(Path::new(&dir), schema::MIGRATIONS, today()).unwrap();
    let existing = days(&store);
    let mut n = existing.len() as u64;
    // Continue after the latest stored day.
    let mut day = existing
        .last()
        .map_or(first_day(), |(d, _)| parse(d).succ_opt().unwrap());
    let report_commit = |n: u64| {
        let mut out = std::io::stdout().lock();
        writeln!(out, "committed {n}").unwrap();
        out.flush().unwrap();
    };
    for _ in 0..MAX_WRITER_COMMITS {
        while matches!(day.weekday(), Weekday::Sat | Weekday::Sun) {
            day = day.succ_opt().unwrap();
        }
        report::set_day_off(
            &mut store,
            day,
            kind_of(n + 1).0,
            1_767_225_600_000,
            &common::manual_only(),
        )
        .unwrap();
        day = day.succ_opt().unwrap();
        n += 1;
        report_commit(n);
    }
}

#[test]
fn crash_keeps_half_and_whole_days_off_consistent() {
    let (_tmp, dir) = test_store_dir();
    let kills = [
        KillAt::AfterCommits(1),
        KillAt::AfterDelay(Duration::from_millis(150)),
        KillAt::AfterCommits(5),
        KillAt::AfterDelay(Duration::from_millis(500)),
        KillAt::AfterCommits(20),
        KillAt::AfterDelay(Duration::from_millis(900)),
        KillAt::AfterCommits(3),
        KillAt::AfterDelay(Duration::from_millis(333)),
        KillAt::AfterCommits(41),
        KillAt::AfterDelay(Duration::from_millis(1200)),
    ];
    for (round, kill) in kills.into_iter().enumerate() {
        let last = run_writer_until(WRITER, &dir, kill);

        let store = Store::open(&dir, schema::MIGRATIONS, today())
            .unwrap_or_else(|e| panic!("round {round} ({kill:?}): open failed: {e:?}"));
        let check: String = store
            .read::<_, WorkTimeError>(|c| Ok(c.query_row("PRAGMA quick_check", [], |r| r.get(0))?))
            .unwrap();
        assert_eq!(check, "ok", "round {round} ({kill:?})");

        let all = days(&store);
        assert!(
            all.len() as u64 >= last,
            "round {round} ({kill:?}): lost commit, {} < reported {last}",
            all.len()
        );
        // The writer marks weekdays in order from `first_day()`, so the rows must
        // be exactly the first N weekdays: no gap, no weekend, nothing torn.
        let expected: Vec<(String, String)> = first_weekdays(all.len())
            .into_iter()
            .enumerate()
            .map(|(i, day)| (day, kind_of(i as u64 + 1).1.to_string()))
            .collect();
        assert_eq!(
            all, expected,
            "round {round} ({kill:?}): rows are not the contiguous weekday run with the expected kinds"
        );
    }
}

fn first_weekdays(n: usize) -> Vec<String> {
    let mut out = Vec::with_capacity(n);
    let mut day = first_day();
    while out.len() < n {
        if !matches!(day.weekday(), Weekday::Sat | Weekday::Sun) {
            out.push(day.format("%Y-%m-%d").to_string());
        }
        day = day.succ_opt().unwrap();
    }
    out
}
