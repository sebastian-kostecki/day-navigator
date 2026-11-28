//! One refresh round's hidden merge request reconciliation (`check_mr_round` +
//! `commit_mr_round`, S-07) against `fake-cli` and a real temp store. Invented
//! data only.
//!
//! The fake GitLab lists assigned MRs 6001, 6002 (also awaiting review), 6003 and
//! review-only MR 6101 (plus 6102, never hidden here). Notes exist for 6002 and
//! 6101 only (`MR_NOTES_KEYS`). Every MR is hidden with an older `updated_at`, so
//! a comment check is due.

mod common;

use std::path::PathBuf;
use std::time::Duration;

use chrono::Local;
use common::test_store_dir;
use day_navigator_lib::cli::{CliRunner, GitlabConfig, GitlabHost, GitlabUser};
use day_navigator_lib::gitlab::fetch::fetch_assigned;
use day_navigator_lib::gitlab::{AssignedItems, ChangeReason, GitlabItem};
use day_navigator_lib::hidden_issues::items::{
    HiddenItemRecord, HiddenKind, HiddenSource, NewHiddenItem, hide_item, list_items,
};
use day_navigator_lib::hidden_issues::merge_requests::{list_mr_records, new_hidden_mr};
use day_navigator_lib::hidden_issues::{
    self, HiddenIssueError, MrRoundOutcome, PRUNE_AFTER_MS, SETTLE_MS, check_mr_round,
    commit_mr_round,
};
use day_navigator_lib::store::{StoreError, StoreHandle};
use tempfile::TempDir;

const NOW: i64 = 1_800_000_000_000;
const DAY: i64 = 86_400_000;
/// Before every listed MR `updated_at` and before the fake notes of 2026-03-04.
const MR_OLD: &str = "2026-03-01T08:00:00.000Z";
const ASSIGNED: u64 = 6002;
const OTHER_ASSIGNED: u64 = 6003;
const REVIEW: u64 = 6101;
/// Marker that must never appear in an error or a `Debug` output.
const BAD_NOTES_MARKER: &str = "LEAK-MARKER-NOTE-4b2d";

fn fake() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_fake-cli"))
}

fn host() -> GitlabHost {
    GitlabHost::parse("gitlab.example.invalid").unwrap()
}

fn config(user: Option<&str>) -> GitlabConfig {
    GitlabConfig {
        host: host(),
        user: user.map(|u| GitlabUser::parse(u).unwrap()),
    }
}

struct Env {
    _tmp: TempDir,
    store: StoreHandle,
    log: PathBuf,
}

fn env() -> Env {
    let (tmp, dir) = test_store_dir();
    let log = tmp.path().join("notes-calls.log");
    Env {
        store: StoreHandle::new(Some(dir)),
        log,
        _tmp: tmp,
    }
}

impl Env {
    fn runner(&self, scenario: &str) -> CliRunner {
        CliRunner::with_programs(
            fake(),
            fake(),
            Duration::from_secs(10),
            vec![
                ("FAKE_CLI_SCENARIO".to_string(), scenario.to_string()),
                (
                    "FAKE_CLI_NOTES_LOG".to_string(),
                    self.log.to_string_lossy().into_owned(),
                ),
            ],
        )
    }

    /// Notes calls the fake CLI has served so far.
    fn notes_calls(&self) -> usize {
        std::fs::read_to_string(&self.log).map_or(0, |s| s.lines().count())
    }

    fn today() -> chrono::NaiveDate {
        Local::now().date_naive()
    }

    /// Hides the listed MR `id` (assigned or review-only) as it was `updated` ago.
    fn hide_mr(&self, full: &AssignedItems, id: u64, updated: &str) {
        let mut item = full
            .merge_requests
            .iter()
            .chain(full.review_requests.iter())
            .find(|i| i.id == id)
            .unwrap()
            .clone();
        item.updated_at = updated.to_string();
        self.store
            .with_store(Self::today(), |s| {
                hide_item(s, &new_hidden_mr(&host(), &item), NOW)
            })
            .unwrap();
    }

