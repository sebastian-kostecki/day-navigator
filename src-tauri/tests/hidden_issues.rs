//! Hidden-issue records against a real store with the production migrations.

mod common;

use chrono::NaiveDate;
use common::test_store_dir;
use day_navigator_lib::cli::GitlabHost;
use day_navigator_lib::gitlab::{ChangeReason, GitlabItem, ItemKind};
use day_navigator_lib::hidden_issues::{self, HiddenIssueError, SETTLE_MS, Transition};
use day_navigator_lib::store::{Store, StoreError, schema};

const NOW: i64 = 1_800_000_000_000;
const DAY: i64 = 86_400_000;
const T0: &str = "2026-03-02T10:00:00Z";
const T1: &str = "2026-03-02T10:05:00Z";

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 23).unwrap()
}

fn host() -> GitlabHost {
    GitlabHost::parse("gitlab.example.invalid").unwrap()
}

fn other_host() -> GitlabHost {
    GitlabHost::parse("other.example.invalid").unwrap()
}

fn open(dir: &std::path::Path) -> Store {
    Store::open(dir, schema::MIGRATIONS, today()).unwrap()
}

fn issue(id: u64, labels: &[&str]) -> GitlabItem {
    GitlabItem {
        kind: ItemKind::Issue,
        id,
        project_id: 7,
        iid: id + 100,
        reference: format!("acme/widgets#{id}"),
        title: format!("Invented issue {id}"),
        web_url: None,
        updated_at: T0.into(),
        labels: labels.iter().map(|s| s.to_string()).collect(),
        draft: false,
        changed: None,
        milestone: None,
        is_new: false,
        hidden_at_ms: None,
    }
}

fn user_version(store: &Store) -> i64 {
    store
        .read::<_, StoreError>(|c| Ok(c.query_row("PRAGMA user_version", [], |r| r.get(0))?))
        .unwrap()
}

fn exec(store: &mut Store, sql: &str) -> Result<(), StoreError> {
    store.write::<_, StoreError>(|tx| {
        tx.execute_batch(sql)?;
        Ok(())
    })
}

fn resurface(store: &mut Store, id: u64, reason: ChangeReason) {
    let hidden_at_ms = hidden_issues::list(store, &host())
        .unwrap()
        .iter()
        .find(|r| r.issue_id == id)
        .unwrap()
        .hidden_at_ms;
    hidden_issues::apply_round(
        store,
        &host(),
        &[Transition::Resurface {
            issue_id: id,
            hidden_at_ms,
            reason,
        }],
        NOW + 1,
    )
    .unwrap();
}

#[test]
fn v6_applies_on_a_fresh_store() {
    let (_tmp, dir) = test_store_dir();
    let store = open(&dir);
    assert_eq!(
        user_version(&store),
        i64::from(schema::MIGRATIONS.last().unwrap().version)
    );
    assert!(schema::MIGRATIONS.len() >= 6);
    assert!(hidden_issues::list(&store, &host()).unwrap().is_empty());
}

/// `hidden_issues` was planned as v3 and renumbered to v6 when merged after
/// S-07 (v3, v4) and S-09 (v5): a v5 store with data gets it on open.
#[test]
fn v6_applies_on_a_v5_store_with_data() {
    let (_tmp, dir) = test_store_dir();
    {
        let mut v5 = Store::open(&dir, &schema::MIGRATIONS[..5], today()).unwrap();
        exec(
            &mut v5,
            "INSERT INTO own_tasks(space, title, created_at, updated_at) \
             VALUES ('work', 'Zadanie testowe', '2026-11-23T08:00:00', '2026-11-23T08:00:00');
             INSERT INTO work_entries(start_ms, end_ms, created_at_ms, updated_at_ms) \
             VALUES (1000, 2000, 1000, 2000);",
        )
        .unwrap();
        assert_eq!(user_version(&v5), 5);
    }
    let store = open(&dir);
    assert_eq!(
        user_version(&store),
        i64::from(schema::MIGRATIONS.last().unwrap().version)
    );
    let counts: (i64, i64) = store
        .read::<_, StoreError>(|c| {
            Ok((
                c.query_row("SELECT count(*) FROM own_tasks", [], |r| r.get(0))?,
                c.query_row("SELECT count(*) FROM work_entries", [], |r| r.get(0))?,
            ))
        })
        .unwrap();
    assert_eq!(counts, (1, 1));
    assert!(hidden_issues::list(&store, &host()).unwrap().is_empty());
}

