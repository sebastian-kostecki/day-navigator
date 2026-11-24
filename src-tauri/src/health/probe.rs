//! Turns the settings file and the other checked sources into source
//! statuses. Nothing here imports `tauri`.

use std::path::Path;

use chrono::NaiveDate;

use super::{SourceId, SourceState, SourceStatus};
use crate::github::{GithubSnapshot, GithubState};
use crate::gitlab::{GitlabSnapshot, GitlabState};
use crate::settings::{SettingsError, check_settings, load_reminder_settings};
use crate::store::{StoreError, StoreHandle};

/// One check of `settings.json` as a whole. Always one `settings` entry: `Ok`,
/// or the error that makes the file unusable.
pub fn probe(settings_path: &Path) -> Vec<SourceStatus> {
    let status = match check_settings(settings_path) {
        Ok(()) => SourceStatus {
            id: SourceId::Settings,
            label: "Ustawienia".to_string(),
            state: SourceState::Ok,
        },
        Err(e) => error_status(&e),
    };
    vec![status]
}

/// The check result when the app config dir cannot be resolved, so there is no
/// `settings.json` to read. Used for every round.
pub fn unresolved_config_dir() -> Vec<SourceStatus> {
    let e = SettingsError::SettingsInvalid {
        reason: "nie można ustalić folderu konfiguracji aplikacji".to_string(),
    };
    vec![error_status(&e)]
}

/// Status of the app data store ("Dane aplikacji"): opens it if needed, checks
/// it and runs the daily backup. The panel message names the data folder (the
/// user's own path, like the settings message); `StoreError` itself stays
/// path-free.
pub fn store_status(handle: &StoreHandle, today: NaiveDate) -> SourceStatus {
    let state = match handle.status(today) {
        Ok(()) => SourceState::Ok,
        Err(e) => SourceState::Error {
            kind: e.kind().to_string(),
            message: store_message_pl(&e, handle.dir()),
        },
    };
    SourceStatus {
        id: SourceId::AppData,
        label: "Dane aplikacji".to_string(),
        state,
    }
}

/// The GitLab source derived from the last snapshot. `None` while the first
/// round has not finished (`starting`), so the source is absent, not grey.
/// Messages come from `GitlabError::message_pl` via the snapshot, never from
/// stderr, hosts or response content.
pub fn gitlab_status(snapshot: &GitlabSnapshot) -> Option<SourceStatus> {
    let state = match &snapshot.state {
        GitlabState::Starting => return None,
        GitlabState::Ok => SourceState::Ok,
        GitlabState::NotConfigured => SourceState::NotConfigured,
        GitlabState::Unavailable { message } => SourceState::Unavailable {
            message: message.clone(),
        },
        GitlabState::Error { kind, message } => SourceState::Error {
            kind: kind.clone(),
            message: message.clone(),
        },
    };
    Some(SourceStatus {
        id: SourceId::Gitlab,
        label: "GitLab".to_string(),
        state,
    })
}

/// The GitHub source derived from the last snapshot. `None` while the first
/// round has not finished. A successful round with any failing repo is an
/// error of the fixed kind `RepoFailed`, so the status (and its transition) stays
/// stable while the set of failing repos or their kinds change. Messages never
/// carry repo names.
pub fn github_status(snapshot: &GithubSnapshot) -> Option<SourceStatus> {
    let state = match &snapshot.state {
        GithubState::Starting => return None,
        GithubState::Ok => {
            let failed = snapshot.repos.iter().filter(|r| r.error.is_some()).count();
            if failed == 0 {
                SourceState::Ok
            } else {
                SourceState::Error {
                    kind: "RepoFailed".to_string(),
                    message: format!(
                        "Nie udało się pobrać {failed} z {} repozytoriów GitHub",
                        snapshot.repos.len()
                    ),
                }
            }
        }
        GithubState::NotConfigured => SourceState::NotConfigured,
        GithubState::Unavailable { message } => SourceState::Unavailable {
            message: message.clone(),
        },
        GithubState::Error { kind, message } => SourceState::Error {
            kind: kind.clone(),
            message: message.clone(),
        },
    };
    Some(SourceStatus {
        id: SourceId::Github,
        label: "GitHub".to_string(),
        state,
    })
}

/// Fixed text for an invalid `reminders` block. Names the accepted shape and
/// never echoes the user's values. The "Przypomnienia" panel in "Czas pracy"
/// shows the same text from `src/worktime/api.ts`; a test keeps the two equal.
const REMINDERS_INVALID_PL: &str = "Blok „reminders” w settings.json jest nieprawidłowy: longRunningHours to liczba godzin 1–24 albo false, notRunningAfter to godzina „GG:MM” albo false.";

/// The "Przypomnienia" source: Ok when the `reminders` block parses (also when
/// it or the file is absent), otherwise `SettingsInvalid` with a fixed message.
pub fn reminders_status(settings_path: &Path) -> SourceStatus {
    let state = match load_reminder_settings(settings_path) {
        Ok(_) => SourceState::Ok,
        Err(_) => SourceState::Error {
            kind: "SettingsInvalid".to_string(),
            message: REMINDERS_INVALID_PL.to_string(),
        },
    };
    SourceStatus {
        id: SourceId::Reminders,
        label: "Przypomnienia".to_string(),
        state,
    }
}