    fn mr_records(&self) -> Vec<HiddenItemRecord> {
        self.store
            .with_store::<_, HiddenIssueError>(Self::today(), |s| list_mr_records(s, &host()))
            .unwrap()
    }

    fn mr_record(&self, id: u64) -> HiddenItemRecord {
        self.mr_records()
            .into_iter()
            .find(|r| r.item_id == id)
            .unwrap()
    }
}

fn fetch(runner: &CliRunner, cfg: &GitlabConfig) -> AssignedItems {
    fetch_assigned(runner, cfg).expect("fake list")
}

/// One MR round at `now`, as the refresh thread runs it.
fn round_at(
    env: &Env,
    runner: &CliRunner,
    cfg: &GitlabConfig,
    full: &AssignedItems,
    now: i64,
    resume_after: Option<u64>,
) -> (MrRoundOutcome, Option<u64>) {
    let checks = check_mr_round(&env.store, runner, cfg, full, now, resume_after, true);
    let stopped = checks.stopped_at();
    (
        commit_mr_round(&env.store, &cfg.host, full, checks, now),
        stopped,
    )
}

fn mr_round(
    env: &Env,
    runner: &CliRunner,
    cfg: &GitlabConfig,
    full: &AssignedItems,
) -> MrRoundOutcome {
    round_at(env, runner, cfg, full, NOW + 1_000, None).0
}

fn ids(items: &[GitlabItem]) -> Vec<u64> {
    items.iter().map(|i| i.id).collect()
}

/// The reason of a visible merge request, from either list.
fn mr_marked(out: &MrRoundOutcome, id: u64) -> Option<ChangeReason> {
    out.merge_requests
        .iter()
        .chain(out.review_requests.iter())
        .find(|i| i.id == id)
        .and_then(|i| i.changed)
}

#[test]
fn others_comments_resurface_both_kinds_of_list() {
    let env = env();
    let cfg = config(Some("invented.user"));
    let runner = env.runner("gitlab-notes-other");
    let full = fetch(&runner, &cfg);
    env.hide_mr(&full, REVIEW, MR_OLD);
    env.hide_mr(&full, ASSIGNED, MR_OLD);

    let out = mr_round(&env, &runner, &cfg, &full);

    assert_eq!(mr_marked(&out, REVIEW), Some(ChangeReason::Comment));
    assert_eq!(mr_marked(&out, ASSIGNED), Some(ChangeReason::Comment));
    assert!(out.review_requests.iter().any(|i| i.id == REVIEW));
    assert!(out.merge_requests.iter().any(|i| i.id == ASSIGNED));
    assert!(out.hidden_merge_requests.is_empty());
    assert!(out.store_error.is_none());
}

#[test]
fn an_own_comment_advances_the_baseline_and_stays_hidden() {
    let env = env();
    let cfg = config(Some("invented.user"));
    let runner = env.runner("gitlab-notes-own");
    let full = fetch(&runner, &cfg);
    env.hide_mr(&full, ASSIGNED, MR_OLD);

    let out = mr_round(&env, &runner, &cfg, &full);

    assert_eq!(ids(&out.hidden_merge_requests), [ASSIGNED]);
    assert_eq!(out.check_failed, 0);
    let rec = env.mr_record(ASSIGNED);
    assert!(rec.changed_reason.is_none());
    // The baseline is the listed `updated_at`, and one settling check is due.
    assert_eq!(rec.baseline_updated_at, "2026-03-02T10:00:00.000Z");
    assert_eq!(rec.settle_after_ms, Some(NOW + 1_000 + 120_000));

    // The next round has nothing due: no further notes call.
    let again = mr_round(&env, &runner, &cfg, &full);
    assert_eq!(env.notes_calls(), 1);
    assert_eq!(ids(&again.hidden_merge_requests), [ASSIGNED]);
}

