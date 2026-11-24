use std::fs;

use chrono::NaiveDate;

use day_navigator_lib::health::{
    Overall, SourceId, SourceState, SourceStatus, overall, probe, store_probe, store_status,
};
use day_navigator_lib::store::{DB_FILE, StoreError, StoreHandle};

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
    let sources = probe(&tmp.path().join("settings.json"));
    assert_eq!(ids(&sources), ["settings"]);
    assert_eq!(kind_of(&sources[0].state), Some("NotConfigured"));
    assert_eq!(overall(&sources, false, true), Overall::Error);
}

#[test]
fn invalid_settings_is_a_settings_error_only() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("settings.json");
    fs::write(&path, "{ not json").unwrap();
    let sources = probe(&path);
    assert_eq!(ids(&sources), ["settings"]);
    assert_eq!(kind_of(&sources[0].state), Some("SettingsInvalid"));
}

/// An old `settings.json` still holds the retired vault block (`spaces`), here
/// with a missing folder and odd excluded entries: it is ignored, so only the
/// settings source is reported, and it is Ok.
#[test]
fn old_settings_with_spaces_is_only_settings_ok() {
    let tmp = tempfile::tempdir().unwrap();
    let missing = tmp.path().join("no-such-folder");
    let text = serde_json::json!({
        "version": 1,
        "spaces": {
            "work": { "vault": missing.to_string_lossy(), "excluded": ["Dostepy", "../up"] },
            "private": { "vault": "relative/folder", "bogus": 1 },
        },
    });
    let path = tmp.path().join("settings.json");
    fs::write(&path, text.to_string()).unwrap();
    let sources = probe(&path);
    assert!(
        !missing.exists(),
        "probe must not create a configured folder"
    );
    assert_eq!(ids(&sources), ["settings"]);
    assert_eq!(sources[0].id, SourceId::Settings);
    assert_eq!(sources[0].state, SourceState::Ok);
    assert_eq!(overall(&sources, false, true), Overall::Ok);
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

    // A daily backup to restore from, written by a first run elsewhere.
    let good_dir = tmp.path().join("good");
    assert!(
        StoreHandle::new(Some(good_dir.clone()))
            .status(day())
            .is_ok()
    );
    let good = good_dir
        .join("backups")
        .join(format!("day-navigator-{}.db", day()));
    assert!(good.is_file(), "first run must leave today's backup");

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

    // Restore as AGENTS.md says (copy a backup over the file; this data dir
    // has no -wal/-shm to delete): the next status recovers.
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

#[test]
fn damaged_file_still_reports_the_backups_without_a_schema_version() {
    use day_navigator_lib::store::BackupSummary;

    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path().join("data");
    fs::create_dir_all(data.join("backups")).unwrap();
    fs::write(
        data.join("backups")
            .join(format!("day-navigator-{}.db", day())),
        "invented backup",
    )
    .unwrap();
    fs::write(data.join(DB_FILE), "invented text, not a database at all").unwrap();

    let handle = StoreHandle::new(Some(data));
    let (status, details) = store_probe(&handle, day());
    assert_eq!(kind_of(&status.state), Some("Damaged"));
    let details = details.expect("data dir is known");
    assert_eq!(details.schema_version, None);
    match details.backups {
        BackupSummary::Listed {
            kept, last, stale, ..
        } => {
            assert_eq!(kept, 1);
            assert_eq!(last.unwrap().date, day());
            assert!(!stale);
        }
        other => panic!("expected Listed, got {other:?}"),
    }
}

#[test]
fn an_open_store_reports_the_latest_schema_version_and_todays_backup() {
    use day_navigator_lib::store::{BackupSummary, schema::MIGRATIONS};

    let tmp = tempfile::tempdir().unwrap();
    let handle = StoreHandle::new(Some(tmp.path().join("data")));
    let (status, details) = store_probe(&handle, day());
    assert_eq!(status.state, SourceState::Ok);
    let details = details.unwrap();
    assert_eq!(
        details.schema_version,
        Some(MIGRATIONS.last().unwrap().version)
    );
    assert!(matches!(
        details.backups,
        BackupSummary::Listed {
            kept: 1,
            stale: false,
            ..
        }
    ));

    let (_, none) = store_probe(&StoreHandle::new(None), day());
    assert!(none.is_none());
}
