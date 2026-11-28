//! `hidden_items` records against a real store with the production migrations.
//! Invented repos (`octo-test/*`) and ids only.

mod common;

use chrono::NaiveDate;
use common::test_store_dir;
use day_navigator_lib::cli::GitlabHost;
use day_navigator_lib::gitlab::{ChangeReason, GitlabItem, ItemKind};
use day_navigator_lib::hidden_issues::items::{
    HiddenItemKey, HiddenKind, HiddenSource, NewHiddenItem, apply_items_round, dismiss_item,
    hide_item, list_items, unhide_item,
};
use day_navigator_lib::hidden_issues::{self, HiddenIssueError, PRUNE_AFTER_MS, SETTLE_MS};
use day_navigator_lib::hidden_issues::{Transition, rules::HiddenRecord};
use day_navigator_lib::store::{Store, StoreError, schema};

const NOW: i64 = 1_800_000_000_000;
const T0: &str = "2026-03-02T10:00:00Z";
const T1: &str = "2026-03-02T10:05:00Z";
const ALPHA: &str = "octo-test/alpha";
const BETA: &str = "octo-test/beta";

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 28).unwrap()
}

fn open(dir: &std::path::Path) -> Store {
    Store::open(dir, schema::MIGRATIONS, today()).unwrap()
}

fn scalar(store: &Store, sql: &'static str) -> i64 {
    store
        .read::<_, StoreError>(|c| Ok(c.query_row(sql, [], |r| r.get(0))?))
        .unwrap()
}

fn new_item(scope: &str, id: u64) -> NewHiddenItem {
    NewHiddenItem {
        source: HiddenSource::Github,
        scope: scope.into(),
        kind: HiddenKind::Issue,
        item_id: id,
        project_id: None,
        iid: None,
        status_fingerprint: Vec::new(),
        baseline_updated_at: T0.into(),
    }
}

fn key(scope: &str, id: u64) -> HiddenItemKey {
    HiddenItemKey {
        source: HiddenSource::Github,
        scope: scope.into(),
        kind: HiddenKind::Issue,
        item_id: id,
    }
}

fn rows(store: &Store) -> Vec<(String, u64, bool)> {
    list_items(store, HiddenSource::Github, HiddenKind::Issue)
        .unwrap()
        .into_iter()
        .map(|r| (r.scope.clone(), r.item_id, r.changed_reason.is_none()))
        .collect()
}

fn apply(
    store: &mut Store,
    scope: &str,
    t: Vec<Transition>,
    active: &[&str],
    now: i64,
) -> Vec<(String, u64, bool)> {
    let active: Vec<String> = active.iter().map(|s| s.to_string()).collect();
    apply_items_round(
        store,
        HiddenSource::Github,
        HiddenKind::Issue,
        &[(scope.to_string(), t)],
        &active,
        now,
    )
    .unwrap()
    .into_iter()
    .map(|r| (r.scope, r.item_id, r.changed_reason.is_none()))
    .collect()
}

#[test]
fn debug_and_errors_do_not_print_scope_or_number() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    let scope = "octo-test/leakmarker";
    hide_item(&mut store, &new_item(scope, 987_654), NOW).unwrap();
    let rec = list_items(&store, HiddenSource::Github, HiddenKind::Issue)
        .unwrap()
        .remove(0);
    let text = format!("{rec:?}");
    assert!(
        !text.contains("leakmarker") && !text.contains("987654"),
        "{text}"
    );
    for err in [
        unhide_item(&mut store, &key(scope, 987_655)).unwrap_err(),
        dismiss_item(&mut store, &key(scope, 987_654)).unwrap_err(),
    ] {
        for text in [format!("{err:?}"), err.to_string()] {
            assert!(
                !text.contains("leakmarker")
                    && !text.contains("987654")
                    && !text.contains("987655"),
                "{text}"
            );
        }
    }
}