#[test]
fn a_system_note_stays_hidden() {
    let env = env();
    let cfg = config(Some("invented.user"));
    let runner = env.runner("gitlab-notes-system");
    let full = fetch(&runner, &cfg);
    env.hide_mr(&full, REVIEW, MR_OLD);

    let out = mr_round(&env, &runner, &cfg, &full);

    assert_eq!(ids(&out.hidden_merge_requests), [REVIEW]);
    assert!(env.mr_record(REVIEW).changed_reason.is_none());
}

#[test]
fn a_label_change_does_not_bring_a_merge_request_back() {
    let env = env();
    let cfg = config(Some("invented.user"));
    let runner = env.runner("gitlab-notes-none");
    let full = fetch(&runner, &cfg);
    env.hide_mr(&full, ASSIGNED, MR_OLD);
    // The listed MR gets a scoped label it did not have at hide time.
    let mut changed = full.clone();
    let item = changed
        .merge_requests
        .iter_mut()
        .find(|i| i.id == ASSIGNED)
        .unwrap();
    item.labels.push(day_navigator_lib::gitlab::GitlabLabel {
        name: "workflow::doing".to_string(),
        color: None,
        text_color: None,
    });

    let out = mr_round(&env, &runner, &cfg, &changed);

    assert_eq!(ids(&out.hidden_merge_requests), [ASSIGNED]);
    assert!(env.mr_record(ASSIGNED).changed_reason.is_none());
}

#[test]
fn drop_off_marks_absent_then_return_resurfaces_as_returned() {
    let env = env();
    let cfg = config(Some("invented.user"));
    let runner = env.runner("gitlab-notes-none");
    let full = fetch(&runner, &cfg);
    env.hide_mr(&full, REVIEW, MR_OLD);

    // The review request is gone this round (merged, say).
    let mut without = full.clone();
    without.review_requests.retain(|i| i.id != REVIEW);
    let out = mr_round(&env, &runner, &cfg, &without);
    assert!(out.hidden_merge_requests.is_empty());
    assert_eq!(env.notes_calls(), 0, "an absent MR is not checked");
    assert_eq!(env.mr_record(REVIEW).absent_since_ms, Some(NOW + 1_000));
    assert!(env.mr_record(REVIEW).changed_reason.is_none());

    // Listed again: returned, with nothing new in its notes.
    let out = mr_round(&env, &runner, &cfg, &full);
    assert_eq!(mr_marked(&out, REVIEW), Some(ChangeReason::Returned));
    assert_eq!(env.mr_record(REVIEW).absent_since_ms, None);
}

#[test]
fn an_absent_merge_request_is_pruned_after_thirty_days() {
    let env = env();
    let cfg = config(Some("invented.user"));
    let runner = env.runner("gitlab-notes-none");
    let full = fetch(&runner, &cfg);
    env.hide_mr(&full, REVIEW, MR_OLD);
    let mut without = full.clone();
    without.review_requests.retain(|i| i.id != REVIEW);
    mr_round(&env, &runner, &cfg, &without);

    let later = NOW + 1_000 + PRUNE_AFTER_MS;
    round_at(&env, &runner, &cfg, &without, later, None);

    assert!(env.mr_records().iter().all(|r| r.item_id != REVIEW));
}

#[test]
fn moving_between_the_two_lists_keeps_the_merge_request_hidden() {
    let env = env();
    let cfg = config(Some("invented.user"));
    let runner = env.runner("gitlab-notes-none");
    let full = fetch(&runner, &cfg);
    env.hide_mr(&full, REVIEW, MR_OLD);

    // The same MR now listed among the assigned ones instead of review-only.
    let mut moved = full.clone();
    let item = moved
        .review_requests
        .iter()
        .find(|i| i.id == REVIEW)
        .unwrap()
        .clone();
    moved.review_requests.retain(|i| i.id != REVIEW);
    moved.merge_requests.push(item);
    let out = mr_round(&env, &runner, &cfg, &moved);

    assert_eq!(ids(&out.hidden_merge_requests), [REVIEW]);
    let rec = env.mr_record(REVIEW);
    assert!(rec.changed_reason.is_none());
    assert_eq!(rec.absent_since_ms, None);
}

