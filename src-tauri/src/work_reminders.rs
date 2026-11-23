//! The `work-reminders` thread: every minute it reads the `reminders` settings
//! block and the store, asks the pure rules in `work_time::reminders` what is
//! due and sends the toasts. An invalid block skips the round (the health
//! source "Przypomnienia" reports it); so does a store error ("Dane aplikacji"
//! reports that). A failed spawn or a panic is only logged.

use std::path::PathBuf;
use std::time::Duration;

use chrono::{Local, Utc};
use tauri::{AppHandle, Manager};

use crate::runtime;
use crate::settings::{ReminderSettings, load_reminder_settings};
use crate::store::StoreHandle;
use crate::work_time::reminders::{ReminderMemory, due_reminders, reminder_facts};

const REMINDER_INTERVAL: Duration = Duration::from_secs(60);

/// Starts the `work-reminders` thread. `settings_path` is `None` when the app
/// config dir cannot be resolved; the rules then run with the defaults.
pub fn start(app: &AppHandle, settings_path: Option<PathBuf>) {
    let a = app.clone();
    if let Err(e) = std::thread::Builder::new()
        .name("work-reminders".into())
        .spawn(move || reminder_loop(a, settings_path))
    {
        log::error!("reminders: cannot start thread: {e}");
    }
}

fn reminder_loop(app: AppHandle, settings_path: Option<PathBuf>) {
    let mut memory = ReminderMemory::default();
    loop {
        reminder_round(&app, &settings_path, &mut memory);
        std::thread::sleep(REMINDER_INTERVAL);
    }
}

fn reminder_round(app: &AppHandle, settings_path: &Option<PathBuf>, memory: &mut ReminderMemory) {
    let settings = match settings_path {
        Some(path) => match load_reminder_settings(path) {
            Ok(s) => s,
            // Health surfaces an invalid block.
            Err(_) => return,
        },
        None => ReminderSettings::default(),
    };
    let now = Utc::now().timestamp_millis();
    let facts = app
        .state::<StoreHandle>()
        .with_store(Local::now().date_naive(), |s| {
            reminder_facts(s, now, &Local)
        });
    // The store lock is released here, before any toast.
    let facts = match facts {
        Ok(f) => f,
        // An unopenable store is reported by the health round.
        Err(e) => {
            log::debug!("reminders: facts unavailable: {e}");
            return;
        }
    };
    let (due, next) = due_reminders(&facts, &settings, now, &Local, memory);
    *memory = next;
    for r in due {
        runtime::toast(app, "reminders", &r.title, &r.body);
    }
}