#[test]
fn hide_list_unhide_dismiss_round_trip() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    hide_item(&mut store, &new_item(ALPHA, 2), NOW).unwrap();
    hide_item(&mut store, &new_item(ALPHA, 1), NOW).unwrap();
    hide_item(&mut store, &new_item(BETA, 1), NOW).unwrap();
    let all = list_items(&store, HiddenSource::Github, HiddenKind::Issue).unwrap();
    // Ordered by scope, then id.
    assert_eq!(
        rows(&store),
        [
            (ALPHA.to_string(), 1, true),
            (ALPHA.to_string(), 2, true),
            (BETA.to_string(), 1, true)
        ]
    );
    assert_eq!(all[0].settle_after_ms, Some(NOW + SETTLE_MS));
    assert_eq!(all[0].hidden_at_ms, NOW);
    assert!(
        list_items(&store, HiddenSource::Github, HiddenKind::MergeRequest)
            .unwrap()
            .is_empty()
    );

    // Wrong state is NotFound.
    assert!(matches!(
        dismiss_item(&mut store, &key(ALPHA, 1)),
        Err(HiddenIssueError::NotFound)
    ));
    assert!(matches!(
        unhide_item(&mut store, &key(ALPHA, 9)),
        Err(HiddenIssueError::NotFound)
    ));

    apply(
        &mut store,
        ALPHA,
        vec![Transition::Resurface {
            issue_id: 2,
            hidden_at_ms: NOW,
            reason: ChangeReason::Comment,
        }],
        &[ALPHA, BETA],
        NOW + 1,
    );
    assert!(matches!(
        unhide_item(&mut store, &key(ALPHA, 2)),
        Err(HiddenIssueError::NotFound)
    ));
    dismiss_item(&mut store, &key(ALPHA, 2)).unwrap();
    unhide_item(&mut store, &key(ALPHA, 1)).unwrap();
    assert_eq!(rows(&store), [(BETA.to_string(), 1, true)]);
}

#[test]
fn rehiding_a_marked_record_replaces_it() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    hide_item(&mut store, &new_item(ALPHA, 1), NOW).unwrap();
    apply(
        &mut store,
        ALPHA,
        vec![Transition::Resurface {
            issue_id: 1,
            hidden_at_ms: NOW,
            reason: ChangeReason::Returned,
        }],
        &[ALPHA],
        NOW + 1,
    );
    assert_eq!(rows(&store), [(ALPHA.to_string(), 1, false)]);
    let mut again = new_item(ALPHA, 1);
    again.baseline_updated_at = T1.into();
    hide_item(&mut store, &again, NOW + 10).unwrap();
    let r = &list_items(&store, HiddenSource::Github, HiddenKind::Issue).unwrap()[0];
    assert!(r.changed_reason.is_none() && r.changed_at_ms.is_none());
    assert_eq!(r.baseline_updated_at, T1);
    assert_eq!(r.hidden_at_ms, NOW + 10);
}

#[test]
fn guarded_transitions_are_skipped_after_a_rehide() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    hide_item(&mut store, &new_item(ALPHA, 1), NOW).unwrap();
    // The round planned against the first hide; the user re-hid it meanwhile.
    hide_item(&mut store, &new_item(ALPHA, 1), NOW + 50).unwrap();
    let after = apply(
        &mut store,
        ALPHA,
        vec![
            Transition::Resurface {
                issue_id: 1,
                hidden_at_ms: NOW,
                reason: ChangeReason::Comment,
            },
            Transition::AdvanceBaseline {
                issue_id: 1,
                hidden_at_ms: NOW,
                updated_at: T1.into(),
                settle_after_ms: None,
            },
        ],
        &[ALPHA],
        NOW + 60,
    );
    assert_eq!(after, [(ALPHA.to_string(), 1, true)]);
    let r = &list_items(&store, HiddenSource::Github, HiddenKind::Issue).unwrap()[0];
    assert_eq!(r.baseline_updated_at, T0);
    assert_eq!(r.settle_after_ms, Some(NOW + 50 + SETTLE_MS));
}

