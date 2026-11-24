//! Durability: killing the writer at any moment keeps the log whole. The
//! writer's clock advances a fixed step per write, so the day changes every
//! `PER_DAY` writes; the reopened rows must equal the events of the day of the
//! last stored write, and nothing else.

mod common;

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use chrono::{Duration as Span, NaiveDate, NaiveDateTime};
use common::crash::{KillAt, MAX_WRITER_COMMITS, WRITER_DIR_ENV, run_writer_until};
use common::test_store_dir;
use day_navigator_lib::events::{NewEvent, record};
use day_navigator_lib::store::{Store, StoreError, schema};

const WRITER: &str = "writer_process";
/// Writes per calendar day.
const PER_DAY: i64 = 7;

fn start() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
}

/// Write `n` (1-based) happens at this instant: `PER_DAY` writes per day.
fn now_of(n: i64) -> NaiveDateTime {
    start() + Span::seconds(86_400 / PER_DAY * (n - 1))
}

fn stamp(n: i64) -> String {
    now_of(n).format("%Y-%m-%dT%H:%M:%S").to_string()
}

/// Which write stored this instant.
fn write_of(at: &str) -> i64 {
    (1..)
        .find(|n| stamp(*n) == at)
        .expect("an instant of a known write")
}

/// Every write stores two events, so a torn write would show as an odd row.
fn events_of() -> Vec<NewEvent> {
    vec![NewEvent::WorkStarted, NewEvent::WorkStopped]
}

/// The `at` values after writes `1..=m`: those of the last write's day.
fn model_after(m: i64) -> Vec<String> {
    if m == 0 {
        return Vec::new();
    }
    let day = now_of(m).date();
    (1..=m)
        .filter(|n| now_of(*n).date() == day)
        .flat_map(|n| events_of().into_iter().map(move |_| stamp(n)))
        .collect()
}

/// Child side: runs only when the parent test re-launches this binary.
#[test]
#[ignore = "writer process for crash_keeps_only_the_last_days_whole_writes"]
fn writer_process() {
    let Some(dir) = std::env::var_os(WRITER_DIR_ENV) else {
        return;
    };
    let mut store = Store::open(Path::new(&dir), schema::MIGRATIONS, start().date()).unwrap();
    let newest: Option<String> = store
        .read::<_, StoreError>(|c| Ok(c.query_row("SELECT MAX(at) FROM events", [], |r| r.get(0))?))
        .unwrap();
    let mut n = newest.map_or(0, |t| write_of(&t));
    for _ in 0..MAX_WRITER_COMMITS {
        n += 1;
        let now = now_of(n);
        record(&mut store, now.date(), now, &events_of()).unwrap();
        let mut out = std::io::stdout().lock();
        writeln!(out, "committed {n}").unwrap();
        out.flush().unwrap();
    }
}

#[test]
fn crash_keeps_only_the_last_days_whole_writes() {
    let (_tmp, dir) = test_store_dir();
    let kills = [
        KillAt::AfterCommits(1),
        KillAt::AfterDelay(Duration::from_millis(150)),
        KillAt::AfterCommits(9),
        KillAt::AfterDelay(Duration::from_millis(500)),
        KillAt::AfterCommits(20),
        KillAt::AfterDelay(Duration::from_millis(900)),
        KillAt::AfterCommits(3),
        KillAt::AfterDelay(Duration::from_millis(333)),
    ];
    let mut previous = 0;
    let mut crossed_a_day = false;
    for (round, kill) in kills.into_iter().enumerate() {
        let last = run_writer_until(WRITER, &dir, kill) as i64;
        let store = Store::open(&dir, schema::MIGRATIONS, start().date())
            .unwrap_or_else(|e| panic!("round {round} ({kill:?}): open failed: {e:?}"));
        let check: String = store
            .read::<_, StoreError>(|c| Ok(c.query_row("PRAGMA quick_check", [], |r| r.get(0))?))
            .unwrap();
        assert_eq!(check, "ok", "round {round} ({kill:?})");

        let ats: Vec<String> = store
            .read::<_, StoreError>(|c| {
                let mut stmt = c.prepare("SELECT at FROM events ORDER BY id")?;
                Ok(stmt
                    .query_map([], |r| r.get::<_, String>(0))?
                    .collect::<Result<_, _>>()?)
            })
            .unwrap();
        let committed = ats.last().map_or(0, |t| write_of(t));
        if last > 0 {
            assert!(
                committed == last || committed == last + 1,
                "round {round} ({kill:?}): reported {last}, stored {committed}"
            );
        }
        assert!(committed >= previous, "round {round} ({kill:?}): went back");
        assert_eq!(
            ats,
            model_after(committed),
            "round {round} ({kill:?}): rows differ from the model after {committed} writes"
        );
        if committed > PER_DAY {
            crossed_a_day = true;
        }
        previous = committed;
    }
    assert!(crossed_a_day, "the writer never crossed a day change");
}
