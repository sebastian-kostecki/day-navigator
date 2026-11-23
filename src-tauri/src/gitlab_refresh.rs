//! Side effects of the GitLab list: the `gitlab-refresh` thread, the managed
//! state, the commands and the `gitlab-changed` event. The rules live in
//! `gitlab/` and `health/` and stay free of `tauri`.
//!
//! Locking rule (same as `runtime.rs`): the snapshot lock is only held for a
//! clone or a store, never across `emit`, a tray call or a process. The refresh
//! thread runs `glab` directly, so the commands only read state or send a wake.
//!
//! Logging: state kinds and elapsed time only. Never the host, a title, a
//! reference, a URL, a count tied to names or a serde error.

use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};

use crate::cli::{CliRunner, GitlabConfig};
use crate::gitlab::fetch::fetch_assigned;
use crate::gitlab::{self, GitlabError, GitlabSnapshot};
use crate::health;
use crate::runtime::{HealthState, now_ms};
use crate::settings::load_gitlab_settings;

const EVENT: &str = "gitlab-changed";
const REFRESH_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// Managed state of the GitLab list, shared by the refresh thread, the health
/// round and the commands.
pub struct GitlabState {
    snapshot: RwLock<GitlabSnapshot>,
    /// The host and user the kept items came from. Only the refresh thread touches it.
    kept_host: Mutex<Option<GitlabConfig>>,
    /// Wakes the refresh thread for an immediate round.
    wake: mpsc::Sender<()>,
    /// `seq` of the last snapshot pushed to the webview.
    published: Mutex<u64>,
}

impl GitlabState {
    fn new() -> (Self, mpsc::Receiver<()>) {
        let (wake, rx) = mpsc::channel();
        let state = GitlabState {
            snapshot: RwLock::new(GitlabSnapshot::starting()),
            kept_host: Mutex::new(None),
            wake,
            published: Mutex::new(0),
        };
        (state, rx)
    }

    /// The stored snapshot (`starting` before the first round): a clone under a
    /// short lock.
    pub fn snapshot(&self) -> GitlabSnapshot {
        self.read().clone()
    }

    /// Asks the refresh thread for a round. Bursts coalesce; fails only when the
    /// thread is gone.
    pub fn wake(&self) -> Result<(), mpsc::SendError<()>> {
        self.wake.send(())
    }

    fn read(&self) -> RwLockReadGuard<'_, GitlabSnapshot> {
        self.snapshot.read().unwrap_or_else(|p| p.into_inner())
    }

    fn write(&self) -> RwLockWriteGuard<'_, GitlabSnapshot> {
        self.snapshot.write().unwrap_or_else(|p| p.into_inner())
    }
}

/// The stored snapshot. Async so it never runs on the main thread.
#[tauri::command]
pub async fn gitlab_snapshot(
    state: tauri::State<'_, GitlabState>,
) -> Result<GitlabSnapshot, String> {
    Ok(state.snapshot())
}

/// Wakes the refresh thread for an immediate round.
#[tauri::command]
pub async fn refresh_gitlab(state: tauri::State<'_, GitlabState>) -> Result<(), String> {
    state
        .wake()
        .map_err(|_| "Wątek odświeżania GitLaba nie działa".to_string())
}

/// Registers `GitlabState` and returns the wake receiver for `start`. Must run
/// before `runtime::start`: the health thread reads this state in its first
/// round, and `app.state` panics on unmanaged state.
pub fn manage(app: &tauri::App) -> mpsc::Receiver<()> {
    let (state, rx) = GitlabState::new();
    app.manage(state);
    rx
}

