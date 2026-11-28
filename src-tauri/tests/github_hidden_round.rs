//! One refresh round's hidden-issue reconciliation for GitHub
//! (`check_github_round` + `commit_github_round`) against `fake-cli` and a
//! real temp store. Invented data only.
//!
//! The fake GitHub lists `octo-test/alpha#3` (updated 2026-02-20T08:00:00Z),
//! `octo-test/alpha#11` (updated 2026-03-03T08:00:00Z) and `octo-test/beta#5`
//! (updated 2026-02-25T08:00:00Z). Issues are hidden here with an older
//! baseline where a comment check has to be due.

mod common;

use std::path::PathBuf;
use std::time::Duration;

use chrono::Local;
use common::test_store_dir;
use day_navigator_lib::cli::{CliRunner, GithubConfig, GithubRepo};
use day_navigator_lib::github::GithubIssue;
use day_navigator_lib::github::fetch::{RepoResult, fetch_assigned};
use day_navigator_lib::hidden_issues::github_round::{
    GithubRoundOutcome, check_github_round, commit_github_round,
};
use day_navigator_lib::hidden_issues::items::{
    HiddenItemRecord, HiddenKind, HiddenSource, NewHiddenItem, hide_item, list_items,
};
use day_navigator_lib::hidden_issues::{ChangeReason, HiddenIssueError, PRUNE_AFTER_MS, SETTLE_MS};
use day_navigator_lib::store::{StoreError, StoreHandle};
use tempfile::TempDir;

const NOW: i64 = 1_800_000_000_000;
const ALPHA: &str = "octo-test/alpha";
const BETA: &str = "octo-test/beta";
const COMMENT_MARKER: &str = "LEAK-MARKER-GH-COMMENT";
const LOGIN_MARKER: &str = "LEAK-MARKER-GH-LOGIN";

fn fake() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_fake-cli"))
}

fn config() -> GithubConfig {
    GithubConfig {
        repos: vec![
            GithubRepo::parse(ALPHA).unwrap(),
            GithubRepo::parse(BETA).unwrap(),
        ],
    }
}

struct Env {
    _tmp: TempDir,
    store: StoreHandle,
    log: PathBuf,
}

fn env() -> Env {
    let (tmp, dir) = test_store_dir();
    let log = tmp.path().join("calls.log");
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
                    "FAKE_CLI_CALL_LOG".to_string(),
                    self.log.to_string_lossy().into_owned(),
                ),
            ],
        )
    }

    /// The argv lines of the calls the fake CLI has served so far.
    fn calls(&self) -> Vec<String> {
        std::fs::read_to_string(&self.log)
            .map(|s| s.lines().map(str::to_string).collect())
            .unwrap_or_default()
    }

    fn view_calls(&self) -> Vec<String> {
        self.calls()
            .into_iter()
            .filter(|l| l.starts_with("issue view"))
            .collect()
    }

    fn user_calls(&self) -> usize {
        self.calls().iter().filter(|l| *l == "api user").count()
    }

    fn today() -> chrono::NaiveDate {
        Local::now().date_naive()
    }

    fn hide_in(&self, scope: &str, number: u64, baseline: &str, at: i64) {
        let item = NewHiddenItem {
            source: HiddenSource::Github,
            scope: scope.to_string(),
            kind: HiddenKind::Issue,
            item_id: number,
            project_id: None,
            iid: None,
            status_fingerprint: vec![],
            baseline_updated_at: baseline.to_string(),
        };
        self.store
            .with_store(Self::today(), |s| hide_item(s, &item, at))
            .unwrap();
    }

    fn hide(&self, scope: &str, number: u64, baseline: &str) {
        self.hide_in(scope, number, baseline, NOW);
    }

    fn records(&self) -> Vec<HiddenItemRecord> {
        self.store
            .with_store::<_, HiddenIssueError>(Self::today(), |s| {
                list_items(s, HiddenSource::Github, HiddenKind::Issue)
            })
            .unwrap()
    }

    fn record(&self, scope: &str, number: u64) -> Option<HiddenItemRecord> {
        self.records()
            .into_iter()
            .find(|r| r.scope == scope && r.item_id == number)
    }

    fn fetch(&self, runner: &CliRunner) -> Vec<RepoResult> {
        fetch_assigned(runner, &config()).expect("fake list")
    }

    /// One round after a successful fetch, as the refresh thread runs it.
    fn round(&self, scenario: &str, now_ms: i64) -> GithubRoundOutcome {
        let runner = self.runner(scenario);
        let results = self.fetch(&runner);
        let checks = check_github_round(&self.store, &runner, &results, now_ms, None);
        commit_github_round(&self.store, &config(), &results, checks, now_ms)
    }
}

fn numbers(issues: &[GithubIssue]) -> Vec<u64> {
    issues.iter().map(|i| i.number).collect()
}

fn alpha(out: &GithubRoundOutcome) -> &day_navigator_lib::github::RepoView {
    &out.repos[0]
}

