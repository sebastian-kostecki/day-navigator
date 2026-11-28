//! Rule and store-op tests with invented ids, labels and instants.

use std::collections::HashMap;

use chrono::{DateTime, FixedOffset, NaiveDate};
use tempfile::TempDir;

use super::rules::*;
use super::*;
use crate::gitlab::GitlabLabel;
use crate::gitlab::notes::IssueNote;
use crate::store::schema;

const DAY: i64 = 86_400_000;
const NOW: i64 = 1_800_000_000_000;

fn ts(s: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(s).unwrap()
}

fn host() -> GitlabHost {
    GitlabHost::parse("gitlab.example.invalid").unwrap()
}

fn user() -> &'static str {
    "Invented.User"
}

fn issue(id: u64, labels: &[&str], updated: &str) -> GitlabItem {
    GitlabItem {
        kind: ItemKind::Issue,
        id,
        project_id: 7,
        iid: id + 100,
        reference: format!("acme/widgets#{id}"),
        title: format!("Invented issue {id}"),
        web_url: None,
        updated_at: updated.into(),
        labels: labels
            .iter()
            .map(|s| GitlabLabel {
                name: s.to_string(),
                color: None,
                text_color: None,
            })
            .collect(),
        draft: false,
        changed: None,
        milestone: None,
        is_new: false,
        hidden_at_ms: None,
        review_requested: false,
    }
}

fn record(item: &GitlabItem) -> HiddenRecord {
    HiddenRecord {
        issue_id: item.id,
        project_id: item.project_id,
        iid: item.iid,
        scoped_labels: scoped_labels(item.label_names()),
        baseline_updated_at: item.updated_at.clone(),
        settle_after_ms: None,
        hidden_at_ms: NOW - DAY,
        absent_since_ms: None,
        changed_reason: None,
        changed_at_ms: None,
    }
}

fn note(author: &str, system: bool, at: &str) -> Comment {
    Comment::from(&IssueNote {
        author: author.into(),
        system,
        created_at: ts(at),
    })
}

const NO_ITEMS: &[GitlabItem] = &[];
const T0: &str = "2026-03-02T10:00:00Z";
const T1: &str = "2026-03-02T10:05:00Z";

fn checks(id: u64, v: Result<CommentVerdict, ()>) -> HashMap<u64, Result<CommentVerdict, ()>> {
    HashMap::from([(id, v)])
}

// ----------------------------------------------------------------- rules

#[test]
fn scoped_labels_keep_only_scoped_sorted_and_unique() {
    let names = ["bug", "workflow::doing", "area::api", "workflow::doing"];
    assert_eq!(
        scoped_labels(names),
        ["area::api", "workflow::doing"].map(String::from)
    );
    // The same names as a label list give the same result.
    let item = issue(1, &names, T0);
    assert_eq!(scoped_labels(item.label_names()), scoped_labels(names));
}

#[test]
fn coloured_labels_do_not_change_the_status() {
    let plain = issue(1, &["workflow::doing", "bug"], T0);
    let rec = record(&plain);
    let mut coloured = plain.clone();
    for (i, l) in coloured.labels.iter_mut().enumerate() {
        l.color = Some(format!("#00{i}"));
        l.text_color = Some("#fff".to_string());
    }
    assert!(!status_changed(&rec, &coloured));
    // A stored record is made from names only, whatever the colours were.
    assert_eq!(record(&coloured).scoped_labels, ["workflow::doing"]);
}

#[test]
fn non_scoped_label_change_is_ignored() {
    let base = issue(1, &["workflow::doing", "bug"], T0);
    let rec = record(&base);
    let changed = issue(1, &["workflow::doing", "urgent"], T0);
    assert!(!status_changed(&rec, &changed));
}

#[test]
fn scoped_label_add_remove_and_swap_change_status() {
    let rec = record(&issue(1, &["workflow::doing"], T0));
    assert!(status_changed(
        &rec,
        &issue(1, &["workflow::doing", "area::api"], T0)
    ));
    assert!(status_changed(&rec, &issue(1, &[], T0)));
    assert!(status_changed(&rec, &issue(1, &["workflow::done"], T0)));
}

#[test]
fn comment_exactly_at_the_baseline_is_not_new() {
    let n = [note("someone", false, T0)];
    assert_eq!(
        comment_verdict(&n, ts(T0), Some(user())),
        CommentVerdict::NothingNew
    );
    let n = [note("someone", false, T1)];
    assert_eq!(
        comment_verdict(&n, ts(T0), Some(user())),
        CommentVerdict::NewFromOthers
    );
}

