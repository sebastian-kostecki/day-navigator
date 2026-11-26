//! Side effects of source health: the check and watchdog threads, the tray,
//! toasts, log lines and the `health-changed` event. The rules live in
//! `health/` and stay free of `tauri`.
//!
//! Locking rule: `TrayIcon::set_icon`/`set_tooltip` block until the main thread
//! runs them, and sync commands run on the main thread. So the `snapshot` lock
//! is never held across a tray, notification or emit call.
//! The one lock held across tray and emit calls is `published`, which only the
//! check and watchdog threads take, never the main thread.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use chrono::{Local, NaiveDate};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::events::{self, EventError};
use crate::github_refresh::GithubRefreshState;
use crate::gitlab_refresh::GitlabRefreshState;
use crate::health::{self, AppDataDetails, HealthSnapshot, Overall, SourceStatus};
use crate::store::{StoreError, StoreHandle};
use crate::tray_view::{self, TrayIconKind, WorkLine};

const TRAY_ID: &str = "health";
const CHECK_INTERVAL: Duration = Duration::from_secs(30);
const WATCHDOG_INTERVAL: Duration = Duration::from_secs(15);
const STALE_AFTER: Duration = Duration::from_secs(75);
/// A watchdog gap above two intervals means the machine slept.
const RESUME_GAP: Duration = Duration::from_secs(30);
const EVENT: &str = "health-changed";
const STALL_TITLE: &str = "Nawigator dnia — sprawdzanie stanu nie działa";

/// Managed state shared by the threads, the tray and (in Phase 3) the commands.
pub struct HealthState {
    snapshot: RwLock<HealthSnapshot>,
    /// Wakes the check thread for an immediate round.
    pub wake: mpsc::Sender<()>,
    origin: Instant,
    /// Milliseconds since `origin` when the last check's probe returned.
    heartbeat_ms: AtomicU64,
    /// Set and cleared only by the watchdog.
    stale: AtomicBool,
    /// `seq` of the last snapshot pushed to the tray and the webview.
    published: Mutex<u64>,
}

impl HealthState {
    fn new() -> (Self, mpsc::Receiver<()>) {
        let (wake, rx) = mpsc::channel();
        let state = HealthState {
            snapshot: RwLock::new(HealthSnapshot {
                overall: Overall::Starting,
                sources: Vec::new(),
                checked_at_ms: None,
                seq: 0,
                app_data: None,
            }),
            wake,
            origin: Instant::now(),
            heartbeat_ms: AtomicU64::new(0),
            stale: AtomicBool::new(false),
            published: Mutex::new(0),
        };
        (state, rx)
    }

    /// The stored snapshot (`Starting` before the first check).
    pub fn snapshot(&self) -> HealthSnapshot {
        self.read().clone()
    }

    fn read(&self) -> RwLockReadGuard<'_, HealthSnapshot> {
        self.snapshot.read().unwrap_or_else(|p| p.into_inner())
    }

    fn write(&self) -> RwLockWriteGuard<'_, HealthSnapshot> {
        self.snapshot.write().unwrap_or_else(|p| p.into_inner())
    }

    fn stamp_heartbeat(&self) {
        self.heartbeat_ms
            .store(self.origin.elapsed().as_millis() as u64, Ordering::Release);
    }

    fn heartbeat_age(&self) -> Duration {
        let last = Duration::from_millis(self.heartbeat_ms.load(Ordering::Acquire));
        self.origin.elapsed().saturating_sub(last)
    }
}

/// The stored snapshot (`Starting` before the first check). Async so it never
/// runs on the main thread; it returns `Result` because async commands that
/// borrow `State` must.
#[tauri::command]
pub async fn health_status(state: tauri::State<'_, HealthState>) -> Result<HealthSnapshot, String> {
    Ok(state.snapshot())
}

/// Wakes the check thread for an immediate round, and the GitLab and GitHub
/// threads so the health round sees a fresh status (each wakes health again on a
/// change).
#[tauri::command]
pub async fn check_health_now(
    state: tauri::State<'_, HealthState>,
    gitlab: tauri::State<'_, GitlabRefreshState>,
    github: tauri::State<'_, GithubRefreshState>,
) -> Result<(), String> {
    // A dead refresh thread is already shown as its own source error.
    let _ = gitlab.wake();
    let _ = github.wake();
    state
        .wake
        .send(())
        .map_err(|_| "Wątek sprawdzania nie działa".to_string())
}