#[test]
fn advance_baseline_and_clear_absent() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    hide_item(&mut store, &new_item(ALPHA, 1), NOW).unwrap();
    apply(
        &mut store,
        ALPHA,
        vec![
            Transition::AdvanceBaseline {
                issue_id: 1,
                hidden_at_ms: NOW,
                updated_at: T1.into(),
                settle_after_ms: Some(NOW + 500),
            },
            Transition::MarkAbsent { issue_id: 1 },
        ],
        &[ALPHA],
        NOW + 100,
    );
    let r = &list_items(&store, HiddenSource::Github, HiddenKind::Issue).unwrap()[0];
    assert_eq!(r.baseline_updated_at, T1);
    assert_eq!(r.settle_after_ms, Some(NOW + 500));
    assert_eq!(r.absent_since_ms, Some(NOW + 100));
    apply(
        &mut store,
        ALPHA,
        vec![Transition::ClearAbsent { issue_id: 1 }],
        &[ALPHA],
        NOW + 200,
    );
    let r = &list_items(&store, HiddenSource::Github, HiddenKind::Issue).unwrap()[0];
    assert_eq!(r.absent_since_ms, None);
}

#[test]
fn mark_absent_then_prune_only_after_thirty_days() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    hide_item(&mut store, &new_item(ALPHA, 1), NOW).unwrap();
    apply(
        &mut store,
        ALPHA,
        vec![Transition::MarkAbsent { issue_id: 1 }],
        &[ALPHA],
        NOW + 10,
    );
    let early = apply(
        &mut store,
        ALPHA,
        vec![Transition::Prune { issue_id: 1 }],
        &[ALPHA],
        NOW + 10 + PRUNE_AFTER_MS - 1,
    );
    assert_eq!(early, [(ALPHA.to_string(), 1, true)]);
    let late = apply(
        &mut store,
        ALPHA,
        vec![Transition::Prune { issue_id: 1 }],
        &[ALPHA],
        NOW + 10 + PRUNE_AFTER_MS,
    );
    assert!(late.is_empty());
}

#[test]
fn unconfigured_scope_is_pruned_only_after_thirty_days_hidden() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    hide_item(&mut store, &new_item(ALPHA, 1), NOW).unwrap();
    hide_item(&mut store, &new_item(BETA, 1), NOW).unwrap();
    // BETA is no longer configured.
    let early = apply(
        &mut store,
        ALPHA,
        vec![],
        &[ALPHA],
        NOW + PRUNE_AFTER_MS - 1,
    );
    assert_eq!(early.len(), 2);
    let late = apply(&mut store, ALPHA, vec![], &[ALPHA], NOW + PRUNE_AFTER_MS);
    assert_eq!(late, [(ALPHA.to_string(), 1, true)]);
}

#[test]
fn ops_leave_other_kinds_and_the_gitlab_issue_table_alone() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    let host = GitlabHost::parse("gitlab.example.invalid").unwrap();
    let issue = GitlabItem {
        kind: ItemKind::Issue,
        id: 1,
        project_id: 7,
        iid: 101,
        reference: "acme/widgets#1".into(),
        title: "Invented".into(),
        web_url: None,
        updated_at: T0.into(),
        labels: Vec::new(),
        draft: false,
        changed: None,
        milestone: None,
        is_new: false,
        hidden_at_ms: None,
    };
    hidden_issues::hide(&mut store, &host, &issue, NOW - PRUNE_AFTER_MS - 5).unwrap();
    // A future merge-request row of the same source, old enough to be pruned
    // were it an issue of an unconfigured scope.
    let mut mr = new_item(ALPHA, 1);
    mr.kind = HiddenKind::MergeRequest;
    hide_item(&mut store, &mr, NOW - PRUNE_AFTER_MS - 5).unwrap();
    hide_item(&mut store, &new_item(ALPHA, 1), NOW).unwrap();

    let after = apply(&mut store, BETA, vec![], &[BETA], NOW);
    // The ALPHA issue is recent, so it stays; the old MR row and the GitLab
    // issue row are not the round's business.
    assert_eq!(after, [(ALPHA.to_string(), 1, true)]);
    assert_eq!(scalar(&store, "SELECT count(*) FROM hidden_issues"), 1);
    assert_eq!(
        scalar(
            &store,
            "SELECT count(*) FROM hidden_items WHERE kind = 'mergeRequest'"
        ),
        1
    );
    let records: Vec<HiddenRecord> = hidden_issues::list(&store, &host).unwrap();
    assert_eq!(records.len(), 1);
    // And the other way round: GitHub ops never touch hidden_issues.
    unhide_item(&mut store, &key(ALPHA, 1)).unwrap();
    assert_eq!(scalar(&store, "SELECT count(*) FROM hidden_issues"), 1);
}

