//! One refresh round's hidden-issue reconciliation (`check_round` +
//! `commit_round`) against `fake-cli` and a real temp store. Invented data only.
//!
//! The fake GitLab lists two issues: id 5001 (`acme/gizmo#5`) and id 5002
//! (`acme/widgets#12`, updated 2026-03-03T08:00:00Z). Only 5002 has notes in the
//! notes scenarios. Issues are hidden here with an older `updated_at`, so the
//! listed one has moved past the baseline and a comment check is due.

mod common;

use std::path::PathBuf;
use std::time::Duration;

use chrono::Local;
use common::test_store_dir;
use day_navigator_lib::cli::{CliRunner, GitlabConfig, GitlabHost, GitlabUser};
use day_navigator_lib::gitlab::fetch::fetch_assigned;
use day_navigator_lib::gitlab::{AssignedItems, ChangeReason, GitlabLabel};
use day_navigator_lib::hidden_issues::{
    self, HiddenIssueError, HiddenRecord, RoundOutcome, SETTLE_MS, check_round, commit_round,
};
use day_navigator_lib::store::{StoreError, StoreHandle};
use tempfile::TempDir;

const NOW: i64 = 1_800_000_000_000;
/// Before the listed `updated_at` of 5002 (2026-03-03T08:00:00Z).
const OLD: &str = "2026-03-02T08:00:00.000Z";
const LISTED: &str = "2026-03-03T08:00:00.000Z";
const GIZMO: u64 = 5001;
const WIDGET: u64 = 5002;

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

    /// Hides issue `id` as it was `updated` ago with `labels` at hide time.
    fn hide(&self, full: &AssignedItems, id: u64, labels: &[&str], updated: &str) {
        let mut item = full.issues.iter().find(|i| i.id == id).unwrap().clone();
        item.updated_at = updated.to_string();
        item.labels = labels
            .iter()
            .map(|l| GitlabLabel {
                name: l.to_string(),
                color: None,
                text_color: None,
            })
            .collect();
        self.store
            .with_store(Self::today(), |s| {
                hidden_issues::hide(s, &host(), &item, NOW)
            })
            .unwrap();
    }

    fn records(&self) -> Vec<HiddenRecord> {
        self.store
            .with_store::<_, HiddenIssueError>(Self::today(), |s| hidden_issues::list(s, &host()))
            .unwrap()
    }

    fn record(&self, id: u64) -> HiddenRecord {
        self.records()
            .into_iter()
            .find(|r| r.issue_id == id)
            .unwrap()
    }
}

fn fetch(runner: &CliRunner, cfg: &GitlabConfig) -> AssignedItems {
    fetch_assigned(runner, cfg).expect("fake list")
}

/// One round after a successful fetch, as the refresh thread runs it.
fn round(env: &Env, runner: &CliRunner, cfg: &GitlabConfig, full: &AssignedItems) -> RoundOutcome {
    let checks = check_round(&env.store, runner, cfg, full, NOW + 1_000, None);
    commit_round(&env.store, &cfg.host, full, checks, NOW + 1_000)
}

fn ids(items: &[day_navigator_lib::gitlab::GitlabItem]) -> Vec<u64> {
    items.iter().map(|i| i.id).collect()
}

fn marked(out: &RoundOutcome, id: u64) -> Option<ChangeReason> {
    out.visible_issues
        .iter()
        .find(|i| i.id == id)
        .and_then(|i| i.changed)
}

#[test]
fn an_others_comment_resurfaces_with_comment() {
    let env = env();
    let cfg = config(Some("invented.user"));
    let runner = env.runner("gitlab-notes-other");
    let full = fetch(&runner, &cfg);
    env.hide(&full, WIDGET, &[], OLD);

    let out = round(&env, &runner, &cfg, &full);

    assert_eq!(env.notes_calls(), 1);
    assert_eq!(out.check_failed, 0);
    assert!(out.store_error.is_none());
    assert!(out.hidden_issues.is_empty());
    assert_eq!(ids(&out.visible_issues), [WIDGET, GIZMO]);
    assert_eq!(marked(&out, WIDGET), Some(ChangeReason::Comment));
    assert_eq!(marked(&out, GIZMO), None);
}

