//! Side effects of the GitLab list: the `gitlab-refresh` thread, the managed
//! state, the commands and the `gitlab-changed` event. The rules live in
//! `gitlab/` and `health/` and stay free of `tauri`.
//!
//! Locking rule (same as `runtime.rs`): the snapshot lock is only held for a
//! clone or a store, never across `emit`, a tray call or a process. The refresh
//! thread runs `glab` directly, so the commands only read state or send a wake.
//!
//! `hidden_gate` serializes the hidden-issue and hidden-MR state: a round's commit + snapshot
//! store and every hide/unhide/dismiss command run under it, so a late round can
//! never publish a partition older than a command's. It is never held across a
//! `glab` call or an `emit`. Lock order: gate, then `kept_config`, then
//! `kept_full`, then `kept_filter`, then `kept_new`, then `shown_until`, then
//! `last_mark`, then the snapshot lock (which now lives in the shared
//! `SourceCell` of `source_refresh.rs`). The store lock is taken inside the
//! gate, but never while a `kept_*`, `shown_until` or `last_mark` lock is held.
//! `check_mr_round` runs `glab` with no lock, right after `check_round`;
//! `commit_mr_round` runs under the gate, right after `commit_round`.
//!
//! NEW flags (S-02): a round records its seen set before the gate and brings
//! the fresh keys under it. `kept_new` holds them so every path that shapes the
//! list sets the same flags. `shown_until` is the `now_ms` of the last round
//! stored in the snapshot whose seen set was written: `mark_seen` saves exactly
//! that, never the wall clock. A failed seen-set write leaves the flags and
//! `shown_until` as they were, so leaving the view never marks unshown items
//! as seen.
//! `last_mark` is the last mark saved this session; a round that read an older
//! mark drops the fresh keys it already covers.
//!
//! Logging: state kinds and elapsed time only. Never the host, a title, a
//! reference, a URL, a count tied to names or a serde error.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Mutex, RwLockWriteGuard};
use std::time::{Duration, Instant};

use chrono::{Local, TimeZone};
use tauri::{AppHandle, Manager};

use crate::cli::{CliRunner, GitlabConfig, GitlabHost};
use crate::events::{self, EventError, NewEvent};
use crate::gitlab::fetch::{closed_window, fetch_assigned, fetch_recently_closed};
use crate::gitlab::filter::GitlabFilter;
use crate::gitlab::{
    self, AssignedItems, ClosedItem, ClosedSection, FilteredOut, GitlabError, GitlabItem,
    GitlabSnapshot, ItemKind,
};
use crate::health;
use crate::hidden_issues::{
    GitlabHidden, HiddenIssueError, check_mr_round, check_round, commit_mr_round, commit_round,
    list_gitlab, mr_calls_allowed, partition, partition_merge_requests,
};
use crate::new_assignments::{
    ItemKey, NewAssignmentError, SessionSeen, items_for, keys_of, mark_fresh, mark_view_seen,
    notices_for, record_round, review_only_keys,
};
use crate::runtime::{now_ms, toast};
use crate::settings::{
    DEFAULT_REFRESH_MINUTES, load_gitlab_filter, load_gitlab_refresh_minutes, load_gitlab_settings,
};
use crate::source_refresh::{self, RefreshSnapshot, SourceCell, SourceId};
use crate::store::{StoreError, StoreHandle};
use crate::todo;

const SOURCE: SourceId = SourceId {
    event: "gitlab-changed",
    thread: "gitlab-refresh",
    prefix: "gitlab",
    target: module_path!(),
};
/// Set after the first seen-set write failure of a session was logged as a
/// warning; later failures log at debug level.
static SEEN_SET_WARNED: AtomicBool = AtomicBool::new(false);

impl RefreshSnapshot for GitlabSnapshot {
    fn seq(&self) -> u64 {
        self.seq
    }

    fn begin_refresh(&mut self) {
        self.refreshing = true;
        self.seq += 1;
    }
}

/// Managed state of the GitLab list, shared by the refresh thread, the health
/// round and the commands.
pub struct GitlabRefreshState {
    cell: SourceCell<GitlabSnapshot>,
    /// The host and user the kept items came from. Read by the commands under
    /// `hidden_gate`; written by the refresh thread under it.
    kept_config: Mutex<Option<GitlabConfig>>,
    /// The last full (unpartitioned) list, kept with `kept_config`. Never
    /// serialized: the commands re-partition it, the UI never sends issue data.
    kept_full: Mutex<Option<AssignedItems>>,
    /// The visibility filter the snapshot's items were last shaped with, written
    /// with `kept_full` so the commands re-apply the round's rule.
    kept_filter: Mutex<GitlabFilter>,
    /// The keys flagged NEW on the kept list, so every path that shapes it
    /// (a failed round, a hide command) sets the same flags.
    kept_new: Mutex<HashSet<ItemKey>>,
    /// The account and the `now_ms` of the last successful round stored in the
    /// snapshot whose seen set was written: the value `mark_seen` saves. A round
    /// whose seen-set write failed leaves it unchanged. Cleared with `kept_full`.
    shown_until: Mutex<Option<(String, i64)>>,
    /// The account and the last `seen_until_ms` saved this session.
    last_mark: Mutex<Option<(String, i64)>>,
    /// Serializes a round's commit-and-store step with the hidden-issue commands.
    hidden_gate: Mutex<()>,
}

impl GitlabRefreshState {
    fn new() -> (Self, mpsc::Receiver<()>) {
        let (cell, rx) = SourceCell::new(GitlabSnapshot::starting(), SOURCE);
        let state = GitlabRefreshState {
            cell,
            kept_config: Mutex::new(None),
            kept_full: Mutex::new(None),
            kept_filter: Mutex::new(GitlabFilter::default()),
            kept_new: Mutex::new(HashSet::new()),
            shown_until: Mutex::new(None),
            last_mark: Mutex::new(None),
            hidden_gate: Mutex::new(()),
        };
        (state, rx)
    }

    /// The stored snapshot (`starting` before the first round): a clone under a
    /// short lock.
    pub fn snapshot(&self) -> GitlabSnapshot {
        self.cell.snapshot()
    }