#[test]
fn notes_unavailable_runs_one_call_and_fails_two_checks() {
    let env = env();
    let cfg = config(Some("invented.user"));
    let runner = env.runner("gitlab-notes-unavailable");
    let full = fetch(&runner, &cfg);
    env.hide_mr(&full, ASSIGNED, MR_OLD);
    env.hide_mr(&full, REVIEW, MR_OLD);

    let (out, stopped) = round_at(&env, &runner, &cfg, &full, NOW + 1_000, None);

    assert_eq!(stopped, Some(ASSIGNED));
    assert_eq!(env.notes_calls(), 1, "no call after the first unavailable");
    assert_eq!(out.check_failed, 2);
    // Hidden assigned first, then review-only.
    assert_eq!(ids(&out.hidden_merge_requests), [ASSIGNED, REVIEW]);
    assert!(
        out.merge_requests
            .iter()
            .all(|i| i.id != ASSIGNED && i.id != REVIEW)
    );
    assert!(
        out.review_requests
            .iter()
            .all(|i| i.id != ASSIGNED && i.id != REVIEW)
    );
}

#[test]
fn a_failing_mr_call_does_not_starve_the_checks_after_it() {
    let env = env();
    let cfg = config(Some("invented.user"));
    // OTHER_ASSIGNED's notes call is unreachable every time; REVIEW has another's comment.
    let runner = env.runner("gitlab-notes-one-unavailable");
    let full = fetch(&runner, &cfg);
    env.hide_mr(&full, OTHER_ASSIGNED, MR_OLD);
    env.hide_mr(&full, REVIEW, MR_OLD);

    // Round 1: OTHER_ASSIGNED comes first, fails and stops the round.
    let (out, stopped) = round_at(&env, &runner, &cfg, &full, NOW + 1_000, None);
    assert_eq!(stopped, Some(OTHER_ASSIGNED));
    assert_eq!(env.notes_calls(), 1);
    assert_eq!(out.check_failed, 2);
    assert_eq!(out.hidden_merge_requests.len(), 2);

    // Round 2 resumes after OTHER_ASSIGNED: REVIEW is checked first and resurfaces.
    let (out, stopped) = round_at(
        &env,
        &runner,
        &cfg,
        &full,
        NOW + 2_000,
        Some(OTHER_ASSIGNED),
    );
    assert_eq!(stopped, Some(OTHER_ASSIGNED));
    assert_eq!(env.notes_calls(), 3);
    assert_eq!(mr_marked(&out, REVIEW), Some(ChangeReason::Comment));
    assert_eq!(ids(&out.hidden_merge_requests), [OTHER_ASSIGNED]);
    assert_eq!(out.check_failed, 1);
}

#[test]
fn mr_calls_not_allowed_fail_the_due_checks_and_keep_the_cursor() {
    let env = env();
    let cfg = config(Some("invented.user"));
    let runner = env.runner("gitlab-notes-other");
    let full = fetch(&runner, &cfg);
    env.hide_mr(&full, ASSIGNED, MR_OLD);
    env.hide_mr(&full, REVIEW, MR_OLD);

    let checks = check_mr_round(
        &env.store,
        &runner,
        &cfg,
        &full,
        NOW + 1_000,
        Some(ASSIGNED),
        false,
    );
    assert_eq!(checks.stopped_at(), Some(ASSIGNED));
    let out = commit_mr_round(&env.store, &cfg.host, &full, checks, NOW + 1_000);
    assert_eq!(env.notes_calls(), 0);
    assert_eq!(out.check_failed, 2);
    assert_eq!(ids(&out.hidden_merge_requests), [ASSIGNED, REVIEW]);

    let checks = check_mr_round(&env.store, &runner, &cfg, &full, NOW + 2_000, None, false);
    assert_eq!(checks.stopped_at(), None);
    commit_mr_round(&env.store, &cfg.host, &full, checks, NOW + 2_000);
    assert_eq!(env.notes_calls(), 0);
}