#[test]
fn an_own_comment_advances_the_baseline_and_stays_hidden() {
    let env = env();
    let cfg = config(Some("invented.user"));
    let runner = env.runner("gitlab-notes-own");
    let full = fetch(&runner, &cfg);
    env.hide(&full, WIDGET, &[], OLD);

    let out = round(&env, &runner, &cfg, &full);

    assert_eq!(ids(&out.hidden_issues), [WIDGET]);
    assert_eq!(ids(&out.visible_issues), [GIZMO]);
    assert_eq!(out.check_failed, 0);
    let rec = env.record(WIDGET);
    assert!(rec.is_hidden());
    // The baseline is the listed `updated_at`, and one settling check is due.
    assert_eq!(rec.baseline_updated_at, LISTED);
    assert_eq!(rec.settle_after_ms, Some(NOW + 1_000 + SETTLE_MS));

    // The next round has nothing due: no further notes call.
    let again = round(&env, &runner, &cfg, &full);
    assert_eq!(env.notes_calls(), 1);
    assert_eq!(ids(&again.hidden_issues), [WIDGET]);
}

#[test]
fn a_system_note_stays_hidden() {
    let env = env();
    let cfg = config(Some("invented.user"));
    let runner = env.runner("gitlab-notes-system");
    let full = fetch(&runner, &cfg);
    env.hide(&full, WIDGET, &[], OLD);

    let out = round(&env, &runner, &cfg, &full);

    assert_eq!(ids(&out.hidden_issues), [WIDGET]);
    assert!(out.visible_issues.iter().all(|i| i.changed.is_none()));
    assert!(env.record(WIDGET).is_hidden());
}

#[test]
fn notes_unavailable_counts_a_failed_check_and_still_returns_the_list() {
    let env = env();
    let cfg = config(Some("invented.user"));
    let runner = env.runner("gitlab-notes-unavailable");
    let full = fetch(&runner, &cfg);
    env.hide(&full, WIDGET, &[], OLD);

    let out = round(&env, &runner, &cfg, &full);

    assert_eq!(out.check_failed, 1);
    assert!(out.store_error.is_none());
    assert_eq!(ids(&out.hidden_issues), [WIDGET]);
    assert_eq!(ids(&out.visible_issues), [GIZMO]);
    // Nothing changed in the store: the issue is re-checked next round.
    let rec = env.record(WIDGET);
    assert!(rec.is_hidden());
    assert_eq!(rec.baseline_updated_at, OLD);
}

#[test]
fn two_due_issues_and_notes_unavailable_run_one_call_and_fail_two_checks() {
    let env = env();
    let cfg = config(Some("invented.user"));
    let runner = env.runner("gitlab-notes-unavailable");
    let full = fetch(&runner, &cfg);
    env.hide(
        &full,
        GIZMO,
        &["workflow::doing"],
        "2026-02-19T08:00:00.000Z",
    );
    env.hide(&full, WIDGET, &[], OLD);

    let out = round(&env, &runner, &cfg, &full);

    assert_eq!(env.notes_calls(), 1, "no call after the first unavailable");
    assert_eq!(out.check_failed, 2);
    assert!(out.visible_issues.is_empty());
    assert_eq!(out.hidden_issues.len(), 2);
}

#[test]
fn two_concatenated_notes_pages_are_read_in_full() {
    let env = env();
    let cfg = config(Some("invented.user"));
    let runner = env.runner("gitlab-two-pages");
    let full = fetch(&runner, &cfg);
    env.hide(&full, WIDGET, &[], OLD);

    let out = round(&env, &runner, &cfg, &full);

    // Page one holds the user's own comment; the qualifying one is on page two.
    assert_eq!(marked(&out, WIDGET), Some(ChangeReason::Comment));
    assert_eq!(env.notes_calls(), 1);
}

#[test]
fn a_scoped_label_change_resurfaces_without_a_notes_call() {
    let env = env();
    let cfg = config(Some("invented.user"));
    let runner = env.runner("gitlab-notes-other");
    let full = fetch(&runner, &cfg);
    // Hidden with a scoped label the listed issue no longer has.
    env.hide(&full, WIDGET, &["workflow::doing"], OLD);

    let out = round(&env, &runner, &cfg, &full);

    assert_eq!(marked(&out, WIDGET), Some(ChangeReason::Status));
    assert_eq!(env.notes_calls(), 0);
    assert!(out.hidden_issues.is_empty());
}

#[test]
fn drop_off_marks_absent_and_return_resurfaces_as_returned() {
    let env = env();
    let cfg = config(Some("invented.user"));
    let runner = env.runner("gitlab-notes-none");
    let full = fetch(&runner, &cfg);
    env.hide(&full, WIDGET, &[], OLD);

    // The issue is not on the list this round (closed, say).
    let mut without = full.clone();
    without.issues.retain(|i| i.id != WIDGET);
    let out = round(&env, &runner, &cfg, &without);
    assert!(out.hidden_issues.is_empty());
    assert_eq!(env.notes_calls(), 0, "an absent issue is not checked");
    assert_eq!(env.record(WIDGET).absent_since_ms, Some(NOW + 1_000));
    assert!(env.record(WIDGET).is_hidden());

    // It is back, with nothing new in its notes.
    let out = round(&env, &runner, &cfg, &full);
    assert_eq!(marked(&out, WIDGET), Some(ChangeReason::Returned));
    assert!(out.hidden_issues.is_empty());
    assert_eq!(env.record(WIDGET).absent_since_ms, None);
}