    /// Asks the refresh thread for a round. Bursts coalesce; fails only when the
    /// thread is gone.
    pub fn wake(&self) -> Result<(), mpsc::SendError<()>> {
        self.cell.wake()
    }

    /// One hidden-item action on an issue or a merge request of the kept full
    /// list, under `hidden_gate`. `act` does the store op and returns the host's
    /// records of both kinds as they are afterwards; both lists are
    /// re-partitioned from them and stored with `seq + 1`. The caller publishes
    /// after this returns (the gate is released).
    ///
    /// `NotInList` when `id` of `kind` is not on the kept list (checked before
    /// `act` runs); `ListUnavailable` when there is no kept list or config.
    pub(crate) fn act_on_item(
        &self,
        kind: ItemKind,
        id: u64,
        act: impl FnOnce(&GitlabHost, &GitlabItem) -> Result<GitlabHidden, HiddenIssueError>,
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
        let fresh = self
            .kept_new
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone();
        let (Some(config), Some(full)) = (config, full) else {
            return Err(HiddenIssueError::ListUnavailable);
        };
        let listed: Vec<&GitlabItem> = match kind {
            ItemKind::Issue => full.issues.iter().collect(),
            ItemKind::MergeRequest => full
                .merge_requests
                .iter()
                .chain(&full.review_requests)
                .collect(),
        };
        let item = listed
            .into_iter()
            .find(|i| i.id == id && i.kind == kind)
            .ok_or(HiddenIssueError::NotInList)?;
        let hidden = act(&config.host, item)?;
        let (shown, filtered_out) = shape_items(&full, &hidden, &fresh, &filter);
        // The checks ran on every hidden item, so the cap ignores the filter.
        let hidden_total = shown.hidden_issues.len() as u32
            + filtered_out.hidden_issues
            + shown.hidden_merge_requests.len() as u32
            + filtered_out.hidden_merge_requests;
        // One write: a concurrent "refreshing" flag flip must not be lost.
        let mut snap = self.write();
        // Failed checks are only counted per round; never more than are hidden.
        snap.hidden_check_failed = snap.hidden_check_failed.min(hidden_total);
        snap.items = Some(shown);
        snap.filter_active = filter.is_active();
        snap.filter = filter.clone();
        snap.filtered_out = filtered_out;
        snap.hidden_error = None;
        snap.seq += 1;
        Ok(())
    }

    /// A successful round's NEW keys, under `hidden_gate`: drops the keys the
    /// last saved mark of `account` already covers (`first_seen_ms <=
    /// last_mark`, a round that read the mark before the user left the view),
    /// stores the rest as `kept_new` and records `now_ms` as `shown_until`.
    /// `fresh` is `None` when the seen-set write failed: then `kept_new` and
    /// `shown_until` stay unchanged (leaving the view never marks unshown items
    /// as seen) and the current flags are returned.
    /// `account` is `None` when the round has no account: nothing is flagged.
    /// Returns the set to flag the round's list with.
    fn accept_round(
        &self,
        account: Option<&str>,
        now_ms: i64,
        fresh: Option<&[(ItemKey, i64)]>,
    ) -> HashSet<ItemKey> {
        let mut kept_new = self.kept_new.lock().unwrap_or_else(|p| p.into_inner());
        let mut shown_until = self.shown_until.lock().unwrap_or_else(|p| p.into_inner());
        let last_mark = self.last_mark.lock().unwrap_or_else(|p| p.into_inner());
        let Some(account) = account else {
            kept_new.clear();
            *shown_until = None;
            return HashSet::new();
        };
        let Some(fresh) = fresh else {
            return kept_new.clone();
        };
        let covered = match &*last_mark {
            Some((a, mark)) if a == account => Some(*mark),
            _ => None,
        };
        *kept_new = fresh
            .iter()
            .filter(|(_, first_seen_ms)| covered.is_none_or(|mark| *first_seen_ms > mark))
            .map(|(key, _)| *key)
            .collect();
        *shown_until = Some((account.to_string(), now_ms));
        kept_new.clone()
    }

    /// Ends a visit, under `hidden_gate`: saves the mark of the last shown
    /// round with `save(account, shown_until_ms)` (never the clock), remembers
    /// it as `last_mark`, clears the NEW flags and bumps `seq`. Returns whether
    /// anything was saved: `Ok(false)` without a shown round (no successful
    /// round yet, or the list was dropped). A `save` error changes nothing.
    /// The caller publishes after this returns.
    pub(crate) fn mark_seen(
        &self,
        save: impl FnOnce(&str, i64) -> Result<(), NewAssignmentError>,
    ) -> Result<bool, NewAssignmentError> {
        let _gate = self.hidden_gate.lock().unwrap_or_else(|p| p.into_inner());
        // Cloned and released: `save` takes the store lock.
        let shown = self
            .shown_until
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone();
        let Some((account, until)) = shown else {
            return Ok(false);
        };
        save(&account, until)?;
        let mut kept_new = self.kept_new.lock().unwrap_or_else(|p| p.into_inner());
        let mut last_mark = self.last_mark.lock().unwrap_or_else(|p| p.into_inner());
        kept_new.clear();
        *last_mark = Some((account, until));
        let mut snap = self.write();
        if let Some(items) = snap.items.as_mut() {
            mark_fresh(items, &HashSet::new());
        }
        snap.seq += 1;
        Ok(true)
    }

    fn write(&self) -> RwLockWriteGuard<'_, GitlabSnapshot> {
        self.cell.write()
    }
}

/// Applies the visibility filter to an already partitioned list and counts what
/// each section lost. Never touches the unfiltered list other consumers use.
fn filter_shown(shown: AssignedItems, filter: &GitlabFilter) -> (AssignedItems, FilteredOut) {
    let (merge_requests, mrs_out) = filter.apply(&shown.merge_requests);
    let (issues, issues_out) = filter.apply(&shown.issues);
    let (hidden_issues, hidden_out) = filter.apply(&shown.hidden_issues);
    let (review_requests, review_out) = filter.apply(&shown.review_requests);
    let (hidden_merge_requests, hidden_mrs_out) = filter.apply(&shown.hidden_merge_requests);
    (
        AssignedItems {
            merge_requests,
            issues,
            hidden_issues,
            review_requests,
            hidden_merge_requests,
        },
        FilteredOut {
            merge_requests: mrs_out,
            issues: issues_out,
            hidden_issues: hidden_out,
            review_requests: review_out,
            hidden_merge_requests: hidden_mrs_out,
        },
    )
}

