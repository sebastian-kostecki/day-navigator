//! Side effects of the GitLab list: the `gitlab-refresh` thread, the managed
//! state, the commands and the `gitlab-changed` event. The rules live in
//! `gitlab/` and `health/` and stay free of `tauri`.
//!
//! Locking rule (same as `runtime.rs`): the snapshot lock is only held for a
//! clone or a store, never across `emit`, a tray call or a process. The refresh
//! thread runs `glab` directly, so the commands only read state or send a wake.
//!
//! `hidden_gate` serializes the hidden-issue state: a round's commit + snapshot
//! store and every hide/unhide/dismiss command run under it, so a late round can
//! never publish a partition older than a command's. It is never held across a
//! `glab` call or an `emit`. Lock order: gate, then `kept_config`, then
//! `kept_full`, then `kept_filter`, then the snapshot lock. The store lock is
//! taken inside the gate, but never while a `kept_*` lock is held.
//!
//! Logging: state kinds and elapsed time only. Never the host, a title, a
//! reference, a URL, a count tied to names or a serde error.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::{Duration, Instant};

use chrono::Local;
use tauri::{AppHandle, Emitter, Manager};

use crate::cli::{CliRunner, GitlabConfig, GitlabHost};
use crate::gitlab::fetch::{closed_window, fetch_assigned, fetch_recently_closed};
use crate::gitlab::filter::GitlabFilter;
use crate::gitlab::{
    self, AssignedItems, ClosedItem, ClosedSection, FilteredOut, GitlabError, GitlabItem,
    GitlabSnapshot, ItemKind,
};
use crate::health;
use crate::hidden_issues::{
    HiddenIssueError, HiddenRecord, check_round, commit_round, list as list_hidden, partition,
};
use crate::new_assignments::{
    NewAssignmentError, SessionSeen, items_for, keys_of, notices_for, record_round,
};
use crate::runtime::{HealthState, now_ms, toast};
use crate::settings::{
    DEFAULT_REFRESH_MINUTES, load_gitlab_filter, load_gitlab_refresh_minutes, load_gitlab_settings,
};
use crate::store::{StoreError, StoreHandle};
use crate::todo;

const EVENT: &str = "gitlab-changed";
/// Set after the first seen-set write failure of a session was logged as a
/// warning; later failures log at debug level.
static SEEN_SET_WARNED: AtomicBool = AtomicBool::new(false);

/// Managed state of the GitLab list, shared by the refresh thread, the health
/// round and the commands.
pub struct GitlabRefreshState {
    snapshot: RwLock<GitlabSnapshot>,
    /// The host and user the kept items came from. Read by the commands under
    /// `hidden_gate`; written by the refresh thread under it.
    kept_config: Mutex<Option<GitlabConfig>>,
    /// The last full (unpartitioned) list, kept with `kept_config`. Never
    /// serialized: the commands re-partition it, the UI never sends issue data.
    kept_full: Mutex<Option<AssignedItems>>,
    /// The visibility filter the snapshot's items were last shaped with, written
    /// with `kept_full` so the commands re-apply the round's rule.
    kept_filter: Mutex<GitlabFilter>,
    /// Serializes a round's commit-and-store step with the hidden-issue commands.
    hidden_gate: Mutex<()>,
    /// Wakes the refresh thread for an immediate round.
    wake: mpsc::Sender<()>,
    /// `seq` of the last snapshot pushed to the webview.
    published: Mutex<u64>,
}

impl GitlabRefreshState {
    fn new() -> (Self, mpsc::Receiver<()>) {
        let (wake, rx) = mpsc::channel();
        let state = GitlabRefreshState {
            snapshot: RwLock::new(GitlabSnapshot::starting()),
            kept_config: Mutex::new(None),
            kept_full: Mutex::new(None),
            kept_filter: Mutex::new(GitlabFilter::default()),
            hidden_gate: Mutex::new(()),
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

    /// One hidden-issue action on an issue of the kept full list, under
    /// `hidden_gate`. `act` does the store op and returns the host's records as
    /// they are afterwards; the list is re-partitioned from them and stored with
    /// `seq + 1`. The caller publishes after this returns (the gate is released).
    ///
    /// `NotInList` when `id` is not an issue of the kept list (checked before
    /// `act` runs); `ListUnavailable` when there is no kept list or config.
    pub(crate) fn act_on_issue(
        &self,
        id: u64,
        act: impl FnOnce(&GitlabHost, &GitlabItem) -> Result<Vec<HiddenRecord>, HiddenIssueError>,
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
        let filter = self
            .kept_filter
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone();
        let (Some(config), Some(full)) = (config, full) else {
            return Err(HiddenIssueError::ListUnavailable);
        };
        let item = full
            .issues
            .iter()
            .find(|i| i.id == id && i.kind == ItemKind::Issue)
            .ok_or(HiddenIssueError::NotInList)?;
        let records = act(&config.host, item)?;
        let (shown, filtered_out) = shape_items(&full, &records, &filter);
        // The checks ran on every hidden issue, so the cap ignores the filter.
        let hidden_total = shown.hidden_issues.len() as u32 + filtered_out.hidden_issues;
        // One write: a concurrent "refreshing" flag flip must not be lost.
        let mut snap = self.write();
        // Failed checks are only counted per round; never more than are hidden.
        snap.hidden_check_failed = snap.hidden_check_failed.min(hidden_total);
        snap.items = Some(shown);
        snap.filter_active = filter.is_active();
        snap.filtered_out = filtered_out;
        snap.hidden_error = None;
        snap.seq += 1;
        Ok(())
    }

    fn read(&self) -> RwLockReadGuard<'_, GitlabSnapshot> {
        self.snapshot.read().unwrap_or_else(|p| p.into_inner())
    }

    fn write(&self) -> RwLockWriteGuard<'_, GitlabSnapshot> {
        self.snapshot.write().unwrap_or_else(|p| p.into_inner())
    }
}