#[test]
fn a_returned_issue_with_a_failed_check_is_not_counted_as_failed() {
    let env = env();
    let cfg = config(Some("invented.user"));
    let runner = env.runner("gitlab-notes-unavailable");
    let full = fetch(&runner, &cfg);
    env.hide(&full, WIDGET, &[], OLD);
    let mut without = full.clone();
    without.issues.retain(|i| i.id != WIDGET);
    round(&env, &runner, &cfg, &without);

    // Back on the list; its comment check fails, but returning needs no notes.
    let out = round(&env, &runner, &cfg, &full);

    assert_eq!(env.notes_calls(), 1);
    assert_eq!(marked(&out, WIDGET), Some(ChangeReason::Returned));
    assert_eq!(out.check_failed, 0, "a visible issue is not 'not checked'");
}

#[test]
fn a_call_that_keeps_failing_does_not_starve_the_checks_after_it() {
    let env = env();
    let cfg = config(Some("invented.user"));
    // GIZMO's notes call is unreachable every time; WIDGET has another's comment.
    let runner = env.runner("gitlab-notes-one-unavailable");
    let full = fetch(&runner, &cfg);
    env.hide(
        &full,
        GIZMO,
        &["workflow::doing"],
        "2026-02-19T08:00:00.000Z",
    );
    env.hide(&full, WIDGET, &[], OLD);

    // First round: GIZMO comes first, fails and stops the round.
    let checks = check_round(&env.store, &runner, &cfg, &full, NOW + 1_000, None);
    assert_eq!(checks.stopped_at(), Some(GIZMO));
    let out = commit_round(&env.store, &cfg.host, &full, checks, NOW + 1_000);
    assert_eq!(env.notes_calls(), 1);
    assert_eq!(out.check_failed, 2);
    assert_eq!(out.hidden_issues.len(), 2);

    // Next round resumes after GIZMO: WIDGET is checked first and resurfaces.
    let checks = check_round(&env.store, &runner, &cfg, &full, NOW + 2_000, Some(GIZMO));
    assert_eq!(checks.stopped_at(), Some(GIZMO));
    let out = commit_round(&env.store, &cfg.host, &full, checks, NOW + 2_000);
    assert_eq!(env.notes_calls(), 3);
    assert_eq!(marked(&out, WIDGET), Some(ChangeReason::Comment));
    assert_eq!(ids(&out.hidden_issues), [GIZMO]);
    assert_eq!(out.check_failed, 1);
}

#[test]
fn without_a_configured_user_any_comment_resurfaces() {
    let env = env();
    let cfg = config(None);
    // Only the user's own comments in these notes.
    let runner = env.runner("gitlab-notes-own");
    let full = fetch(&runner, &cfg);
    env.hide(&full, WIDGET, &[], OLD);

    let out = round(&env, &runner, &cfg, &full);

    assert_eq!(marked(&out, WIDGET), Some(ChangeReason::Comment));
}

#[test]
fn a_store_failure_gives_the_unpartitioned_list_and_a_store_error() {
    // No data dir: every store call fails.
    let store = StoreHandle::new(None);
    let env = env();
    let cfg = config(Some("invented.user"));
    let runner = env.runner("gitlab-notes-other");
    let full = fetch(&runner, &cfg);

    let checks = check_round(&store, &runner, &cfg, &full, NOW, None);
    let out = commit_round(&store, &cfg.host, &full, checks, NOW);

    assert_eq!(env.notes_calls(), 0, "a store read failure skips the calls");
    assert!(matches!(
        out.store_error,
        Some(HiddenIssueError::Store(StoreError::DirUnavailable { .. }))
    ));
    assert_eq!(out.visible_issues, full.issues);
    assert!(out.hidden_issues.is_empty());
    assert_eq!(out.check_failed, 0);
}

#[test]
fn an_unreadable_baseline_counts_as_a_failed_check() {
    let env = env();
    let cfg = config(Some("invented.user"));
    let runner = env.runner("gitlab-notes-other");
    let full = fetch(&runner, &cfg);
    env.hide(&full, WIDGET, &[], "not a timestamp");

    let out = round(&env, &runner, &cfg, &full);

    assert_eq!(env.notes_calls(), 0);
    assert_eq!(out.check_failed, 1);
    assert_eq!(ids(&out.hidden_issues), [WIDGET]);
}
