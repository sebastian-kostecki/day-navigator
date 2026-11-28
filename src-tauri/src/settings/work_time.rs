//! The top-level `workTime` block of `settings.json`, read on its own like the
//! `reminders` block.

use std::fs;
use std::path::Path;

use serde_json::Value;

use super::SUPPORTED_VERSION;
use super::field_error::{FieldError, checked, object_with_keys, present};
use crate::work_time::holidays::HolidayCalendar;
use crate::work_time::norm::DEFAULT_DAILY_NORM_MS;

/// Work-time rule settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkTimeSettings {
    pub holidays: HolidayCalendar,
    /// Full-day norm in ms, always a whole number of minutes.
    pub daily_norm_ms: i64,
}

/// Bounds of `dailyNormHours`, in minutes (1 h to 10 h, the scale of the day bar).
pub const DAILY_NORM_MIN_MINUTES: i64 = 60;
pub const DAILY_NORM_MAX_MINUTES: i64 = 600;

impl Default for WorkTimeSettings {
    fn default() -> Self {
        WorkTimeSettings {
            holidays: HolidayCalendar::Pl,
            daily_norm_ms: DEFAULT_DAILY_NORM_MS,
        }
    }
}

/// The `workTime` block (or the file around it) is invalid. Carries no detail:
/// serde errors can quote the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkTimeSettingsInvalid;

/// Reads only the top-level `workTime` block, on its own. Shape:
///
/// ```json
/// "workTime": { "holidays": "pl", "dailyNormHours": 8 }
/// ```
///
/// - `holidays`: `"pl"` (default) or `false` (calendar off).
/// - `dailyNormHours`: hours per workday, a JSON number from 1 to 10 in whole
///   minutes (7.5 is fine, 7.33 is not); default 8.
///
/// A missing file, an absent or `null` block and an absent or `null` field are
/// the defaults. Anything else, a file-level problem included, is
/// `WorkTimeSettingsInvalid`.
pub fn load_work_time_settings(path: &Path) -> Result<WorkTimeSettings, WorkTimeSettingsInvalid> {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(WorkTimeSettings::default());
        }
        Err(_) => return Err(WorkTimeSettingsInvalid),
    };
    let bytes = bytes
        .strip_prefix(&[0xEF, 0xBB, 0xBF][..])
        .unwrap_or(&bytes);
    let text = std::str::from_utf8(bytes).map_err(|_| WorkTimeSettingsInvalid)?;
    parse_work_time_settings(text)
}

fn parse_work_time_settings(text: &str) -> Result<WorkTimeSettings, WorkTimeSettingsInvalid> {
    const INVALID: WorkTimeSettingsInvalid = WorkTimeSettingsInvalid;
    let value: serde_json::Value = serde_json::from_str(text).map_err(|_| INVALID)?;
    if value.get("version").and_then(|v| v.as_u64()) != Some(SUPPORTED_VERSION) {
        return Err(INVALID);
    }
    let Some(block) = value.get("workTime") else {
        return Ok(WorkTimeSettings::default());
    };
    validate_work_time(block).map_err(|_| INVALID)
}

/// Checks a `workTime` block, reporting every bad field.
pub(super) fn validate_work_time(block: &Value) -> Result<WorkTimeSettings, Vec<FieldError>> {
    let mut settings = WorkTimeSettings::default();
    if block.is_null() {
        return Ok(settings);
    }
    let keys = ["holidays", "dailyNormHours"];
    let map = object_with_keys(block, "workTime", &keys).map_err(|e| vec![e])?;
    let mut errors = Vec::new();
    match present(map, "holidays") {
        None => {}
        Some(Value::Bool(false)) => settings.holidays = HolidayCalendar::Off,
        Some(Value::String(s)) if s == "pl" => settings.holidays = HolidayCalendar::Pl,
        Some(_) => errors.push(FieldError::new(
            "workTime.holidays",
            "Dozwolone wartości to \"pl\" albo false.",
        )),
    }
    if let Some(value) = present(map, "dailyNormHours") {
        match value.as_f64().and_then(norm_ms_from_hours) {
            Some(ms) => settings.daily_norm_ms = ms,
            None => errors.push(FieldError::new(
                "workTime.dailyNormHours",
                "Podaj liczbę godzin od 1 do 10, w pełnych minutach.",
            )),
        }
    }
    checked(settings, errors)
}

