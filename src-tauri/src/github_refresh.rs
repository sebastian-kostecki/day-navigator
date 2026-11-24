//! Side effects of the GitHub list: the `github-refresh` thread, the managed
//! state, the commands and the `github-changed` event. The rules live in
//! `github/` and `health/` and stay free of `tauri`.
//!
//! Locking rule (same as `gitlab_refresh.rs`): the snapshot lock is only held
//! for a clone or a store, never across `emit`, a tray call or a process. The
//! refresh thread runs `gh` directly, so the commands only read state or send a
//! wake.
//!
//! Logging: state kinds and elapsed time only. Never a repository name, a
//! title, a reference, a URL or a serde error.

use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::{Duration, Instant};

use chrono::Local;
use tauri::{AppHandle, Emitter, Manager};

use crate::cli::{CliRunner, GithubConfig};
use crate::github::fetch::{RepoResult, fetch_assigned};
use crate::github::{self, GithubError, GithubIssue, GithubSnapshot, GithubState};
use crate::health;
use crate::runtime::{HealthState, now_ms};
use crate::settings::load_github_settings;
use crate::store::StoreHandle;
use crate::todo;

const EVENT: &str = "github-changed";
const REFRESH_INTERVAL: Duration = Duration::from_secs(10 * 60);

/// Managed state of the GitHub list, shared by the refresh thread, the health
/// round and the commands.
pub struct GithubRefreshState {
    snapshot: RwLock<GithubSnapshot>,
    /// The config the kept repos came from. Written by the refresh thread only.
    kept_config: Mutex<Option<GithubConfig>>,
    /// Wakes the refresh thread for an immediate round.
    wake: mpsc::Sender<()>,
    /// `seq` of the last snapshot pushed to the webview.
    published: Mutex<u64>,
}

impl GithubRefreshState {
    fn new() -> (Self, mpsc::Receiver<()>) {
        let (wake, rx) = mpsc::channel();
        let state = GithubRefreshState {
            snapshot: RwLock::new(GithubSnapshot::starting()),
            kept_config: Mutex::new(None),
            wake,
            published: Mutex::new(0),
        };
        (state, rx)
    }

    /// The stored snapshot (`starting` before the first round): a clone under a
    /// short lock.
    pub fn snapshot(&self) -> GithubSnapshot {
        self.read().clone()
    }

    /// Asks the refresh thread for a round. Bursts coalesce; fails only when the
    /// thread is gone.
    pub fn wake(&self) -> Result<(), mpsc::SendError<()>> {
        self.wake.send(())
    }

    fn read(&self) -> RwLockReadGuard<'_, GithubSnapshot> {
        self.snapshot.read().unwrap_or_else(|p| p.into_inner())
    }

    fn write(&self) -> RwLockWriteGuard<'_, GithubSnapshot> {
        self.snapshot.write().unwrap_or_else(|p| p.into_inner())
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
    if let Err(e) = std::thread::Builder::new()
        .name("github-refresh".into())
        .spawn(move || refresh_loop(a, settings_path, wake_rx))
    {
        log::error!("github: cannot start refresh thread: {e}");
        // Never leave the view on "Pobieram…" and the tray without GitHub:
        // store an error the view and the health round both show.
        let state = app.state::<GithubRefreshState>();
        let next = {
            let prev = state.snapshot();
            github::next_snapshot(&prev, None, None, Err(GithubError::ThreadDown), now_ms()).0
        };
        *state.write() = next;
        publish(app, &state);
        let _ = app.state::<HealthState>().wake.send(());
    }
}

fn refresh_loop(app: AppHandle, settings_path: Option<PathBuf>, wake_rx: mpsc::Receiver<()>) {
    let runner = CliRunner::system();
    loop {
        refresh_round(&app, &settings_path, &runner);
        match wake_rx.recv_timeout(REFRESH_INTERVAL) {
            Ok(()) => while wake_rx.try_recv().is_ok() {},
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
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

fn refresh_round(app: &AppHandle, settings_path: &Option<PathBuf>, runner: &CliRunner) {
    let state = app.state::<GithubRefreshState>();
    let started = Instant::now();
    let before = state.snapshot();
    let before_status = health::github_status(&before);

    // Show "refreshing" first; the stored repos and state stay as they were.
    {
        let mut guard = state.write();
        guard.refreshing = true;
        guard.seq += 1;
    }
    publish(app, &state);

    let (config, fetched) = run_fetch(settings_path, runner);

    let kept = state
        .kept_config
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    let (next, next_kept) = {
        let prev = state.snapshot();
        github::next_snapshot(&prev, kept.as_ref(), config.as_ref(), fetched, now_ms())
    };
    *state.kept_config.lock().unwrap_or_else(|p| p.into_inner()) = next_kept;
    let after_status = health::github_status(&next);
    let from = state_label(&before);
    let to = state_label(&next);
    let todo_changed = reconcile_todo(app, &next);
    *state.write() = next;
    publish(app, &state);
    if todo_changed && let Err(e) = app.emit(todo::CHANGED_EVENT, ()) {
        log::warn!("todo: emit failed: {e}");
    }

    if from != to {
        log::info!("github: {from} -> {to}");
    }
    log::debug!("github: round {to} in {} ms", started.elapsed().as_millis());

    if before_status != after_status {
        // The GitHub source status changed: run a health round now instead of
        // waiting up to 30 s for the tray to catch up.
        let _ = app.state::<HealthState>().wake.send(());
    }
}

/// The repos a "To do" reconcile may use: only those that loaded in a
/// successful round, as `(lowercase repo key, issues)`. A stale list kept after
/// a failed round, and a repo with its own error, never tick anything.
fn reconcile_input(snap: &GithubSnapshot) -> Vec<(String, &[GithubIssue])> {
    match snap.state {
        GithubState::Ok => snap
            .repos
            .iter()
            .filter(|r| r.error.is_none())
            .map(|r| (r.repo.to_lowercase(), r.issues.as_slice()))
            .collect(),
        _ => Vec::new(),
    }
}

/// Reconciles "To do" entries with the repos that loaded. Returns whether at
/// least one row changed. A store error is logged by kind and changes nothing
/// else.
fn reconcile_todo(app: &AppHandle, next: &GithubSnapshot) -> bool {
    let loaded = reconcile_input(next);
    if loaded.is_empty() {
        return false;
    }
    let now = Local::now().naive_local();
    let result = app.state::<StoreHandle>().with_store(now.date(), |s| {
        todo::reconcile_github(s, &loaded, now.date(), now)
    });
    match result {
        Ok(changed) => {
            if changed > 0 {
                log::debug!("todo: reconcile changed {changed} rows");
            }
            changed > 0
        }
        // An unopenable store is reported by the health round; a warning every
        // GitHub round would only repeat it.
        Err(e @ todo::TodoError::Store(_)) => {
            log::debug!("todo: reconcile failed: {}", e.kind());
            false
        }
        Err(e) => {
            log::warn!("todo: reconcile failed: {}", e.kind());
            false
        }
    }
}

/// Pushes the stored snapshot to the webview unless a newer one was already
/// pushed. Re-reading under `published` keeps events ordered by `seq`.
pub(crate) fn publish(app: &AppHandle, state: &GithubRefreshState) {
    let mut last = state.published.lock().unwrap_or_else(|p| p.into_inner());
    let snap = state.snapshot();
    if snap.seq <= *last {
        return;
    }
    *last = snap.seq;
    if let Err(e) = app.emit(EVENT, &snap) {
        log::warn!("github: emit failed: {e}");
    }
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
