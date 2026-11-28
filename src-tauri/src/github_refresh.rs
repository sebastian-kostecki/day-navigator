//! Side effects of the GitHub list: the `github-refresh` thread, the managed
//! state, the commands and the `github-changed` event. The rules live in
//! `github/` and `health/` and stay free of `tauri`. The shared mechanics (the
//! snapshot cell, the thread start, the wait loop, publication and the round
//! tail) live in `source_refresh.rs`.
//!
//! Locking rule (same as `gitlab_refresh.rs`): the snapshot lock is only held
//! for a clone or a store, never across `emit`, a tray call or a process. The
//! refresh thread runs `gh` directly, so the commands only read state or send a
//! wake.
//!
//! Lock order: `hidden_gate`, then `kept_config`, then `kept_full`, then the
//! snapshot lock (in `SourceCell`). The store lock is taken inside the gate but
//! never while a `kept_*` lock is held. The comment checks (processes) run
//! before the gate; the commit, the snapshot store and the kept list change
//! under it; publication happens after the gate is released.
//!
//! Logging: state kinds and elapsed time only. Never a repository name, a
//! title, a reference, a URL or a serde error.

use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Mutex, RwLockWriteGuard};
use std::time::{Duration, Instant};

use chrono::Local;
use tauri::{AppHandle, Manager};

use crate::cli::{CliRunner, GithubConfig, GithubRepo};
use crate::github::fetch::{RepoResult, fetch_assigned};
use crate::github::{self, GithubError, GithubIssue, GithubSnapshot, GithubState};
use crate::health;
use crate::hidden_issues::HiddenIssueError;
use crate::hidden_issues::github_round::{
    check_github_round, commit_github_round, partition_github,
};
use crate::hidden_issues::items::HiddenItemRecord;
use crate::runtime::now_ms;
use crate::settings::{DEFAULT_REFRESH_MINUTES, load_github_refresh_minutes, load_github_settings};
use crate::source_refresh::{self, RefreshSnapshot, SourceCell, SourceId};
use crate::store::StoreHandle;
use crate::todo;

const SOURCE: SourceId = SourceId {
    event: "github-changed",
    thread: "github-refresh",
    prefix: "github",
    target: module_path!(),
};

impl RefreshSnapshot for GithubSnapshot {
    fn seq(&self) -> u64 {
        self.seq
    }

    fn begin_refresh(&mut self) {
        self.refreshing = true;
        self.seq += 1;
    }
}

/// Managed state of the GitHub list, shared by the refresh thread, the health
/// round and the commands.
pub struct GithubRefreshState {
    cell: SourceCell<GithubSnapshot>,
    /// The config the kept repos came from. Written by the refresh thread only.
    kept_config: Mutex<Option<GithubConfig>>,
    /// The unpartitioned results of the last successful round, never
    /// serialized; kept and dropped together with `kept_config`.
    kept_full: Mutex<Option<Vec<RepoResult>>>,
    /// Serializes the round's commit and snapshot store with the hide commands.
    hidden_gate: Mutex<()>,
}

impl GithubRefreshState {
    fn new() -> (Self, mpsc::Receiver<()>) {
        let (cell, rx) = SourceCell::new(GithubSnapshot::starting(), SOURCE);
        let state = GithubRefreshState {
            cell,
            kept_config: Mutex::new(None),
            kept_full: Mutex::new(None),
            hidden_gate: Mutex::new(()),
        };
        (state, rx)
    }

    /// The stored snapshot (`starting` before the first round): a clone under a
    /// short lock.
    pub fn snapshot(&self) -> GithubSnapshot {
        self.cell.snapshot()
    }

    /// Asks the refresh thread for a round. Bursts coalesce; fails only when the
    /// thread is gone.
    pub fn wake(&self) -> Result<(), mpsc::SendError<()>> {
        self.cell.wake()
    }

