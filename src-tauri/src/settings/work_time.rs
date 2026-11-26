//! The top-level `workTime` block of `settings.json`, read on its own like the
//! `reminders` block.

use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::SUPPORTED_VERSION;
use crate::work_time::holidays::HolidayCalendar;

/// Work-time rule settings. S-02 adds its daily-norm field here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkTimeSettings {
    pub holidays: HolidayCalendar,
}

impl Default for WorkTimeSettings {
    fn default() -> Self {
        WorkTimeSettings {
            holidays: HolidayCalendar::Pl,
        }
    }
}

/// The `workTime` block (or the file around it) is invalid. Carries no detail:
/// serde errors can quote the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkTimeSettingsInvalid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawWorkTime {
    #[serde(default)]
    holidays: Option<serde_json::Value>,
}

/// Reads only the top-level `workTime` block, on its own. Shape:
///
/// ```json
/// "workTime": { "holidays": "pl" }
/// ```
///
/// - `holidays`: `"pl"` (default) or `false` (calendar off).
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
    if block.is_null() {
        return Ok(WorkTimeSettings::default());
    }
    if !block.is_object() {
        return Err(INVALID);
    }
    let raw: RawWorkTime = serde_json::from_value(block.clone()).map_err(|_| INVALID)?;
    let mut settings = WorkTimeSettings::default();
    match raw.holidays {
        None | Some(serde_json::Value::Null) => {}
        Some(serde_json::Value::Bool(false)) => settings.holidays = HolidayCalendar::Off,
        Some(serde_json::Value::String(s)) if s == "pl" => settings.holidays = HolidayCalendar::Pl,
        Some(_) => return Err(INVALID),
    }
    Ok(settings)
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
        WorkTimeSettings {
            holidays: HolidayCalendar::Pl,
        }
    }

    fn off() -> WorkTimeSettings {
        WorkTimeSettings {
            holidays: HolidayCalendar::Off,
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
}