#[test]
fn a_malformed_notes_body_is_a_failed_check_without_leaking_its_text() {
    let env = env();
    let cfg = config(Some("invented.user"));
    let runner = env.runner("gitlab-notes-bad-json");
    let full = fetch(&runner, &cfg);
    env.hide_mr(&full, ASSIGNED, MR_OLD);

    let checks = check_mr_round(&env.store, &runner, &cfg, &full, NOW + 1_000, None, true);
    let out = commit_mr_round(&env.store, &cfg.host, &full, checks, NOW + 1_000);

    assert_eq!(out.check_failed, 1);
    assert_eq!(ids(&out.hidden_merge_requests), [ASSIGNED]);
    assert!(!format!("{out:?}").contains(BAD_NOTES_MARKER));
}

#[test]
fn without_a_configured_user_any_comment_resurfaces() {
    let env = env();
    let cfg = config(None);
    let runner = env.runner("gitlab-notes-own");
    let full = fetch(&runner, &cfg);
    env.hide_mr(&full, ASSIGNED, MR_OLD);

    let out = mr_round(&env, &runner, &cfg, &full);

    assert_eq!(mr_marked(&out, ASSIGNED), Some(ChangeReason::Comment));
}

#[test]
fn an_unreadable_baseline_counts_as_a_failed_check() {
    let env = env();
    let cfg = config(Some("invented.user"));
    let runner = env.runner("gitlab-notes-other");
    let full = fetch(&runner, &cfg);
    env.hide_mr(&full, ASSIGNED, "not a timestamp");

    let out = mr_round(&env, &runner, &cfg, &full);

    assert_eq!(env.notes_calls(), 0);
    assert_eq!(out.check_failed, 1);
    assert_eq!(ids(&out.hidden_merge_requests), [ASSIGNED]);
}

#[test]
fn a_store_failure_gives_the_listed_lists_and_a_store_error() {
    // No data dir: every store call fails.
    let store = StoreHandle::new(None);
    let env = env();
    let cfg = config(Some("invented.user"));
    let runner = env.runner("gitlab-notes-other");
    let full = fetch(&runner, &cfg);

    let checks = check_mr_round(&store, &runner, &cfg, &full, NOW, None, true);
    let out = commit_mr_round(&store, &cfg.host, &full, checks, NOW);

    assert_eq!(env.notes_calls(), 0, "a store read failure skips the calls");
    assert!(matches!(
        out.store_error,
        Some(HiddenIssueError::Store(StoreError::DirUnavailable { .. }))
    ));
    assert_eq!(out.merge_requests, full.merge_requests);
    assert_eq!(out.review_requests, full.review_requests);
    assert!(out.hidden_merge_requests.is_empty());
    assert_eq!(out.check_failed, 0);
}

#[test]
fn another_hosts_rows_are_pruned_when_old_and_never_shown_when_recent() {
    let env = env();
    let cfg = config(Some("invented.user"));
    let runner = env.runner("gitlab-notes-none");
    let full = fetch(&runner, &cfg);
    let other = |item_id: u64| NewHiddenItem {
        source: HiddenSource::Gitlab,
        scope: "other.example.invalid".to_string(),
        kind: HiddenKind::MergeRequest,
        item_id,
        project_id: Some(9),
        iid: Some(1),
        status_fingerprint: Vec::new(),
        baseline_updated_at: MR_OLD.to_string(),
    };
    env.store
        .with_store::<_, HiddenIssueError>(Env::today(), |s| {
            hide_item(s, &other(7001), NOW - 31 * DAY)?;
            hide_item(s, &other(7002), NOW)
        })
        .unwrap();

    let out = mr_round(&env, &runner, &cfg, &full);

    let rows = env
        .store
        .with_store::<_, HiddenIssueError>(Env::today(), |s| {
            list_items(s, HiddenSource::Gitlab, HiddenKind::MergeRequest)
        })
        .unwrap();
    assert!(rows.iter().all(|r| r.item_id != 7001));
    assert!(rows.iter().any(|r| r.item_id == 7002));
    assert!(out.hidden_merge_requests.iter().all(|i| i.id != 7002));
    assert!(out.merge_requests.iter().all(|i| i.id != 7002));
    assert!(out.review_requests.iter().all(|i| i.id != 7002));
}