/// Polish panel text for a store error. `dir` is the data folder, if known.
pub fn store_message_pl(e: &StoreError, dir: Option<&Path>) -> String {
    let backups = dir.map_or_else(
        || "backups w folderze danych aplikacji".to_string(),
        |d| d.join("backups").display().to_string(),
    );
    match e {
        StoreError::Damaged { .. } => format!(
            "Plik danych aplikacji jest uszkodzony. Zamknij aplikację i przywróć kopię z folderu {backups}."
        ),
        StoreError::Missing => format!(
            "Brak pliku danych aplikacji, choć istnieją kopie. Przywróć kopię z folderu {backups}."
        ),
        StoreError::NewerSchema { .. } => {
            "Dane aplikacji pochodzą z nowszej wersji aplikacji. Zaktualizuj aplikację.".to_string()
        }
        StoreError::MigrationFailed { version, .. } => {
            format!("Aktualizacja danych aplikacji do wersji {version} nie powiodła się.")
        }
        StoreError::DirUnavailable { .. } => match dir {
            Some(d) => format!("Folder danych aplikacji jest niedostępny: {}", d.display()),
            None => "Nie można ustalić folderu danych aplikacji.".to_string(),
        },
        StoreError::Busy => "Plik danych aplikacji jest chwilowo zajęty.".to_string(),
        StoreError::Io { kind } => format!("Błąd odczytu danych aplikacji: {kind}"),
        StoreError::Sql { .. } => "Błąd bazy danych aplikacji.".to_string(),
    }
}

/// The "Ustawienia" source for a settings file that cannot be used.
fn error_status(e: &SettingsError) -> SourceStatus {
    SourceStatus {
        id: SourceId::Settings,
        label: "Ustawienia".to_string(),
        state: SourceState::Error {
            kind: e.kind().to_string(),
            message: message_pl(e),
        },
    }
}