#[test]
fn own_comment_is_ignored_case_insensitively() {
    let n = [note("INVENTED.user", false, T1)];
    assert_eq!(
        comment_verdict(&n, ts(T0), Some(user())),
        CommentVerdict::NothingNew
    );
}

#[test]
fn without_a_user_any_author_counts() {
    let n = [note("invented.user", false, T1)];
    assert_eq!(
        comment_verdict(&n, ts(T0), None),
        CommentVerdict::NewFromOthers
    );
}

#[test]
fn system_note_is_ignored_and_a_bot_counts() {
    let n = [note("someone", true, T1)];
    assert_eq!(
        comment_verdict(&n, ts(T0), None),
        CommentVerdict::NothingNew
    );
    let n = [note("project_7_bot", false, T1)];
    assert_eq!(
        comment_verdict(&n, ts(T0), Some(user())),
        CommentVerdict::NewFromOthers
    );
}

#[test]
fn comment_check_is_due_when_updated_at_moves_or_settling_is_due() {
    let item = issue(1, &[], T0);
    let mut rec = record(&item);
    assert!(!needs_comment_check(&rec, &item, NOW));
    // Equal instant in another notation: not moved.
    let same = issue(1, &[], "2026-03-02T11:00:00+01:00");
    assert!(!needs_comment_check(&rec, &same, NOW));
    assert!(needs_comment_check(&rec, &issue(1, &[], T1), NOW));

    rec.settle_after_ms = Some(NOW);
    assert!(needs_comment_check(&rec, &item, NOW));
    assert!(!needs_comment_check(&rec, &item, NOW - 1));

    // A marked record is never checked.
    rec.changed_reason = Some(ChangeReason::Comment);
    rec.changed_at_ms = Some(NOW);
    assert!(!needs_comment_check(&rec, &issue(1, &[], T1), NOW));
}

#[test]
fn nothing_planned_when_nothing_changed_and_no_check_ran() {
    let item = issue(1, &["workflow::doing"], T0);
    let plan = plan_round(&[record(&item)], &[item], &HashMap::new(), NOW);
    assert!(plan.is_empty());
}

#[test]
fn new_comment_resurfaces_with_comment() {
    let item = issue(1, &[], T1);
    let rec = record(&issue(1, &[], T0));
    let plan = plan_round(
        std::slice::from_ref(&rec),
        &[item],
        &checks(1, Ok(CommentVerdict::NewFromOthers)),
        NOW,
    );
    assert_eq!(
        plan,
        [Transition::Resurface {
            issue_id: 1,
            hidden_at_ms: rec.hidden_at_ms,
            reason: ChangeReason::Comment
        }]
    );
}

#[test]
fn moved_updated_at_with_nothing_new_advances_and_schedules_settling() {
    let item = issue(1, &[], T1);
    let rec = record(&issue(1, &[], T0));
    let plan = plan_round(
        std::slice::from_ref(&rec),
        &[item],
        &checks(1, Ok(CommentVerdict::NothingNew)),
        NOW,
    );
    assert_eq!(
        plan,
        [Transition::AdvanceBaseline {
            issue_id: 1,
            hidden_at_ms: rec.hidden_at_ms,
            updated_at: T1.into(),
            settle_after_ms: Some(NOW + SETTLE_MS)
        }]
    );
}

#[test]
fn settling_only_check_with_nothing_new_clears_settling() {
    // updated_at did not move: a throttled reply would show up in the notes.
    let item = issue(1, &[], T0);
    let mut rec = record(&item);
    rec.settle_after_ms = Some(NOW - 1);
    let plan = plan_round(
        std::slice::from_ref(&rec),
        &[item],
        &checks(1, Ok(CommentVerdict::NothingNew)),
        NOW,
    );
    assert_eq!(
        plan,
        [Transition::AdvanceBaseline {
            issue_id: 1,
            hidden_at_ms: rec.hidden_at_ms,
            updated_at: T0.into(),
            settle_after_ms: None
        }]
    );
}

