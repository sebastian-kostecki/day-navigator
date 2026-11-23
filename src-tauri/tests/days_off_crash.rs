//! S-09 crash proof: killing a process that marks days off at any moment
//! leaves the store consistent and never loses a reported commit.
//!
//! The writer marks distinct invented weekdays, one commit each, and does not
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

fn days(store: &Store) -> Vec<String> {
    store
        .read::<_, WorkTimeError>(|c| {
            let mut stmt = c.prepare("SELECT day FROM days_off ORDER BY day")?;
            let out = stmt.query_map([], |r| r.get::<_, String>(0))?;
            Ok(out.collect::<Result<_, _>>()?)
        })
        .unwrap()
}

fn parse(day: &str) -> NaiveDate {
    NaiveDate::parse_from_str(day, "%Y-%m-%d").unwrap()
}

/// Child side: runs only when the parent test re-launches this binary.
#[test]
#[ignore = "writer process for crash_keeps_days_off_consistent"]
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
        .map_or(first_day(), |d| parse(d).succ_opt().unwrap());
    let report_commit = |n: u64| {
        let mut out = std::io::stdout().lock();
        writeln!(out, "committed {n}").unwrap();
        out.flush().unwrap();
    };
    for _ in 0..MAX_WRITER_COMMITS {
        while matches!(day.weekday(), Weekday::Sat | Weekday::Sun) {
            day = day.succ_opt().unwrap();
        }
        report::set_day_off(&mut store, day, true, 1_767_225_600_000).unwrap();
        day = day.succ_opt().unwrap();
        n += 1;
        report_commit(n);
    }
}

#[test]
fn crash_keeps_days_off_consistent() {
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
        for d in &all {
            assert!(
                !matches!(parse(d).weekday(), Weekday::Sat | Weekday::Sun),
                "round {round} ({kill:?}): weekend row {d}"
            );
        }
    }
}
