//! S-06 crash proof: killing a process that adds work-time entries with a note
//! at any moment leaves the store consistent, never loses a reported commit and
//! never leaves a torn or foreign note.
//!
//! The writer adds entry `k` (start `BASE_MS + k * 2 min`, one minute long)
//! with the invented note `note_for(k)`, one commit each, and does not pause
//! inside a transaction, so kills land between commits (see
//! `tests/work_time_crash.rs`).

mod common;

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use chrono::{DateTime, NaiveDate, Utc};
use common::crash::{KillAt, MAX_WRITER_COMMITS, WRITER_DIR_ENV, run_writer_until};
use common::test_store_dir;
use day_navigator_lib::store::{Store, schema};
use day_navigator_lib::work_time::note::NOTE_MAX;
use day_navigator_lib::work_time::rules::LOCAL_FORMAT;
use day_navigator_lib::work_time::{self, WorkTimeError};

const WRITER: &str = "writer_process";
/// Invented base instant (2026-01-01T00:00:00Z) of entry 0.
const BASE_MS: i64 = 1_767_225_600_000;
const MINUTE_MS: i64 = 60_000;
/// Fixed "now", later than every entry the writer can add.
const NOW_MS: i64 = BASE_MS + 400 * 24 * 60 * MINUTE_MS;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 23).unwrap()
}

/// Invented one-line note of entry `k`.
fn note_for(k: u64) -> String {
    format!("Wymyślona notatka {k} — żółć")
}

fn start_ms(k: u64) -> i64 {
    BASE_MS + k as i64 * 2 * MINUTE_MS
}

fn local(ms: i64) -> String {
    DateTime::<Utc>::from_timestamp_millis(ms)
        .unwrap()
        .format(LOCAL_FORMAT)
        .to_string()
}

type Row = (i64, i64, Option<i64>, String);

fn rows(store: &Store) -> Vec<Row> {
    store
        .read::<_, WorkTimeError>(|c| {
            let mut stmt =
                c.prepare("SELECT id, start_ms, end_ms, note FROM work_entries ORDER BY id")?;
            let out = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?;
            Ok(out.collect::<Result<_, _>>()?)
        })
        .unwrap()
}

/// Child side: runs only when the parent test re-launches this binary.
#[test]
#[ignore = "writer process for crash_keeps_entry_notes_consistent"]
fn writer_process() {
    let Some(dir) = std::env::var_os(WRITER_DIR_ENV) else {
        return;
    };
    let mut store = Store::open(Path::new(&dir), schema::MIGRATIONS, today()).unwrap();
    let mut k = rows(&store).len() as u64;
    let report = |n: u64| {
        let mut out = std::io::stdout().lock();
        writeln!(out, "committed {n}").unwrap();
        out.flush().unwrap();
    };
    for _ in 0..MAX_WRITER_COMMITS {
        let start = local(start_ms(k));
        let end = local(start_ms(k) + MINUTE_MS);
        work_time::add(&mut store, &start, &end, &note_for(k), NOW_MS, &Utc).unwrap();
        k += 1;
        report(k);
    }
}

#[test]
fn crash_keeps_entry_notes_consistent() {
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
        assert!(
            all.len() as u64 >= last,
            "round {round} ({kill:?}): lost commit, {} < reported {last}",
            all.len()
        );
        // Rows are exactly entries 0..n of the model: right instants and the
        // note of that very entry, so nothing is torn, shifted or missing.
        for (k, (_, start, end, note)) in all.iter().enumerate() {
            let k = k as u64;
            assert_eq!(
                *start,
                start_ms(k),
                "round {round} ({kill:?}): entry {k} start"
            );
            assert_eq!(
                *end,
                Some(start_ms(k) + MINUTE_MS),
                "round {round} ({kill:?}): entry {k} end"
            );
            assert_eq!(
                *note,
                note_for(k),
                "round {round} ({kill:?}): entry {k} note"
            );
            assert!(note.chars().count() <= NOTE_MAX);
            assert!(!note.contains(['\r', '\n']));
        }
    }
}
