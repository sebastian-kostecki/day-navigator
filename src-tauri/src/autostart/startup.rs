//! Runs the autostart sync, once per start on its own thread and again after a
//! settings save, and keeps the result for the health round. No test module and no `unwrap`/`expect`: a
//! release build aborts on panic.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tauri::{AppHandle, Manager};

use super::{FailedStep, Outcome, status as outcome_status, sync_from_file};
use crate::health::SourceStatus;

#[cfg(windows)]
use super::hkcu::HkcuRun;

/// Stand-in on targets the app does not ship for: never applied.
#[cfg(not(windows))]
struct HkcuRun;

#[cfg(not(windows))]
impl super::RunEntry for HkcuRun {
    fn read(&self) -> std::io::Result<Option<String>> {
        Ok(None)
    }
    fn write(&self, _value: &str) -> std::io::Result<()> {
        Ok(())
    }
    fn remove(&self) -> std::io::Result<()> {
        Ok(())
    }
}

pub struct AutostartState {
    /// The status of the last sync; `None` until it has finished.
    status: Mutex<Option<SourceStatus>>,
    /// Held across a whole sync, so two syncs never race on the Run entry.
    sync: Mutex<()>,
}

fn store(app: &AppHandle, outcome: &Outcome) {
    if let Some(state) = app.try_state::<AutostartState>() {
        *state.status.lock().unwrap_or_else(|p| p.into_inner()) = Some(outcome_status(outcome));
    }
}

fn log_outcome(outcome: &Outcome) {
    match outcome {
        Outcome::On => log::info!("autostart: entry in place"),
        Outcome::Off => log::info!("autostart: off, entry absent"),
        Outcome::DevOnly => log::info!("autostart: debug build, registry untouched"),
        Outcome::Invalid => log::warn!("autostart: settings block invalid, entry unchanged"),
        Outcome::Failed { step, kind } => {
            log::warn!("autostart: failed at {step:?} ({kind:?})");
        }
    }
}

/// Syncs the Run entry with the settings file at `path`, keeps the result for
/// the health round and wakes it. A sync already running is waited for, and
/// this one then reads the file again. Blocks: never call it on the main thread.
pub fn sync_now(app: &AppHandle, path: &Path) -> SourceStatus {
    let state = app.try_state::<AutostartState>();
    // The lock guards no data, so a poisoned one is still good.
    let guard = state
        .as_ref()
        .map(|s| s.sync.lock().unwrap_or_else(|p| p.into_inner()));
    let exe = std::env::current_exe().ok();
    let outcome = sync_from_file(path, !cfg!(debug_assertions), &HkcuRun, exe.as_deref());
    log_outcome(&outcome);
    store(app, &outcome);
    drop(guard);
    crate::runtime::refresh_tray(app);
    outcome_status(&outcome)
}

/// Manages the state, then syncs the Run entry on an `autostart-sync` thread.
/// Without a settings path nothing more happens: "Ustawienia" reports that.
pub fn start(app: &AppHandle, settings_path: Option<PathBuf>) {
    app.manage(AutostartState {
        status: Mutex::new(None),
        sync: Mutex::new(()),
    });
    let Some(path) = settings_path else {
        return;
    };
    let handle = app.clone();
    let spawned = std::thread::Builder::new()
        .name("autostart-sync".into())
        .spawn(move || {
            sync_now(&handle, &path);
        });
    if let Err(e) = spawned {
        let outcome = Outcome::Failed {
            step: FailedStep::Thread,
            kind: e.kind(),
        };
        log_outcome(&outcome);
        store(app, &outcome);
    }
}

/// The "Autostart" source; absent until the sync has finished.
pub fn status(app: &AppHandle) -> Option<SourceStatus> {
    let state = app.try_state::<AutostartState>()?;
    state
        .status
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
}