    fn write(&self) -> RwLockWriteGuard<'_, GithubSnapshot> {
        self.cell.write()
    }

    /// One hidden-issue action on an issue of the kept full list, under
    /// `hidden_gate`. `act` does the store op and returns the GitHub records as
    /// they are afterwards; the list is re-partitioned from them and stored
    /// with `seq + 1`. Only `repos`, `hidden_error`, `hidden_check_failed` and
    /// `seq` of the current snapshot change; every other field stays. The
    /// caller publishes after this returns (the gate is released).
    ///
    /// `NotInList` when `reference` is not an issue of an `Ok` repo of the kept
    /// list (checked before `act` runs); `ListUnavailable` when there is no
    /// kept list or config.
    pub(crate) fn act_on_issue(
        &self,
        reference: &str,
        act: impl FnOnce(&GithubRepo, &GithubIssue) -> Result<Vec<HiddenItemRecord>, HiddenIssueError>,
    ) -> Result<(), HiddenIssueError> {
        let _gate = self.hidden_gate.lock().unwrap_or_else(|p| p.into_inner());
        let config = self
            .kept_config
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone();
        let full = self
            .kept_full
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone();
        let (Some(_), Some(full)) = (config, full) else {
            return Err(HiddenIssueError::ListUnavailable);
        };
        let (repo, issue) = full
            .iter()
            .filter_map(|r| r.outcome.as_ref().ok().map(|issues| (&r.repo, issues)))
            .find_map(|(repo, issues)| {
                issues
                    .iter()
                    .find(|i| i.reference == reference)
                    .map(|i| (repo, i))
            })
            .ok_or(HiddenIssueError::NotInList)?;
        let records = act(repo, issue)?;
        let repos = partition_github(&full, &records);
        let hidden_total: usize = repos.iter().map(|r| r.hidden.len()).sum();
        // One write on the current snapshot: a concurrent "refreshing" flag
        // flip and every field this action does not own must not be lost.
        let mut snap = self.write();
        snap.repos = repos;
        // Failed checks are only counted per round; never more than are hidden.
        snap.hidden_check_failed = snap.hidden_check_failed.min(hidden_total as u32);
        snap.hidden_error = None;
        snap.seq += 1;
        Ok(())
    }
}

/// The stored snapshot. Async so it never runs on the main thread.
#[tauri::command]
pub async fn github_snapshot(
    state: tauri::State<'_, GithubRefreshState>,
) -> Result<GithubSnapshot, String> {
    Ok(state.snapshot())
}

/// Wakes the refresh thread for an immediate round.
#[tauri::command]
pub async fn refresh_github(state: tauri::State<'_, GithubRefreshState>) -> Result<(), String> {
    state
        .wake()
        .map_err(|_| "Wątek odświeżania GitHuba nie działa".to_string())
}

/// Registers `GithubRefreshState` and returns the wake receiver for `start`. Must
/// run before `runtime::start`: the health thread reads this state in its first
/// round, and `app.state` panics on unmanaged state.
pub fn manage(app: &tauri::App) -> mpsc::Receiver<()> {
    let (state, rx) = GithubRefreshState::new();
    app.manage(state);
    rx
}

/// Starts the `github-refresh` thread. Runs after `runtime::start`, because the
/// thread wakes `HealthState`. `settings_path` is `None` when the app config
/// dir cannot be resolved.
pub fn start(app: &AppHandle, settings_path: Option<PathBuf>, wake_rx: mpsc::Receiver<()>) {
    let a = app.clone();
    let state = app.state::<GithubRefreshState>();
    // Never leave the view on "Pobieram…" and the tray without GitHub: when the
    // thread cannot start, store an error the view and the health round both
    // show.
    state.cell.start_thread(
        app,
        move || refresh_loop(a, settings_path, wake_rx),
        |prev| github::next_snapshot(prev, None, None, Err(GithubError::ThreadDown), now_ms()).0,
    );
}

