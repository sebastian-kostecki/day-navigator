//! Read-only loader for the hand-written `settings.json`. Nothing here imports `tauri`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::vault::edit::check_plain;
use crate::vault::{VaultError, VaultPath};

const SUPPORTED_VERSION: u64 = 1;

#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub work: Option<SpaceSettings>,
    pub private: Option<SpaceSettings>,
}

#[derive(Debug, Clone)]
pub struct SpaceSettings {
    pub vault: PathBuf,
    pub excluded: Vec<VaultPath>,
    pub tasks: Option<TaskSettings>,
}

/// Where tasks live and which statuses and priorities exist.
#[derive(Debug, Clone)]
pub struct TaskSettings {
    pub folder: VaultPath,
    pub template: VaultPath,
    pub statuses: Vec<String>,
    pub done_status: String,
    pub priorities: Vec<String>,
}

#[derive(Deserialize)]
struct RawSettings {
    #[serde(default)]
    spaces: RawSpaces,
}

#[derive(Deserialize, Default)]
struct RawSpaces {
    #[serde(default)]
    work: Option<RawSpace>,
    #[serde(default)]
    private: Option<RawSpace>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSpace {
    vault: PathBuf,
    #[serde(default)]
    excluded: Vec<String>,
    #[serde(default)]
    tasks: Option<RawTasks>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTasks {
    folder: String,
    template: String,
    statuses: Vec<String>,
    done_status: String,
    #[serde(default)]
    priorities: Vec<String>,
}

fn invalid(reason: impl Into<String>) -> VaultError {
    VaultError::SettingsInvalid {
        reason: reason.into(),
    }
}

pub fn load_settings(path: &Path) -> Result<Settings, VaultError> {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(VaultError::NotConfigured {
                settings_path: path.to_path_buf(),
            });
        }
        Err(e) => {
            // File name only: error paths never carry absolute locations.
            return Err(VaultError::Io {
                path: path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                kind: format!("{:?}", e.kind()),
            });
        }
    };
    let bytes = bytes
        .strip_prefix(&[0xEF, 0xBB, 0xBF][..])
        .unwrap_or(&bytes);
    let text =
        std::str::from_utf8(bytes).map_err(|_| invalid("settings file is not valid UTF-8"))?;
    parse_settings(text)
}

fn parse_settings(text: &str) -> Result<Settings, VaultError> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|e| invalid(format!("malformed JSON: {e}")))?;
    match value.get("version").and_then(|v| v.as_u64()) {
        Some(SUPPORTED_VERSION) => {}
        Some(other) => return Err(invalid(format!("unsupported version {other}"))),
        None => return Err(invalid("missing or non-numeric version")),
    }
    let raw: RawSettings =
        serde_json::from_value(value).map_err(|e| invalid(format!("bad settings shape: {e}")))?;
    Ok(Settings {
        work: raw.spaces.work.map(|s| convert("work", s)).transpose()?,
        private: raw
            .spaces
            .private
            .map(|s| convert("private", s))
            .transpose()?,
    })
}

