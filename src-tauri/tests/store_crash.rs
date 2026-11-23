//! F-01 proof: killing the writer process at any moment never leaves a
//! half-applied write and never loses a commit that was reported.

mod common;

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use chrono::NaiveDate;
use common::crash::{KillAt, MAX_WRITER_COMMITS, WRITER_DIR_ENV, run_writer_until};
use common::test_store_dir;
use day_navigator_lib::store::{Migration, Store, StoreError};

const TICKS: Migration = Migration {
    version: 1,
    sql: "CREATE TABLE ticks(n INTEGER PRIMARY KEY, payload BLOB NOT NULL);
          CREATE TABLE counter(id INTEGER PRIMARY KEY CHECK (id = 1), n INTEGER NOT NULL);
          INSERT INTO counter(id, n) VALUES (1, 0);",
};

const WRITER: &str = "writer_process";

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 23).unwrap()
}

/// Child side: runs only when the parent test re-launches this binary.
#[test]
#[ignore = "writer process for crash_never_loses_or_half_applies_a_write"]
fn writer_process() {
    let Some(dir) = std::env::var_os(WRITER_DIR_ENV) else {
        return;
    };
    let mut store = Store::open(Path::new(&dir), &[TICKS], today()).unwrap();
    let mut n: i64 = store
        .read::<_, StoreError>(|c| {
            Ok(c.query_row("SELECT coalesce(max(n), 0) FROM ticks", [], |r| r.get(0))?)
        })
        .unwrap();
    let payload = vec![0xA5u8; 64 * 1024];
    for _ in 0..MAX_WRITER_COMMITS {
        n += 1;
        store
            .write::<_, StoreError>(|tx| {
                tx.execute(
                    "INSERT INTO ticks(n, payload) VALUES (?1, ?2)",
                    (n, &payload),
                )?;
                // Widens the window between the two statements so a kill can land inside the transaction.
                std::thread::sleep(std::time::Duration::from_millis(2));
                tx.execute("UPDATE counter SET n = ?1 WHERE id = 1", [n])?;
                Ok(())
            })
            .unwrap();
        let mut out = std::io::stdout().lock();
        writeln!(out, "committed {n}").unwrap();
        out.flush().unwrap();
    }
}

#[test]
fn crash_never_loses_or_half_applies_a_write() {
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
        KillAt::AfterCommits(40),
        KillAt::AfterDelay(Duration::from_millis(1200)),
    ];
    for (round, kill) in kills.into_iter().enumerate() {
        let last = run_writer_until(WRITER, &dir, kill);

        let store = Store::open(&dir, &[TICKS], today())
            .unwrap_or_else(|e| panic!("round {round} ({kill:?}): open failed: {e:?}"));
        let (check, count, max, counter): (String, i64, i64, i64) = store
            .read::<_, StoreError>(|c| {
                Ok((
                    c.query_row("PRAGMA quick_check", [], |r| r.get(0))?,
                    c.query_row("SELECT count(*) FROM ticks", [], |r| r.get(0))?,
                    c.query_row("SELECT coalesce(max(n), 0) FROM ticks", [], |r| r.get(0))?,
                    c.query_row("SELECT n FROM counter WHERE id = 1", [], |r| r.get(0))?,
                ))
            })
            .unwrap();
        assert_eq!(check, "ok", "round {round} ({kill:?})");
        assert_eq!(count, max, "round {round} ({kill:?}): gap in ticks");
        assert_eq!(
            max, counter,
            "round {round} ({kill:?}): counter out of step"
        );
        assert!(
            max >= last as i64,
            "round {round} ({kill:?}): lost commit, max {max} < reported {last}"
        );
    }
}