#[test]
fn settling_check_catches_a_throttled_reply() {
    let item = issue(1, &[], T0);
    let mut rec = record(&item);
    rec.settle_after_ms = Some(NOW);
    assert!(needs_comment_check(&rec, &item, NOW));
    let plan = plan_round(
        &[rec],
        &[item],
        &checks(1, Ok(CommentVerdict::NewFromOthers)),
        NOW,
    );
    assert!(matches!(
        plan[0],
        Transition::Resurface {
            reason: ChangeReason::Comment,
            ..
        }
    ));
}

#[test]
fn status_beats_comment_beats_returned() {
    let rec_item = issue(1, &["workflow::doing"], T0);
    let mut rec = record(&rec_item);
    rec.absent_since_ms = Some(NOW - DAY);
    let new_comment = checks(1, Ok(CommentVerdict::NewFromOthers));

    let all = issue(1, &["workflow::done"], T1);
    let p = plan_round(std::slice::from_ref(&rec), &[all], &new_comment, NOW);
    assert!(matches!(
        p[0],
        Transition::Resurface {
            reason: ChangeReason::Status,
            ..
        }
    ));

    let comment_and_return = issue(1, &["workflow::doing"], T1);
    let p = plan_round(
        std::slice::from_ref(&rec),
        &[comment_and_return],
        &new_comment,
        NOW,
    );
    assert!(matches!(
        p[0],
        Transition::Resurface {
            reason: ChangeReason::Comment,
            ..
        }
    ));

    let only_returned = issue(1, &["workflow::doing"], T0);
    let p = plan_round(&[rec], &[only_returned], &HashMap::new(), NOW);
    assert!(matches!(
        p[0],
        Transition::Resurface {
            reason: ChangeReason::Returned,
            ..
        }
    ));
}

#[test]
fn status_change_resurfaces_without_a_notes_check() {
    let rec = record(&issue(1, &["workflow::doing"], T0));
    let p = plan_round(
        &[rec],
        &[issue(1, &["workflow::done"], T0)],
        &HashMap::new(),
        NOW,
    );
    assert!(matches!(
        p[0],
        Transition::Resurface {
            reason: ChangeReason::Status,
            ..
        }
    ));
}

#[test]
fn drop_off_marks_absent_then_return_resurfaces() {
    let item = issue(1, &[], T0);
    let rec = record(&item);
    assert_eq!(
        plan_round(std::slice::from_ref(&rec), NO_ITEMS, &HashMap::new(), NOW),
        [Transition::MarkAbsent { issue_id: 1 }]
    );
    let mut absent = rec;
    absent.absent_since_ms = Some(NOW - DAY);
    // Still missing a day later: nothing to do.
    assert!(plan_round(&[absent.clone()], NO_ITEMS, &HashMap::new(), NOW).is_empty());
    let p = plan_round(&[absent], &[item], &HashMap::new(), NOW);
    assert!(matches!(
        p[0],
        Transition::Resurface {
            reason: ChangeReason::Returned,
            ..
        }
    ));
}

#[test]
fn prune_boundary_is_exactly_thirty_days() {
    let mut rec = record(&issue(1, &[], T0));
    rec.absent_since_ms = Some(NOW - 30 * DAY + 1);
    assert!(plan_round(std::slice::from_ref(&rec), NO_ITEMS, &HashMap::new(), NOW).is_empty());
    rec.absent_since_ms = Some(NOW - 30 * DAY);
    assert_eq!(
        plan_round(&[rec], NO_ITEMS, &HashMap::new(), NOW),
        [Transition::Prune { issue_id: 1 }]
    );
}

#[test]
fn failed_check_gives_no_transition() {
    let rec = record(&issue(1, &[], T0));
    let p = plan_round(&[rec], &[issue(1, &[], T1)], &checks(1, Err(())), NOW);
    assert!(p.is_empty());
}

#[test]
fn marked_rows_get_no_new_reason_only_absence_handling() {
    let item = issue(1, &["workflow::doing"], T0);
    let mut rec = record(&item);
    rec.changed_reason = Some(ChangeReason::Comment);
    rec.changed_at_ms = Some(NOW - DAY);
    let changed = issue(1, &["workflow::done"], T1);
    let verdict = checks(1, Ok(CommentVerdict::NewFromOthers));
    assert!(plan_round(std::slice::from_ref(&rec), &[changed], &verdict, NOW).is_empty());
    assert_eq!(
        plan_round(std::slice::from_ref(&rec), NO_ITEMS, &HashMap::new(), NOW),
        [Transition::MarkAbsent { issue_id: 1 }]
    );
    rec.absent_since_ms = Some(NOW - DAY);
    assert_eq!(
        plan_round(std::slice::from_ref(&rec), &[item], &HashMap::new(), NOW),
        [Transition::ClearAbsent { issue_id: 1 }]
    );
    rec.absent_since_ms = Some(NOW - 31 * DAY);
    assert_eq!(
        plan_round(&[rec], NO_ITEMS, &HashMap::new(), NOW),
        [Transition::Prune { issue_id: 1 }]
    );
}

