//! Read-only loader for the hand-written `settings.json`. Nothing here imports `tauri`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::cli::{GitlabConfig, GitlabHost, GitlabUser};
use crate::gitlab::GitlabError;
use crate::todo::{DEFAULT_TODO_LIMIT, TodoError};
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
    /// The retired vault-task block: still accepted so an old hand-written
    /// `settings.json` loads, but any content is ignored.
    #[serde(default, rename = "tasks")]
    _legacy_tasks: Option<serde::de::IgnoredAny>,
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGitlab {
    host: String,
    #[serde(default)]
    user: Option<String>,
}

/// Reads only the top-level `gitlab` block, on its own: a typo there never
/// breaks the vaults and a vault typo never hides the GitLab list. A missing
/// file, an absent block or `null` is `Ok(None)`. Every failure is
/// `SettingsInvalid` with no detail: serde errors can quote the value.
pub fn load_gitlab_settings(path: &Path) -> Result<Option<GitlabConfig>, GitlabError> {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(GitlabError::SettingsInvalid),
    };
    let bytes = bytes
        .strip_prefix(&[0xEF, 0xBB, 0xBF][..])
        .unwrap_or(&bytes);
    let text = std::str::from_utf8(bytes).map_err(|_| GitlabError::SettingsInvalid)?;
    parse_gitlab_settings(text)
}

fn parse_gitlab_settings(text: &str) -> Result<Option<GitlabConfig>, GitlabError> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|_| GitlabError::SettingsInvalid)?;
    if value.get("version").and_then(|v| v.as_u64()) != Some(SUPPORTED_VERSION) {
        return Err(GitlabError::SettingsInvalid);
    }
    let Some(block) = value.get("gitlab") else {
        return Ok(None);
    };
    // serde would also accept a bare array for a struct; only an object is valid.
    if !block.is_object() && !block.is_null() {
        return Err(GitlabError::SettingsInvalid);
    }
    let raw: Option<RawGitlab> =
        serde_json::from_value(block.clone()).map_err(|_| GitlabError::SettingsInvalid)?;
    raw.map(|r| {
        let host = GitlabHost::parse(&r.host).map_err(|_| GitlabError::SettingsInvalid)?;
        let user = r
            .user
            .map(|u| GitlabUser::parse(&u).map_err(|_| GitlabError::SettingsInvalid))
            .transpose()?;
        Ok(GitlabConfig { host, user })
    })
    .transpose()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTodo {
    #[serde(default)]
    limit: Option<serde_json::Value>,
}

const TODO_LIMIT_MAX: u64 = 20;

/// Reads only the top-level `todo` block, on its own, like the GitLab block.
/// A missing file, an absent or `null` block or an absent `limit` is the
/// default (5). Anything else that is not an integer 1..=20 is
/// `SettingsInvalid`, with no detail: serde errors can quote the value.
pub fn load_todo_limit(path: &Path) -> Result<u32, TodoError> {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(DEFAULT_TODO_LIMIT),
        Err(_) => return Err(TodoError::SettingsInvalid),
    };
    let bytes = bytes
        .strip_prefix(&[0xEF, 0xBB, 0xBF][..])
        .unwrap_or(&bytes);
    let text = std::str::from_utf8(bytes).map_err(|_| TodoError::SettingsInvalid)?;
    parse_todo_limit(text)
}