/// Shows, unminimises and focuses the main window.
pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn tray_image(icon: TrayIconKind) -> Image<'static> {
    match icon {
        TrayIconKind::Idle => tauri::include_image!("icons/tray/idle.png"),
        TrayIconKind::Ok => tauri::include_image!("icons/tray/ok.png"),
        TrayIconKind::Running => tauri::include_image!("icons/tray/running.png"),
        TrayIconKind::Error => tauri::include_image!("icons/tray/error.png"),
    }
}

/// Registers `HealthState` and builds the tray in the grey `Starting` state.
/// Returns the wake receiver for `start`.
pub fn build_tray(app: &tauri::App) -> tauri::Result<mpsc::Receiver<()>> {
    let (state, rx) = HealthState::new();
    app.manage(state);

    let open = MenuItem::with_id(app, "open", "Otwórz", true, None::<&str>)?;
    let check = MenuItem::with_id(app, "check", "Sprawdź teraz", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Zakończ", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &check, &quit])?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(tray_image(TrayIconKind::Idle))
        .tooltip(health::tooltip(Overall::Starting, &[]))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_main(app),
            "check" => {
                let _ = app.state::<GitlabRefreshState>().wake();
                let _ = app.state::<GithubRefreshState>().wake();
                let _ = app.state::<HealthState>().wake.send(());
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(rx)
}

/// Starts the check and watchdog threads. `settings_path` is `None` when the
/// app config dir cannot be resolved; every check then reports that failure.
pub fn start(app: &AppHandle, settings_path: Option<PathBuf>, wake_rx: mpsc::Receiver<()>) {
    let a = app.clone();
    if let Err(e) = std::thread::Builder::new()
        .name("health-check".into())
        .spawn(move || check_loop(a, settings_path, wake_rx))
    {
        log::error!("health: cannot start check thread: {e}");
    }
    let a = app.clone();
    if let Err(e) = std::thread::Builder::new()
        .name("health-watchdog".into())
        .spawn(move || watchdog_loop(a))
    {
        log::error!("health: cannot start watchdog thread: {e}");
    }
}

fn update_tray(app: &AppHandle, snap: &HealthSnapshot) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    if let Err(e) = tray.set_icon(Some(tray_image(tray_view::tray_icon(
        snap.overall,
        &WorkLine::Loading,
    )))) {
        log::warn!("health: tray icon update failed: {e}");
    }
    if let Err(e) = tray.set_tooltip(Some(health::tooltip(snap.overall, &snap.sources))) {
        log::warn!("health: tray tooltip update failed: {e}");
    }
}

fn emit_snapshot(app: &AppHandle, snap: &HealthSnapshot) {
    if let Err(e) = app.emit(EVENT, snap) {
        log::warn!("health: emit failed: {e}");
    }
}

/// Pushes the stored snapshot to the tray and the webview unless a newer one
/// was already pushed. The check and watchdog threads publish independently;
/// re-reading the stored snapshot under `published` keeps the tray and the
/// panel on the latest state whatever order the two threads run in.
fn publish(app: &AppHandle, state: &HealthState) {
    let mut last = state.published.lock().unwrap_or_else(|p| p.into_inner());
    let snap = state.snapshot();
    if snap.seq <= *last {
        return;
    }
    *last = snap.seq;
    update_tray(app, &snap);
    emit_snapshot(app, &snap);
}

/// Sends a toast unless notices are gated off. `area` prefixes the log line.
pub(crate) fn toast(app: &AppHandle, area: &str, title: &str, body: &str) {
    if !health::notices_allowed() {
        return;
    }
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        log::warn!("{area}: toast failed: {e}");
    }
}

/// Settings first, then the app data store, which is reported even when
/// `settings.json` is broken, then GitLab, GitHub and reminders.
fn probe_round(
    settings_path: &Option<PathBuf>,
    store: &StoreHandle,
    today: NaiveDate,
    gitlab: Option<SourceStatus>,
    github: Option<SourceStatus>,
) -> (Vec<SourceStatus>, Option<AppDataDetails>) {
    let mut sources = match settings_path {
        Some(p) => health::probe(p),
        None => health::unresolved_config_dir(),
    };
    let (store_source, app_data) = health::store_probe(store, today);
    sources.push(store_source);
    // Absent until the first GitLab round has finished.
    sources.extend(gitlab);
    // Likewise for GitHub.
    sources.extend(github);
    // Omitted without a settings path: the Settings source reports that.
    if let Some(p) = settings_path {
        sources.push(health::reminders_status(p));
    }
    (sources, app_data)
}

