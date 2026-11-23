//! S-08 crash proof: killing a process that loops START/STOP at any moment
//! leaves the store consistent and never loses a reported commit.

mod common;

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use chrono::NaiveDate;
use common::crash::{KillAt, MAX_WRITER_COMMITS, WRITER_DIR_ENV, run_writer_until};
use common::test_store_dir;
use day_navigator_lib::store::{Store, schema};
use day_navigator_lib::work_time::{self, WorkTimeError};

const WRITER: &str = "writer_process";
/// Invented base instant (2026-01-01T00:00:00Z) for the looped actions.
const BASE_MS: i64 = 1_767_225_600_000;
const STEP_MS: i64 = 1_000;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 23).unwrap()
}

type Row = (i64, i64, Option<i64>, Option<i64>);

fn rows(store: &Store) -> Vec<Row> {
    store
        .read::<_, WorkTimeError>(|c| {
            let mut stmt = c.prepare(
                "SELECT id, start_ms, end_ms, removed_at_ms FROM work_entries ORDER BY start_ms",
            )?;
            let out = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?;
            Ok(out.collect::<Result<_, _>>()?)
        })
        .unwrap()
}

/// Every START and every STOP that was committed.
fn actions(rows: &[Row]) -> u64 {
    rows.iter().map(|r| if r.2.is_some() { 2 } else { 1 }).sum()
}

/// Child side: runs only when the parent test re-launches this binary.
#[test]
#[ignore = "writer process for crash_keeps_work_entries_consistent"]
fn writer_process() {
    let Some(dir) = std::env::var_os(WRITER_DIR_ENV) else {
        return;
    };
    let mut store = Store::open(Path::new(&dir), schema::MIGRATIONS, today()).unwrap();
    let existing = rows(&store);
    let mut n = actions(&existing);
    // Continue after the latest stored instant, one step per action.
    let mut now = existing
        .iter()
        .flat_map(|r| [Some(r.1), r.2])
        .flatten()
        .max()
        .map_or(BASE_MS, |m| m + STEP_MS);
    let report = |n: u64| {
        let mut out = std::io::stdout().lock();
        writeln!(out, "committed {n}").unwrap();
        out.flush().unwrap();
    };
    for _ in 0..MAX_WRITER_COMMITS {
        // A previous kill may have landed after START: stop first.
        if work_time::is_running(&store).unwrap() {
            work_time::stop(&mut store, now).unwrap();
        } else {
            work_time::start(&mut store, now).unwrap();
        }
        n += 1;
        now += STEP_MS;
        report(n);
    }
}

#[test]
fn crash_keeps_work_entries_consistent() {
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

        let all = rows(&store);
        let running = all
            .iter()
            .filter(|r| r.2.is_none() && r.3.is_none())
            .count();
        assert!(running <= 1, "round {round} ({kill:?}): {running} running");
        for r in &all {
            if let Some(end) = r.2 {
                assert!(end > r.1, "round {round} ({kill:?}): end not after start");
            }
        }
        // Counted entries, ordered by start, never overlap.
        let counted: Vec<&Row> = all.iter().filter(|r| r.3.is_none()).collect();
        for w in counted.windows(2) {
            let end = w[0].2.expect("only the last entry may be running");
            assert!(end <= w[1].1, "round {round} ({kill:?}): overlap");
        }
        assert!(
            actions(&all) >= last,
            "round {round} ({kill:?}): lost commit, {} < reported {last}",
            actions(&all)
        );
    }
}
