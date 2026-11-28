//! The settings dialog's read and save path end to end on a temp config dir,
//! checked through the public loaders. Invented data only.

use std::fs;

use day_navigator_lib::settings::commands::{
    FileState, SaveRequest, SaveSettingsError, SectionInput, SettingsWriteLock, read_view,
    save_section,
};
use day_navigator_lib::settings::{load_github_settings, load_gitlab_settings, load_todo_limit};

#[test]
fn a_new_file_is_built_up_section_by_section() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let lock = SettingsWriteLock::default();
    let view = read_view(&path).unwrap();
    assert_eq!(view.file, FileState::Missing);

    let mut gitlab = view.sections.gitlab.form.clone();
    gitlab.host = "gitlab.example.com".into();
    let first = SaveRequest {
        section: SectionInput::Gitlab(gitlab),
        fingerprints: view.sections.gitlab.fingerprints.clone(),
        replace_invalid: false,
    };
    let view = save_section(&path, &lock, &first).unwrap().view;
    assert_eq!(view.file, FileState::Present);

    let mut github = view.sections.github.form.clone();
    github.repos = vec!["octo-org/alpha".into()];
    let second = SaveRequest {
        section: SectionInput::Github(github),
        fingerprints: view.sections.github.fingerprints.clone(),
        replace_invalid: false,
    };
    save_section(&path, &lock, &second).unwrap();

    let host = load_gitlab_settings(&path).unwrap().unwrap().host;
    assert_eq!(host.as_str(), "gitlab.example.com");
    let repos = load_github_settings(&path).unwrap().unwrap().repos;
    assert_eq!(repos[0].as_str(), "octo-org/alpha");
    assert_eq!(load_todo_limit(&path).unwrap(), 5);

    // The first request is stale now: it still says the blocks are absent.
    let written = fs::read(&path).unwrap();
    assert!(matches!(
        save_section(&path, &lock, &first),
        Err(SaveSettingsError::Conflict { .. })
    ));
    assert_eq!(fs::read(&path).unwrap(), written);
}