/// Applies the visibility filter to an already partitioned list and counts what
/// each section lost. Never touches the unfiltered list other consumers use.
fn filter_shown(shown: AssignedItems, filter: &GitlabFilter) -> (AssignedItems, FilteredOut) {
    let (merge_requests, mrs_out) = filter.apply(&shown.merge_requests);
    let (issues, issues_out) = filter.apply(&shown.issues);
    let (hidden_issues, hidden_out) = filter.apply(&shown.hidden_issues);
    (
        AssignedItems {
            merge_requests,
            issues,
            hidden_issues,
        },
        FilteredOut {
            merge_requests: mrs_out,
            issues: issues_out,
            hidden_issues: hidden_out,
        },
    )
}

/// The shown list for a kept full list: partition by the stored hidden records,
/// then filter. Shared by the hide commands and a failed round that re-shapes
/// the kept list.
fn shape_items(
    full: &AssignedItems,
    records: &[HiddenRecord],
    filter: &GitlabFilter,
) -> (AssignedItems, FilteredOut) {
    let (visible, hidden) = partition(&full.issues, records);
    filter_shown(
        AssignedItems {
            merge_requests: full.merge_requests.clone(),
            issues: visible,
            hidden_issues: hidden,
        },
        filter,
    )
}

/// A failed round that keeps the items shapes them again with this round's
/// filter, so an offline filter edit takes effect and an invalid-block notice
/// never sits over a list still filtered by the old rule. Returns whether the
/// list was re-shaped; the caller then stores `filter` as the kept filter.
///
/// `records` is `None` when the hidden records could not be read: the list then
/// stays as it was shown. The closed section cannot be re-fetched offline, so it
/// is dropped when this round's filter is invalid or has `closedDays` off, and
/// kept as it was otherwise.
fn reshape_kept(
    next: &mut GitlabSnapshot,
    full: Option<&AssignedItems>,
    records: Option<&[HiddenRecord]>,
    filter: &GitlabFilter,
    filter_invalid: bool,
) -> bool {
    if next.items.is_none() {
        return false;
    }
    if filter_invalid || filter.closed_days.is_none() {
        next.closed = None;
    }
    let (Some(full), Some(records)) = (full, records) else {
        return false;
    };
    let (shown, out) = shape_items(full, records, filter);
    next.items = Some(shown);
    next.filter_active = filter.is_active();
    next.filtered_out = out;
    true
}

/// Reads the `gitlabFilter` block for a round: the filter and whether the block
/// was invalid (then the default filter, which shows everything).
fn load_filter(settings_path: &Option<PathBuf>) -> (GitlabFilter, bool) {
    match settings_path {
        Some(path) => match load_gitlab_filter(path) {
            Ok(filter) => (filter, false),
            Err(_) => (GitlabFilter::default(), true),
        },
        None => (GitlabFilter::default(), false),
    }
}

/// Fetches and filters the recently-closed section. A failure (including a
/// window the calendar cannot express) becomes the section's own fixed-text
/// error with no items; the open list is not affected.
fn fetch_closed_section(
    runner: &CliRunner,
    config: &GitlabConfig,
    filter: &GitlabFilter,
    days: u32,
) -> ClosedSection {
    let today = Local::now().date_naive();
    let fetched = closed_window(today, days)
        .ok_or(GitlabError::ResponseInvalid)
        .and_then(|(after, cutoff)| fetch_recently_closed(runner, config, after, cutoff));
    match fetched {
        Ok(items) => {
            let total = items.len();
            let items: Vec<ClosedItem> = items
                .into_iter()
                .filter(|c| filter.keeps(&c.item))
                .collect();
            let filtered_out = (total - items.len()) as u32;
            ClosedSection {
                days,
                items,
                filtered_out,
                error: None,
            }
        }
        Err(e) => ClosedSection {
            days,
            items: Vec::new(),
            filtered_out: 0,
            error: Some(e.message_pl()),
        },
    }
}

/// The stored snapshot. Async so it never runs on the main thread.
#[tauri::command]
pub async fn gitlab_snapshot(
    state: tauri::State<'_, GitlabRefreshState>,
) -> Result<GitlabSnapshot, String> {
    Ok(state.snapshot())
}