// ------------------------------------------------------------- store ops

fn open() -> (TempDir, Store) {
    let tmp = tempfile::tempdir().unwrap();
    let today = NaiveDate::from_ymd_opt(2026, 11, 23).unwrap();
    let store = Store::open(&tmp.path().join("data"), schema::MIGRATIONS, today).unwrap();
    (tmp, store)
}

#[test]
fn partition_splits_marks_and_keeps_order() {
    let a = issue(1, &[], T0);
    let b = issue(2, &[], T0);
    let c = issue(3, &[], T0);
    let d = issue(4, &[], T0);
    let hidden = record(&b);
    let mut marked = record(&c);
    marked.changed_reason = Some(ChangeReason::Status);
    marked.changed_at_ms = Some(NOW);
    let (visible, hid) = partition(
        &[a.clone(), b.clone(), c.clone(), d.clone()],
        &[hidden, marked],
    );
    assert_eq!(hid.len(), 1);
    assert_eq!(hid[0].id, b.id);
    assert_eq!(hid[0].hidden_at_ms, Some(NOW - DAY));
    assert!(visible.iter().all(|i| i.hidden_at_ms.is_none()));
    assert_eq!(visible.iter().map(|i| i.id).collect::<Vec<_>>(), [1, 3, 4]);
    assert_eq!(visible[1].changed, Some(ChangeReason::Status));
    assert_eq!(visible[0].changed, None);
}

#[test]
fn hide_unhide_dismiss_and_not_found() {
    let (_tmp, mut store) = open();
    let item = issue(1, &["workflow::doing"], T0);
    hide(&mut store, &host(), &item, NOW).unwrap();
    let rows = list(&store, &host()).unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].is_hidden());
    assert_eq!(rows[0].settle_after_ms, Some(NOW + SETTLE_MS));
    assert!(matches!(
        dismiss(&mut store, &host(), 1),
        Err(HiddenIssueError::NotFound)
    ));
    unhide(&mut store, &host(), 1).unwrap();
    assert!(list(&store, &host()).unwrap().is_empty());
    assert!(matches!(
        unhide(&mut store, &host(), 1),
        Err(HiddenIssueError::NotFound)
    ));
}

#[test]
fn a_merge_request_cannot_be_hidden() {
    let (_tmp, mut store) = open();
    let mut mr = issue(1, &[], T0);
    mr.kind = ItemKind::MergeRequest;
    assert!(matches!(
        hide(&mut store, &host(), &mr, NOW),
        Err(HiddenIssueError::NotInList)
    ));
}

// ------------------------------------------------------------ leak check

#[test]
fn errors_never_contain_submitted_values() {
    let (_tmp, mut store) = open();
    let mut item = issue(987_654_321, &["secret-marker::value"], T0);
    item.title = "secret-title-marker".into();
    hide(&mut store, &host(), &item, NOW).unwrap();
    let errors = [
        unhide(&mut store, &host(), 987_654_322).unwrap_err(),
        dismiss(&mut store, &host(), 987_654_321).unwrap_err(),
        HiddenIssueError::NotInList,
        HiddenIssueError::ListUnavailable,
        corrupt(),
    ];
    for e in errors {
        let text = format!("{e} {e:?} {}", serde_json::to_string(&e).unwrap());
        for needle in [
            "987654321",
            "987654322",
            "secret",
            "gitlab.example.invalid",
            "example",
        ] {
            assert!(!text.contains(needle), "{needle} leaked in {text}");
        }
    }
}