/// Starts the `gitlab-refresh` thread. Runs after `runtime::start`, because the
/// thread wakes `HealthState`. `settings_path` is `None` when the app config
/// dir cannot be resolved.
pub fn start(app: &AppHandle, settings_path: Option<PathBuf>, wake_rx: mpsc::Receiver<()>) {
    let a = app.clone();
    if let Err(e) = std::thread::Builder::new()
        .name("gitlab-refresh".into())
        .spawn(move || refresh_loop(a, settings_path, wake_rx))
    {
        log::error!("gitlab: cannot start refresh thread: {e}");
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
fn state_label(snap: &GitlabSnapshot) -> String {
    match &snap.state {
        gitlab::GitlabState::Starting => "starting".to_string(),
        gitlab::GitlabState::Ok => "ok".to_string(),
        gitlab::GitlabState::NotConfigured => "notConfigured".to_string(),
        gitlab::GitlabState::Unavailable { .. } => "unavailable".to_string(),
        gitlab::GitlabState::Error { kind, .. } => format!("error:{kind}"),
    }
}

/// Re-reads the host and user (so a change needs no restart) and runs the fetch when one
/// is configured. Returns this round's config (if any) and its outcome.
fn run_fetch(
    settings_path: &Option<PathBuf>,
    runner: &CliRunner,
) -> (
    Option<GitlabConfig>,
    Result<gitlab::AssignedItems, GitlabError>,
) {
    let Some(path) = settings_path else {
        return (None, Err(GitlabError::SettingsInvalid));
    };
    match load_gitlab_settings(path) {
        Ok(Some(config)) => {
            let outcome = fetch_assigned(runner, &config);
            (Some(config), outcome)
        }
        Ok(None) => (None, Err(GitlabError::NotConfigured)),
        Err(e) => (None, Err(e)),
    }
}

fn refresh_round(app: &AppHandle, settings_path: &Option<PathBuf>, runner: &CliRunner) {
    let state = app.state::<GitlabState>();
    let started = Instant::now();
    let before = state.snapshot();
    let before_status = health::gitlab_status(&before);

    // Show "refreshing" first; the stored items and state stay as they were.
    {
        let mut guard = state.write();
        guard.refreshing = true;
        guard.seq += 1;
    }
    publish(app, &state);

    let (config, outcome) = run_fetch(settings_path, runner);

    let kept = state
        .kept_host
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    let (next, next_kept) = {
        let prev = state.snapshot();
        gitlab::next_snapshot(&prev, kept.as_ref(), config.as_ref(), outcome, now_ms())
    };
    *state.kept_host.lock().unwrap_or_else(|p| p.into_inner()) = next_kept;
    let after_status = health::gitlab_status(&next);
    let from = state_label(&before);
    let to = state_label(&next);
    *state.write() = next;
    publish(app, &state);

    if from != to {
        log::info!("gitlab: {from} -> {to}");
    }
    log::debug!("gitlab: round {to} in {} ms", started.elapsed().as_millis());

    if before_status != after_status {
        // The GitLab source status changed: run a health round now instead of
        // waiting up to 30 s for the tray to catch up.
        let _ = app.state::<HealthState>().wake.send(());
    }
}

/// Pushes the stored snapshot to the webview unless a newer one was already
/// pushed. Re-reading under `published` keeps events ordered by `seq`.
fn publish(app: &AppHandle, state: &GitlabState) {
    let mut last = state.published.lock().unwrap_or_else(|p| p.into_inner());
    let snap = state.snapshot();
    if snap.seq <= *last {
        return;
    }
    *last = snap.seq;
    if let Err(e) = app.emit(EVENT, &snap) {
        log::warn!("gitlab: emit failed: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gitlab::GitlabState as State;

    fn snap(state: State) -> GitlabSnapshot {
        GitlabSnapshot {
            state,
            ..GitlabSnapshot::starting()
        }
    }

    #[test]
    fn state_labels_carry_kinds_only() {
        assert_eq!(state_label(&snap(State::Starting)), "starting");
        assert_eq!(state_label(&snap(State::Ok)), "ok");
        assert_eq!(state_label(&snap(State::NotConfigured)), "notConfigured");
        assert_eq!(
            state_label(&snap(State::Unavailable {
                message: "secret-host.example.invalid".into()
            })),
            "unavailable"
        );
        assert_eq!(
            state_label(&snap(State::Error {
                kind: "LoggedOut".into(),
                message: "secret-host.example.invalid".into()
            })),
            "error:LoggedOut"
        );
    }

    #[test]
    fn state_holds_a_starting_snapshot_and_wakes_coalesce() {
        let (state, rx) = GitlabState::new();
        assert_eq!(state.snapshot(), GitlabSnapshot::starting());
        state.wake().unwrap();
        state.wake().unwrap();
        assert!(rx.recv().is_ok());
        while rx.try_recv().is_ok() {}
        assert!(rx.try_recv().is_err());
        drop(rx);
        assert!(state.wake().is_err());
    }
}
