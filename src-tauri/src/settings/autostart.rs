//! The top-level `autostart` block of `settings.json`, read on its own like the
//! `workTime` block.

use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::SUPPORTED_VERSION;

/// Autostart settings. Off unless the block says `"enabled": true`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AutostartSettings {
    pub enabled: bool,
}

/// The `autostart` block (or the file around it) is invalid. Carries no detail:
/// serde errors can quote the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutostartSettingsInvalid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawAutostart {
    #[serde(default)]
    enabled: Option<serde_json::Value>,
}

/// Reads only the top-level `autostart` block, on its own. Shape:
///
/// ```json
/// "autostart": { "enabled": true }
/// ```
///
/// - `enabled`: `true` starts the app with Windows into the tray; `false`
///   (default) leaves it off.
///
/// A missing file, an absent or `null` block and an absent or `null` field are
/// the defaults (off). Anything else, a file-level problem included, is
/// `AutostartSettingsInvalid`.
pub fn load_autostart_settings(path: &Path) -> Result<AutostartSettings, AutostartSettingsInvalid> {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(AutostartSettings::default());
        }
        Err(_) => return Err(AutostartSettingsInvalid),
    };
    let bytes = bytes
        .strip_prefix(&[0xEF, 0xBB, 0xBF][..])
        .unwrap_or(&bytes);
    let text = std::str::from_utf8(bytes).map_err(|_| AutostartSettingsInvalid)?;
    parse_autostart_settings(text)
}

fn parse_autostart_settings(text: &str) -> Result<AutostartSettings, AutostartSettingsInvalid> {
    const INVALID: AutostartSettingsInvalid = AutostartSettingsInvalid;
    let value: serde_json::Value = serde_json::from_str(text).map_err(|_| INVALID)?;
    if value.get("version").and_then(|v| v.as_u64()) != Some(SUPPORTED_VERSION) {
        return Err(INVALID);
    }
    let Some(block) = value.get("autostart") else {
        return Ok(AutostartSettings::default());
    };
    if block.is_null() {
        return Ok(AutostartSettings::default());
    }
    if !block.is_object() {
        return Err(INVALID);
    }
    let raw: RawAutostart = serde_json::from_value(block.clone()).map_err(|_| INVALID)?;
    let enabled = match raw.enabled {
        None | Some(serde_json::Value::Null) | Some(serde_json::Value::Bool(false)) => false,
        Some(serde_json::Value::Bool(true)) => true,
        Some(_) => return Err(INVALID),
    };
    Ok(AutostartSettings { enabled })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{load_reminder_settings, load_todo_limit, load_work_time_settings};
    use std::path::PathBuf;

    fn write(dir: &tempfile::TempDir, text: &str) -> PathBuf {
        let p = dir.path().join("settings.json");
        fs::write(&p, text).unwrap();
        p
    }

    fn load(text: &str) -> Result<AutostartSettings, AutostartSettingsInvalid> {
        let dir = tempfile::tempdir().unwrap();
        load_autostart_settings(&write(&dir, text))
    }

    fn on() -> AutostartSettings {
        AutostartSettings { enabled: true }
    }

    fn off() -> AutostartSettings {
        AutostartSettings { enabled: false }
    }

    #[test]
    fn default_is_off() {
        assert_eq!(AutostartSettings::default(), off());
    }

    #[test]
    fn missing_file_is_the_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let got = load_autostart_settings(&dir.path().join("settings.json"));
        assert_eq!(got, Ok(off()));
    }

    #[test]
    fn absent_or_null_block_and_field_are_off() {
        for text in [
            r#"{"version":1}"#,
            r#"{"version":1,"autostart":null}"#,
            r#"{"version":1,"autostart":{}}"#,
            r#"{"version":1,"autostart":{"enabled":null}}"#,
            r#"{"version":1,"autostart":{"enabled":false}}"#,
        ] {
            assert_eq!(load(text), Ok(off()), "{text}");
        }
    }

    #[test]
    fn true_is_on() {
        assert_eq!(
            load(r#"{"version":1,"autostart":{"enabled":true}}"#),
            Ok(on())
        );
    }

    #[test]
    fn other_enabled_values_are_refused() {
        for v in [r#""true""#, "1", "0", r#""yes""#, "[]", "{}"] {
            let text = format!(r#"{{"version":1,"autostart":{{"enabled":{v}}}}}"#);
            assert_eq!(load(&text), Err(AutostartSettingsInvalid), "{v}");
        }
    }

    #[test]
    fn non_object_block_is_refused() {
        for v in ["true", "false", "1", r#""on""#, "[]"] {
            let text = format!(r#"{{"version":1,"autostart":{v}}}"#);
            assert_eq!(load(&text), Err(AutostartSettingsInvalid), "{v}");
        }
    }

    #[test]
    fn unknown_key_is_refused() {
        assert_eq!(
            load(r#"{"version":1,"autostart":{"enable":true}}"#),
            Err(AutostartSettingsInvalid)
        );
    }

    #[test]
    fn file_level_problems_are_refused() {
        for text in [
            "{ not json",
            r#"{"autostart":{"enabled":true}}"#,
            r#"{"version":2}"#,
            r#"{"version":"1"}"#,
        ] {
            assert_eq!(load(text), Err(AutostartSettingsInvalid), "{text}");
        }
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        fs::write(&p, [0xFF, 0xFE, 0x00]).unwrap();
        assert_eq!(load_autostart_settings(&p), Err(AutostartSettingsInvalid));
    }

    #[test]
    fn unreadable_path_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        // A directory where the file should be: readable as a path, not as a file.
        assert_eq!(
            load_autostart_settings(dir.path()),
            Err(AutostartSettingsInvalid)
        );
    }

    #[test]
    fn bom_is_accepted() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(br#"{"version":1,"autostart":{"enabled":true}}"#);
        fs::write(&p, bytes).unwrap();
        assert_eq!(load_autostart_settings(&p), Ok(on()));
    }

    #[test]
    fn a_broken_autostart_block_leaves_the_other_blocks_alone() {
        let dir = tempfile::tempdir().unwrap();
        let p = write(&dir, r#"{"version":1,"autostart":{"enabled":"yes"}}"#);
        assert!(load_autostart_settings(&p).is_err());
        assert!(load_work_time_settings(&p).is_ok());
        assert!(load_reminder_settings(&p).is_ok());
        assert!(load_todo_limit(&p).is_ok());
    }

    #[test]
    fn a_broken_reminders_block_leaves_autostart_alone() {
        let dir = tempfile::tempdir().unwrap();
        let p = write(
            &dir,
            r#"{"version":1,"reminders":{"longRunningHours":99},"autostart":{"enabled":true}}"#,
        );
        assert!(load_reminder_settings(&p).is_err());
        assert_eq!(load_autostart_settings(&p), Ok(on()));
    }
}