#[test]
fn error_json_shape_is_tagged() {
    let json = serde_json::to_string(&HiddenIssueError::NotInList).unwrap();
    assert_eq!(json, r#"{"kind":"NotInList"}"#);
    let json = serde_json::to_string(&HiddenIssueError::Store(StoreError::Busy)).unwrap();
    assert_eq!(json, r#"{"kind":"Store","detail":{"kind":"Busy"}}"#);
}

// ------------------------------------------- the same rules, GitHub-shaped

fn gh_issue(number: u64, updated: &str) -> crate::github::GithubIssue {
    crate::github::GithubIssue {
        repo: "octo-test/alpha".into(),
        number,
        reference: format!("octo-test/alpha#{number}"),
        title: format!("Invented issue {number}"),
        web_url: None,
        updated_at: updated.into(),
        labels: vec!["area::api".into(), "bug".into()],
        changed: None,
        hidden_at_ms: None,
    }
}

fn gh_record(number: u64, updated: &str) -> items::HiddenItemRecord {
    items::HiddenItemRecord {
        source: items::HiddenSource::Github,
        scope: "octo-test/alpha".into(),
        kind: items::HiddenKind::Issue,
        item_id: number,
        project_id: None,
        iid: None,
        status_fingerprint: Vec::new(),
        baseline_updated_at: updated.into(),
        settle_after_ms: None,
        hidden_at_ms: NOW - DAY,
        absent_since_ms: None,
        changed_reason: None,
        changed_at_ms: None,
    }
}

fn gh_comment(author: Option<&str>, at: &str) -> Comment {
    Comment {
        author: author.map(String::from),
        counts: true,
        created_at: ts(at),
    }
}

#[test]
fn github_labels_never_change_the_status() {
    let rec = gh_record(5, T0);
    assert!(!status_changed(&rec, &gh_issue(5, T0)));
    let mut relabelled = gh_issue(5, T0);
    relabelled.labels = vec!["workflow::done".into()];
    assert!(!status_changed(&rec, &relabelled));
    let plan = plan_round(&[rec], &[relabelled], &HashMap::new(), NOW);
    assert!(plan.is_empty());
}

#[test]
fn github_ghost_author_counts_as_someone_else() {
    let n = [gh_comment(None, T1)];
    assert_eq!(
        comment_verdict(&n, ts(T0), Some("invented-user")),
        CommentVerdict::NewFromOthers
    );
}

#[test]
fn github_own_comment_in_another_letter_case_does_not_count() {
    let n = [gh_comment(Some("INVENTED-user"), T1)];
    assert_eq!(
        comment_verdict(&n, ts(T0), Some("invented-User")),
        CommentVerdict::NothingNew
    );
}

#[test]
fn github_comment_at_exactly_the_baseline_does_not_count() {
    let n = [gh_comment(Some("someone-else"), T0)];
    assert_eq!(
        comment_verdict(&n, ts(T0), Some("invented-user")),
        CommentVerdict::NothingNew
    );
}

#[test]
fn github_without_a_user_any_author_counts() {
    let n = [gh_comment(Some("invented-user"), T1)];
    assert_eq!(
        comment_verdict(&n, ts(T0), None),
        CommentVerdict::NewFromOthers
    );
}

#[test]
fn github_record_resurfaces_on_a_verdict_and_returns() {
    let rec = gh_record(5, T0);
    let verdict = checks(5, Ok(CommentVerdict::NewFromOthers));
    let p = plan_round(
        std::slice::from_ref(&rec),
        &[gh_issue(5, T1)],
        &verdict,
        NOW,
    );
    assert_eq!(
        p,
        [Transition::Resurface {
            issue_id: 5,
            hidden_at_ms: NOW - DAY,
            reason: ChangeReason::Comment
        }]
    );
    let mut absent = rec;
    absent.absent_since_ms = Some(NOW - 1);
    let p = plan_round(&[absent], &[gh_issue(5, T0)], &HashMap::new(), NOW);
    assert!(matches!(
        p[..],
        [Transition::Resurface {
            reason: ChangeReason::Returned,
            ..
        }]
    ));
}

// ------------------------------------------------------- merge requests (S-07)

fn mr(id: u64, labels: &[&str], updated: &str) -> GitlabItem {
    GitlabItem {
        kind: ItemKind::MergeRequest,
        ..issue(id, labels, updated)
    }
}

fn mr_record(id: u64, reason: Option<ChangeReason>) -> items::HiddenItemRecord {
    items::HiddenItemRecord {
        source: items::HiddenSource::Gitlab,
        scope: host().as_str().to_string(),
        kind: items::HiddenKind::MergeRequest,
        item_id: id,
        project_id: Some(7),
        iid: Some(id + 100),
        status_fingerprint: Vec::new(),
        baseline_updated_at: T0.into(),
        settle_after_ms: None,
        hidden_at_ms: NOW - 1_000,
        absent_since_ms: None,
        changed_reason: reason,
        changed_at_ms: reason.map(|_| NOW),
    }
}

#[test]
fn issue_fingerprint_keeps_scoped_labels_and_merge_request_is_empty() {
    let labels = ["a::b", "workflow::doing"];
    let i = issue(1, &labels, T0);
    let m = mr(2, &labels, T0);
    assert_eq!(
        i.status_fingerprint(),
        vec!["a::b".to_string(), "workflow::doing".to_string()]
    );
    assert!(m.status_fingerprint().is_empty());
}

#[test]
fn merge_request_label_change_is_not_a_status_change() {
    let m = mr(2, &["a::b"], T0);
    let rec = items::HiddenItemRecord {
        status_fingerprint: Vec::new(),
        ..mr_record(2, None)
    };
    let relabelled = mr(2, &["a::c", "x::y"], T0);
    assert!(!status_changed(&rec, &relabelled));
    assert!(!status_changed(&rec, &m));
}

#[test]
fn only_a_hidden_review_only_merge_request_keeps_the_review_flag() {
    let assigned = vec![GitlabItem {
        review_requested: true,
        ..mr(10, &[], T0)
    }];
    let review = vec![GitlabItem {
        review_requested: true,
        ..mr(20, &[], T0)
    }];
    let records = vec![mr_record(10, None), mr_record(20, None)];
    let p = merge_requests::partition_merge_requests(&assigned, &review, &records);
    assert_eq!(p.hidden.iter().map(|i| i.id).collect::<Vec<_>>(), [10, 20]);
    assert!(!p.hidden[0].review_requested, "assigned: main table only");
    assert!(p.hidden[1].review_requested, "review-only: Do review");
}

#[test]
fn partition_splits_both_lists_and_keeps_order() {
    let assigned = vec![mr(10, &[], T0), mr(11, &[], T0), mr(12, &[], T0)];
    let review = vec![
        GitlabItem {
            review_requested: true,
            ..mr(20, &[], T0)
        },
        GitlabItem {
            review_requested: true,
            ..mr(21, &[], T0)
        },
    ];
    let records = vec![
        mr_record(11, None),
        mr_record(21, None),
        mr_record(12, Some(ChangeReason::Comment)),
    ];
    let p = merge_requests::partition_merge_requests(&assigned, &review, &records);
    let ids = |v: &[GitlabItem]| v.iter().map(|i| i.id).collect::<Vec<_>>();
    assert_eq!(ids(&p.merge_requests), vec![10, 12]);
    assert_eq!(ids(&p.review_requests), vec![20]);
    assert_eq!(ids(&p.hidden), vec![11, 21]);
    assert_eq!(p.merge_requests[1].changed, Some(ChangeReason::Comment));
    assert_eq!(p.merge_requests[1].hidden_at_ms, None);
    assert_eq!(p.hidden[0].hidden_at_ms, Some(NOW - 1_000));
    assert!(p.hidden[1].review_requested, "review flag is kept");
    assert!(p.review_requests[0].review_requested);
}

#[test]
fn partition_ignores_records_of_other_ids() {
    let assigned = vec![mr(30, &[], T0)];
    let records = vec![mr_record(31, None)];
    let p = merge_requests::partition_merge_requests(&assigned, &[], &records);
    assert_eq!(p.merge_requests.len(), 1);
    assert!(p.hidden.is_empty());
    assert_eq!(p.merge_requests[0].hidden_at_ms, None);
}

#[test]
fn new_hidden_mr_has_empty_fingerprint_and_both_refs() {
    let m = GitlabItem {
        project_id: 77,
        iid: 34,
        ..mr(40, &["a::b"], T0)
    };
    let n = merge_requests::new_hidden_mr(&host(), &m);
    assert_eq!(n.kind, items::HiddenKind::MergeRequest);
    assert_eq!(n.project_id, Some(77));
    assert_eq!(n.iid, Some(34));
    assert!(n.status_fingerprint.is_empty());
    assert_eq!(n.baseline_updated_at, T0);
}

#[test]
fn mr_calls_allowed_skips_after_an_issue_stop_but_never_twice() {
    assert!(mr_calls_allowed(false, false));
    assert!(mr_calls_allowed(false, true));
    assert!(!mr_calls_allowed(true, false));
    assert!(mr_calls_allowed(true, true));
}
