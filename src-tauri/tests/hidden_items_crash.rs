//! Durability: killing the writer at any moment never loses a reported hide,
//! mark, dismiss or unhide of a `hidden_items` row, and never leaves a
//! half-applied record. Invented repos and ids only.

mod common;

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use chrono::NaiveDate;
use common::crash::{KillAt, MAX_WRITER_COMMITS, WRITER_DIR_ENV, run_writer_until};
use common::test_store_dir;
use day_navigator_lib::gitlab::ChangeReason;
use day_navigator_lib::hidden_issues::Transition;
use day_navigator_lib::hidden_issues::items::{
    HiddenItemKey, HiddenItemRecord, HiddenKind, HiddenSource, NewHiddenItem, apply_items_round,
    dismiss_item, hide_item, list_items, unhide_item,
};
use day_navigator_lib::store::{Store, StoreError, schema};

const WRITER: &str = "writer_process";
const BASE_MS: i64 = 1_800_000_000_000;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 28).unwrap()
}

fn scope(n: u64) -> String {
    format!("octo-test/repo{}", n % 3)
}

fn updated(n: u64) -> String {
    format!("2026-03-02T10:{:02}:00Z", n % 60)
}

fn new_item(n: u64) -> NewHiddenItem {
    NewHiddenItem {
        source: HiddenSource::Github,
        scope: scope(n),
        kind: HiddenKind::Issue,
        item_id: n,
        project_id: None,
        iid: None,
        status_fingerprint: Vec::new(),
        baseline_updated_at: updated(n),
    }
}

fn key(n: u64) -> HiddenItemKey {
    HiddenItemKey {
        source: HiddenSource::Github,
        scope: scope(n),
        kind: HiddenKind::Issue,
        item_id: n,
    }
}

/// Whether item `k` is gone once step `k + 1` has run: step `n` divisible by 3
/// marks item `n - 1` and dismisses it, a step `n` with `n % 3 == 1` (n > 1)
/// unhides `n - 1`.
fn removed_by_next_step(k: u64) -> bool {
    k % 3 != 1
}

fn row(store: &Store, n: u64) -> Option<HiddenItemRecord> {
    list_items(store, HiddenSource::Github, HiddenKind::Issue)
        .unwrap()
        .into_iter()
        .find(|r| r.item_id == n)
}

/// Step `n`: hide item `n`, then dismiss or unhide item `n - 1`. Every op is
/// its own write and idempotent, so a writer resumed after a kill finishes a
/// half-done step.
fn step(store: &mut Store, n: u64) {
    hide_item(store, &new_item(n), BASE_MS + n as i64).unwrap();
    let prev = n - 1;
    if n.is_multiple_of(3) {
        if row(store, prev).is_some_and(|r| r.changed_reason.is_none()) {
            let mark = Transition::Resurface {
                issue_id: prev,
                hidden_at_ms: BASE_MS + prev as i64,
                reason: ChangeReason::Comment,
            };
            let all: Vec<String> = (0..3).map(scope).collect();
            apply_items_round(
                store,
                HiddenSource::Github,
                HiddenKind::Issue,
                &[(scope(prev), vec![mark])],
                &all,
                BASE_MS + n as i64,
            )
            .unwrap();
        }
        if row(store, prev).is_some_and(|r| r.changed_reason.is_some()) {
            dismiss_item(store, &key(prev)).unwrap();
        }
    } else if n % 3 == 1 && n > 1 && row(store, prev).is_some() {
        unhide_item(store, &key(prev)).unwrap();
    }
}

/// Child side: runs only when the parent test re-launches this binary.
#[test]
#[ignore = "writer process for crash_never_loses_or_half_applies_a_hidden_item"]
fn writer_process() {
    let Some(dir) = std::env::var_os(WRITER_DIR_ENV) else {
        return;
    };
    let mut store = Store::open(Path::new(&dir), schema::MIGRATIONS, today()).unwrap();
    let started = list_items(&store, HiddenSource::Github, HiddenKind::Issue)
        .unwrap()
        .iter()
        .map(|r| r.item_id)
        .max()
        .unwrap_or(0);
    let mut n = started.saturating_sub(1);
    for _ in 0..MAX_WRITER_COMMITS {
        n += 1;
        step(&mut store, n);
        let mut out = std::io::stdout().lock();
        writeln!(out, "committed {n}").unwrap();
        out.flush().unwrap();
    }
}

#[test]
fn crash_never_loses_or_half_applies_a_hidden_item() {
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

        let rows = list_items(&store, HiddenSource::Github, HiddenKind::Issue).unwrap();
        let present = |k: u64| rows.iter().any(|r| r.item_id == k);
        // The unreported step `last + 1` may already have removed `last`.
        let next_started = present(last + 1);
        for k in 1..=last {
            if k == last && removed_by_next_step(k) && next_started {
                continue;
            }
            let gone = k < last && removed_by_next_step(k);
            assert_eq!(
                present(k),
                !gone,
                "round {round} ({kill:?}): item {k} after reported step {last}"
            );
        }
        for r in &rows {
            let n = r.item_id;
            assert_eq!(r.scope, scope(n));
            assert_eq!(r.project_id, None);
            assert_eq!(r.iid, None);
            assert!(r.status_fingerprint.is_empty());
            assert_eq!(r.baseline_updated_at, updated(n));
            assert_eq!(r.hidden_at_ms, BASE_MS + n as i64);
            assert_eq!(r.absent_since_ms, None);
            if r.changed_reason.is_none() {
                assert_eq!(r.settle_after_ms, Some(BASE_MS + n as i64 + 120_000));
                assert_eq!(r.changed_at_ms, None);
            } else {
                assert_eq!(
                    n % 3,
                    2,
                    "round {round} ({kill:?}): only n-1 of a divisible-by-3 step is marked"
                );
                assert!(
                    n >= last,
                    "round {round} ({kill:?}): a reported dismiss was lost"
                );
                assert_eq!(r.changed_reason, Some(ChangeReason::Comment));
                assert_eq!(r.changed_at_ms, Some(BASE_MS + n as i64 + 1));
                assert_eq!(r.settle_after_ms, None);
            }
        }
    }
}