/// Wakes the refresh thread for an immediate round.
#[tauri::command]
pub async fn refresh_gitlab(state: tauri::State<'_, GitlabRefreshState>) -> Result<(), String> {
    state
        .wake()
        .map_err(|_| "Wątek odświeżania GitLaba nie działa".to_string())
}

/// Registers `GitlabRefreshState` and returns the wake receiver for `start`. Must run
/// before `runtime::start`: the health thread reads this state in its first
/// round, and `app.state` panics on unmanaged state.
pub fn manage(app: &tauri::App) -> mpsc::Receiver<()> {
    let (state, rx) = GitlabRefreshState::new();
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
        // Never leave the view on "Pobieram…" and the tray green: store an error
        // the view and the health round both show.
        let state = app.state::<GitlabRefreshState>();
        let next = {
            let prev = state.snapshot();
            gitlab::next_snapshot(&prev, None, None, Err(GitlabError::ThreadDown), now_ms()).0
        };
        *state.write() = next;
        publish(app, &state);
        let _ = app.state::<HealthState>().wake.send(());
    }
}

fn refresh_loop(app: AppHandle, settings_path: Option<PathBuf>, wake_rx: mpsc::Receiver<()>) {
    let runner = CliRunner::system();
    // The issue whose notes call stopped the last round; checks resume after it.
    let mut resume_after = None;
    // Session memory for new-assignment toasts while the store fails.
    let mut session_seen = SessionSeen::new();
    loop {
        let wait = refresh_round(
            &app,
            &settings_path,
            &runner,
            &mut resume_after,
            &mut session_seen,
        );
        match wake_rx.recv_timeout(wait) {
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
        return (None, Err(GitlabError::ConfigDirUnresolved));
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

/// The wait before the next round and whether the setting was invalid. Read
/// every round, so a change needs no restart. No settings path or an invalid
/// block gives the default.
fn refresh_interval(settings_path: &Option<PathBuf>) -> (Duration, bool) {
    let (minutes, invalid) = match settings_path {
        Some(path) => match load_gitlab_refresh_minutes(path) {
            Ok(m) => (m, false),
            Err(_) => (DEFAULT_REFRESH_MINUTES, true),
        },
        None => (DEFAULT_REFRESH_MINUTES, false),
    };
    (Duration::from_secs(u64::from(minutes) * 60), invalid)
}

/// Runs one round and returns how long to wait before the next one.
fn refresh_round(
    app: &AppHandle,
    settings_path: &Option<PathBuf>,
    runner: &CliRunner,
    resume_after: &mut Option<u64>,
    session_seen: &mut SessionSeen,
) -> Duration {
    let (wait, refresh_invalid) = refresh_interval(settings_path);
    let (filter, filter_invalid) = load_filter(settings_path);
    let state = app.state::<GitlabRefreshState>();
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

    let (config, fetched) = run_fetch(settings_path, runner);

    // Comment checks run `glab` and read the store: no lock held.
    let store = app.state::<StoreHandle>();
    let checks = match (&config, &fetched) {
        (Some(cfg), Ok(full)) => {
            let checks = check_round(&store, runner, cfg, full, now_ms() as i64, *resume_after);
            *resume_after = checks.stopped_at();
            Some(checks)
        }
        _ => None,
    };

    // The recently-closed section runs `glab` too: only after a successful open
    // fetch, with no lock held. Its failure stays inside the section.
    let closed = match (&config, &fetched) {
        (Some(cfg), Ok(_)) if !filter_invalid => filter
            .closed_days
            .map(|days| fetch_closed_section(runner, cfg, &filter, days)),
        _ => None,
    };

    // The commit, the snapshot and the kept list change together under the
    // gate, so a command can never see (or publish over) a half-updated state.
    let gate = state.hidden_gate.lock().unwrap_or_else(|p| p.into_inner());
    let mut check_failed = 0;
    let mut hidden_error = None;
    let mut full_list = None;
    let mut filtered_out = FilteredOut::default();
    let outcome = match (&config, fetched, checks) {
        (Some(cfg), Ok(full), Some(checks)) => {
            let round = commit_round(&store, &cfg.host, &full, checks, now_ms() as i64);
            check_failed = round.check_failed;
            hidden_error = round.store_error.as_ref().map(HiddenIssueError::message_pl);
            // The filter applies after the partition; `full` stays unfiltered.
            let (shown, out) = filter_shown(
                AssignedItems {
                    merge_requests: full.merge_requests.clone(),
                    issues: round.visible_issues,
                    hidden_issues: round.hidden_issues,
                },
                &filter,
            );
            filtered_out = out;
            full_list = Some(full);
            Ok(shown)
        }
        // A list without a config does not occur; show it as fetched.
        (_, Ok(full), _) => {
            full_list = Some(full.clone());
            let (shown, out) = filter_shown(full, &filter);
            filtered_out = out;
            Ok(shown)
        }
        (_, Err(e), _) => Err(e),
    };
    let succeeded = outcome.is_ok();
    // The full list (hidden issues included) is the notification input, kept
    // before it moves into `kept_full`.
    let round_full = full_list.clone();

    let kept = state
        .kept_config
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    let (mut next, next_kept) = {
        let prev = state.snapshot();
        gitlab::next_snapshot(&prev, kept.as_ref(), config.as_ref(), outcome, now_ms())
    };
    if succeeded {
        next.hidden_check_failed = check_failed;
        next.hidden_error = hidden_error;
        next.filter_active = filter.is_active();
        next.filtered_out = filtered_out;
        next.closed = closed;
    }
    next.refresh_setting_invalid = refresh_invalid;
    next.filter_invalid = filter_invalid;
    // A failed round that keeps the items re-shapes them below. The hidden
    // records are read here, under the gate but before any `kept_*` lock.
    let kept_records = match (&full_list, &next_kept) {
        (None, Some(cfg)) if next.items.is_some() => {
            let today = Local::now().naive_local().date();
            store.with_store(today, |s| list_hidden(s, &cfg.host)).ok()
        }
        _ => None,
    };
    {
        let mut kept_full = state.kept_full.lock().unwrap_or_else(|p| p.into_inner());
        let mut kept_filter = state.kept_filter.lock().unwrap_or_else(|p| p.into_inner());
        match full_list {
            Some(full) => {
                *kept_full = Some(full);
                *kept_filter = filter.clone();
            }
            // A failed round keeps the list only while `next_snapshot` keeps it.
            None if next_kept.is_none() => {
                *kept_full = None;
                *kept_filter = GitlabFilter::default();
            }
            // A kept list is shaped again with this round's filter.
            None => {
                if reshape_kept(
                    &mut next,
                    kept_full.as_ref(),
                    kept_records.as_deref(),
                    &filter,
                    filter_invalid,
                ) {
                    *kept_filter = filter.clone();
                }
            }
        }
    }
    *state.kept_config.lock().unwrap_or_else(|p| p.into_inner()) = next_kept;
    let after_status = health::gitlab_status(&next);
    let from = state_label(&before);
    let to = state_label(&next);
    let notify_account = notify_input(&next).map(str::to_string);
    // "To do" reconciles on the full list, never the filtered one.
    let todo_changed = reconcile_todo(app, &next, round_full.as_ref());
    *state.write() = next;
    drop(gate);
    publish(app, &state);
    if todo_changed && let Err(e) = app.emit(todo::CHANGED_EVENT, ()) {
        log::warn!("todo: emit failed: {e}");
    }
    // After the gate and the publish: no lock is held while a toast shows.
    if let (Some(full), Some(account)) = (&round_full, &notify_account) {
        notify_new_assignments(app, account, full, session_seen);
    }

    if from != to {
        log::info!("gitlab: {from} -> {to}");
    }
    log::debug!("gitlab: round {to} in {} ms", started.elapsed().as_millis());

    if before_status != after_status {
        // The GitLab source status changed: run a health round now instead of
        // waiting up to 30 s for the tray to catch up.
        let _ = app.state::<HealthState>().wake.send(());
    }
    wait
}

/// The list a "To do" reconcile may use, with the account it came from: only
/// this round's full (unfiltered) list after a successful round. A stale list
/// kept after a failed round must never tick anything, and neither may the
/// filtered list the snapshot shows, which lacks the items the filter removed.
fn reconcile_input<'a>(
    snap: &'a GitlabSnapshot,
    full: Option<&'a AssignedItems>,
) -> Option<(&'a AssignedItems, &'a str)> {
    match snap.state {
        gitlab::GitlabState::Ok => Some((full?, snap.account.as_deref()?)),
        _ => None,
    }
}

/// The account a new-assignment step may act on: only a successful round with
/// an account. A stale list kept after a failed round must never notify.
fn notify_input(snap: &GitlabSnapshot) -> Option<&str> {
    match snap.state {
        gitlab::GitlabState::Ok => snap.account.as_deref(),
        _ => None,
    }
}

/// Records the round's full list in the seen set and shows a toast for every
/// new item. Runs outside every lock. A store error falls back to the session
/// memory and is logged by kind only; nothing logs a reference, title or URL.
fn notify_new_assignments(
    app: &AppHandle,
    account: &str,
    full: &AssignedItems,
    session_seen: &mut SessionSeen,
) {
    let current = keys_of(full);
    let now = Local::now().naive_local();
    let stored = app.state::<StoreHandle>().with_store(now.date(), |s| {
        record_round(s, account, &current, now_ms() as i64)
    });
    if let Err(e) = &stored {
        // The health round reports an unopenable store, not a failing write,
        // so the first non-busy failure of a session is a warning: until it
        // clears, toasts run on session memory and repeat after a restart.
        let busy = matches!(e, NewAssignmentError::Store(StoreError::Busy));
        if !busy && !SEEN_SET_WARNED.swap(true, Ordering::Relaxed) {
            log::warn!(
                "gitlab: seen set failed: {}; using session memory",
                e.kind()
            );
        } else {
            log::debug!("gitlab: seen set failed: {}", e.kind());
        }
    }
    let new_keys = session_seen.resolve(account, &current, stored);
    if new_keys.is_empty() {
        return;
    }
    let items = items_for(full, &new_keys);
    log::info!("gitlab: {} new assignments", items.len());
    for notice in notices_for(&items) {
        toast(app, "gitlab", &notice.title, &notice.body);
    }
}

/// Reconciles "To do" entries with a successful round's full list. Returns
/// whether at least one row changed. A store error is logged by kind and changes
/// nothing else.
fn reconcile_todo(app: &AppHandle, next: &GitlabSnapshot, full: Option<&AssignedItems>) -> bool {
    let Some((items, account)) = reconcile_input(next, full) else {
        return false;
    };
    let now = Local::now().naive_local();
    let result = app.state::<StoreHandle>().with_store(now.date(), |s| {
        todo::reconcile_gitlab(s, account, items, now.date(), now)
    });
    match result {
        Ok(changed) => {
            if changed > 0 {
                log::debug!("todo: reconcile changed {changed} rows");
            }
            changed > 0
        }
        // An unopenable store is reported by the health round; a warning every
        // GitLab round would only repeat it.
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
pub(crate) fn publish(app: &AppHandle, state: &GitlabRefreshState) {
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

    /// A snapshot with only `state` set; the hidden-issue fields keep their
    /// `starting` defaults.
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
    fn reconcile_input_only_for_ok_with_a_full_list_and_account() {
        // The shown (filtered) list differs from the round's full list.
        let full = AssignedItems {
            merge_requests: vec![],
            issues: vec![entry(1, ItemKind::Issue)],
            hidden_issues: vec![],
        };
        let with = |state| GitlabSnapshot {
            state,
            items: Some(AssignedItems::default()),
            account: Some("account-a".into()),
            ..GitlabSnapshot::starting()
        };
        // The full list is what comes back, never the snapshot's items.
        assert_eq!(
            reconcile_input(&with(State::Ok), Some(&full)),
            Some((&full, "account-a"))
        );
        // No full list from this round: nothing, even with shown items.
        assert!(reconcile_input(&with(State::Ok), None).is_none());
        // Items without the account they came from are never reconciled.
        let no_account = GitlabSnapshot {
            account: None,
            ..with(State::Ok)
        };
        assert!(reconcile_input(&no_account, Some(&full)).is_none());
        // A failed round keeps the last items; they must not be used.
        for state in [
            State::Unavailable {
                message: String::new(),
            },
            State::Error {
                kind: "LoggedOut".into(),
                message: String::new(),
            },
            State::NotConfigured,
            State::Starting,
        ] {
            assert!(reconcile_input(&with(state), Some(&full)).is_none());
        }
        // Ok without an account (not reachable in practice) gives nothing.
        assert!(reconcile_input(&snap(State::Ok), Some(&full)).is_none());
    }

    fn project_filter(include: &str) -> GitlabFilter {
        let mut filter = GitlabFilter::default();
        filter.projects.include = vec![include.to_string()];
        filter
    }

    fn entry_in(id: u64, kind: ItemKind, project: &str) -> GitlabItem {
        let sep = if kind == ItemKind::Issue { '#' } else { '!' };
        GitlabItem {
            reference: format!("{project}{sep}{id}"),
            ..entry(id, kind)
        }
    }

    #[test]
    fn filter_shown_filters_each_section_and_counts() {
        let shown = AssignedItems {
            merge_requests: vec![
                entry_in(1, ItemKind::MergeRequest, "acme/a"),
                entry_in(2, ItemKind::MergeRequest, "acme/b"),
            ],
            issues: vec![
                entry_in(3, ItemKind::Issue, "acme/b"),
                entry_in(4, ItemKind::Issue, "acme/b"),
            ],
            hidden_issues: vec![entry_in(5, ItemKind::Issue, "acme/b")],
        };
        let (out, counts) = filter_shown(shown.clone(), &project_filter("acme/a"));
        assert_eq!(out.merge_requests, vec![shown.merge_requests[0].clone()]);
        assert!(out.issues.is_empty() && out.hidden_issues.is_empty());
        assert_eq!(
            counts,
            FilteredOut {
                merge_requests: 1,
                issues: 2,
                hidden_issues: 1
            }
        );
        // The default filter changes nothing.
        let (same, none) = filter_shown(shown.clone(), &GitlabFilter::default());
        assert_eq!(same, shown);
        assert_eq!(none, FilteredOut::default());
    }

    #[test]
    fn a_filtered_out_issue_does_not_leave_the_full_list_the_round_reconciles() {
        // The round hands the full list to the reconcile and only the filtered
        // list to the snapshot.
        let full = AssignedItems {
            merge_requests: vec![],
            issues: vec![
                entry_in(1, ItemKind::Issue, "acme/a"),
                entry_in(2, ItemKind::Issue, "acme/b"),
            ],
            hidden_issues: vec![],
        };
        let (shown, _) = filter_shown(full.clone(), &project_filter("acme/a"));
        let snapshot = GitlabSnapshot {
            state: State::Ok,
            items: Some(shown),
            account: Some("account-a".into()),
            ..GitlabSnapshot::starting()
        };
        let (input, _) = reconcile_input(&snapshot, Some(&full)).unwrap();
        assert_eq!(input.issues.len(), 2);
        assert_eq!(snapshot.items.as_ref().unwrap().issues.len(), 1);
    }

    #[test]
    fn shape_items_partitions_then_filters_a_kept_list() {
        let full = AssignedItems {
            merge_requests: vec![entry_in(3, ItemKind::MergeRequest, "acme/b")],
            issues: vec![
                entry_in(1, ItemKind::Issue, "acme/a"),
                entry_in(2, ItemKind::Issue, "acme/a"),
                entry_in(4, ItemKind::Issue, "acme/b"),
            ],
            hidden_issues: vec![],
        };
        let records = [record(2, None)];
        let (shown, out) = shape_items(&full, &records, &project_filter("acme/a"));
        assert_eq!(shown.issues.len(), 1);
        assert_eq!(shown.issues[0].id, 1);
        assert_eq!(shown.hidden_issues.len(), 1);
        assert_eq!(shown.hidden_issues[0].id, 2);
        assert!(shown.merge_requests.is_empty());
        assert_eq!(
            out,
            FilteredOut {
                merge_requests: 1,
                issues: 1,
                hidden_issues: 0
            }
        );
        // After an invalid block the default filter shows every item again.
        let (all, none) = shape_items(&full, &records, &GitlabFilter::default());
        assert_eq!(all.issues.len(), 2);
        assert_eq!(all.merge_requests.len(), 1);
        assert_eq!(none, FilteredOut::default());
    }

    fn closed_item(id: u64) -> ClosedItem {
        ClosedItem {
            item: entry(id, ItemKind::Issue),
            closed_at: "2026-03-02T10:00:00+00:00".into(),
        }
    }

    /// The state of a failed round that kept the items: last shown with an
    /// `acme/a` project filter, closed section on.
    fn kept_round() -> (GitlabSnapshot, AssignedItems) {
        let full = AssignedItems {
            merge_requests: vec![entry_in(3, ItemKind::MergeRequest, "acme/b")],
            issues: vec![
                entry_in(1, ItemKind::Issue, "acme/a"),
                entry_in(2, ItemKind::Issue, "acme/a"),
                entry_in(4, ItemKind::Issue, "acme/b"),
            ],
            hidden_issues: vec![],
        };
        let (shown, out) = shape_items(&full, &[], &project_filter("acme/a"));
        let next = GitlabSnapshot {
            state: State::Unavailable {
                message: String::new(),
            },
            items: Some(shown),
            filter_active: true,
            filtered_out: out,
            closed: Some(ClosedSection {
                days: 7,
                items: vec![closed_item(9)],
                filtered_out: 0,
                error: None,
            }),
            ..GitlabSnapshot::starting()
        };
        (next, full)
    }

    fn closed_on(mut filter: GitlabFilter) -> GitlabFilter {
        filter.closed_days = Some(7);
        filter
    }

    #[test]
    fn reshape_kept_applies_this_rounds_filter_and_hidden_records() {
        let (mut next, full) = kept_round();
        let closed = next.closed.clone();
        let records = [record(2, None)];
        let filter = closed_on(project_filter("acme/b"));
        assert!(reshape_kept(
            &mut next,
            Some(&full),
            Some(&records),
            &filter,
            false
        ));
        let items = next.items.as_ref().unwrap();
        assert_eq!(items.issues.iter().map(|i| i.id).collect::<Vec<_>>(), [4]);
        assert_eq!(items.merge_requests.len(), 1);
        assert!(items.hidden_issues.is_empty());
        assert!(next.filter_active);
        assert_eq!(
            next.filtered_out,
            FilteredOut {
                merge_requests: 0,
                issues: 1,
                hidden_issues: 1
            }
        );
        // A valid rule with the closed section on keeps the section as it was.
        assert_eq!(next.closed, closed);
    }

    #[test]
    fn reshape_kept_after_an_invalid_block_shows_everything_and_drops_closed() {
        let (mut next, full) = kept_round();
        assert!(reshape_kept(
            &mut next,
            Some(&full),
            Some(&[]),
            &GitlabFilter::default(),
            true
        ));
        let items = next.items.as_ref().unwrap();
        assert_eq!(items.issues.len(), 3);
        assert_eq!(items.merge_requests.len(), 1);
        assert!(!next.filter_active);
        assert_eq!(next.filtered_out, FilteredOut::default());
        assert_eq!(next.closed, None);
    }

    #[test]
    fn reshape_kept_drops_closed_when_closed_days_is_switched_off() {
        let (mut next, full) = kept_round();
        assert!(reshape_kept(
            &mut next,
            Some(&full),
            Some(&[]),
            &project_filter("acme/a"),
            false
        ));
        assert_eq!(next.closed, None);
    }

    #[test]
    fn reshape_kept_without_records_keeps_the_shown_list() {
        let (mut next, full) = kept_round();
        let before = next.clone();
        // The store read failed: the list stays as it was shown and the kept
        // filter is not replaced, but an invalid block still drops `closed`.
        assert!(!reshape_kept(
            &mut next,
            Some(&full),
            None,
            &GitlabFilter::default(),
            true
        ));
        assert_eq!(next.items, before.items);
        assert_eq!(next.filtered_out, before.filtered_out);
        assert!(next.filter_active);
        assert_eq!(next.closed, None);
    }

    #[test]
    fn reshape_kept_without_kept_items_changes_nothing() {
        let (mut next, full) = kept_round();
        next.items = None;
        let before = next.clone();
        assert!(!reshape_kept(
            &mut next,
            Some(&full),
            Some(&[]),
            &GitlabFilter::default(),
            true
        ));
        assert_eq!(next, before);
        // No kept full list: nothing to re-shape.
        let (mut next, _) = kept_round();
        let filter = closed_on(GitlabFilter::default());
        assert!(!reshape_kept(&mut next, None, Some(&[]), &filter, false));
        assert!(next.closed.is_some());
    }

    #[test]
    fn load_filter_falls_back_to_the_default_and_flags_an_invalid_block() {
        assert_eq!(load_filter(&None), (GitlabFilter::default(), false));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(
            &path,
            r#"{"version":1,"gitlabFilter":{"projects":{"include":["acme/a"]}}}"#,
        )
        .unwrap();
        let (filter, invalid) = load_filter(&Some(path.clone()));
        assert!(filter.is_active() && !invalid);
        std::fs::write(&path, r#"{"version":1,"gitlabFilter":{"projcts":{}}}"#).unwrap();
        assert_eq!(load_filter(&Some(path)), (GitlabFilter::default(), true));
    }

    #[test]
    fn notify_input_only_for_ok_with_an_account() {
        let with = |state, account: Option<&str>| GitlabSnapshot {
            state,
            account: account.map(str::to_string),
            ..GitlabSnapshot::starting()
        };
        assert_eq!(
            notify_input(&with(State::Ok, Some("account-a"))),
            Some("account-a")
        );
        assert!(notify_input(&with(State::Ok, None)).is_none());
        // A failed round keeps the last account; it must not notify.
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
            assert!(notify_input(&with(state, Some("account-a"))).is_none());
        }
    }

    #[test]
    fn state_holds_a_starting_snapshot_and_wakes_coalesce() {
        let (state, rx) = GitlabRefreshState::new();
        assert_eq!(state.snapshot(), GitlabSnapshot::starting());
        state.wake().unwrap();
        state.wake().unwrap();
        assert!(rx.recv().is_ok());
        while rx.try_recv().is_ok() {}
        assert!(rx.try_recv().is_err());
        drop(rx);
        assert!(state.wake().is_err());
    }

    fn host() -> GitlabHost {
        GitlabHost::parse("gitlab.example.invalid").unwrap()
    }

    fn entry(id: u64, kind: ItemKind) -> GitlabItem {
        GitlabItem {
            kind,
            id,
            project_id: 100,
            iid: id,
            reference: format!("acme/invented#{id}"),
            title: format!("Invented {id}"),
            web_url: None,
            updated_at: "2026-03-03T08:00:00.000Z".into(),
            labels: vec![],
            draft: false,
            changed: None,
            milestone: None,
        }
    }

    fn record(id: u64, reason: Option<gitlab::ChangeReason>) -> HiddenRecord {
        HiddenRecord {
            issue_id: id,
            project_id: 100,
            iid: id,
            scoped_labels: vec![],
            baseline_updated_at: "2026-03-03T08:00:00.000Z".into(),
            settle_after_ms: None,
            hidden_at_ms: 1,
            absent_since_ms: None,
            changed_reason: reason,
            changed_at_ms: reason.map(|_| 2),
        }
    }

    /// State with a kept list of issues 1 and 2 and MR 3, as a round leaves it.
    fn kept_state() -> GitlabRefreshState {
        let (state, _rx) = GitlabRefreshState::new();
        *state.kept_config.lock().unwrap() = Some(GitlabConfig {
            host: host(),
            user: None,
        });
        *state.kept_full.lock().unwrap() = Some(AssignedItems {
            merge_requests: vec![entry(3, ItemKind::MergeRequest)],
            issues: vec![entry(1, ItemKind::Issue), entry(2, ItemKind::Issue)],
            hidden_issues: vec![],
        });
        state
    }

    #[test]
    fn act_on_issue_without_a_kept_list_is_list_unavailable() {
        let (state, _rx) = GitlabRefreshState::new();
        let out = state.act_on_issue(1, |_, _| panic!("no store op without a list"));
        assert!(matches!(out, Err(HiddenIssueError::ListUnavailable)));
        assert_eq!(state.snapshot().seq, 0);
    }

    #[test]
    fn act_on_issue_refuses_an_mr_or_an_unknown_id() {
        let state = kept_state();
        for id in [3, 99] {
            let out = state.act_on_issue(id, |_, _| panic!("no store op for {id}"));
            assert!(matches!(out, Err(HiddenIssueError::NotInList)));
        }
        assert_eq!(state.snapshot().seq, 0);
    }

    #[test]
    fn act_on_issue_repartitions_from_the_returned_rows() {
        let state = kept_state();
        {
            let mut snap = state.write();
            snap.seq = 7;
            snap.hidden_error = Some("stary błąd".into());
            snap.hidden_check_failed = 2;
        }

        state
            .act_on_issue(1, |h, item| {
                assert_eq!(h, &host());
                assert_eq!(item.id, 1);
                Ok(vec![
                    record(1, None),
                    record(2, Some(gitlab::ChangeReason::Comment)),
                ])
            })
            .unwrap();

        let snap = state.snapshot();
        assert_eq!(snap.seq, 8);
        assert_eq!(snap.hidden_error, None);
        // Never more failed checks than hidden issues.
        assert_eq!(snap.hidden_check_failed, 1);
        let items = snap.items.unwrap();
        assert_eq!(items.merge_requests, vec![entry(3, ItemKind::MergeRequest)]);
        assert_eq!(items.hidden_issues, vec![entry(1, ItemKind::Issue)]);
        assert_eq!(items.issues.len(), 1);
        assert_eq!(items.issues[0].id, 2);
        assert_eq!(items.issues[0].changed, Some(gitlab::ChangeReason::Comment));
    }

    #[test]
    fn act_on_issue_reapplies_the_kept_filter_and_stores_the_counts() {
        let (state, _rx) = GitlabRefreshState::new();
        *state.kept_config.lock().unwrap() = Some(GitlabConfig {
            host: host(),
            user: None,
        });
        *state.kept_full.lock().unwrap() = Some(AssignedItems {
            merge_requests: vec![
                entry_in(10, ItemKind::MergeRequest, "acme/a"),
                entry_in(11, ItemKind::MergeRequest, "acme/b"),
            ],
            issues: vec![
                entry_in(1, ItemKind::Issue, "acme/a"),
                entry_in(2, ItemKind::Issue, "acme/a"),
                entry_in(3, ItemKind::Issue, "acme/b"),
                entry_in(4, ItemKind::Issue, "acme/b"),
            ],
            hidden_issues: vec![],
        });
        *state.kept_filter.lock().unwrap() = project_filter("acme/a");
        {
            let mut snap = state.write();
            snap.filter_invalid = true;
            snap.hidden_check_failed = 3;
        }

        // Hide issues 1 (kept by the filter) and 3 (removed by it).
        state
            .act_on_issue(1, |_, _| Ok(vec![record(1, None), record(3, None)]))
            .unwrap();

        let snap = state.snapshot();
        let items = snap.items.unwrap();
        assert_eq!(items.merge_requests.len(), 1);
        assert_eq!(items.merge_requests[0].id, 10);
        assert_eq!(items.issues.iter().map(|i| i.id).collect::<Vec<_>>(), [2]);
        assert_eq!(
            items.hidden_issues.iter().map(|i| i.id).collect::<Vec<_>>(),
            [1]
        );
        assert!(snap.filter_active);
        assert_eq!(
            snap.filtered_out,
            FilteredOut {
                merge_requests: 1,
                issues: 1,
                hidden_issues: 1
            }
        );
        // The round flag is left as the last round set it; the cap uses the
        // hidden count before filtering (2).
        assert!(snap.filter_invalid);
        assert_eq!(snap.hidden_check_failed, 2);
    }

    #[test]
    fn act_on_issue_with_the_default_filter_reports_no_filter() {
        let state = kept_state();
        state
            .act_on_issue(1, |_, _| Ok(vec![record(1, None)]))
            .unwrap();
        let snap = state.snapshot();
        assert!(!snap.filter_active);
        assert_eq!(snap.filtered_out, FilteredOut::default());
        assert_eq!(snap.items.unwrap().merge_requests.len(), 1);
    }

    #[test]
    fn act_on_issue_leaves_the_closed_section_unchanged() {
        let state = kept_state();
        let closed = ClosedSection {
            days: 7,
            items: vec![closed_item(50)],
            filtered_out: 2,
            error: None,
        };
        state.write().closed = Some(closed.clone());
        state
            .act_on_issue(1, |_, _| Ok(vec![record(1, None)]))
            .unwrap();
        assert_eq!(state.snapshot().closed, Some(closed));
    }

    #[test]
    fn act_on_issue_keeps_the_refresh_setting_flag() {
        let state = kept_state();
        state.write().refresh_setting_invalid = true;
        state
            .act_on_issue(1, |_, _| Ok(vec![record(1, None)]))
            .unwrap();
        assert!(state.snapshot().refresh_setting_invalid);
    }

    #[test]
    fn refresh_interval_reads_the_setting_and_falls_back_to_ten_minutes() {
        assert_eq!(
            refresh_interval(&None),
            (Duration::from_secs(10 * 60), false)
        );
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, r#"{"version":1,"gitlabRefresh":{"minutes":3}}"#).unwrap();
        let p = Some(path.clone());
        assert_eq!(refresh_interval(&p), (Duration::from_secs(3 * 60), false));
        std::fs::write(&path, r#"{"version":1,"gitlabRefresh":{"minutes":15}}"#).unwrap();
        assert_eq!(refresh_interval(&p), (Duration::from_secs(10 * 60), true));
    }

    #[test]
    fn act_on_issue_store_error_leaves_the_snapshot_alone() {
        let state = kept_state();
        let out = state.act_on_issue(1, |_, _| Err(HiddenIssueError::NotFound));
        assert!(matches!(out, Err(HiddenIssueError::NotFound)));
        assert_eq!(state.snapshot(), GitlabSnapshot::starting());
    }
}