/// Hours as whole minutes in range, as ms; `None` otherwise.
fn norm_ms_from_hours(hours: f64) -> Option<i64> {
    let minutes = hours * 60.0;
    if !minutes.is_finite() || (minutes - minutes.round()).abs() > 1e-6 {
        return None;
    }
    let minutes = minutes.round() as i64;
    (DAILY_NORM_MIN_MINUTES..=DAILY_NORM_MAX_MINUTES)
        .contains(&minutes)
        .then(|| minutes * 60_000)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{load_reminder_settings, load_todo_limit};
    use std::path::PathBuf;

    fn write(dir: &tempfile::TempDir, text: &str) -> PathBuf {
        let p = dir.path().join("settings.json");
        fs::write(&p, text).unwrap();
        p
    }

    fn load(text: &str) -> Result<WorkTimeSettings, WorkTimeSettingsInvalid> {
        let dir = tempfile::tempdir().unwrap();
        load_work_time_settings(&write(&dir, text))
    }

    fn pl() -> WorkTimeSettings {
        WorkTimeSettings::default()
    }

    fn off() -> WorkTimeSettings {
        WorkTimeSettings {
            holidays: HolidayCalendar::Off,
            ..WorkTimeSettings::default()
        }
    }

    #[test]
    fn default_calendar_is_pl() {
        assert_eq!(WorkTimeSettings::default(), pl());
    }

    #[test]
    fn missing_file_is_the_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let got = load_work_time_settings(&dir.path().join("settings.json"));
        assert_eq!(got, Ok(pl()));
    }

    #[test]
    fn absent_or_null_block_and_field_are_the_defaults() {
        for text in [
            r#"{"version":1}"#,
            r#"{"version":1,"workTime":null}"#,
            r#"{"version":1,"workTime":{}}"#,
            r#"{"version":1,"workTime":{"holidays":null}}"#,
        ] {
            assert_eq!(load(text), Ok(pl()), "{text}");
        }
    }

    #[test]
    fn pl_and_false_map_to_the_calendar() {
        assert_eq!(
            load(r#"{"version":1,"workTime":{"holidays":"pl"}}"#),
            Ok(pl())
        );
        assert_eq!(
            load(r#"{"version":1,"workTime":{"holidays":false}}"#),
            Ok(off())
        );
    }

    #[test]
    fn other_holiday_values_are_refused() {
        for v in [
            "true",
            r#""PL""#,
            r#""""#,
            r#""de""#,
            "1",
            "0",
            "[]",
            r#"["pl"]"#,
            "{}",
        ] {
            let text = format!(r#"{{"version":1,"workTime":{{"holidays":{v}}}}}"#);
            assert_eq!(load(&text), Err(WorkTimeSettingsInvalid), "{v}");
        }
    }

    #[test]
    fn non_object_block_is_refused() {
        for v in ["true", "false", "1", r#""pl""#, "[]"] {
            let text = format!(r#"{{"version":1,"workTime":{v}}}"#);
            assert_eq!(load(&text), Err(WorkTimeSettingsInvalid), "{v}");
        }
    }

    #[test]
    fn unknown_key_is_refused() {
        assert_eq!(
            load(r#"{"version":1,"workTime":{"holidayz":"pl"}}"#),
            Err(WorkTimeSettingsInvalid)
        );
    }

    #[test]
    fn file_level_problems_are_refused() {
        for text in [
            "{ not json",
            r#"{"workTime":{"holidays":"pl"}}"#,
            r#"{"version":2}"#,
            r#"{"version":"1"}"#,
        ] {
            assert_eq!(load(text), Err(WorkTimeSettingsInvalid), "{text}");
        }
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        fs::write(&p, [0xFF, 0xFE, 0x00]).unwrap();
        assert_eq!(load_work_time_settings(&p), Err(WorkTimeSettingsInvalid));
    }

    #[test]
    fn unreadable_path_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        // A directory where the file should be: readable as a path, not as a file.
        assert_eq!(
            load_work_time_settings(dir.path()),
            Err(WorkTimeSettingsInvalid)
        );
    }

    #[test]
    fn bom_is_accepted() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(br#"{"version":1,"workTime":{"holidays":false}}"#);
        fs::write(&p, bytes).unwrap();
        assert_eq!(load_work_time_settings(&p), Ok(off()));
    }

    #[test]
    fn a_broken_work_time_block_leaves_the_other_blocks_alone() {
        let dir = tempfile::tempdir().unwrap();
        let p = write(&dir, r#"{"version":1,"workTime":{"holidays":1}}"#);
        assert!(load_work_time_settings(&p).is_err());
        assert!(load_reminder_settings(&p).is_ok());
        assert!(load_todo_limit(&p).is_ok());
    }

    #[test]
    fn a_broken_reminders_block_leaves_work_time_alone() {
        let dir = tempfile::tempdir().unwrap();
        let p = write(
            &dir,
            r#"{"version":1,"reminders":{"longRunningHours":99},"workTime":{"holidays":false}}"#,
        );
        assert!(load_reminder_settings(&p).is_err());
        assert_eq!(load_work_time_settings(&p), Ok(off()));
    }

    fn with_norm(v: &str) -> Result<WorkTimeSettings, WorkTimeSettingsInvalid> {
        load(&format!(
            r#"{{"version":1,"workTime":{{"dailyNormHours":{v}}}}}"#
        ))
    }

    #[test]
    fn default_norm_is_eight_hours() {
        assert_eq!(WorkTimeSettings::default().daily_norm_ms, 8 * 3_600_000);
        assert_eq!(with_norm("null"), Ok(pl()));
        assert_eq!(load(r#"{"version":1,"workTime":{}}"#), Ok(pl()));
    }

    #[test]
    fn norm_hours_in_range_and_whole_minutes_are_accepted() {
        for (v, ms) in [
            ("1", 3_600_000),
            ("7.5", 27_000_000),
            ("8", 28_800_000),
            ("8.0", 28_800_000),
            ("10", 36_000_000),
            ("7.25", 26_100_000),
        ] {
            assert_eq!(with_norm(v).unwrap().daily_norm_ms, ms, "{v}");
        }
    }

    #[test]
    fn norm_hours_outside_the_range_or_not_whole_minutes_are_refused() {
        for v in [
            "0", "0.99", "10.01", "11", "7.33", "-8", "-1", "1e3", r#""8""#, "true", "false", "[]",
            "{}", "[8]",
        ] {
            assert_eq!(with_norm(v), Err(WorkTimeSettingsInvalid), "{v}");
        }
    }

    #[test]
    fn norm_works_with_the_calendar_setting() {
        let got = load(r#"{"version":1,"workTime":{"holidays":false,"dailyNormHours":7.5}}"#);
        assert_eq!(
            got,
            Ok(WorkTimeSettings {
                holidays: HolidayCalendar::Off,
                daily_norm_ms: 27_000_000,
            })
        );
    }

    #[test]
    fn a_norm_typo_is_refused() {
        assert_eq!(
            load(r#"{"version":1,"workTime":{"dailyNormHour":7.5}}"#),
            Err(WorkTimeSettingsInvalid)
        );
    }

    #[test]
    fn a_broken_norm_leaves_the_other_blocks_alone() {
        let dir = tempfile::tempdir().unwrap();
        let p = write(
            &dir,
            r#"{"version":1,"workTime":{"dailyNormHours":11},"reminders":{"longRunningHours":8}}"#,
        );
        assert!(load_work_time_settings(&p).is_err());
        assert!(load_reminder_settings(&p).is_ok());
        assert!(load_todo_limit(&p).is_ok());
    }
}
