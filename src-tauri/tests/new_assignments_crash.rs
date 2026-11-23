//! Durability: killing the writer at any moment never leaves a partial round,
//! nor a seeded account without that round's rows.

mod common;

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use chrono::NaiveDate;
use common::crash::{KillAt, MAX_WRITER_COMMITS, WRITER_DIR_ENV, run_writer_until};
use common::test_store_dir;
use day_navigator_lib::gitlab::ItemKind;
use day_navigator_lib::new_assignments::{ItemKey, record_round};
use day_navigator_lib::store::{Store, StoreError, schema};

const WRITER: &str = "writer_process";
const ACCOUNT: &str = "invented-crash-account";
const BASE_MS: i64 = 1_800_000_000_000;
/// Items per round: round `n` holds ids `n ..= n + WIDTH - 1`, so each round
/// adds one id and keeps the rest (gaps stay far under 30 days).
const WIDTH: u64 = 4;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 23).unwrap()
}

fn round_keys(n: u64) -> Vec<ItemKey> {
    (n..n + WIDTH)
        .map(|id| ItemKey::new(ItemKind::Issue, id))
        .collect()
}

/// Child side: runs only when the parent test re-launches this binary.
#[test]
#[ignore = "writer process for crash_never_leaves_a_partial_round"]
fn writer_process() {
    let Some(dir) = std::env::var_os(WRITER_DIR_ENV) else {
        return;
    };
    let mut store = Store::open(Path::new(&dir), schema::MIGRATIONS, today()).unwrap();
    // Continue from the newest stored id: a round is replayable, so one killed
    // halfway is simply run again.
    let newest: i64 = store
        .read::<_, StoreError>(|c| {
            Ok(c.query_row(
                "SELECT COALESCE(MAX(item_id), 0) FROM seen_assignments",
                [],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    let mut n = (newest as u64).saturating_sub(WIDTH - 1).max(1) - 1;
    for _ in 0..MAX_WRITER_COMMITS {
        n += 1;
        record_round(&mut store, ACCOUNT, &round_keys(n), BASE_MS + n as i64).unwrap();
        let mut out = std::io::stdout().lock();
        writeln!(out, "committed {n}").unwrap();
        out.flush().unwrap();
    }
}

#[test]
fn crash_never_leaves_a_partial_round() {
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
    ];
    for (round, kill) in kills.into_iter().enumerate() {
        let last = run_writer_until(WRITER, &dir, kill);

        let store = Store::open(&dir, schema::MIGRATIONS, today())
            .unwrap_or_else(|e| panic!("round {round} ({kill:?}): open failed: {e:?}"));
        let check: String = store
            .read::<_, StoreError>(|c| Ok(c.query_row("PRAGMA quick_check", [], |r| r.get(0))?))
            .unwrap();
        assert_eq!(check, "ok", "round {round} ({kill:?})");

        let (seeded, ids): (bool, Vec<u64>) = store
            .read::<_, StoreError>(|c| {
                let seeded: bool = c.query_row(
                    "SELECT EXISTS(SELECT 1 FROM assignment_accounts WHERE account = ?1)",
                    [ACCOUNT],
                    |r| r.get(0),
                )?;
                let mut stmt = c.prepare(
                    "SELECT item_id FROM seen_assignments WHERE account = ?1 ORDER BY item_id",
                )?;
                let ids = stmt
                    .query_map([ACCOUNT], |r| r.get::<_, i64>(0))?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok((seeded, ids.into_iter().map(|i| i as u64).collect()))
            })
            .unwrap();

        // A seeded account always has its round's rows, and the reverse.
        assert_eq!(
            seeded,
            !ids.is_empty(),
            "round {round} ({kill:?}): marker and rows disagree"
        );
        if last > 0 {
            // A reported round is whole: all of its ids are stored.
            for id in last..last + WIDTH {
                assert!(
                    ids.contains(&id),
                    "round {round} ({kill:?}): reported round {last} lost an id"
                );
            }
        }
        // Rows form one gap-free run ending on a whole round: the newest id
        // is the last id of some round, never a partial tail.
        if let (Some(&min), Some(&max)) = (ids.first(), ids.last()) {
            assert_eq!(
                ids.len() as u64,
                max - min + 1,
                "round {round} ({kill:?}): gap in {ids:?}"
            );
            assert!(
                max >= WIDTH,
                "round {round} ({kill:?}): partial first round"
            );
        }
    }
}