fn refresh_loop(app: AppHandle, settings_path: Option<PathBuf>, wake_rx: mpsc::Receiver<()>) {
    let runner = CliRunner::system();
    // The record whose comment call stopped the last round; checks resume after it.
    let mut resume_after: Option<(String, u64)> = None;
    source_refresh::run_loop(wake_rx, || {
        refresh_round(&app, &settings_path, &runner, &mut resume_after)
    });
}

/// Short, value-free name of a snapshot state for log lines.
fn state_label(snap: &GithubSnapshot) -> String {
    match &snap.state {
        GithubState::Starting => "starting".to_string(),
        GithubState::Ok => "ok".to_string(),
        GithubState::NotConfigured => "notConfigured".to_string(),
        GithubState::Unavailable { .. } => "unavailable".to_string(),
        GithubState::Error { kind, .. } => format!("error:{kind}"),
    }
}

/// Re-reads the repo list (so a change needs no restart) and runs the fetch when
/// one is configured. Returns this round's config (if any) and its outcome.
fn run_fetch(
    settings_path: &Option<PathBuf>,
    runner: &CliRunner,
) -> (Option<GithubConfig>, Result<Vec<RepoResult>, GithubError>) {
    let Some(path) = settings_path else {
        return (None, Err(GithubError::ConfigDirUnresolved));
    };
    match load_github_settings(path) {
        Ok(Some(config)) => {
            let outcome = fetch_assigned(runner, &config);
            (Some(config), outcome)
        }
        Ok(None) => (None, Err(GithubError::NotConfigured)),
        Err(e) => (None, Err(e)),
    }
}

/// `(minutes, invalid)` for this round: `githubRefresh.minutes`, or the default
/// when it is invalid (`invalid` is then true).
fn refresh_interval(settings_path: &Option<PathBuf>) -> (u32, bool) {
    let Some(path) = settings_path else {
        return (DEFAULT_REFRESH_MINUTES, false);
    };
    match load_github_refresh_minutes(path) {
        Ok(minutes) => (minutes, false),
        Err(_) => (DEFAULT_REFRESH_MINUTES, true),
    }
}

fn refresh_round(
    app: &AppHandle,
    settings_path: &Option<PathBuf>,
    runner: &CliRunner,
    resume_after: &mut Option<(String, u64)>,
) -> Duration {
    let state = app.state::<GithubRefreshState>();
    let (minutes, refresh_invalid) = refresh_interval(settings_path);
    let wait = Duration::from_secs(u64::from(minutes) * 60);
    let started = Instant::now();
    let before = state.snapshot();
    let before_status = health::github_status(&before);

    // Show "refreshing" first; the stored repos and state stay as they were.
    state.cell.begin_round(app);

    let (config, fetched) = run_fetch(settings_path, runner);

    // Comment checks run `gh` and read the store: no lock held.
    let store = app.state::<StoreHandle>();
    let checks = match (&config, &fetched) {
        (Some(_), Ok(results)) => {
            let checks = check_github_round(
                &store,
                runner,
                results,
                now_ms() as i64,
                resume_after.clone(),
            );
            *resume_after = checks.stopped_at();
            Some(checks)
        }
        _ => None,
    };

    // The commit, the snapshot and the kept list change together under the
    // gate, so a command can never see (or publish over) a half-updated state.
    let gate = state.hidden_gate.lock().unwrap_or_else(|p| p.into_inner());
    let kept = state
        .kept_config
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    let results = fetched.as_ref().ok().cloned();
    let (mut next, next_kept) = {
        let prev = state.snapshot();
        github::next_snapshot(&prev, kept.as_ref(), config.as_ref(), fetched, now_ms())
    };
    if let (Some(cfg), Some(results), Some(checks)) = (&config, &results, checks) {
        let round = commit_github_round(&store, cfg, results, checks, now_ms() as i64);
        next.repos = round.repos;
        next.hidden_check_failed = round.check_failed;
        next.hidden_error = round.store_error.as_ref().map(HiddenIssueError::message_pl);
    }
    {
        let mut kept_full = state.kept_full.lock().unwrap_or_else(|p| p.into_inner());
        match results {
            Some(results) => *kept_full = Some(results),
            // A failed round keeps the list only while `next_snapshot` keeps it.
            None if next_kept.is_none() => *kept_full = None,
            None => {}
        }
    }
    next.refresh_setting_invalid = refresh_invalid;
    next.refresh_minutes = minutes;
    *state.kept_config.lock().unwrap_or_else(|p| p.into_inner()) = next_kept;
    let after_status = health::github_status(&next);
    let from = state_label(&before);
    let to = state_label(&next);
    let todo_changed = reconcile_todo(app, &next);
    // The wait starts when the snapshot is stored.
    next.next_refresh_at_ms = Some(source_refresh::next_refresh_at_ms(wait));
    *state.write() = next;
    drop(gate);
    publish(app, &state);
    SOURCE.notify_todo_changed(app, todo_changed);

    // A changed GitHub source status runs a health round now instead of waiting
    // up to 30 s for the tray to catch up.
    SOURCE.end_round(app, &from, &to, started, before_status != after_status);
    wait
}

