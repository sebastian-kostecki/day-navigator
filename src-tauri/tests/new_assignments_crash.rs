//! Durability: killing the writer at any moment never leaves a partial round,
//! nor a seeded account without that round's rows. A round adds, refreshes
//! and forgets rows of both kinds, and the reopened rows must equal a model of
//! some whole round exactly (first and last instants included).

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::Path;
use std::time::Duration;

use chrono::NaiveDate;
use common::crash::{KillAt, MAX_WRITER_COMMITS, WRITER_DIR_ENV, run_writer_until};
use common::test_store_dir;
use day_navigator_lib::gitlab::ItemKind;
use day_navigator_lib::new_assignments::{FORGET_AFTER_MS, ItemKey, record_round};
use day_navigator_lib::store::{Store, StoreError, schema};

const WRITER: &str = "writer_process";
const ACCOUNT: &str = "invented-crash-account";
const BASE_MS: i64 = 1_800_000_000_000;
/// Rounds are 11 days apart, so an id absent for three rounds (33 days) is
/// forgotten while one absent for two (22 days) is kept.
const STEP_MS: i64 = 11 * 86_400_000;
/// Issues per round: round `n` holds issue ids `n ..= n + WIDTH - 1`.
const WIDTH: u64 = 4;

/// A stored row as the test sees it: (kind text, id) -> (first, last seen).
type Rows = BTreeMap<(String, u64), (i64, i64)>;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 23).unwrap()
}

fn now_of(n: u64) -> i64 {
    BASE_MS + n as i64 * STEP_MS
}

/// Round `n`: a sliding window of issues, plus on even rounds an MR sharing
/// the first issue's id, so both kinds are inserted, refreshed and forgotten.
fn round_spec(n: u64) -> Vec<(&'static str, u64)> {
    let mut spec: Vec<_> = (n..n + WIDTH).map(|id| ("issue", id)).collect();
    if n.is_multiple_of(2) {
        spec.push(("mergeRequest", n));
    }
    spec
}

fn round_keys(n: u64) -> Vec<ItemKey> {
    round_spec(n)
        .into_iter()
        .map(|(kind, id)| {
            let kind = match kind {
                "issue" => ItemKind::Issue,
                _ => ItemKind::MergeRequest,
            };
            ItemKey::new(kind, id)
        })
        .collect()
}

/// The rows after rounds `1..=m`, computed without the store.
fn model_after(m: u64) -> Rows {
    let mut rows = Rows::new();
    for n in 1..=m {
        let now = now_of(n);
        let current: BTreeSet<(String, u64)> = round_spec(n)
            .into_iter()
            .map(|(kind, id)| (kind.to_string(), id))
            .collect();
        for key in &current {
            rows.entry(key.clone())
                .and_modify(|r| r.1 = now)
                .or_insert((now, now));
        }
        rows.retain(|key, r| current.contains(key) || now - r.1 < FORGET_AFTER_MS);
    }
    rows
}

/// Child side: runs only when the parent test re-launches this binary.
#[test]
#[ignore = "writer process for crash_never_leaves_a_partial_round"]
fn writer_process() {
    let Some(dir) = std::env::var_os(WRITER_DIR_ENV) else {
        return;
    };
    let mut store = Store::open(Path::new(&dir), schema::MIGRATIONS, today()).unwrap();
    // Every round stores its keys with its own instant, so the newest
    // `last_seen_ms` names the last committed round; continue after it.
    let newest: Option<i64> = store
        .read::<_, StoreError>(|c| {
            Ok(
                c.query_row("SELECT MAX(last_seen_ms) FROM seen_assignments", [], |r| {
                    r.get(0)
                })?,
            )
        })
        .unwrap();
    let mut n = newest.map_or(0, |t| ((t - BASE_MS) / STEP_MS) as u64);
    for _ in 0..MAX_WRITER_COMMITS {
        n += 1;
        record_round(&mut store, ACCOUNT, &round_keys(n), &[], now_of(n)).unwrap();
        let mut out = std::io::stdout().lock();
        writeln!(out, "committed {n}").unwrap();
        out.flush().unwrap();
    }
}

#[test]
fn crash_never_leaves_a_partial_round() {
    // The model forgets both kinds, or the test would not cover the delete:
    // issue 1 (last seen in round 1) and MR 2 (round 2) are gone by round 6,
    // while issue 4 (absent since round 5, 22 days) is still kept.
    let six = model_after(6);
    assert!(!six.contains_key(&("issue".to_string(), 1)));
    assert!(!six.contains_key(&("mergeRequest".to_string(), 2)));
    assert!(six.contains_key(&("issue".to_string(), 4)));
    assert!(!round_spec(6).contains(&("issue", 4)));

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
    let mut previous = 0;
    for (round, kill) in kills.into_iter().enumerate() {
        let last = run_writer_until(WRITER, &dir, kill);

        let store = Store::open(&dir, schema::MIGRATIONS, today())
            .unwrap_or_else(|e| panic!("round {round} ({kill:?}): open failed: {e:?}"));
        let check: String = store
            .read::<_, StoreError>(|c| Ok(c.query_row("PRAGMA quick_check", [], |r| r.get(0))?))
            .unwrap();
        assert_eq!(check, "ok", "round {round} ({kill:?})");

        let (seeded, rows): (bool, Rows) = store
            .read::<_, StoreError>(|c| {
                let seeded: bool = c.query_row(
                    "SELECT EXISTS(SELECT 1 FROM assignment_accounts WHERE account = ?1)",
                    [ACCOUNT],
                    |r| r.get(0),
                )?;
                let mut stmt = c.prepare(
                    "SELECT kind, item_id, first_seen_ms, last_seen_ms
                     FROM seen_assignments WHERE account = ?1",
                )?;
                let rows = stmt
                    .query_map([ACCOUNT], |r| {
                        Ok((
                            (r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u64),
                            (r.get::<_, i64>(2)?, r.get::<_, i64>(3)?),
                        ))
                    })?
                    .collect::<Result<Rows, _>>()?;
                Ok((seeded, rows))
            })
            .unwrap();

        // The newest instant names the last whole round in the database.
        let committed = rows
            .values()
            .map(|r| r.1)
            .max()
            .map_or(0, |t| ((t - BASE_MS) / STEP_MS) as u64);
        // A seeded account always has its round's rows, and the reverse.
        assert_eq!(
            seeded,
            committed > 0,
            "round {round} ({kill:?}): marker and rows disagree"
        );
        // Every reported round is stored; at most one more committed before
        // the kill landed, ahead of its report.
        if last > 0 {
            assert!(
                committed == last || committed == last + 1,
                "round {round} ({kill:?}): reported {last}, stored {committed}"
            );
        }
        assert!(committed >= previous, "round {round} ({kill:?}): went back");
        previous = committed;
        // The rows are exactly those of a whole round: no torn insert,
        // upsert or forget.
        assert_eq!(
            rows,
            model_after(committed),
            "round {round} ({kill:?}): rows differ from whole round {committed}"
        );
    }
    assert!(previous > 4, "the writer never reached a forgetting round");
}