#[test]
fn checks_refuse_rows_that_break_the_shape() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    let base = "INSERT INTO hidden_items(source, scope, kind, item_id, project_id, iid, \
        status_fingerprint, baseline_updated_at, hidden_at_ms) VALUES";
    let bad = [
        // a github row with a project id
        format!("{base} ('github','octo-test/alpha','issue',1,5,6,'[]','t',1)"),
        // a gitlab row without it
        format!("{base} ('gitlab','gitlab.example.invalid','mergeRequest',1,NULL,NULL,'[]','t',1)"),
        // project id without iid
        format!("{base} ('gitlab','gitlab.example.invalid','mergeRequest',1,5,NULL,'[]','t',1)"),
        // item id zero
        format!("{base} ('github','octo-test/alpha','issue',0,NULL,NULL,'[]','t',1)"),
        // empty scope
        format!("{base} ('github','','issue',1,NULL,NULL,'[]','t',1)"),
        // unknown source and kind
        format!("{base} ('bitbucket','octo-test/alpha','issue',1,NULL,NULL,'[]','t',1)"),
        format!("{base} ('github','octo-test/alpha','epic',1,NULL,NULL,'[]','t',1)"),
    ];
    for sql in bad {
        let result = store.write::<_, StoreError>(|tx| {
            tx.execute(&sql, [])?;
            Ok(())
        });
        assert!(
            matches!(&result, Err(StoreError::Sql { reason }) if reason.starts_with("ConstraintViolation")),
            "{sql}"
        );
    }
    // A reason without its instant, and the other way round.
    hide_item(&mut store, &new_item(ALPHA, 1), NOW).unwrap();
    for sql in [
        "UPDATE hidden_items SET changed_reason = 'status'",
        "UPDATE hidden_items SET changed_at_ms = 1",
    ] {
        let result = store.write::<_, StoreError>(|tx| {
            tx.execute(sql, [])?;
            Ok(())
        });
        assert!(matches!(result, Err(StoreError::Sql { .. })), "{sql}");
    }
    // A gitlab merge-request row is storable once project and iid are given.
    let ok = format!("{base} ('gitlab','gitlab.example.invalid','mergeRequest',1,5,6,'[]','t',1)");
    store
        .write::<_, StoreError>(|tx| {
            tx.execute(&ok, [])?;
            Ok(())
        })
        .unwrap();
}

#[test]
fn v14_database_upgrades_to_the_hidden_items_table_and_keeps_its_data() {
    let (_tmp, dir) = test_store_dir();
    let pos = schema::MIGRATIONS
        .iter()
        .position(|m| m.sql.contains("CREATE TABLE hidden_items"))
        .expect("the hidden items migration");
    assert_eq!(schema::MIGRATIONS[pos].version, 15);
    {
        let mut old = Store::open(&dir, &schema::MIGRATIONS[..pos], today()).unwrap();
        assert_eq!(scalar(&old, "PRAGMA user_version"), 14);
        old.write::<_, StoreError>(|tx| {
            tx.execute(
                "INSERT INTO days_off(day, created_at_ms) VALUES ('2026-11-12', 1000)",
                [],
            )?;
            Ok(())
        })
        .unwrap();
    }
    let store = open(&dir);
    assert_eq!(
        scalar(&store, "PRAGMA user_version"),
        schema::MIGRATIONS.len() as i64
    );
    assert_eq!(scalar(&store, "SELECT count(*) FROM days_off"), 1);
    assert!(rows(&store).is_empty());
}
