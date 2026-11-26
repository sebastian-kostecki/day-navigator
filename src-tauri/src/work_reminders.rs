//! The `work-reminders` thread: every minute it reads the `reminders` settings
//! block and the store, asks the pure rules in `work_time::reminders` what is
//! due and sends the toasts. An invalid block skips the round (the health
//! source "Przypomnienia" reports it); so does a store error ("Dane aplikacji"
//! reports that). The `workTime` block (holiday calendar) is read each round
//! too; an invalid one falls back to the default calendar instead of skipping
//! the round. A failed spawn is only logged. A panic is not caught: the
//! release profile sets `panic = "abort"`, so it would end the app; the round
//! has no known panic path (capped arithmetic, `earliest()` / `_opt` chrono).

use std::path::PathBuf;
use std::time::Duration;

use chrono::{Local, Utc};
use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::runtime;
use crate::settings::{
    ReminderOrigin, ReminderSettings, ReminderSettingsError, WorkTimeSettings,
    load_reminder_settings, load_reminder_settings_detailed,
};
use crate::settings_file;
use crate::store::StoreHandle;
use crate::work_time::WorkTimeError;
use crate::work_time::commands::{Policy, work_time_settings};
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
    // Read before the store lock. An invalid `workTime` block or an unresolved
    // path gives the default calendar (logged at debug by the helper).
    let work_time = work_time_settings(settings_path.as_deref(), Policy::Lenient)
        .unwrap_or_else(|_| WorkTimeSettings::default());
    let now = Utc::now().timestamp_millis();
    let facts = app
        .state::<StoreHandle>()
        .with_store(Local::now().date_naive(), |s| {
            reminder_facts(s, now, &Local, &work_time)
        });
    // The store lock is released here, before any toast.
    let facts = match facts {
        Ok(f) => f,
        // An unopenable store is reported by the health round; a warning every
        // minute would only repeat it.
        Err(e @ WorkTimeError::Store(_)) => {
            log::debug!("reminders: facts unavailable: {}", e.kind());
            return;
        }
        Err(e) => {
            log::warn!("reminders: facts unavailable: {}", e.kind());
            return;
        }
    };
    let (due, next) = due_reminders(&facts, &settings, now, &Local, memory);
    *memory = next;
    for r in due {
        runtime::toast(app, "reminders", &r.title, &r.body);
    }
}

/// The reminder thresholds as the panel shows them.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderThresholds {
    pub long_running: LongRunningThreshold,
    pub not_running_after: NotRunningAfterThreshold,
}

/// `hours` is `None` when the reminder is off.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LongRunningThreshold {
    pub hours: Option<u32>,
    pub from_file: bool,
}

/// `time` is a zero-padded `"HH:MM"`, or `None` when the reminder is off.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotRunningAfterThreshold {
    pub time: Option<String>,
    pub from_file: bool,
}

/// Why the reminder settings could not be read. Carries no path or value.
#[derive(Debug, Clone, PartialEq, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum ReminderSettingsQueryError {
    #[error("the reminders settings block is invalid")]
    SettingsInvalid,
    /// The blocking task could not finish (no content).
    #[error("background task failed")]
    Background,
}

impl From<ReminderSettingsError> for ReminderSettingsQueryError {
    fn from(e: ReminderSettingsError) -> Self {
        match e {
            ReminderSettingsError::SettingsInvalid => ReminderSettingsQueryError::SettingsInvalid,
        }
    }
}

impl From<crate::BackgroundFailed> for ReminderSettingsQueryError {
    fn from(_: crate::BackgroundFailed) -> Self {
        ReminderSettingsQueryError::Background
    }
}

impl ReminderSettingsQueryError {
    /// The variant name, for logs (carries no content).
    fn kind(&self) -> &'static str {
        match self {
            ReminderSettingsQueryError::SettingsInvalid => "SettingsInvalid",
            ReminderSettingsQueryError::Background => "Background",
        }
    }
}

fn thresholds(settings: &ReminderSettings, origin: ReminderOrigin) -> ReminderThresholds {
    ReminderThresholds {
        long_running: LongRunningThreshold {
            hours: settings.long_running.map(|d| (d.as_secs() / 3600) as u32),
            from_file: origin.long_running_from_file,
        },
        not_running_after: NotRunningAfterThreshold {
            time: settings
                .not_running_after
                .map(|t| t.format("%H:%M").to_string()),
            from_file: origin.not_running_after_from_file,
        },
    }
}