#[test]
fn hide_unhide_dismiss_round_trip() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    hidden_issues::hide(&mut store, &host(), &issue(1, &["workflow::doing"]), NOW).unwrap();
    hidden_issues::hide(&mut store, &host(), &issue(2, &[]), NOW).unwrap();

    let rows = hidden_issues::list(&store, &host()).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].issue_id, 1);
    assert_eq!(rows[0].scoped_labels, ["workflow::doing"]);
    assert_eq!(rows[0].baseline_updated_at, T0);
    assert_eq!(rows[0].settle_after_ms, Some(NOW + SETTLE_MS));
    assert_eq!(rows[0].hidden_at_ms, NOW);
    assert_eq!((rows[0].project_id, rows[0].iid), (7, 101));
    assert!(rows[0].is_hidden());

    // A hidden row cannot be dismissed; a marked row cannot be unhidden.
    assert!(matches!(
        hidden_issues::dismiss(&mut store, &host(), 1),
        Err(HiddenIssueError::NotFound)
    ));
    resurface(&mut store, 2, ChangeReason::Comment);
    assert!(matches!(
        hidden_issues::unhide(&mut store, &host(), 2),
        Err(HiddenIssueError::NotFound)
    ));

    hidden_issues::unhide(&mut store, &host(), 1).unwrap();
    hidden_issues::dismiss(&mut store, &host(), 2).unwrap();
    assert!(hidden_issues::list(&store, &host()).unwrap().is_empty());
}

#[test]
fn hide_over_a_marked_row_resets_it() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    hidden_issues::hide(&mut store, &host(), &issue(1, &["workflow::doing"]), NOW).unwrap();
    resurface(&mut store, 1, ChangeReason::Status);
    let marked = &hidden_issues::list(&store, &host()).unwrap()[0];
    assert_eq!(marked.changed_reason, Some(ChangeReason::Status));
    assert_eq!(marked.changed_at_ms, Some(NOW + 1));

    let mut now = issue(1, &["workflow::done"]);
    now.updated_at = T1.into();
    hidden_issues::hide(&mut store, &host(), &now, NOW + 5).unwrap();
    let rows = hidden_issues::list(&store, &host()).unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].is_hidden());
    assert_eq!(rows[0].changed_at_ms, None);
    assert_eq!(rows[0].absent_since_ms, None);
    assert_eq!(rows[0].scoped_labels, ["workflow::done"]);
    assert_eq!(rows[0].baseline_updated_at, T1);
    assert_eq!(rows[0].hidden_at_ms, NOW + 5);
}

#[test]
fn records_survive_close_and_reopen() {
    let (_tmp, dir) = test_store_dir();
    {
        let mut store = open(&dir);
        hidden_issues::hide(&mut store, &host(), &issue(1, &["a::b"]), NOW).unwrap();
        hidden_issues::hide(&mut store, &host(), &issue(2, &[]), NOW).unwrap();
        resurface(&mut store, 2, ChangeReason::Returned);
    }
    let store = open(&dir);
    let rows = hidden_issues::list(&store, &host()).unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows[0].is_hidden());
    assert_eq!(rows[1].changed_reason, Some(ChangeReason::Returned));
}

#[test]
fn records_are_per_host() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    hidden_issues::hide(&mut store, &host(), &issue(1, &[]), NOW).unwrap();
    assert!(
        hidden_issues::list(&store, &other_host())
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        hidden_issues::unhide(&mut store, &other_host(), 1),
        Err(HiddenIssueError::NotFound)
    ));
}