fn parse_todo_limit(text: &str) -> Result<u32, TodoError> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|_| TodoError::SettingsInvalid)?;
    if value.get("version").and_then(|v| v.as_u64()) != Some(SUPPORTED_VERSION) {
        return Err(TodoError::SettingsInvalid);
    }
    let Some(block) = value.get("todo") else {
        return Ok(DEFAULT_TODO_LIMIT);
    };
    if block.is_null() {
        return Ok(DEFAULT_TODO_LIMIT);
    }
    if !block.is_object() {
        return Err(TodoError::SettingsInvalid);
    }
    let raw: RawTodo =
        serde_json::from_value(block.clone()).map_err(|_| TodoError::SettingsInvalid)?;
    match raw.limit {
        None => Ok(DEFAULT_TODO_LIMIT),
        Some(v) => match v.as_u64() {
            Some(n) if (1..=TODO_LIMIT_MAX).contains(&n) => Ok(n as u32),
            _ => Err(TodoError::SettingsInvalid),
        },
    }
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
    Ok(SpaceSettings {
        vault: raw.vault,
        excluded,
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

    fn gitlab_text(block: &str) -> String {
        format!(r#"{{"version":1,"spaces":{{}},"gitlab":{block}}}"#)
    }

    #[test]
    fn gitlab_valid_block_parses_and_lowercases() {
        let dir = tempfile::tempdir().unwrap();
        let text = gitlab_text(r#"{"host":"GitLab.Example.Invalid"}"#);
        let cfg = load_gitlab_settings(&write(&dir, &text)).unwrap().unwrap();
        assert_eq!(cfg.host.as_str(), "gitlab.example.invalid");
        assert!(cfg.user.is_none());
    }

    #[test]
    fn gitlab_optional_user_parses_and_null_means_none() {
        let dir = tempfile::tempdir().unwrap();
        let text = gitlab_text(r#"{"host":"gitlab.example.invalid","user":"Invented.User"}"#);
        let cfg = load_gitlab_settings(&write(&dir, &text)).unwrap().unwrap();
        // The login keeps its case.
        assert_eq!(cfg.user.unwrap().as_str(), "Invented.User");
        let text = gitlab_text(r#"{"host":"gitlab.example.invalid","user":null}"#);
        let cfg = load_gitlab_settings(&write(&dir, &text)).unwrap().unwrap();
        assert!(cfg.user.is_none());
    }

    #[test]
    fn gitlab_invalid_user_is_refused_without_leaking_it() {
        const SECRET: &str = "secret-login";
        let dir = tempfile::tempdir().unwrap();
        let cases = [
            r#"{"host":"gitlab.example.invalid","user":""}"#.to_string(),
            r#"{"host":"gitlab.example.invalid","user":5}"#.to_string(),
            r#"{"host":"gitlab.example.invalid","user":["x"]}"#.to_string(),
            format!(r#"{{"host":"gitlab.example.invalid","user":"{SECRET}&x"}}"#),
            format!(r#"{{"host":"gitlab.example.invalid","user":"{SECRET}/x"}}"#),
            format!(r#"{{"host":"gitlab.example.invalid","user":"{SECRET} x"}}"#),
            format!(r#"{{"host":"gitlab.example.invalid","user":"-{SECRET}"}}"#),
            format!(r#"{{"host":"gitlab.example.invalid","user":".{SECRET}"}}"#),
            format!(r#"{{"host":"gitlab.example.invalid","user":"{SECRET}","extra":1}}"#),
        ];
        for block in cases {
            let err = load_gitlab_settings(&write(&dir, &gitlab_text(&block))).unwrap_err();
            assert_eq!(err, GitlabError::SettingsInvalid, "{block}");
            let shown = [
                format!("{err:?}"),
                err.to_string(),
                serde_json::to_string(&err).unwrap(),
                err.message_pl(),
            ];
            for text in shown {
                assert!(!text.contains(SECRET), "leak for {block}: {text}");
            }
        }
    }

    #[test]
    fn gitlab_bom_is_stripped() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(gitlab_text(r#"{"host":"gitlab.example.invalid"}"#).as_bytes());
        fs::write(&p, bytes).unwrap();
        assert!(load_gitlab_settings(&p).unwrap().is_some());
    }

    #[test]
    fn gitlab_missing_file_absent_and_null_block_are_not_configured() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            load_gitlab_settings(&dir.path().join("settings.json")),
            Ok(None)
        );
        let text = r#"{"version":1,"spaces":{}}"#;
        assert_eq!(load_gitlab_settings(&write(&dir, text)), Ok(None));
        let text = gitlab_text("null");
        assert_eq!(load_gitlab_settings(&write(&dir, &text)), Ok(None));
    }

    #[test]
    fn gitlab_invalid_blocks_are_refused_without_leaking_the_value() {
        const SECRET: &str = "secret.example.invalid";
        let dir = tempfile::tempdir().unwrap();
        let cases = [
            format!(r#"{{"host":"{SECRET}","extra":1}}"#),
            format!(r#""{SECRET}""#),
            format!(r#"[ "{SECRET}" ]"#),
            format!(r#"{{"host":5,"x":"{SECRET}"}}"#),
            r#"{}"#.to_string(),
            r#"{"host":""}"#.to_string(),
            format!(r#"{{"host":"https://{SECRET}"}}"#),
            format!(r#"{{"host":"{SECRET}:8443"}}"#),
            format!(r#"{{"host":"{SECRET}/path"}}"#),
            format!(r#"{{"host":" {SECRET}"}}"#),
            format!(r#"{{"host":"-{SECRET}"}}"#),
            format!(r#"{{"host":"{}"}}"#, SECRET.split('.').next().unwrap()),
        ];
        for block in cases {
            let err = load_gitlab_settings(&write(&dir, &gitlab_text(&block))).unwrap_err();
            assert_eq!(err, GitlabError::SettingsInvalid, "{block}");
            let shown = [
                format!("{err:?}"),
                err.to_string(),
                serde_json::to_string(&err).unwrap(),
                err.message_pl(),
            ];
            for text in shown {
                assert!(!text.contains("secret"), "leak for {block}: {text}");
            }
        }
    }

    #[test]
    fn gitlab_broken_file_is_settings_invalid() {
        let dir = tempfile::tempdir().unwrap();
        for text in [
            "{ not json",
            r#"{"gitlab":{"host":"gitlab.example.invalid"}}"#,
            r#"{"version":2,"gitlab":{"host":"gitlab.example.invalid"}}"#,
        ] {
            assert_eq!(
                load_gitlab_settings(&write(&dir, text)),
                Err(GitlabError::SettingsInvalid),
                "{text}"
            );
        }
        let p = dir.path().join("settings.json");
        fs::write(&p, [0xFF, 0xFE, 0x00]).unwrap();
        assert_eq!(load_gitlab_settings(&p), Err(GitlabError::SettingsInvalid));
        // A directory where the file should be: unreadable, not "missing".
        assert_eq!(
            load_gitlab_settings(dir.path()),
            Err(GitlabError::SettingsInvalid)
        );
    }

    #[test]
    fn vault_settings_ignore_the_gitlab_block_valid_or_not() {
        let dir = tempfile::tempdir().unwrap();
        let root = json_path(&dir);
        for block in [r#"{"host":"gitlab.example.invalid"}"#, r#""not-an-object""#] {
            let text = format!(
                r#"{{"version":1,"spaces":{{"work":{{"vault":"{root}","excluded":["Dostepy"]}}}},"gitlab":{block}}}"#
            );
            let s = load_settings(&write(&dir, &text)).unwrap();
            let work = s.work.unwrap();
            assert_eq!(work.vault, PathBuf::from(dir.path()));
            assert_eq!(work.excluded.len(), 1);
            assert!(s.private.is_none());
        }
    }

    fn todo_text(block: &str) -> String {
        format!(r#"{{"version":1,"spaces":{{}},"todo":{block}}}"#)
    }

    #[test]
    fn todo_limit_defaults() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            load_todo_limit(&dir.path().join("settings.json")).unwrap(),
            DEFAULT_TODO_LIMIT
        );
        for text in [
            r#"{"version":1,"spaces":{}}"#.to_string(),
            todo_text("null"),
            todo_text("{}"),
            todo_text(r#"{"limit":null}"#),
        ] {
            assert_eq!(load_todo_limit(&write(&dir, &text)).unwrap(), 5, "{text}");
        }
    }

    #[test]
    fn todo_limit_range_edges() {
        let dir = tempfile::tempdir().unwrap();
        for n in [1u32, 7, 20] {
            let text = todo_text(&format!(r#"{{"limit":{n}}}"#));
            assert_eq!(load_todo_limit(&write(&dir, &text)).unwrap(), n);
        }
    }

    #[test]
    fn todo_limit_invalid_values_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let blocks = [
            r#"{"limit":0}"#,
            r#"{"limit":21}"#,
            r#"{"limit":-1}"#,
            r#"{"limit":"5"}"#,
            r#"{"limit":5.5}"#,
            r#"{"limit":true}"#,
            r#"{"limit":5,"extra":1}"#,
            r#"{"other":1}"#,
            r#""5""#,
            r#"[5]"#,
        ];
        for block in blocks {
            let err = load_todo_limit(&write(&dir, &todo_text(block))).unwrap_err();
            assert!(matches!(err, TodoError::SettingsInvalid), "{block}");
        }
        for text in [
            "{ not json",
            r#"{"todo":{"limit":5}}"#,
            r#"{"version":2,"todo":{"limit":5}}"#,
        ] {
            let err = load_todo_limit(&write(&dir, text)).unwrap_err();
            assert!(matches!(err, TodoError::SettingsInvalid), "{text}");
        }
        let p = dir.path().join("settings.json");
        fs::write(&p, [0xFF, 0xFE, 0x00]).unwrap();
        assert!(matches!(
            load_todo_limit(&p),
            Err(TodoError::SettingsInvalid)
        ));
    }

    #[test]
    fn todo_limit_bom_is_stripped() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.json");
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(todo_text(r#"{"limit":3}"#).as_bytes());
        fs::write(&p, bytes).unwrap();
        assert_eq!(load_todo_limit(&p).unwrap(), 3);
    }

    #[test]
    fn broken_todo_block_leaves_vault_and_gitlab_loaders_green() {
        let dir = tempfile::tempdir().unwrap();
        let root = json_path(&dir);
        for block in [r#"{"limit":0}"#, r#""nope""#, r#"{"bogus":1}"#] {
            let text = format!(
                r#"{{"version":1,"spaces":{{"work":{{"vault":"{root}","excluded":[]}}}},"gitlab":{{"host":"gitlab.example.invalid"}},"todo":{block}}}"#
            );
            let p = write(&dir, &text);
            assert!(load_settings(&p).unwrap().work.is_some(), "{block}");
            assert!(load_gitlab_settings(&p).unwrap().is_some(), "{block}");
            assert!(load_todo_limit(&p).is_err(), "{block}");
        }
    }

    #[test]
    fn todo_loader_ignores_a_broken_gitlab_block() {
        let dir = tempfile::tempdir().unwrap();
        let text = r#"{"version":1,"spaces":{},"gitlab":"nope","todo":{"limit":4}}"#;
        assert_eq!(load_todo_limit(&write(&dir, text)).unwrap(), 4);
    }

    fn work_with(dir: &tempfile::TempDir, extra: &str) -> String {
        let root = json_path(dir);
        format!(
            r#"{{"version":1,"spaces":{{"work":{{"vault":"{root}","excluded":["Dostepy"],{extra}}}}}}}"#
        )
    }

    #[test]
    fn settings_legacy_tasks_block_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let tasks = r#""tasks":{"folder":"Tasks","template":"Templates/Task.md","statuses":["Todo","In progress","Done"],"done_status":"Done","priorities":["High","Medium","Low"]}"#;
        let s = load_settings(&write(&dir, &work_with(&dir, tasks))).unwrap();
        assert_eq!(s.work.unwrap().excluded.len(), 1);
    }

    #[test]
    fn settings_legacy_tasks_value_of_any_shape_loads() {
        let dir = tempfile::tempdir().unwrap();
        for value in ["null", "42", r#""text""#, "[1,2]", r#"{"unknown":true}"#] {
            let extra = format!(r#""tasks":{value}"#);
            let loaded = load_settings(&write(&dir, &work_with(&dir, &extra)));
            assert!(loaded.is_ok(), "{value}");
        }
    }

    #[test]
    fn settings_unknown_space_key_is_still_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let err = load_settings(&write(&dir, &work_with(&dir, r#""bogus":1"#))).unwrap_err();
        assert!(matches!(err, VaultError::SettingsInvalid { .. }));
    }
}