/// Read-only: the reminder thresholds, their on/off state and whether
/// `settings.json` set them. An unresolved config dir gives the defaults, like
/// the reminder thread.
#[tauri::command]
pub async fn reminder_settings(
    app: AppHandle,
) -> Result<ReminderThresholds, ReminderSettingsQueryError> {
    let path = settings_file::settings_path(&app);
    let result: Result<_, ReminderSettingsQueryError> = crate::blocking(move || match path {
        Some(p) => Ok(load_reminder_settings_detailed(&p)?),
        None => Ok((ReminderSettings::default(), ReminderOrigin::default())),
    })
    .await;
    match result {
        Ok((settings, origin)) => Ok(thresholds(&settings, origin)),
        // An invalid block is reported by the health source; the panel refetches
        // on every focus, so a warning each time would only repeat it.
        Err(e @ ReminderSettingsQueryError::SettingsInvalid) => {
            log::debug!("reminders: settings unavailable: {}", e.kind());
            Err(e)
        }
        Err(e) => {
            log::warn!("reminders: settings unavailable: {}", e.kind());
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;

    fn settings(hours: Option<u64>, time: Option<(u32, u32)>) -> ReminderSettings {
        ReminderSettings {
            long_running: hours.map(|h| Duration::from_secs(h * 3600)),
            not_running_after: time.and_then(|(h, m)| NaiveTime::from_hms_opt(h, m, 0)),
        }
    }

    fn origin(long: bool, not_running: bool) -> ReminderOrigin {
        ReminderOrigin {
            long_running_from_file: long,
            not_running_after_from_file: not_running,
        }
    }

    #[test]
    fn defaults_map_to_ten_hours_and_nine_oclock() {
        let t = thresholds(&ReminderSettings::default(), ReminderOrigin::default());
        assert_eq!(
            t,
            ReminderThresholds {
                long_running: LongRunningThreshold {
                    hours: Some(10),
                    from_file: false
                },
                not_running_after: NotRunningAfterThreshold {
                    time: Some("09:00".to_string()),
                    from_file: false
                },
            }
        );
    }

    #[test]
    fn time_stays_zero_padded() {
        let t = thresholds(&settings(Some(5), Some((9, 5))), origin(true, true));
        assert_eq!(t.not_running_after.time.as_deref(), Some("09:05"));
        assert!(t.not_running_after.from_file);
        let t = thresholds(&settings(Some(5), Some((0, 0))), origin(false, false));
        assert_eq!(t.not_running_after.time.as_deref(), Some("00:00"));
    }

    #[test]
    fn hour_edges_map_to_whole_hours() {
        for h in [1u64, 24] {
            let t = thresholds(&settings(Some(h), Some((9, 0))), origin(true, false));
            assert_eq!(t.long_running.hours, Some(h as u32));
            assert!(t.long_running.from_file);
        }
    }

    #[test]
    fn off_maps_to_none_and_keeps_the_origin() {
        let t = thresholds(&settings(None, None), origin(true, true));
        assert_eq!(t.long_running.hours, None);
        assert_eq!(t.not_running_after.time, None);
        assert!(t.long_running.from_file && t.not_running_after.from_file);
    }

    #[test]
    fn thresholds_serialize_in_camel_case_with_null_when_off() {
        let on = thresholds(&settings(Some(8), Some((7, 30))), origin(true, false));
        assert_eq!(
            serde_json::to_value(&on).unwrap(),
            serde_json::json!({
                "longRunning": { "hours": 8, "fromFile": true },
                "notRunningAfter": { "time": "07:30", "fromFile": false }
            })
        );
        let off = thresholds(&settings(None, None), origin(false, true));
        assert_eq!(
            serde_json::to_value(&off).unwrap(),
            serde_json::json!({
                "longRunning": { "hours": null, "fromFile": false },
                "notRunningAfter": { "time": null, "fromFile": true }
            })
        );
    }

    #[test]
    fn query_error_serializes_as_a_tagged_kind() {
        assert_eq!(
            serde_json::to_string(&ReminderSettingsQueryError::SettingsInvalid).unwrap(),
            r#"{"kind":"SettingsInvalid"}"#
        );
        assert_eq!(
            serde_json::to_string(&ReminderSettingsQueryError::Background).unwrap(),
            r#"{"kind":"Background"}"#
        );
    }

    #[test]
    fn query_error_conversions() {
        assert_eq!(
            ReminderSettingsQueryError::from(ReminderSettingsError::SettingsInvalid),
            ReminderSettingsQueryError::SettingsInvalid
        );
        assert_eq!(
            ReminderSettingsQueryError::from(crate::BackgroundFailed("x".into())),
            ReminderSettingsQueryError::Background
        );
    }
}
