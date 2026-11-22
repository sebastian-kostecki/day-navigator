mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{test_vault, vault_root};
use day_navigator_lib::health::{Overall, SourceId, SourceState, SourceStatus, overall, probe};
use day_navigator_lib::vault::{Space, VaultError};

/// Writes a `settings.json` in `dir`. `work` and `private` are
/// `(vault root, excluded entries)`.
fn write_settings(
    dir: &Path,
    work: Option<(&Path, &[&str])>,
    private: Option<(&Path, &[&str])>,
) -> PathBuf {
    fn space(v: Option<(&Path, &[&str])>) -> serde_json::Value {
        match v {
            None => serde_json::Value::Null,
            Some((root, excl)) => serde_json::json!({
                "vault": root.to_string_lossy(),
                "excluded": excl,
            }),
        }
    }
    let text = serde_json::json!({
        "version": 1,
        "spaces": { "work": space(work), "private": space(private) },
    });
    let path = dir.join("settings.json");
    fs::write(&path, text.to_string()).unwrap();
    path
}

fn find(sources: &[SourceStatus], id: SourceId) -> Option<&SourceState> {
    sources.iter().find(|s| s.id == id).map(|s| &s.state)
}

fn kind_of(state: &SourceState) -> Option<&str> {
    match state {
        SourceState::Error { kind, .. } => Some(kind),
        _ => None,
    }
}

fn ids(sources: &[SourceStatus]) -> Vec<String> {
    sources.iter().map(|s| s.id.key()).collect()
}

#[test]
fn missing_settings_file_is_a_settings_error_only() {
    let tmp = tempfile::tempdir().unwrap();
    let (reg, sources) = probe(&tmp.path().join("settings.json"));
    assert!(matches!(reg, Err(VaultError::NotConfigured { .. })));
    assert_eq!(ids(&sources), ["settings"]);
    assert_eq!(kind_of(&sources[0].state), Some("NotConfigured"));
    assert_eq!(overall(&sources, false, true), Overall::Error);
}

#[test]
fn invalid_settings_is_a_settings_error_only() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("settings.json");
    fs::write(&path, "{ not json").unwrap();
    let (reg, sources) = probe(&path);
    assert!(matches!(reg, Err(VaultError::SettingsInvalid { .. })));
    assert_eq!(ids(&sources), ["settings"]);
    assert_eq!(kind_of(&sources[0].state), Some("SettingsInvalid"));
}

#[test]
fn fully_ok_configuration_in_fixed_order() {
    let (tmp, _) = test_vault();
    let private = tempfile::tempdir().unwrap();
    let path = write_settings(
        tmp.path(),
        Some((&vault_root(&tmp), &["Dostepy"])),
        Some((private.path(), &[])),
    );
    let (reg, sources) = probe(&path);
    assert!(reg.is_ok());
    assert_eq!(
        ids(&sources),
        [
            "settings",
            "vault:work",
            "exclusions:work",
            "vault:private",
            "exclusions:private"
        ]
    );
    assert!(sources.iter().all(|s| s.state == SourceState::Ok));
    assert_eq!(overall(&sources, false, true), Overall::Ok);
}

#[test]
fn unconfigured_space_is_neutral() {
    let (tmp, _) = test_vault();
    let path = write_settings(tmp.path(), Some((&vault_root(&tmp), &["Dostepy"])), None);
    let (_, sources) = probe(&path);
    assert_eq!(
        find(&sources, SourceId::Vault(Space::Private)),
        Some(&SourceState::NotConfigured)
    );
    assert_eq!(
        find(&sources, SourceId::Exclusions(Space::Private)),
        Some(&SourceState::NotConfigured)
    );
    assert_eq!(overall(&sources, false, true), Overall::Ok);
}

#[test]
fn no_space_configured_is_a_settings_error() {
    let tmp = tempfile::tempdir().unwrap();
    let path = write_settings(tmp.path(), None, None);
    let (reg, sources) = probe(&path);
    assert!(reg.is_ok());
    let settings = find(&sources, SourceId::Settings).unwrap();
    assert_eq!(kind_of(settings), Some("NoSpaceConfigured"));
    assert_eq!(overall(&sources, false, true), Overall::Error);
}

#[test]
fn missing_root_is_reported_without_creating_it_or_exclusions() {
    let tmp = tempfile::tempdir().unwrap();
    let missing = tmp.path().join("no-such-vault");
    let path = write_settings(tmp.path(), Some((&missing, &["Dostepy"])), None);
    let (_, sources) = probe(&path);
    let vault = find(&sources, SourceId::Vault(Space::Work)).unwrap();
    assert_eq!(kind_of(vault), Some("RootMissing"));
    assert!(find(&sources, SourceId::Exclusions(Space::Work)).is_none());
    assert!(!missing.exists(), "probe must not create the folder");
    assert_eq!(overall(&sources, false, true), Overall::Error);
}

#[test]
fn overlapping_roots_are_a_vault_error() {
    let (tmp, _) = test_vault();
    let root = vault_root(&tmp);
    let inner = root.join("Notes");
    let path = write_settings(tmp.path(), Some((&root, &[])), Some((&inner, &[])));
    let (_, sources) = probe(&path);
    let work = find(&sources, SourceId::Vault(Space::Work)).unwrap();
    assert_eq!(kind_of(work), Some("RootsOverlap"));
    assert!(find(&sources, SourceId::Exclusions(Space::Work)).is_none());
}

#[test]
fn unmatched_exclusion_is_an_exclusions_error() {
    let (tmp, _) = test_vault();
    let path = write_settings(
        tmp.path(),
        Some((&vault_root(&tmp), &["Dostepy", "Klient-Testowy/hasla.md"])),
        None,
    );
    let (_, sources) = probe(&path);
    assert_eq!(
        find(&sources, SourceId::Vault(Space::Work)),
        Some(&SourceState::Ok)
    );
    let excl = find(&sources, SourceId::Exclusions(Space::Work)).unwrap();
    assert_eq!(kind_of(excl), Some("UnmatchedExclusions"));
    match excl {
        SourceState::Error { message, .. } => assert!(message.contains("Klient-Testowy/hasla.md")),
        _ => unreachable!(),
    }
    assert_eq!(overall(&sources, false, true), Overall::Error);
}