fn check_loop(app: AppHandle, settings_path: Option<PathBuf>, wake_rx: mpsc::Receiver<()>) {
    let mut baseline: Vec<SourceStatus> = Vec::new();
    loop {
        check_round(&app, &settings_path, &mut baseline);

        #[cfg(debug_assertions)]
        if std::env::var("DAY_NAVIGATOR_STALL_CHECKS").as_deref() == Ok("1") {
            log::warn!("health: stall switch on, check thread blocked");
            loop {
                std::thread::park();
            }
        }

        match wake_rx.recv_timeout(CHECK_INTERVAL) {
            Ok(()) => while wake_rx.try_recv().is_ok() {},
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

fn check_round(app: &AppHandle, settings_path: &Option<PathBuf>, baseline: &mut Vec<SourceStatus>) {
    let state = app.state::<HealthState>();
    let (sources, app_data) = probe_round(
        settings_path,
        &app.state::<StoreHandle>(),
        Local::now().date_naive(),
        health::gitlab_status(&app.state::<GitlabRefreshState>().snapshot()),
        health::github_status(&app.state::<GithubRefreshState>().snapshot()),
    );
    // Stamp as soon as the probe returns. The probe is the part that can hang;
    // stamping after the blocking tray calls below let the watchdog flag a
    // finished round as stalled.
    state.stamp_heartbeat();

    let transitions = health::transitions(baseline, &sources);
    *baseline = health::baseline(baseline, &sources);

    {
        let mut guard = state.write();
        // The watchdog owns the stall flag and sets it before taking this lock,
        // so reading it here keeps the stored overall consistent with it.
        let stale = state.stale.load(Ordering::Acquire);
        let seq = guard.seq + 1;
        *guard = HealthSnapshot {
            overall: health::overall(&sources, stale, true),
            sources,
            checked_at_ms: Some(now_ms()),
            seq,
            app_data,
        };
    }

    // Tray and panel first: the reliable channel never lags the toast.
    publish(app, &state);
    if let Some(notice) = health::notice_for(&transitions) {
        toast(app, "health", &notice.title, &notice.body);
    }
    if !transitions.is_empty() {
        let now = Local::now().naive_local();
        let today = now.date();
        let recorded = app.state::<StoreHandle>().with_store(today, |s| {
            events::record(s, today, now, &events::from_transitions(&transitions))
        });
        match recorded {
            Ok(n) if n > 0 => events::commands::notify(app),
            Ok(_) => {}
            Err(EventError::Store(StoreError::Busy)) => log::debug!("events: record busy"),
            Err(e) => log::warn!("events: record failed: {}", e.kind()),
        }
    }
    for t in &transitions {
        log::info!(
            "health: {} {} -> {}",
            t.id.key(),
            // No from-kind: Unknown, Ok or NotConfigured. No to-kind: recovery to Ok.
            t.from_kind.as_deref().unwrap_or("-"),
            t.to_kind.as_deref().unwrap_or("ok"),
        );
    }
}

fn watchdog_loop(app: AppHandle) {
    let state = app.state::<HealthState>();
    state.stamp_heartbeat();
    let mut last_wake = Instant::now();
    let mut resumed_at: Option<Instant> = None;
    loop {
        std::thread::sleep(WATCHDOG_INTERVAL);
        let now = Instant::now();
        let gap = now.duration_since(last_wake);
        last_wake = now;

        if health::is_resume(gap, RESUME_GAP) {
            // The machine slept: give the check loop a fresh grace period and
            // run a round now instead of waiting out its 30 s timer.
            resumed_at = Some(now);
            let _ = state.wake.send(());
            let _ = app.state::<GitlabRefreshState>().wake();
            let _ = app.state::<GithubRefreshState>().wake();
        }

        let action = health::watchdog_step(
            state.stale.load(Ordering::Acquire),
            state.heartbeat_age(),
            gap,
            resumed_at.map(|t| now.duration_since(t)),
            STALE_AFTER,
            RESUME_GAP,
        );
        let stale = match action {
            health::WatchdogAction::Stall => true,
            health::WatchdogAction::Clear => false,
            health::WatchdogAction::Nothing => continue,
        };
        state.stale.store(stale, Ordering::Release);
        {
            let mut guard = state.write();
            guard.overall = health::overall(&guard.sources, stale, guard.checked_at_ms.is_some());
            guard.seq += 1;
        }
        publish(&app, &state);
        if stale {
            log::warn!("health: check loop stalled");
            toast(&app, "health", STALL_TITLE, "");
        } else {
            log::info!("health: check loop resumed");
        }
    }
}
