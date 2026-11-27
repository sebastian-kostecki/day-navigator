//! Runs the autostart sync once per start on its own thread and keeps the
//! result for the health round. No test module and no `unwrap`/`expect`: a
//! release build aborts on panic.

use std::path::PathBuf;
use std::sync::Mutex;

use tauri::{AppHandle, Manager};

use super::{FailedStep, Outcome, run, status as outcome_status};
use crate::health::SourceStatus;
use crate::settings::load_autostart_settings;

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

/// The status of the last sync; `None` until it has finished.
pub struct AutostartState(Mutex<Option<SourceStatus>>);

fn store(app: &AppHandle, outcome: &Outcome) {
    if let Some(state) = app.try_state::<AutostartState>() {
        *state.0.lock().unwrap_or_else(|p| p.into_inner()) = Some(outcome_status(outcome));
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

/// Manages the state, then syncs the Run entry on an `autostart-sync` thread.
/// Without a settings path nothing more happens: "Ustawienia" reports that.
pub fn start(app: &AppHandle, settings_path: Option<PathBuf>) {
    app.manage(AutostartState(Mutex::new(None)));
    let Some(path) = settings_path else {
        return;
    };
    let handle = app.clone();
    let spawned = std::thread::Builder::new()
        .name("autostart-sync".into())
        .spawn(move || {
            let exe = std::env::current_exe().ok();
            let outcome = run(
                load_autostart_settings(&path),
                !cfg!(debug_assertions),
                &HkcuRun,
                exe.as_deref(),
            );
            log_outcome(&outcome);
            store(&handle, &outcome);
            crate::runtime::refresh_tray(&handle);
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
    state.0.lock().unwrap_or_else(|p| p.into_inner()).clone()
}