#[test]
fn apply_round_skips_stale_transitions_and_returns_rows_after_the_write() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    for id in 1..=5 {
        hidden_issues::hide(&mut store, &host(), &issue(id, &[]), NOW).unwrap();
    }
    // The user unhid 1, re-hid 2 later, and 3 already carries a mark.
    hidden_issues::unhide(&mut store, &host(), 1).unwrap();
    hidden_issues::hide(&mut store, &host(), &issue(2, &[]), NOW + 50).unwrap();
    resurface(&mut store, 3, ChangeReason::Comment);

    let transitions = [
        Transition::Resurface {
            issue_id: 1,
            hidden_at_ms: NOW,
            reason: ChangeReason::Comment,
        },
        Transition::AdvanceBaseline {
            issue_id: 2,
            hidden_at_ms: NOW,
            updated_at: T1.into(),
            settle_after_ms: None,
        },
        Transition::Resurface {
            issue_id: 3,
            hidden_at_ms: NOW,
            reason: ChangeReason::Status,
        },
        Transition::Prune { issue_id: 4 },
        Transition::ClearAbsent { issue_id: 4 },
        Transition::MarkAbsent { issue_id: 5 },
        Transition::MarkAbsent { issue_id: 5 },
    ];
    let rows = hidden_issues::apply_round(&mut store, &host(), &transitions, NOW + 100).unwrap();
    let by_id = |id| rows.iter().find(|r| r.issue_id == id);

    assert!(by_id(1).is_none(), "gone row stays gone");
    let r2 = by_id(2).unwrap();
    assert_eq!(r2.baseline_updated_at, T0, "re-hidden row not advanced");
    assert_eq!(
        by_id(3).unwrap().changed_reason,
        Some(ChangeReason::Comment),
        "a mark is not overwritten"
    );
    assert_eq!(by_id(4).unwrap().absent_since_ms, None, "not absent: kept");
    assert_eq!(by_id(5).unwrap().absent_since_ms, Some(NOW + 100));
    assert_eq!(rows, hidden_issues::list(&store, &host()).unwrap());
}

#[test]
fn apply_round_prunes_only_after_thirty_days_absent() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    hidden_issues::hide(&mut store, &host(), &issue(1, &[]), NOW - 40 * DAY).unwrap();
    hidden_issues::apply_round(
        &mut store,
        &host(),
        &[Transition::MarkAbsent { issue_id: 1 }],
        NOW - 30 * DAY + 1,
    )
    .unwrap();
    let kept = hidden_issues::apply_round(
        &mut store,
        &host(),
        &[Transition::Prune { issue_id: 1 }],
        NOW,
    )
    .unwrap();
    assert_eq!(kept.len(), 1, "one ms short of 30 days");
    let rows = hidden_issues::apply_round(
        &mut store,
        &host(),
        &[Transition::Prune { issue_id: 1 }],
        NOW + 1,
    )
    .unwrap();
    assert!(rows.is_empty());
}

#[test]
fn apply_round_prunes_other_host_rows_hidden_thirty_days_ago() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    hidden_issues::hide(&mut store, &other_host(), &issue(1, &[]), NOW - 30 * DAY).unwrap();
    hidden_issues::hide(
        &mut store,
        &other_host(),
        &issue(2, &[]),
        NOW - 30 * DAY + 1,
    )
    .unwrap();
    hidden_issues::hide(&mut store, &host(), &issue(3, &[]), NOW - 90 * DAY).unwrap();

    let mine = hidden_issues::apply_round(&mut store, &host(), &[], NOW).unwrap();
    assert_eq!(mine.len(), 1, "own host is never pruned by age");
    let left = hidden_issues::list(&store, &other_host()).unwrap();
    assert_eq!(left.iter().map(|r| r.issue_id).collect::<Vec<_>>(), [2]);
}

#[test]
fn check_constraints_back_the_rules() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    let insert = |extra_cols: &str, extra_vals: &str| {
        format!(
            "INSERT INTO hidden_issues(host, issue_id, project_id, iid, scoped_labels, \
             baseline_updated_at, hidden_at_ms{extra_cols}) \
             VALUES ('h', 1, 1, 1, '[]', '{T0}', 0{extra_vals})"
        )
    };
    // Unknown reason.
    assert!(
        exec(
            &mut store,
            &insert(", changed_reason, changed_at_ms", ", 'other', 1")
        )
        .is_err()
    );
    // Reason without a time, and a time without a reason.
    assert!(exec(&mut store, &insert(", changed_reason", ", 'status'")).is_err());
    assert!(exec(&mut store, &insert(", changed_at_ms", ", 1")).is_err());
    // Valid pair, then a duplicate key.
    exec(
        &mut store,
        &insert(", changed_reason, changed_at_ms", ", 'status', 1"),
    )
    .unwrap();
    assert!(exec(&mut store, &insert("", "")).is_err());
}