/// Polish panel text for a settings error. May include the user's own settings
/// path.
pub fn message_pl(e: &SettingsError) -> String {
    match e {
        SettingsError::NotConfigured { settings_path } => {
            format!("Brak pliku ustawień: {}", settings_path.display())
        }
        SettingsError::SettingsInvalid { reason } => {
            format!("Ustawienia są nieprawidłowe: {reason}")
        }
        SettingsError::Io { path, kind } => {
            if path.is_empty() {
                format!("Błąd odczytu: {kind}")
            } else {
                format!("Błąd odczytu ({path}): {kind}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn message_pl_covers_every_kind() {
        let m = message_pl(&SettingsError::NotConfigured {
            settings_path: PathBuf::from("X:/s.json"),
        });
        assert!(m.starts_with("Brak pliku ustawień: "));
        let m = message_pl(&SettingsError::SettingsInvalid {
            reason: "zły".into(),
        });
        assert_eq!(m, "Ustawienia są nieprawidłowe: zły");
        let m = message_pl(&SettingsError::Io {
            path: "settings.json".into(),
            kind: "PermissionDenied".into(),
        });
        assert_eq!(m, "Błąd odczytu (settings.json): PermissionDenied");
        let m = message_pl(&SettingsError::Io {
            path: String::new(),
            kind: "PermissionDenied".into(),
        });
        assert_eq!(m, "Błąd odczytu: PermissionDenied");
    }

    #[test]
    fn unresolved_config_dir_is_a_settings_error() {
        let sources = unresolved_config_dir();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].id, SourceId::Settings);
        match &sources[0].state {
            SourceState::Error { kind, message } => {
                assert_eq!(kind, "SettingsInvalid");
                assert!(message.starts_with("Ustawienia są nieprawidłowe: "));
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn gitlab_snapshot_maps_to_a_source() {
        use crate::gitlab::GitlabError;
        let snap = |state: GitlabState| GitlabSnapshot {
            state,
            ..GitlabSnapshot::starting()
        };
        assert_eq!(gitlab_status(&GitlabSnapshot::starting()), None);

        let s = gitlab_status(&snap(GitlabState::Ok)).unwrap();
        assert_eq!(s.id, SourceId::Gitlab);
        assert_eq!(s.label, "GitLab");
        assert_eq!(s.state, SourceState::Ok);

        let s = gitlab_status(&snap(GitlabState::NotConfigured)).unwrap();
        assert_eq!(s.state, SourceState::NotConfigured);

        let message = GitlabError::Unavailable.message_pl();
        let s = gitlab_status(&snap(GitlabState::Unavailable {
            message: message.clone(),
        }))
        .unwrap();
        assert_eq!(s.state, SourceState::Unavailable { message });

        let message = GitlabError::LoggedOut.message_pl();
        let s = gitlab_status(&snap(GitlabState::Error {
            kind: "LoggedOut".to_string(),
            message: message.clone(),
        }))
        .unwrap();
        assert_eq!(
            s.state,
            SourceState::Error {
                kind: "LoggedOut".to_string(),
                message
            }
        );
    }

    #[test]
    fn github_snapshot_maps_to_a_source() {
        use crate::github::{GithubError, RepoErrorView, RepoView};
        let snap = |state: GithubState| GithubSnapshot {
            state,
            ..GithubSnapshot::starting()
        };
        assert_eq!(github_status(&GithubSnapshot::starting()), None);

        let s = github_status(&snap(GithubState::Ok)).unwrap();
        assert_eq!(s.id, SourceId::Github);
        assert_eq!(s.label, "GitHub");
        assert_eq!(s.state, SourceState::Ok);

        let s = github_status(&snap(GithubState::NotConfigured)).unwrap();
        assert_eq!(s.state, SourceState::NotConfigured);

        let message = GithubError::Unavailable.message_pl();
        let s = github_status(&snap(GithubState::Unavailable {
            message: message.clone(),
        }))
        .unwrap();
        assert_eq!(s.state, SourceState::Unavailable { message });

        let message = GithubError::LoggedOut.message_pl();
        let s = github_status(&snap(GithubState::Error {
            kind: "LoggedOut".to_string(),
            message: message.clone(),
        }))
        .unwrap();
        assert_eq!(
            s.state,
            SourceState::Error {
                kind: "LoggedOut".to_string(),
                message
            }
        );

        // A repo error under an `ok` round is red with a fixed kind and a
        // message without repo names, whatever the per-repo kind.
        let repo = |name: &str, kind: Option<&str>| RepoView {
            repo: name.to_string(),
            issues: vec![],
            error: kind.map(|k| RepoErrorView {
                kind: k.to_string(),
                message: "m".to_string(),
            }),
        };
        let with = |repos: Vec<RepoView>| GithubSnapshot {
            repos,
            ..snap(GithubState::Ok)
        };
        let all_ok = with(vec![repo("octo-test/alpha", None)]);
        assert_eq!(github_status(&all_ok).unwrap().state, SourceState::Ok);
        let one = with(vec![
            repo("octo-test/alpha", Some("Failed")),
            repo("octo-test/beta", None),
            repo("octo-test/gamma", None),
        ]);
        let other_kind = with(vec![
            repo("octo-test/alpha", None),
            repo("octo-test/beta", Some("Truncated")),
            repo("octo-test/gamma", None),
        ]);
        let expected = SourceState::Error {
            kind: "RepoFailed".to_string(),
            message: "Nie udało się pobrać 1 z 3 repozytoriów GitHub".to_string(),
        };
        assert_eq!(github_status(&one).unwrap().state, expected);
        assert_eq!(github_status(&other_kind).unwrap().state, expected);
    }

    #[test]
    fn reminders_status_follows_the_block() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("settings.json");
        // No file yet: defaults.
        let s = reminders_status(&path);
        assert_eq!(s.id, SourceId::Reminders);
        assert_eq!(s.label, "Przypomnienia");
        assert_eq!(s.state, SourceState::Ok);

        std::fs::write(&path, r#"{"version":1,"spaces":{}}"#).unwrap();
        assert_eq!(reminders_status(&path).state, SourceState::Ok);

        std::fs::write(
            &path,
            r#"{"version":1,"spaces":{},"reminders":{"longRunningHours":2,"notRunningAfter":false}}"#,
        )
        .unwrap();
        assert_eq!(reminders_status(&path).state, SourceState::Ok);

        std::fs::write(
            &path,
            r#"{"version":1,"spaces":{},"reminders":{"longRunningHours":99999,"notRunningAfter":"Sekret-Testowy"}}"#,
        )
        .unwrap();
        let bad = reminders_status(&path);
        assert_eq!(
            bad.state,
            SourceState::Error {
                kind: "SettingsInvalid".to_string(),
                message: REMINDERS_INVALID_PL.to_string(),
            }
        );
        assert!(!REMINDERS_INVALID_PL.contains("99999"));
    }

    #[test]
    fn reminders_invalid_text_matches_the_worktime_panel() {
        let api_ts = include_str!("../../../src/worktime/api.ts");
        assert!(
            api_ts.contains(&format!("\"{REMINDERS_INVALID_PL}\"")),
            "src/worktime/api.ts must carry REMINDERS_INVALID_PL verbatim"
        );
    }

    #[test]
    fn kind_is_the_variant_name_and_the_serde_tag() {
        let cases = [
            (
                SettingsError::NotConfigured {
                    settings_path: PathBuf::from("X:/s.json"),
                },
                "NotConfigured",
            ),
            (
                SettingsError::SettingsInvalid { reason: "x".into() },
                "SettingsInvalid",
            ),
            (
                SettingsError::Io {
                    path: "settings.json".into(),
                    kind: "PermissionDenied".into(),
                },
                "Io",
            ),
        ];
        for (e, name) in cases {
            assert_eq!(e.kind(), name);
            // The wire tag the UI reads must not drift from `kind()`.
            let v = serde_json::to_value(&e).unwrap();
            assert_eq!(v["kind"], name);
        }
    }
}