/// The repos a "To do" reconcile may use: only those that loaded in a
/// successful round, as `(lowercase repo key, visible + hidden issues)`. A stale
/// list kept after a failed round, and a repo with its own error, never tick
/// anything.
fn reconcile_input(snap: &GithubSnapshot) -> Vec<(String, Vec<GithubIssue>)> {
    match snap.state {
        GithubState::Ok => snap
            .repos
            .iter()
            .filter(|r| r.error.is_none())
            .map(|r| {
                let all = r.issues.iter().chain(r.hidden.iter()).cloned().collect();
                (r.repo.to_lowercase(), all)
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Reconciles "To do" entries with the repos that loaded. Returns whether at
/// least one row changed. A store error is logged by kind and changes nothing
/// else.
fn reconcile_todo(app: &AppHandle, next: &GithubSnapshot) -> bool {
    let owned = reconcile_input(next);
    if owned.is_empty() {
        return false;
    }
    let loaded: Vec<(String, &[GithubIssue])> = owned
        .iter()
        .map(|(key, issues)| (key.clone(), issues.as_slice()))
        .collect();
    let now = Local::now().naive_local();
    let result = app.state::<StoreHandle>().with_store(now.date(), |s| {
        todo::reconcile_github(s, &loaded, now.date(), now)
    });
    SOURCE.log_reconcile(result)
}

/// Pushes the stored snapshot to the webview unless a newer one was already
/// pushed.
pub(crate) fn publish(app: &AppHandle, state: &GithubRefreshState) {
    state.cell.publish(app);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::GithubState as State;

    fn snap(state: State) -> GithubSnapshot {
        GithubSnapshot {
            state,
            ..GithubSnapshot::starting()
        }
    }

    #[test]
    fn state_labels_carry_kinds_only() {
        assert_eq!(state_label(&snap(State::Starting)), "starting");
        assert_eq!(state_label(&snap(State::Ok)), "ok");
        assert_eq!(state_label(&snap(State::NotConfigured)), "notConfigured");
        assert_eq!(
            state_label(&snap(State::Unavailable {
                message: "octo-test/alpha".into()
            })),
            "unavailable"
        );
        assert_eq!(
            state_label(&snap(State::Error {
                kind: "LoggedOut".into(),
                message: "octo-test/alpha".into()
            })),
            "error:LoggedOut"
        );
    }

    fn view(repo: &str, error: bool) -> github::RepoView {
        github::RepoView {
            repo: repo.to_string(),
            issues: vec![],
            hidden: vec![],
            error: error.then(|| github::RepoErrorView {
                kind: "Failed".into(),
                message: String::new(),
            }),
        }
    }

    #[test]
    fn reconcile_input_is_the_loaded_repos_of_an_ok_round_in_lowercase() {
        let mut ok = snap(State::Ok);
        ok.repos = vec![view("Octo-Test/Alpha", false), view("octo-test/beta", true)];
        let got = reconcile_input(&ok);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].0, "octo-test/alpha");

        // A kept list after a failed round never reconciles.
        for state in [
            State::Starting,
            State::NotConfigured,
            State::Unavailable {
                message: String::new(),
            },
            State::Error {
                kind: "LoggedOut".into(),
                message: String::new(),
            },
        ] {
            let mut stale = snap(state);
            stale.repos = vec![view("octo-test/alpha", false)];
            assert!(reconcile_input(&stale).is_empty());
        }
    }

    fn gh_issue(repo: &str, n: u64) -> GithubIssue {
        GithubIssue {
            repo: repo.to_string(),
            number: n,
            reference: format!("{}#{n}", repo.to_ascii_lowercase()),
            title: format!("Invented {n}"),
            web_url: None,
            updated_at: "2026-03-03T08:00:00Z".to_string(),
            labels: vec![],
            changed: None,
            hidden_at_ms: None,
        }
    }

    fn hidden_record(scope: &str, n: u64) -> HiddenItemRecord {
        use crate::hidden_issues::items::{HiddenKind, HiddenSource};
        HiddenItemRecord {
            source: HiddenSource::Github,
            scope: scope.to_string(),
            kind: HiddenKind::Issue,
            item_id: n,
            project_id: None,
            iid: None,
            status_fingerprint: vec![],
            baseline_updated_at: "2026-03-03T08:00:00Z".to_string(),
            settle_after_ms: None,
            hidden_at_ms: 1_700_000_000_000,
            absent_since_ms: None,
            changed_reason: None,
            changed_at_ms: None,
        }
    }

    /// A state holding a kept config and list: `octo-test/alpha` with issues 1
    /// and 2, and `octo-test/beta` that failed.
    fn state_with_list() -> GithubRefreshState {
        let (state, _rx) = GithubRefreshState::new();
        let alpha = GithubRepo::parse("octo-test/alpha").unwrap();
        let beta = GithubRepo::parse("octo-test/beta").unwrap();
        *state.kept_config.lock().unwrap() = Some(GithubConfig {
            repos: vec![alpha.clone(), beta.clone()],
        });
        let results = vec![
            RepoResult {
                repo: alpha,
                outcome: Ok(vec![
                    gh_issue("octo-test/alpha", 1),
                    gh_issue("octo-test/alpha", 2),
                ]),
            },
            RepoResult {
                repo: beta,
                outcome: Err(GithubError::Failed { exit_code: Some(1) }),
            },
        ];
        let mut snap = GithubSnapshot::starting();
        snap.state = State::Ok;
        snap.repos = results.iter().cloned().map(github::repo_view).collect();
        snap.last_success_ms = Some(1_700_000_000_000);
        snap.checked_at_ms = Some(1_700_000_000_500);
        snap.refreshing = true;
        snap.seq = 7;
        snap.hidden_check_failed = 5;
        snap.hidden_error = Some("stary błąd".to_string());
        *state.write() = snap;
        *state.kept_full.lock().unwrap() = Some(results);
        state
    }

    #[test]
    fn act_on_issue_needs_a_kept_list() {
        let (state, _rx) = GithubRefreshState::new();
        let r = state.act_on_issue("octo-test/alpha#1", |_, _| Ok(vec![]));
        assert!(matches!(r, Err(HiddenIssueError::ListUnavailable)));
        assert_eq!(state.snapshot(), GithubSnapshot::starting());
    }

    #[test]
    fn act_on_issue_refuses_an_unknown_reference_and_a_failed_repo() {
        let state = state_with_list();
        let before = state.snapshot();
        for reference in ["octo-test/alpha#9", "octo-test/beta#1", "alpha#1"] {
            let mut ran = false;
            let r = state.act_on_issue(reference, |_, _| {
                ran = true;
                Ok(vec![])
            });
            assert!(matches!(r, Err(HiddenIssueError::NotInList)));
            assert!(!ran);
        }
        assert_eq!(state.snapshot(), before);
    }

    #[test]
    fn act_on_issue_repartitions_and_changes_only_its_own_fields() {
        let state = state_with_list();
        let before = state.snapshot();
        state
            .act_on_issue("octo-test/alpha#2", |repo, issue| {
                assert_eq!(repo.key(), "octo-test/alpha");
                assert_eq!(issue.number, 2);
                Ok(vec![hidden_record("octo-test/alpha", 2)])
            })
            .unwrap();
        let after = state.snapshot();
        let alpha = &after.repos[0];
        assert_eq!(alpha.issues.len(), 1);
        assert_eq!(alpha.issues[0].number, 1);
        assert_eq!(alpha.hidden.len(), 1);
        assert_eq!(alpha.hidden[0].number, 2);
        // The failed repo keeps its error view.
        assert!(after.repos[1].error.is_some());
        assert_eq!(after.hidden_error, None);
        // Capped to the hidden total (one), bumped by one.
        assert_eq!(after.hidden_check_failed, 1);
        assert_eq!(after.seq, before.seq + 1);
        // Everything else of the current snapshot survives unchanged.
        assert_eq!(after.state, before.state);
        assert_eq!(after.last_success_ms, before.last_success_ms);
        assert_eq!(after.checked_at_ms, before.checked_at_ms);
        assert_eq!(after.refreshing, before.refreshing);
    }

    #[test]
    fn act_on_issue_keeps_a_lower_failed_count_and_passes_store_errors() {
        let state = state_with_list();
        state.write().hidden_check_failed = 0;
        let r = state.act_on_issue("octo-test/alpha#1", |_, _| Err(HiddenIssueError::NotFound));
        assert!(matches!(r, Err(HiddenIssueError::NotFound)));
        // A refused action changes nothing.
        assert_eq!(state.snapshot().seq, 7);
        state
            .act_on_issue("octo-test/alpha#1", |_, _| {
                Ok(vec![hidden_record("octo-test/alpha", 1)])
            })
            .unwrap();
        assert_eq!(state.snapshot().hidden_check_failed, 0);
    }

    #[test]
    fn reconcile_input_joins_visible_and_hidden_issues_once_per_repo() {
        let mut ok = snap(State::Ok);
        let mut alpha = view("Octo-Test/Alpha", false);
        alpha.issues = vec![gh_issue("Octo-Test/Alpha", 1)];
        alpha.hidden = vec![gh_issue("Octo-Test/Alpha", 2)];
        ok.repos = vec![alpha, view("octo-test/beta", false)];
        let got = reconcile_input(&ok);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].0, "octo-test/alpha");
        let numbers: Vec<u64> = got[0].1.iter().map(|i| i.number).collect();
        assert_eq!(numbers, vec![1, 2]);
        assert!(got[1].1.is_empty());
    }

    #[test]
    fn refresh_interval_reads_the_setting_and_falls_back_to_ten_minutes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let some = Some(path.clone());
        let write = |body: &str| std::fs::write(&path, body).unwrap();

        assert_eq!(refresh_interval(&None), (10, false));
        write(r#"{"version":1,"githubRefresh":{"minutes":3}}"#);
        assert_eq!(refresh_interval(&some), (3, false));
        write(r#"{"version":1,"githubRefresh":{"minutes":15}}"#);
        assert_eq!(refresh_interval(&some), (10, true));
        write(r#"{"version":1,"gitlabRefresh":{"minutes":3}}"#);
        assert_eq!(refresh_interval(&some), (10, false));
        write(r#"{"version":1,"gitlabRefresh":{"minutes":15}}"#);
        assert_eq!(refresh_interval(&some), (10, false));
    }

    #[test]
    fn state_holds_a_starting_snapshot_and_wakes_coalesce() {
        let (state, rx) = GithubRefreshState::new();
        assert_eq!(state.snapshot(), GithubSnapshot::starting());
        state.wake().unwrap();
        state.wake().unwrap();
        assert!(rx.recv().is_ok());
        while rx.try_recv().is_ok() {}
        assert!(rx.try_recv().is_err());
        drop(rx);
        assert!(state.wake().is_err());
    }
}