/// The shown list for a kept full list: partition by the stored hidden records,
/// then flag the NEW keys, then filter. Shared by the hide commands and a
/// failed round that re-shapes the kept list.
fn shape_items(
    full: &AssignedItems,
    hidden: &GitlabHidden,
    fresh: &HashSet<ItemKey>,
    filter: &GitlabFilter,
) -> (AssignedItems, FilteredOut) {
    let (visible, hidden_issues) = partition(&full.issues, &hidden.issues);
    let mrs = partition_merge_requests(
        &full.merge_requests,
        &full.review_requests,
        &hidden.merge_requests,
    );
    let mut items = AssignedItems {
        merge_requests: mrs.merge_requests,
        issues: visible,
        hidden_issues,
        review_requests: mrs.review_requests,
        hidden_merge_requests: mrs.hidden,
    };
    mark_fresh(&mut items, fresh);
    filter_shown(items, filter)
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
    records: Option<&GitlabHidden>,
    fresh: &HashSet<ItemKey>,
    filter: &GitlabFilter,
    filter_invalid: bool,
) -> bool {
    if next.items.is_none() {
        return false;
    }
    if filter_invalid || filter.closed_days.is_none() {
        next.closed = None;
        // The bar must not announce a closed section that is gone.
        next.filter.closed_days = None;
    }
    let (Some(full), Some(records)) = (full, records) else {
        return false;
    };
    let (shown, out) = shape_items(full, records, fresh, filter);
    next.items = Some(shown);
    next.filter_active = filter.is_active();
    next.filter = filter.clone();
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

/// Ends a visit to the GitLab view: saves the mark of the last shown round,
/// clears the NEW flags in the snapshot and publishes it. Writes the app store
/// only; never calls `glab`. Errors are logged by kind only.
#[tauri::command]
pub async fn mark_gitlab_seen(app: AppHandle) -> Result<(), NewAssignmentError> {
    let handle = app.clone();
    let today = Local::now().date_naive();
    let result = crate::blocking(move || {
        let state = handle.state::<GitlabRefreshState>();
        let store = handle.state::<StoreHandle>();
        state.mark_seen(|account, until| {
            store.with_store(today, |s| mark_view_seen(s, account, until))
        })
    })
    .await;
    match &result {
        Ok(true) => publish(&app, &app.state::<GitlabRefreshState>()),
        Ok(false) => {}
        Err(e @ NewAssignmentError::Store(StoreError::Busy)) => {
            log::debug!("gitlab: mark seen failed: {}", e.kind());
        }
        Err(e) => log::warn!("gitlab: mark seen failed: {}", e.kind()),
    }
    result.map(|_| ())
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
    let state = app.state::<GitlabRefreshState>();
    // Never leave the view on "Pobieram…" and the tray green: when the thread
    // cannot start, store an error the view and the health round both show.
    state.cell.start_thread(
        app,
        move || refresh_loop(a, settings_path, wake_rx),
        |prev| gitlab::next_snapshot(prev, None, None, Err(GitlabError::ThreadDown), now_ms()).0,
    );
}

fn refresh_loop(app: AppHandle, settings_path: Option<PathBuf>, wake_rx: mpsc::Receiver<()>) {
    let runner = CliRunner::system();
    // The issue whose notes call stopped the last round; checks resume after it.
    let mut resume_after = None;
    // The merge request whose notes call stopped the last round's MR checks.
    let mut mr_resume_after = None;
    // Whether the last round skipped the MR calls after an issue stop.
    let mut mr_skipped = false;
    // Session memory for new-assignment toasts while the store fails.
    let mut session_seen = SessionSeen::new();
    source_refresh::run_loop(wake_rx, || {
        refresh_round(
            &app,
            &settings_path,
            &runner,
            &mut resume_after,
            &mut mr_resume_after,
            &mut mr_skipped,
            &mut session_seen,
        )
    });
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

/// The interval in minutes and whether the setting was invalid. Read
/// every round, so a change needs no restart. No settings path or an invalid
/// block gives the default.
fn refresh_interval(settings_path: &Option<PathBuf>) -> (u32, bool) {
    let (minutes, invalid) = match settings_path {
        Some(path) => match load_gitlab_refresh_minutes(path) {
            Ok(m) => (m, false),
            Err(_) => (DEFAULT_REFRESH_MINUTES, true),
        },
        None => (DEFAULT_REFRESH_MINUTES, false),
    };
    (minutes, invalid)
}

/// Runs one round and returns how long to wait before the next one.
fn refresh_round(
    app: &AppHandle,
    settings_path: &Option<PathBuf>,
    runner: &CliRunner,
    resume_after: &mut Option<u64>,
    mr_resume_after: &mut Option<u64>,
    mr_skipped: &mut bool,
    session_seen: &mut SessionSeen,
) -> Duration {
    let (minutes, refresh_invalid) = refresh_interval(settings_path);
    let wait = Duration::from_secs(u64::from(minutes) * 60);
    let (filter, filter_invalid) = load_filter(settings_path);
    let state = app.state::<GitlabRefreshState>();
    let started = Instant::now();
    let before = state.snapshot();
    let before_status = health::gitlab_status(&before);

    // Show "refreshing" first; the stored items and state stay as they were.
    state.cell.begin_round(app);

    let (config, fetched) = run_fetch(settings_path, runner);

    // Comment checks run `glab` and read the store: no lock held.
    let store = app.state::<StoreHandle>();
    let checks = match (&config, &fetched) {
        (Some(cfg), Ok(full)) => {
            let checks = check_round(&store, runner, cfg, full, now_ms() as i64, *resume_after);
            *resume_after = checks.stopped_at();
            // An issue call that stopped the round (no VPN, logged out, no glab)
            // skips the MR calls too, but never two rounds in a row.
            let calls_allowed = mr_calls_allowed(checks.stopped_at().is_some(), *mr_skipped);
            *mr_skipped = !calls_allowed;
            let mr_checks = check_mr_round(
                &store,
                runner,
                cfg,
                full,
                now_ms() as i64,
                *mr_resume_after,
                calls_allowed,
            );
            *mr_resume_after = mr_checks.stopped_at();
            Some((checks, mr_checks))
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

    // The seen set is written before the gate, so the snapshot of this round
    // carries its NEW flags. One instant for the seen set and `shown_until`.
    let round_ms = now_ms() as i64;
    let seen = match (&config, &fetched) {
        (Some(cfg), Ok(full)) => {
            let account = gitlab::account_key(cfg);
            let outcome = record_seen(app, &account, full, session_seen, round_ms);
            Some((account, outcome))
        }
        _ => None,
    };

    // The commit, the snapshot and the kept list change together under the
    // gate, so a command can never see (or publish over) a half-updated state.
    let gate = state.hidden_gate.lock().unwrap_or_else(|p| p.into_inner());
    let round_fresh = if fetched.is_ok() {
        let (account, fresh) = match &seen {
            Some((account, outcome)) => (Some(account.as_str()), outcome.fresh.as_deref()),
            None => (None, None),
        };
        state.accept_round(account, round_ms, fresh)
    } else {
        HashSet::new()
    };
    let mut check_failed = 0;
    let mut hidden_error = None;
    let mut full_list = None;
    let mut filtered_out = FilteredOut::default();
    let outcome = match (&config, fetched, checks) {
        (Some(cfg), Ok(full), Some((checks, mr_checks))) => {
            let round = commit_round(&store, &cfg.host, &full, checks, now_ms() as i64);
            let mr = commit_mr_round(&store, &cfg.host, &full, mr_checks, now_ms() as i64);
            check_failed = round.check_failed + mr.check_failed;
            hidden_error = round
                .store_error
                .as_ref()
                .or(mr.store_error.as_ref())
                .map(HiddenIssueError::message_pl);
            // The filter applies after the partition; `full` stays unfiltered.
            let mut items = AssignedItems {
                merge_requests: mr.merge_requests,
                issues: round.visible_issues,
                hidden_issues: round.hidden_issues,
                review_requests: mr.review_requests,
                hidden_merge_requests: mr.hidden_merge_requests,
            };
            mark_fresh(&mut items, &round_fresh);
            let (shown, out) = filter_shown(items, &filter);
            filtered_out = out;
            full_list = Some(full);
            Ok(shown)
        }
        // A list without a config does not occur; show it as fetched.
        (_, Ok(full), _) => {
            full_list = Some(full.clone());
            let mut items = full;
            mark_fresh(&mut items, &round_fresh);
            let (shown, out) = filter_shown(items, &filter);
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
        next.filter = filter.clone();
        next.filtered_out = filtered_out;
        next.closed = closed;
    }
    next.refresh_setting_invalid = refresh_invalid;
    next.refresh_minutes = minutes;
    next.filter_invalid = filter_invalid;
    // A failed round that keeps the items re-shapes them below. The hidden
    // records are read here, under the gate but before any `kept_*` lock.
    let kept_records = match (&full_list, &next_kept) {
        (None, Some(cfg)) if next.items.is_some() => {
            let today = Local::now().naive_local().date();
            store.with_store(today, |s| list_gitlab(s, &cfg.host)).ok()
        }
        _ => None,
    };
    {
        let mut kept_full = state.kept_full.lock().unwrap_or_else(|p| p.into_inner());
        let mut kept_filter = state.kept_filter.lock().unwrap_or_else(|p| p.into_inner());
        let mut kept_new = state.kept_new.lock().unwrap_or_else(|p| p.into_inner());
        let mut shown_until = state.shown_until.lock().unwrap_or_else(|p| p.into_inner());
        match full_list {
            Some(full) => {
                *kept_full = Some(full);
                *kept_filter = filter.clone();
            }
            // A failed round keeps the list only while `next_snapshot` keeps it.
            None if next_kept.is_none() => {
                *kept_full = None;
                *kept_filter = GitlabFilter::default();
                kept_new.clear();
                *shown_until = None;
            }
            // A kept list is shaped again with this round's filter; its NEW
            // flags and `shown_until` stay as they were.
            None => {
                if reshape_kept(
                    &mut next,
                    kept_full.as_ref(),
                    kept_records.as_ref(),
                    &kept_new,
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
    let may_notify = notify_input(&next).is_some();
    // "To do" reconciles on the full list, never the filtered one.
    let todo_changed = reconcile_todo(app, &next, round_full.as_ref());
    // The wait starts when the snapshot is stored.
    next.next_refresh_at_ms = Some(source_refresh::next_refresh_at_ms(wait));
    *state.write() = next;
    drop(gate);
    publish(app, &state);
    SOURCE.notify_todo_changed(app, todo_changed);
    // After the gate and the publish: no lock is held while a toast shows.
    if let (true, Some(full), Some((_, outcome))) = (may_notify, &round_full, &seen) {
        toast_new_assignments(app, full, &outcome.toast_keys);
    }

    // A changed GitLab source status runs a health round now instead of waiting
    // up to 30 s for the tray to catch up.
    SOURCE.end_round(app, &from, &to, started, before_status != after_status);
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

/// What recording a round's seen set decided.
struct SeenOutcome {
    /// The keys to toast (from the store, or the session memory on an error).
    toast_keys: Vec<ItemKey>,
    /// Keys new to the view with their `first_seen_ms`; `None` when the seen-set
    /// write failed.
    fresh: Option<Vec<(ItemKey, i64)>>,
}

/// Records the round's full list in the seen set at `now_ms`. Runs before the
/// gate and holds no lock. A store error falls back to the session memory and
/// is logged by kind only; nothing logs a reference, title or URL.
fn record_seen(
    app: &AppHandle,
    account: &str,
    full: &AssignedItems,
    session_seen: &mut SessionSeen,
    now_ms: i64,
) -> SeenOutcome {
    let current = keys_of(full);
    let review_only = review_only_keys(full);
    // Day and event time both come from the round's instant, so they agree.
    let now_local = Local
        .timestamp_millis_opt(now_ms)
        .earliest()
        .map_or_else(|| Local::now().naive_local(), |d| d.naive_local());
    let today = now_local.date();
    let mut events_stored = false;
    let recorded = app.state::<StoreHandle>().with_store(today, |s| {
        let round = record_round(s, account, &current, &review_only, now_ms)?;
        // The event log must never change the round's outcome: its failure is
        // logged by kind only and the toasts and NEW flags stay as they are.
        if !round.new.is_empty() {
            let new_events: Vec<NewEvent> = items_for(full, &round.new)
                .into_iter()
                .map(|i| NewEvent::Assigned {
                    item_kind: i.kind,
                    reference: i.reference.clone(),
                    title: i.title.clone(),
                })
                .collect();
            match events::record(s, today, now_local, &new_events) {
                Ok(n) => events_stored = n > 0,
                Err(EventError::Store(StoreError::Busy)) => log::debug!("events: record busy"),
                Err(e) => log::warn!("events: record failed: {}", e.kind()),
            }
        }
        Ok::<_, NewAssignmentError>(round)
    });
    if events_stored {
        events::commands::notify(app);
    }
    let (stored, fresh) = match recorded {
        Ok(round) => (Ok(round.new), Some(round.fresh)),
        Err(e) => (Err(e), None),
    };
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
    let toast_keys = session_seen.resolve(account, &current, stored);
    SeenOutcome { toast_keys, fresh }
}

/// Shows a toast for every new item. Runs after the publish, outside every lock.
fn toast_new_assignments(app: &AppHandle, full: &AssignedItems, new_keys: &[ItemKey]) {
    if new_keys.is_empty() {
        return;
    }
    let items = items_for(full, new_keys);
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
    SOURCE.log_reconcile(result)
}

/// Pushes the stored snapshot to the webview unless a newer one was already
/// pushed.
pub(crate) fn publish(app: &AppHandle, state: &GitlabRefreshState) {
    state.cell.publish(app);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gitlab::GitlabState as State;
    use crate::hidden_issues::HiddenRecord;
    use crate::hidden_issues::items::{HiddenItemRecord, HiddenKind, HiddenSource};

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
            review_requests: Vec::new(),
            hidden_merge_requests: Vec::new(),
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
            review_requests: Vec::new(),
            hidden_merge_requests: Vec::new(),
        };
        let (out, counts) = filter_shown(shown.clone(), &project_filter("acme/a"));
        assert_eq!(out.merge_requests, vec![shown.merge_requests[0].clone()]);
        assert!(out.issues.is_empty() && out.hidden_issues.is_empty());
        assert_eq!(
            counts,
            FilteredOut {
                merge_requests: 1,
                issues: 2,
                hidden_issues: 1,
                review_requests: 0,
                hidden_merge_requests: 0,
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
            review_requests: Vec::new(),
            hidden_merge_requests: Vec::new(),
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
            review_requests: Vec::new(),
            hidden_merge_requests: Vec::new(),
        };
        let records = issues_only(vec![record(2, None)]);
        let (shown, out) = shape_items(&full, &records, &HashSet::new(), &project_filter("acme/a"));
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
                hidden_issues: 0,
                review_requests: 0,
                hidden_merge_requests: 0,
            }
        );
        // After an invalid block the default filter shows every item again.
        let (all, none) = shape_items(&full, &records, &HashSet::new(), &GitlabFilter::default());
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
            review_requests: Vec::new(),
            hidden_merge_requests: Vec::new(),
        };
        let (shown, out) = shape_items(
            &full,
            &GitlabHidden::default(),
            &HashSet::new(),
            &project_filter("acme/a"),
        );
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

    #[test]
    fn filter_shown_filters_and_counts_review_requests_like_merge_requests() {
        let shown = AssignedItems {
            review_requests: vec![
                entry_in(1, ItemKind::MergeRequest, "acme/a"),
                entry_in(2, ItemKind::MergeRequest, "acme/b"),
                entry_in(3, ItemKind::MergeRequest, "acme/b"),
            ],
            ..AssignedItems::default()
        };
        let (out, counts) = filter_shown(shown.clone(), &project_filter("acme/a"));
        assert_eq!(out.review_requests, vec![shown.review_requests[0].clone()]);
        assert_eq!(counts.review_requests, 2);
        assert_eq!(counts.merge_requests, 0);
    }

    #[test]
    fn filter_shown_filters_and_counts_hidden_merge_requests() {
        let shown = AssignedItems {
            hidden_merge_requests: vec![
                entry_in(1, ItemKind::MergeRequest, "acme/a"),
                entry_in(2, ItemKind::MergeRequest, "acme/b"),
            ],
            ..AssignedItems::default()
        };
        let (out, counts) = filter_shown(shown.clone(), &project_filter("acme/a"));
        assert_eq!(
            out.hidden_merge_requests,
            vec![shown.hidden_merge_requests[0].clone()]
        );
        assert_eq!(counts.hidden_merge_requests, 1);
        assert_eq!(counts.review_requests, 0);
    }

    #[test]
    fn shape_items_and_reshape_kept_keep_the_review_items() {
        let full = AssignedItems {
            review_requests: vec![entry_in(8, ItemKind::MergeRequest, "acme/a")],
            hidden_merge_requests: Vec::new(),
            ..AssignedItems::default()
        };
        let (shown, out) = shape_items(
            &full,
            &GitlabHidden::default(),
            &HashSet::new(),
            &GitlabFilter::default(),
        );
        assert_eq!(shown.review_requests, full.review_requests);
        assert_eq!(out, FilteredOut::default());
        let mut next = GitlabSnapshot {
            state: State::Unavailable {
                message: String::new(),
            },
            items: Some(shown),
            ..GitlabSnapshot::starting()
        };
        assert!(reshape_kept(
            &mut next,
            Some(&full),
            Some(&GitlabHidden::default()),
            &HashSet::new(),
            &project_filter("acme/zzz"),
            false
        ));
        assert!(next.items.as_ref().unwrap().review_requests.is_empty());
        assert_eq!(next.filtered_out.review_requests, 1);
    }

    fn closed_on(mut filter: GitlabFilter) -> GitlabFilter {
        filter.closed_days = Some(7);
        filter
    }

    #[test]
    fn reshape_kept_applies_this_rounds_filter_and_hidden_records() {
        let (mut next, full) = kept_round();
        let closed = next.closed.clone();
        let records = issues_only(vec![record(2, None)]);
        let filter = closed_on(project_filter("acme/b"));
        assert!(reshape_kept(
            &mut next,
            Some(&full),
            Some(&records),
            &HashSet::new(),
            &filter,
            false
        ));
        let items = next.items.as_ref().unwrap();
        assert_eq!(items.issues.iter().map(|i| i.id).collect::<Vec<_>>(), [4]);
        assert_eq!(items.merge_requests.len(), 1);
        assert!(items.hidden_issues.is_empty());
        assert!(next.filter_active);
        assert_eq!(next.filter, filter);
        assert_eq!(
            next.filtered_out,
            FilteredOut {
                merge_requests: 0,
                issues: 1,
                hidden_issues: 1,
                review_requests: 0,
                hidden_merge_requests: 0,
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
            Some(&GitlabHidden::default()),
            &HashSet::new(),
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
            Some(&GitlabHidden::default()),
            &HashSet::new(),
            &project_filter("acme/a"),
            false
        ));
        assert_eq!(next.closed, None);
    }

    #[test]
    fn reshape_kept_without_records_keeps_the_shown_list() {
        let (mut next, full) = kept_round();
        next.filter = closed_on(project_filter("acme/a"));
        let before = next.clone();
        // The store read failed: the list stays as it was shown and the kept
        // filter is not replaced, but an invalid block still drops `closed`
        // (and the bar's `closedDays` with it).
        assert!(!reshape_kept(
            &mut next,
            Some(&full),
            None,
            &HashSet::new(),
            &GitlabFilter::default(),
            true
        ));
        assert_eq!(next.items, before.items);
        assert_eq!(next.filtered_out, before.filtered_out);
        assert!(next.filter_active);
        assert_eq!(next.closed, None);
        assert_eq!(next.filter.closed_days, None);
        assert_eq!(next.filter.projects, project_filter("acme/a").projects);
    }

    #[test]
    fn reshape_kept_without_kept_items_changes_nothing() {
        let (mut next, full) = kept_round();
        next.items = None;
        let before = next.clone();
        assert!(!reshape_kept(
            &mut next,
            Some(&full),
            Some(&GitlabHidden::default()),
            &HashSet::new(),
            &GitlabFilter::default(),
            true
        ));
        assert_eq!(next, before);
        // No kept full list: nothing to re-shape.
        let (mut next, _) = kept_round();
        let filter = closed_on(GitlabFilter::default());
        assert!(!reshape_kept(
            &mut next,
            None,
            Some(&GitlabHidden::default()),
            &HashSet::new(),
            &filter,
            false
        ));
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
            is_new: false,
            hidden_at_ms: None,
            review_requested: false,
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

    /// The GitLab records of a test with issue records only.
    fn issues_only(issues: Vec<HiddenRecord>) -> GitlabHidden {
        GitlabHidden {
            issues,
            merge_requests: vec![],
        }
    }

    /// A merge request record of the test host, hidden at 1 like `record`.
    fn mr_record(id: u64, reason: Option<gitlab::ChangeReason>) -> HiddenItemRecord {
        HiddenItemRecord {
            source: HiddenSource::Gitlab,
            scope: host().as_str().to_string(),
            kind: HiddenKind::MergeRequest,
            item_id: id,
            project_id: Some(100),
            iid: Some(id),
            status_fingerprint: vec![],
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
            review_requests: Vec::new(),
            hidden_merge_requests: Vec::new(),
        });
        state
    }

    #[test]
    fn act_on_issue_without_a_kept_list_is_list_unavailable() {
        let (state, _rx) = GitlabRefreshState::new();
        let out = state.act_on_item(ItemKind::Issue, 1, |_, _| {
            panic!("no store op without a list")
        });
        assert!(matches!(out, Err(HiddenIssueError::ListUnavailable)));
        assert_eq!(state.snapshot().seq, 0);
    }

    #[test]
    fn act_on_item_matches_id_and_kind() {
        let state = kept_state();
        // An issue id asked as a merge request, a merge request id asked as an
        // issue and unknown ids are refused before the store op.
        for (kind, id) in [
            (ItemKind::MergeRequest, 1),
            (ItemKind::Issue, 3),
            (ItemKind::Issue, 99),
            (ItemKind::MergeRequest, 99),
        ] {
            let out = state.act_on_item(kind, id, |_, _| panic!("no store op for {id}"));
            assert!(matches!(out, Err(HiddenIssueError::NotInList)));
        }
        assert_eq!(state.snapshot().seq, 0);
    }

    #[test]
    fn act_on_item_partitions_both_kinds_from_one_result() {
        let state = kept_state();
        {
            let mut kept = state.kept_full.lock().unwrap();
            kept.as_mut()
                .unwrap()
                .review_requests
                .push(entry(4, ItemKind::MergeRequest));
        }
        state
            .act_on_item(ItemKind::MergeRequest, 4, |_, item| {
                assert_eq!(item.id, 4);
                Ok(GitlabHidden {
                    issues: vec![record(1, None)],
                    merge_requests: vec![
                        mr_record(4, None),
                        mr_record(3, Some(gitlab::ChangeReason::Comment)),
                    ],
                })
            })
            .unwrap();

        let items = state.snapshot().items.unwrap();
        // The issue record hides issue 1 too: one result, both kinds.
        assert_eq!(
            items.hidden_issues.iter().map(|i| i.id).collect::<Vec<_>>(),
            [1]
        );
        // A hidden review-only MR leaves "Do review" for the hidden list.
        assert!(items.review_requests.is_empty());
        assert_eq!(
            items
                .hidden_merge_requests
                .iter()
                .map(|i| (i.id, i.hidden_at_ms))
                .collect::<Vec<_>>(),
            [(4, Some(1))]
        );
        // A marked MR stays visible in its own section with its reason.
        assert_eq!(items.merge_requests.len(), 1);
        assert_eq!(items.merge_requests[0].id, 3);
        assert_eq!(
            items.merge_requests[0].changed,
            Some(gitlab::ChangeReason::Comment)
        );
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
            .act_on_item(ItemKind::Issue, 1, |h, item| {
                assert_eq!(h, &host());
                assert_eq!(item.id, 1);
                Ok(issues_only(vec![
                    record(1, None),
                    record(2, Some(gitlab::ChangeReason::Comment)),
                ]))
            })
            .unwrap();

        let snap = state.snapshot();
        assert_eq!(snap.seq, 8);
        assert_eq!(snap.hidden_error, None);
        // Never more failed checks than hidden issues.
        assert_eq!(snap.hidden_check_failed, 1);
        let items = snap.items.unwrap();
        assert_eq!(items.merge_requests, vec![entry(3, ItemKind::MergeRequest)]);
        let hidden_entry = GitlabItem {
            hidden_at_ms: Some(1),
            ..entry(1, ItemKind::Issue)
        };
        assert_eq!(items.hidden_issues, vec![hidden_entry]);
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
            review_requests: Vec::new(),
            hidden_merge_requests: Vec::new(),
        });
        *state.kept_filter.lock().unwrap() = project_filter("acme/a");
        {
            let mut snap = state.write();
            snap.filter_invalid = true;
            snap.hidden_check_failed = 3;
        }

        // Hide issues 1 (kept by the filter) and 3 (removed by it).
        state
            .act_on_item(ItemKind::Issue, 1, |_, _| {
                Ok(issues_only(vec![record(1, None), record(3, None)]))
            })
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
        assert_eq!(snap.filter, project_filter("acme/a"));
        assert_eq!(
            snap.filtered_out,
            FilteredOut {
                merge_requests: 1,
                issues: 1,
                hidden_issues: 1,
                review_requests: 0,
                hidden_merge_requests: 0,
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
            .act_on_item(ItemKind::Issue, 1, |_, _| {
                Ok(issues_only(vec![record(1, None)]))
            })
            .unwrap();
        let snap = state.snapshot();
        assert!(!snap.filter_active);
        assert_eq!(snap.filter, GitlabFilter::default());
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
            .act_on_item(ItemKind::Issue, 1, |_, _| {
                Ok(issues_only(vec![record(1, None)]))
            })
            .unwrap();
        assert_eq!(state.snapshot().closed, Some(closed));
    }

    #[test]
    fn act_on_issue_keeps_the_refresh_setting_flag() {
        let state = kept_state();
        {
            let mut snap = state.write();
            snap.refresh_setting_invalid = true;
            snap.refresh_minutes = 4;
            snap.next_refresh_at_ms = Some(99);
        }
        state
            .act_on_item(ItemKind::Issue, 1, |_, _| {
                Ok(issues_only(vec![record(1, None)]))
            })
            .unwrap();
        let snap = state.snapshot();
        assert!(snap.refresh_setting_invalid);
        assert_eq!(snap.refresh_minutes, 4);
        assert_eq!(snap.next_refresh_at_ms, Some(99));
    }

    #[test]
    fn refresh_interval_reads_the_setting_and_falls_back_to_ten_minutes() {
        assert_eq!(refresh_interval(&None), (10, false));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, r#"{"version":1,"gitlabRefresh":{"minutes":3}}"#).unwrap();
        let p = Some(path.clone());
        assert_eq!(refresh_interval(&p), (3, false));
        std::fs::write(&path, r#"{"version":1,"gitlabRefresh":{"minutes":15}}"#).unwrap();
        assert_eq!(refresh_interval(&p), (10, true));
    }

    #[test]
    fn act_on_issue_store_error_leaves_the_snapshot_alone() {
        let state = kept_state();
        let out = state.act_on_item(ItemKind::Issue, 1, |_, _| Err(HiddenIssueError::NotFound));
        assert!(matches!(out, Err(HiddenIssueError::NotFound)));
        assert_eq!(state.snapshot(), GitlabSnapshot::starting());
    }

    fn key(kind: ItemKind, id: u64) -> ItemKey {
        ItemKey::new(kind, id)
    }

    fn fresh_set(keys: &[ItemKey]) -> HashSet<ItemKey> {
        keys.iter().copied().collect()
    }

    /// `kept_state` after a successful round at `now_ms`: issue 1 and MR 3 are
    /// NEW, the snapshot shows them flagged.
    fn shown_state(account: &str, now_ms: i64) -> GitlabRefreshState {
        let state = kept_state();
        let fresh = [
            (key(ItemKind::Issue, 1), now_ms),
            (key(ItemKind::MergeRequest, 3), now_ms),
        ];
        let set = state.accept_round(Some(account), now_ms, Some(&fresh));
        let full = state.kept_full.lock().unwrap().clone().unwrap();
        let (shown, _) = shape_items(
            &full,
            &GitlabHidden::default(),
            &set,
            &GitlabFilter::default(),
        );
        let mut snap = state.write();
        snap.state = State::Ok;
        snap.items = Some(shown);
        snap.seq = 5;
        drop(snap);
        state
    }

    fn new_flags(items: &AssignedItems) -> Vec<(u64, bool)> {
        items
            .merge_requests
            .iter()
            .chain(&items.issues)
            .chain(&items.hidden_issues)
            .map(|i| (i.id, i.is_new))
            .collect()
    }

    #[test]
    fn shape_items_flags_only_mrs_and_visible_issues_from_the_set() {
        let full = AssignedItems {
            merge_requests: vec![
                entry(3, ItemKind::MergeRequest),
                entry(4, ItemKind::MergeRequest),
            ],
            issues: vec![entry(1, ItemKind::Issue), entry(2, ItemKind::Issue)],
            hidden_issues: vec![],
            review_requests: Vec::new(),
            hidden_merge_requests: Vec::new(),
        };
        // Issue 2 is hidden: even when in the set it is never NEW. The MR 1
        // key (same id as issue 1, other kind) flags nothing.
        let fresh = fresh_set(&[
            key(ItemKind::MergeRequest, 3),
            key(ItemKind::Issue, 1),
            key(ItemKind::Issue, 2),
            key(ItemKind::MergeRequest, 1),
        ]);
        let (shown, _) = shape_items(
            &full,
            &issues_only(vec![record(2, None)]),
            &fresh,
            &GitlabFilter::default(),
        );
        assert_eq!(
            new_flags(&shown),
            [(3, true), (4, false), (1, true), (2, false)]
        );
        assert_eq!(shown.hidden_issues[0].hidden_at_ms, Some(1));
    }

    #[test]
    fn act_on_issue_keeps_is_new_and_hidden_at_after_hiding_another_issue() {
        let state = shown_state("account-a", 100);
        state
            .act_on_item(ItemKind::Issue, 2, |_, _| {
                Ok(issues_only(vec![record(2, None)]))
            })
            .unwrap();
        let items = state.snapshot().items.unwrap();
        assert_eq!(new_flags(&items), [(3, true), (1, true), (2, false)]);
        assert_eq!(items.hidden_issues[0].hidden_at_ms, Some(1));
    }

    #[test]
    fn reshape_kept_keeps_the_flags_after_a_failed_round() {
        let (mut next, full) = kept_round();
        let fresh = fresh_set(&[key(ItemKind::Issue, 1), key(ItemKind::MergeRequest, 3)]);
        assert!(reshape_kept(
            &mut next,
            Some(&full),
            Some(&GitlabHidden::default()),
            &fresh,
            &GitlabFilter::default(),
            false
        ));
        let items = next.items.unwrap();
        assert_eq!(
            new_flags(&items),
            [(3, true), (1, true), (2, false), (4, false)]
        );
    }

    #[test]
    fn mark_seen_without_shown_until_saves_nothing() {
        let state = kept_state();
        state.write().seq = 4;
        let out = state.mark_seen(|_, _| panic!("nothing was shown"));
        assert!(matches!(out, Ok(false)));
        assert_eq!(state.snapshot().seq, 4);
        assert!(state.last_mark.lock().unwrap().is_none());
    }

    #[test]
    fn mark_seen_saves_shown_until_clears_flags_and_bumps_seq() {
        let state = shown_state("account-a", 100);
        let mut saved = None;
        let out = state.mark_seen(|account, until| {
            saved = Some((account.to_string(), until));
            Ok(())
        });
        assert!(matches!(out, Ok(true)));
        // The round's instant, whatever the wall clock says.
        assert_eq!(saved, Some(("account-a".to_string(), 100)));
        let snap = state.snapshot();
        assert_eq!(snap.seq, 6);
        assert!(
            new_flags(snap.items.as_ref().unwrap())
                .iter()
                .all(|(_, n)| !n)
        );
        assert!(state.kept_new.lock().unwrap().is_empty());
        assert_eq!(
            *state.last_mark.lock().unwrap(),
            Some(("account-a".to_string(), 100))
        );
    }

    #[test]
    fn an_item_from_a_round_stored_after_shown_until_stays_new() {
        let state = shown_state("account-a", 100);
        state.mark_seen(|_, _| Ok(())).unwrap();
        // The next round first saw issue 2 at 150, after the saved mark.
        let set = state.accept_round(
            Some("account-a"),
            150,
            Some(&[(key(ItemKind::Issue, 2), 150)]),
        );
        assert_eq!(set, fresh_set(&[key(ItemKind::Issue, 2)]));
        assert_eq!(
            *state.shown_until.lock().unwrap(),
            Some(("account-a".to_string(), 150))
        );
    }

    #[test]
    fn a_save_error_leaves_the_snapshot_and_state_unchanged() {
        let state = shown_state("account-a", 100);
        let before = state.snapshot();
        let out = state.mark_seen(|_, _| Err(NewAssignmentError::Store(StoreError::Busy)));
        assert!(matches!(
            out,
            Err(NewAssignmentError::Store(StoreError::Busy))
        ));
        assert_eq!(state.snapshot(), before);
        assert_eq!(state.kept_new.lock().unwrap().len(), 2);
        assert!(state.last_mark.lock().unwrap().is_none());
    }

    #[test]
    fn fresh_keys_covered_by_the_last_mark_are_dropped() {
        let state = shown_state("account-a", 100);
        state.mark_seen(|_, _| Ok(())).unwrap();
        // A round that read the mark before the user left the view.
        let fresh = [
            (key(ItemKind::Issue, 1), 90),
            (key(ItemKind::Issue, 2), 100),
            (key(ItemKind::MergeRequest, 3), 101),
        ];
        let set = state.accept_round(Some("account-a"), 120, Some(&fresh));
        assert_eq!(set, fresh_set(&[key(ItemKind::MergeRequest, 3)]));
        // Another account's mark covers nothing.
        let set = state.accept_round(Some("account-b"), 130, Some(&fresh));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn a_failed_seen_set_write_keeps_the_flags_and_shown_until() {
        let state = shown_state("account-a", 100);
        let set = state.accept_round(Some("account-a"), 200, None);
        assert!(set.contains(&key(ItemKind::Issue, 1)));
        assert_eq!(set.len(), 2);
        assert_eq!(
            *state.shown_until.lock().unwrap(),
            Some(("account-a".to_string(), 100))
        );
    }

    #[test]
    fn a_failed_first_round_saves_no_mark() {
        let state = kept_state();
        let set = state.accept_round(Some("account-a"), 100, None);
        assert!(set.is_empty());
        let out = state.mark_seen(|_, _| panic!("nothing was shown"));
        assert!(matches!(out, Ok(false)));
    }

    #[test]
    fn accept_round_without_an_account_flags_nothing() {
        let state = shown_state("account-a", 100);
        let set = state.accept_round(None, 120, Some(&[(key(ItemKind::Issue, 1), 120)]));
        assert!(set.is_empty());
        assert!(state.kept_new.lock().unwrap().is_empty());
        assert!(state.shown_until.lock().unwrap().is_none());
    }
}