#[test]
fn a_hidden_merge_request_and_a_hidden_issue_with_one_id_do_not_touch_each_other() {
    let env = env();
    let cfg = config(Some("invented.user"));
    let runner = env.runner("gitlab-notes-none");
    let full = fetch(&runner, &cfg);
    env.hide_mr(&full, ASSIGNED, MR_OLD);
    // An issue hidden under the same global id, with an older baseline.
    let mut issue = full.issues[0].clone();
    issue.id = ASSIGNED;
    issue.updated_at = MR_OLD.to_string();
    env.store
        .with_store::<_, HiddenIssueError>(Env::today(), |s| {
            hidden_issues::hide(s, &host(), &issue, NOW)
        })
        .unwrap();

    mr_round(&env, &runner, &cfg, &full);

    let issues = env
        .store
        .with_store::<_, HiddenIssueError>(Env::today(), |s| hidden_issues::list(s, &host()))
        .unwrap();
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].issue_id, ASSIGNED);
    assert_eq!(issues[0].baseline_updated_at, MR_OLD);
    assert_eq!(issues[0].settle_after_ms, Some(NOW + SETTLE_MS));
    // The merge request row itself was checked and advanced.
    assert_eq!(
        env.mr_record(ASSIGNED).baseline_updated_at,
        "2026-03-02T10:00:00.000Z"
    );
}

#[test]
fn a_write_failure_after_a_good_read_gives_the_listed_lists_and_keeps_the_rows() {
    let env = env();
    let cfg = config(Some("invented.user"));
    let runner = env.runner("gitlab-notes-other");
    let full = fetch(&runner, &cfg);
    env.hide_mr(&full, ASSIGNED, MR_OLD);

    // The read and the notes calls work; the commit goes to a store that fails.
    let checks = check_mr_round(&env.store, &runner, &cfg, &full, NOW + 1_000, None, true);
    assert_eq!(env.notes_calls(), 1);
    let broken = StoreHandle::new(None);
    let out = commit_mr_round(&broken, &cfg.host, &full, checks, NOW + 1_000);

    assert!(matches!(
        out.store_error,
        Some(HiddenIssueError::Store(StoreError::DirUnavailable { .. }))
    ));
    // Never an empty list in place of a failed write: both lists as listed.
    assert_eq!(out.merge_requests, full.merge_requests);
    assert_eq!(out.review_requests, full.review_requests);
    assert!(out.hidden_merge_requests.is_empty());
    assert_eq!(out.check_failed, 0);
    // Nothing was stored: the row is still hidden with its old baseline.
    let rec = env.mr_record(ASSIGNED);
    assert!(rec.changed_reason.is_none());
    assert_eq!(rec.baseline_updated_at, MR_OLD);
}

#[test]
fn a_hidden_review_only_merge_request_that_drops_off_returns_through_the_review_list() {
    let env = env();
    let cfg = config(Some("invented.user"));
    let runner = env.runner("gitlab-notes-none");
    let full = fetch(&runner, &cfg);
    env.hide_mr(&full, REVIEW, MR_OLD);

    let mut without = full.clone();
    without.review_requests.retain(|i| i.id != REVIEW);
    let gone = mr_round(&env, &runner, &cfg, &without);
    assert!(gone.hidden_merge_requests.is_empty());
    assert!(gone.review_requests.iter().all(|i| i.id != REVIEW));

    let back = mr_round(&env, &runner, &cfg, &full);
    // It returns in "Do review", not among the assigned ones, and not hidden.
    assert!(back.review_requests.iter().any(|i| i.id == REVIEW));
    assert!(back.merge_requests.iter().all(|i| i.id != REVIEW));
    assert!(back.hidden_merge_requests.is_empty());
    assert_eq!(mr_marked(&back, REVIEW), Some(ChangeReason::Returned));
}
