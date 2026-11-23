mod common;

use std::fs;
use std::path::{Path, PathBuf};

use chrono::NaiveDate;

use common::{test_vault, vault_root};
use day_navigator_lib::health::{
    Overall, SourceId, SourceState, SourceStatus, overall, probe, store_status,
};
use day_navigator_lib::store::{DB_FILE, StoreError, StoreHandle};
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

fn day() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 3, 4).unwrap()
}

#[test]
fn open_store_is_ok_as_app_data() {
    let tmp = tempfile::tempdir().unwrap();
    let handle = StoreHandle::new(Some(tmp.path().join("data")));
    let status = store_status(&handle, day());
    assert_eq!(status.id, SourceId::AppData);
    assert_eq!(status.label, "Dane aplikacji");
    assert_eq!(status.state, SourceState::Ok);
    assert_eq!(overall(&[status], false, true), Overall::Ok);
}

#[test]
fn damaged_file_is_an_error_naming_the_data_dir_and_recovers_from_backup() {
    let tmp = tempfile::tempdir().unwrap();

    // A valid database to restore from, made by a first run elsewhere.
    let good_dir = tmp.path().join("good");
    assert!(
        StoreHandle::new(Some(good_dir.clone()))
            .status(day())
            .is_ok()
    );
    let good = good_dir.join(DB_FILE);

    let data = tmp.path().join("data");
    fs::create_dir_all(&data).unwrap();
    let db = data.join(DB_FILE);
    fs::write(&db, "invented text, not a database at all").unwrap();
    let before = fs::read(&db).unwrap();

    let handle = StoreHandle::new(Some(data.clone()));
    let status = store_status(&handle, day());
    match &status.state {
        SourceState::Error { kind, message } => {
            assert_eq!(kind, "Damaged");
            assert!(message.starts_with("Plik danych aplikacji jest uszkodzony."));
            assert!(message.contains(&data.join("backups").display().to_string()));
        }
        other => panic!("expected Error, got {other:?}"),
    }
    assert_eq!(
        fs::read(&db).unwrap(),
        before,
        "refusal must not touch the file"
    );

    // The serialised error is path-free.
    let err = handle.status(day()).unwrap_err();
    assert!(matches!(err, StoreError::Damaged { .. }));
    let json = serde_json::to_string(&err).unwrap();
    assert!(!json.contains(&*data.to_string_lossy()), "{json}");

    // Replace the file with a valid one: the next status recovers.
    fs::copy(&good, &db).unwrap();
    assert_eq!(store_status(&handle, day()).state, SourceState::Ok);
}

#[test]
fn missing_file_with_backups_names_the_backup_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path().join("data");
    let handle = StoreHandle::new(Some(data.clone()));
    assert!(handle.status(day()).is_ok()); // creates db and today's backup
    drop(handle);
    for ext in ["", "-wal", "-shm"] {
        let _ = fs::remove_file(data.join(format!("{DB_FILE}{ext}")));
    }
    let handle = StoreHandle::new(Some(data.clone()));
    match store_status(&handle, day()).state {
        SourceState::Error { kind, message } => {
            assert_eq!(kind, "Missing");
            assert!(message.contains(&data.join("backups").display().to_string()));
        }
        other => panic!("expected Error, got {other:?}"),
    }
    assert!(!data.join(DB_FILE).exists());
}

#[test]
fn handle_without_a_dir_is_dir_unavailable() {
    let handle = StoreHandle::new(None);
    assert!(matches!(
        handle.status(day()),
        Err(StoreError::DirUnavailable { .. })
    ));
    match store_status(&handle, day()).state {
        SourceState::Error { kind, .. } => assert_eq!(kind, "DirUnavailable"),
        other => panic!("expected Error, got {other:?}"),
    }
}