fn no_leak(text: &str) {
    assert!(!text.contains(COMMENT_MARKER), "{text}");
    assert!(!text.contains(LOGIN_MARKER), "{text}");
    assert!(!text.contains("octo-other"), "{text}");
}

const OLD: &str = "2026-03-02T08:00:00Z";
const LISTED_11: &str = "2026-03-03T08:00:00Z";
const LISTED_3: &str = "2026-02-20T08:00:00Z";

#[test]
fn a_comment_by_someone_else_resurfaces_the_issue() {
    let env = env();
    env.hide(ALPHA, 11, OLD);
    let out = env.round("gh-hidden-other", NOW + 1_000);
    assert!(out.store_error.is_none());
    let a = alpha(&out);
    assert_eq!(numbers(&a.issues), [11, 3]);
    assert!(a.hidden.is_empty());
    let back = &a.issues[0];
    assert_eq!(back.changed, Some(ChangeReason::Comment));
    assert_eq!(back.hidden_at_ms, None);
    assert_eq!(env.user_calls(), 1);
    assert_eq!(env.view_calls().len(), 1);
    no_leak(&format!("{out:?}"));
}

#[test]
fn the_users_own_comment_keeps_it_hidden_and_advances_the_baseline() {
    let env = env();
    env.hide(ALPHA, 11, OLD);
    let out = env.round("gh-hidden-own", NOW + 1_000);
    let a = alpha(&out);
    assert_eq!(numbers(&a.hidden), [11]);
    assert_eq!(a.hidden[0].hidden_at_ms, Some(NOW));
    assert_eq!(numbers(&a.issues), [3]);
    assert_eq!(out.check_failed, 0);
    let rec = env.record(ALPHA, 11).unwrap();
    assert_eq!(rec.baseline_updated_at, LISTED_11);
    assert!(rec.settle_after_ms.is_some());

    // Nothing moved and the settle check is not due: no further call.
    let before = env.view_calls().len();
    let out = env.round("gh-hidden-own", NOW + 2_000);
    assert_eq!(numbers(&alpha(&out).hidden), [11]);
    assert_eq!(env.view_calls().len(), before);
}

#[test]
fn a_ghost_or_empty_author_counts_as_someone_else() {
    let env = env();
    env.hide(ALPHA, 11, OLD);
    let out = env.round("gh-hidden-ghost", NOW + 1_000);
    let a = alpha(&out);
    assert!(a.hidden.is_empty());
    assert_eq!(a.issues[0].number, 11);
    assert_eq!(a.issues[0].changed, Some(ChangeReason::Comment));
}

#[test]
fn a_comment_before_the_baseline_leaves_it_hidden() {
    let env = env();
    env.hide(ALPHA, 11, OLD);
    let out = env.round("gh-hidden-old", NOW + 1_000);
    assert_eq!(numbers(&alpha(&out).hidden), [11]);
    assert_eq!(
        env.record(ALPHA, 11).unwrap().baseline_updated_at,
        LISTED_11
    );
}

#[test]
fn the_settle_check_runs_once_and_not_again() {
    let env = env();
    // Baseline equals the listed `updatedAt`: only the settle check can be due.
    env.hide(ALPHA, 3, LISTED_3);
    let out = env.round("gh-hidden-own", NOW + 1_000);
    assert_eq!(numbers(&alpha(&out).hidden), [3]);
    assert!(env.view_calls().is_empty());

    let due = NOW + SETTLE_MS + 1_000;
    let out = env.round("gh-hidden-own", due);
    assert_eq!(numbers(&alpha(&out).hidden), [3]);
    assert_eq!(env.view_calls().len(), 1);
    assert_eq!(env.record(ALPHA, 3).unwrap().settle_after_ms, None);

    env.round("gh-hidden-own", due + SETTLE_MS * 10);
    assert_eq!(env.view_calls().len(), 1);
}

#[test]
fn a_closed_issue_is_absent_and_comes_back_marked_returned() {
    let env = env();
    env.hide(ALPHA, 3, LISTED_3);
    let out = env.round("gh-hidden-closed", NOW + 1_000);
    assert!(alpha(&out).hidden.is_empty());
    assert_eq!(numbers(&alpha(&out).issues), [11]);
    assert_eq!(
        env.record(ALPHA, 3).unwrap().absent_since_ms,
        Some(NOW + 1_000)
    );

    let out = env.round("gh-hidden-none", NOW + 2_000);
    let a = alpha(&out);
    assert!(a.hidden.is_empty());
    let back = a.issues.iter().find(|i| i.number == 3).unwrap();
    assert_eq!(back.changed, Some(ChangeReason::Returned));
}

#[test]
fn thirty_days_absent_prunes_the_record() {
    let env = env();
    env.hide(ALPHA, 3, LISTED_3);
    env.round("gh-hidden-closed", NOW + 1_000);
    assert!(env.record(ALPHA, 3).is_some());
    env.round("gh-hidden-closed", NOW + 1_000 + PRUNE_AFTER_MS);
    assert!(env.record(ALPHA, 3).is_none());
}

