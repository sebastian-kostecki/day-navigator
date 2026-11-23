//! Durability: killing the writer at any moment never loses a reported hide,
//! mark, dismiss or unhide, and never leaves a half-applied record.

mod common;

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use chrono::NaiveDate;
use common::crash::{KillAt, MAX_WRITER_COMMITS, WRITER_DIR_ENV, run_writer_until};
use common::test_store_dir;
use day_navigator_lib::cli::GitlabHost;
use day_navigator_lib::gitlab::{ChangeReason, GitlabItem, ItemKind};
use day_navigator_lib::hidden_issues::{self, Transition};
use day_navigator_lib::store::{Store, StoreError, schema};

const WRITER: &str = "writer_process";
const BASE_MS: i64 = 1_800_000_000_000;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 23).unwrap()
}

fn host() -> GitlabHost {
    GitlabHost::parse("gitlab.example.invalid").unwrap()
}

fn labels(n: u64) -> Vec<String> {
    vec![format!("workflow::step{}", n % 5), format!("plain{n}")]
}

fn updated(n: u64) -> String {
    format!("2026-03-02T10:{:02}:00Z", n % 60)
}

fn item(n: u64) -> GitlabItem {
    GitlabItem {
        kind: ItemKind::Issue,
        id: n,
        project_id: 1_000 + n,
        iid: n + 10,
        reference: format!("acme/widgets#{n}"),
        title: format!("Invented issue {n}"),
        web_url: None,
        updated_at: updated(n),
        labels: labels(n),
        draft: false,
        changed: None,
        milestone: None,
    }
}

/// Whether issue `k` is gone once step `k + 1` has run: step `n ≡ 0 (mod 3)`
/// marks issue `n - 1` and dismisses it, step `n ≡ 1` (n > 1) unhides `n - 1`.
fn removed_by_next_step(k: u64) -> bool {
    k % 3 != 1
}

/// Step `n`: hide issue `n`, then dismiss or unhide issue `n - 1` (see
/// `removed_by_next_step`). Every op is its own write and idempotent, so a
/// writer resumed after a kill finishes a half-done step.
fn step(store: &mut Store, n: u64) {
    hidden_issues::hide(store, &host(), &item(n), BASE_MS + n as i64).unwrap();
    let prev = n - 1;
    let row = |store: &Store| {
        hidden_issues::list(store, &host())
            .unwrap()
            .into_iter()
            .find(|r| r.issue_id == prev)
    };
    if n.is_multiple_of(3) {
        if row(store).is_some_and(|r| r.is_hidden()) {
            let mark = Transition::Resurface {
                issue_id: prev,
                hidden_at_ms: BASE_MS + prev as i64,
                reason: ChangeReason::Comment,
            };
            hidden_issues::apply_round(store, &host(), &[mark], BASE_MS + n as i64).unwrap();
        }
        if row(store).is_some_and(|r| !r.is_hidden()) {
            hidden_issues::dismiss(store, &host(), prev).unwrap();
        }
    } else if n % 3 == 1 && n > 1 && row(store).is_some() {
        hidden_issues::unhide(store, &host(), prev).unwrap();
    }
}

/// Child side: runs only when the parent test re-launches this binary.
#[test]
#[ignore = "writer process for crash_never_loses_or_half_applies_a_hide_record"]
fn writer_process() {
    let Some(dir) = std::env::var_os(WRITER_DIR_ENV) else {
        return;
    };
    let mut store = Store::open(Path::new(&dir), schema::MIGRATIONS, today()).unwrap();
    // The newest hidden issue is the step that last started: run it again.
    let started = hidden_issues::list(&store, &host())
        .unwrap()
        .iter()
        .map(|r| r.issue_id)
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
fn crash_never_loses_or_half_applies_a_hide_record() {
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

        let rows = hidden_issues::list(&store, &host()).unwrap();
        let present = |k: u64| rows.iter().any(|r| r.issue_id == k);
        // Every reported step is fully applied: its hide is there, and the
        // issue the following reported step removed is gone.
        for k in 1..=last {
            let gone = k < last && removed_by_next_step(k);
            assert_eq!(
                present(k),
                !gone,
                "round {round} ({kill:?}): issue {k} after reported step {last}"
            );
        }
        // No row is half-applied: each is a whole hide, or a whole mark of an
        // issue whose dismiss had not run yet.
        for r in &rows {
            let n = r.issue_id;
            assert_eq!(r.project_id, 1_000 + n);
            assert_eq!(r.iid, n + 10);
            assert_eq!(r.scoped_labels, [format!("workflow::step{}", n % 5)]);
            assert_eq!(r.baseline_updated_at, updated(n));
            assert_eq!(r.hidden_at_ms, BASE_MS + n as i64);
            assert_eq!(r.absent_since_ms, None);
            if r.is_hidden() {
                assert_eq!(r.settle_after_ms, Some(BASE_MS + n as i64 + 120_000));
                assert_eq!(r.changed_at_ms, None);
            } else {
                assert_eq!(
                    n % 3,
                    2,
                    "round {round} ({kill:?}): only n-1 of a ≡0 step is marked"
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
