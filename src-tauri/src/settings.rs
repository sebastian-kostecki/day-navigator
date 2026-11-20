//! Read-only loader for the hand-written `settings.json`. Nothing here imports `tauri`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

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
}