#[test]
fn a_failed_repo_leaves_its_records_untouched_while_the_other_is_reconciled() {
    let env = env();
    env.hide(BETA, 5, "2026-02-24T08:00:00Z");
    env.hide(ALPHA, 11, OLD);
    let out = env.round("gh-hidden-beta-fails", NOW + 1_000);
    assert_eq!(out.repos[1].error.as_ref().unwrap().kind, "Failed");
    assert!(out.repos[1].issues.is_empty() && out.repos[1].hidden.is_empty());
    let beta = env.record(BETA, 5).unwrap();
    assert_eq!(beta.absent_since_ms, None);
    assert_eq!(beta.baseline_updated_at, "2026-02-24T08:00:00Z");
    // Alpha was reconciled: no comments, so the baseline advanced.
    assert_eq!(
        env.record(ALPHA, 11).unwrap().baseline_updated_at,
        LISTED_11
    );
    assert_eq!(numbers(&alpha(&out).hidden), [11]);
}

#[test]
fn a_failed_login_fails_every_due_check_and_starts_no_comment_call() {
    let env = env();
    env.hide(ALPHA, 11, OLD);
    env.hide(BETA, 5, "2026-02-24T08:00:00Z");
    let runner = env.runner("gh-hidden-user-fails");
    let results = env.fetch(&runner);
    let resume = Some((ALPHA.to_string(), 3));
    let checks = check_github_round(&env.store, &runner, &results, NOW + 1_000, resume.clone());
    assert_eq!(checks.stopped_at(), resume);
    let out = commit_github_round(&env.store, &config(), &results, checks, NOW + 1_000);
    assert_eq!(out.check_failed, 2);
    assert_eq!(numbers(&alpha(&out).hidden), [11]);
    assert_eq!(numbers(&out.repos[1].hidden), [5]);
    assert!(env.view_calls().is_empty());
    assert_eq!(env.user_calls(), 1);
    no_leak(&format!("{out:?}"));
}

#[test]
fn an_unavailable_comment_call_stops_the_round_and_the_next_one_resumes_after_it() {
    let env = env();
    env.hide(ALPHA, 11, OLD);
    env.hide(BETA, 5, "2026-02-24T08:00:00Z");
    let runner = env.runner("gh-hidden-view-unavailable");
    let results = env.fetch(&runner);
    let checks = check_github_round(&env.store, &runner, &results, NOW + 1_000, None);
    let stopped = checks.stopped_at();
    assert_eq!(stopped, Some((ALPHA.to_string(), 11)));
    let out = commit_github_round(&env.store, &config(), &results, checks, NOW + 1_000);
    assert_eq!(out.check_failed, 2);
    assert_eq!(env.view_calls().len(), 1);
    assert!(env.view_calls()[0].contains("octo-test/alpha"));

    let checks = check_github_round(&env.store, &runner, &results, NOW + 2_000, stopped);
    let calls = env.view_calls();
    assert_eq!(calls.len(), 2);
    assert!(calls[1].contains("octo-test/beta"), "{calls:?}");
    assert_eq!(checks.stopped_at(), Some((BETA.to_string(), 5)));
}

#[test]
fn an_unreadable_comment_answer_is_a_failed_check_without_a_leak() {
    let env = env();
    env.hide(ALPHA, 11, OLD);
    let out = env.round("gh-hidden-bad-json", NOW + 1_000);
    assert_eq!(out.check_failed, 1);
    assert_eq!(numbers(&alpha(&out).hidden), [11]);
    no_leak(&format!("{out:?}"));
}

#[test]
fn a_store_failure_gives_unpartitioned_repos_and_a_store_error() {
    let store = StoreHandle::new(None);
    let tmp = tempfile::tempdir().unwrap();
    let runner = CliRunner::with_programs(
        fake(),
        fake(),
        Duration::from_secs(10),
        vec![(
            "FAKE_CLI_SCENARIO".to_string(),
            "gh-hidden-none".to_string(),
        )],
    );
    let results = fetch_assigned(&runner, &config()).expect("fake list");
    let checks = check_github_round(&store, &runner, &results, NOW, None);
    let out = commit_github_round(&store, &config(), &results, checks, NOW);
    assert!(matches!(
        out.store_error,
        Some(HiddenIssueError::Store(StoreError::DirUnavailable { .. }))
    ));
    assert_eq!(out.check_failed, 0);
    assert_eq!(out.repos.len(), 2);
    assert_eq!(numbers(&out.repos[0].issues), [11, 3]);
    assert_eq!(numbers(&out.repos[1].issues), [5]);
    assert!(out.repos.iter().all(|r| r.hidden.is_empty()));
    drop(tmp);
}

#[test]
fn a_record_of_an_unconfigured_repo_is_pruned_only_after_thirty_days_hidden() {
    let env = env();
    env.hide("octo-test/gone", 1, LISTED_3);
    env.round("gh-hidden-none", NOW + 1_000);
    assert!(env.record("octo-test/gone", 1).is_some());
    env.round("gh-hidden-none", NOW + PRUNE_AFTER_MS + 1_000);
    assert!(env.record("octo-test/gone", 1).is_none());
}