fn convert(name: &str, raw: RawSpace) -> Result<SpaceSettings, VaultError> {
    if !raw.vault.is_absolute() {
        return Err(invalid(format!("{name}: vault path must be absolute")));
    }
    let excluded = raw
        .excluded
        .iter()
        .map(|e| {
            VaultPath::parse(e)
                .map_err(|err| invalid(format!("{name}: invalid excluded entry: {err}")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let tasks = raw.tasks.map(|t| convert_tasks(name, t)).transpose()?;
    Ok(SpaceSettings {
        vault: raw.vault,
        excluded,
        tasks,
    })
}

fn convert_tasks(name: &str, raw: RawTasks) -> Result<TaskSettings, VaultError> {
    let folder = VaultPath::parse(&raw.folder)
        .map_err(|err| invalid(format!("{name}: invalid tasks folder: {err}")))?;
    let template = VaultPath::parse(&raw.template)
        .map_err(|err| invalid(format!("{name}: invalid tasks template: {err}")))?;
    if !template.is_markdown() {
        return Err(invalid(format!(
            "{name}: tasks template must be a .md note"
        )));
    }
    if raw.statuses.is_empty() {
        return Err(invalid(format!("{name}: tasks statuses must not be empty")));
    }
    // Case-insensitive: "Todo" and "todo" would be two picker entries for one
    // status as the user reads it.
    for (what, list) in [("status", &raw.statuses), ("priority", &raw.priorities)] {
        for (i, v) in list.iter().enumerate() {
            let folded = v.to_lowercase();
            if list[..i].iter().any(|p| p.to_lowercase() == folded) {
                return Err(invalid(format!("{name}: duplicate task {what} {v:?}")));
            }
        }
    }
    if !raw.statuses.contains(&raw.done_status) {
        return Err(invalid(format!(
            "{name}: tasks done_status must be one of statuses"
        )));
    }
    for v in raw.statuses.iter().chain(raw.priorities.iter()) {
        check_plain(v).map_err(|err| {
            invalid(format!(
                "{name}: task status or priority {v:?} is not usable: {err}"
            ))
        })?;
    }
    Ok(TaskSettings {
        folder,
        template,
        statuses: raw.statuses,
        done_status: raw.done_status,
        priorities: raw.priorities,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &tempfile::TempDir, text: &str) -> PathBuf {
        let p = dir.path().join("settings.json");
        fs::write(&p, text).unwrap();
        p
    }

    fn json_path(dir: &tempfile::TempDir) -> String {
        dir.path().to_string_lossy().replace('\\', "\\\\")
    }

    #[test]
    fn settings_missing_file_is_not_configured() {
        let dir = tempfile::tempdir().unwrap();
        let err = load_settings(&dir.path().join("settings.json")).unwrap_err();
        assert!(matches!(err, VaultError::NotConfigured { .. }));
    }

    #[test]
    fn settings_malformed_json_is_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let err = load_settings(&write(&dir, "{ not json")).unwrap_err();
        assert!(matches!(err, VaultError::SettingsInvalid { .. }));
    }

    #[test]
    fn settings_wrong_version_is_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let err = load_settings(&write(&dir, r#"{"version": 2, "spaces": {}}"#)).unwrap_err();
        assert!(matches!(err, VaultError::SettingsInvalid { .. }));
        let err = load_settings(&write(&dir, r#"{"spaces": {}}"#)).unwrap_err();
        assert!(matches!(err, VaultError::SettingsInvalid { .. }));
    }

    #[test]
    fn settings_relative_vault_path_is_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let text = r#"{"version":1,"spaces":{"work":{"vault":"relative/vault","excluded":[]}}}"#;
        let err = load_settings(&write(&dir, text)).unwrap_err();
        assert!(matches!(err, VaultError::SettingsInvalid { .. }));
    }

    #[test]
    fn settings_invalid_excluded_entry_is_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let root = json_path(&dir);
        for bad in ["../up", "/abs", ".hidden", "CON"] {
            let text = format!(
                r#"{{"version":1,"spaces":{{"work":{{"vault":"{root}","excluded":["{bad}"]}}}}}}"#
            );
            let err = load_settings(&write(&dir, &text)).unwrap_err();
            assert!(matches!(err, VaultError::SettingsInvalid { .. }), "{bad}");
        }
    }

    #[test]
    fn settings_null_and_absent_space_mean_not_configured() {
        let dir = tempfile::tempdir().unwrap();
        let root = json_path(&dir);
        let text = format!(
            r#"{{"version":1,"spaces":{{"work":{{"vault":"{root}","excluded":["Dostepy","Klienci/hasla.md"]}},"private":null}}}}"#
        );
        let s = load_settings(&write(&dir, &text)).unwrap();
        assert!(s.private.is_none());
        assert_eq!(s.work.unwrap().excluded.len(), 2);
        let s = load_settings(&write(&dir, r#"{"version":1,"spaces":{}}"#)).unwrap();
        assert!(s.work.is_none() && s.private.is_none());
    }

    fn tasks_text(dir: &tempfile::TempDir, tasks: &str) -> String {
        let root = json_path(dir);
        format!(
            r#"{{"version":1,"spaces":{{"work":{{"vault":"{root}","excluded":["Dostepy"],"tasks":{tasks}}}}}}}"#
        )
    }

    const VALID_TASKS: &str = r#"{"folder":"Tasks","template":"Templates/Task.md","statuses":["Todo","In progress","Done"],"done_status":"Done","priorities":["High","Medium","Low"]}"#;

    #[test]
    fn settings_valid_tasks_block_is_parsed() {
        let dir = tempfile::tempdir().unwrap();
        let s = load_settings(&write(&dir, &tasks_text(&dir, VALID_TASKS))).unwrap();
        let t = s.work.unwrap().tasks.unwrap();
        assert_eq!(t.folder.to_string(), "Tasks");
        assert_eq!(t.template.to_string(), "Templates/Task.md");
        assert_eq!(t.statuses, ["Todo", "In progress", "Done"]);
        assert_eq!(t.done_status, "Done");
        assert_eq!(t.priorities, ["High", "Medium", "Low"]);
    }

    #[test]
    fn settings_absent_tasks_block_is_none_and_priorities_may_be_empty() {
        let dir = tempfile::tempdir().unwrap();
        let root = json_path(&dir);
        let text =
            format!(r#"{{"version":1,"spaces":{{"work":{{"vault":"{root}","excluded":[]}}}}}}"#);
        assert!(
            load_settings(&write(&dir, &text))
                .unwrap()
                .work
                .unwrap()
                .tasks
                .is_none()
        );
        let no_prio = r#"{"folder":"Tasks","template":"T.md","statuses":["Todo","Done"],"done_status":"Done"}"#;
        let s = load_settings(&write(&dir, &tasks_text(&dir, no_prio))).unwrap();
        assert!(s.work.unwrap().tasks.unwrap().priorities.is_empty());
    }

    #[test]
    fn settings_invalid_tasks_blocks_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let cases = [
            // unknown key
            r#"{"folder":"Tasks","template":"T.md","statuses":["Done"],"done_status":"Done","extra":1}"#,
            // empty statuses
            r#"{"folder":"Tasks","template":"T.md","statuses":[],"done_status":"Done"}"#,
            // done_status not listed
            r#"{"folder":"Tasks","template":"T.md","statuses":["Todo"],"done_status":"Done"}"#,
            // duplicate statuses
            r#"{"folder":"Tasks","template":"T.md","statuses":["Todo","Todo","Done"],"done_status":"Done"}"#,
            // duplicate statuses differing only in case
            r#"{"folder":"Tasks","template":"T.md","statuses":["Todo","todo","Done"],"done_status":"Done"}"#,
            // duplicate priorities, also case-insensitively
            r#"{"folder":"Tasks","template":"T.md","statuses":["Done"],"done_status":"Done","priorities":["High","high"]}"#,
            // template is not a note
            r#"{"folder":"Tasks","template":"T.txt","statuses":["Done"],"done_status":"Done"}"#,
            // invalid folder
            r#"{"folder":"../up","template":"T.md","statuses":["Done"],"done_status":"Done"}"#,
            // invalid template path
            r#"{"folder":"Tasks","template":"/abs.md","statuses":["Done"],"done_status":"Done"}"#,
            // status needing quoting
            r#"{"folder":"Tasks","template":"T.md","statuses":["a: b","Done"],"done_status":"Done"}"#,
            // priority needing quoting
            r#"{"folder":"Tasks","template":"T.md","statuses":["Done"],"done_status":"Done","priorities":["[x]"]}"#,
            // empty status
            r#"{"folder":"Tasks","template":"T.md","statuses":["","Done"],"done_status":"Done"}"#,
        ];
        for bad in cases {
            let err = load_settings(&write(&dir, &tasks_text(&dir, bad))).unwrap_err();
            assert!(matches!(err, VaultError::SettingsInvalid { .. }), "{bad}");
        }
    }
}
