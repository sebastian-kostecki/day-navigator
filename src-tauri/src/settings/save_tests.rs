//! Reading the view and saving a section, end to end on a temp folder.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tempfile::TempDir;

use super::commands::*;
use super::*;

const MARKER: &str = "LEAK-MARKER-SETTINGS";

const FILLED: &str = r#"{
  "spaces": {"old": true},
  "version": 1,
  "gitlab": {"host": "GitLab.Example.com", "user": "Invented.User"},
  "gitlabRefresh": {"minutes": 5},
  "gitlabFilter": {
    "projects": {"include": [" Acme/Backend "], "exclude": ["acme/backend/legacy"]},
    "labels": {"exclude": ["Blocked"]},
    "milestones": {"include": ["Sprint 42"]},
    "closedDays": 7
  },
  "github": {"repos": ["octo-org/alpha", "Octo-Org/Beta"]},
  "githubRefresh": {"minutes": 14},
  "todo": {"limit": 7},
  "workTime": {"holidays": false, "dailyNormHours": 7.5},
  "reminders": {"longRunningHours": false, "notRunningAfter": "08:30"},
  "notices": {"newAssignments": false, "sourceProblems": true, "daySummary": false},
  "autostart": {"enabled": true},
  "mine": [1, 2]
}"#;

const SECTIONS: [&str; 7] = [
    "gitlab",
    "github",
    "todo",
    "workTime",
    "reminders",
    "notices",
    "autostart",
];

fn setup(text: &str) -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(&path, text).unwrap();
    (dir, path)
}

fn on_disk(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn keys(value: &Value) -> Vec<String> {
    value.as_object().unwrap().keys().cloned().collect()
}

/// The unchanged form of a section, with the fingerprints it was loaded with.
fn request(view: &SettingsView, name: &str) -> SaveRequest {
    let s = &view.sections;
    macro_rules! of {
        ($field:ident, $variant:ident) => {
            (
                SectionInput::$variant(s.$field.form.clone()),
                s.$field.fingerprints.clone(),
            )
        };
    }
    let (section, fingerprints) = match name {
        "gitlab" => of!(gitlab, Gitlab),
        "github" => of!(github, Github),
        "todo" => of!(todo, Todo),
        "workTime" => of!(work_time, WorkTime),
        "reminders" => of!(reminders, Reminders),
        "notices" => of!(notices, Notices),
        "autostart" => of!(autostart, Autostart),
        other => panic!("unknown section {other}"),
    };
    assert_eq!(section.name(), name);
    SaveRequest {
        section,
        fingerprints,
        replace_invalid: false,
    }
}

fn save(path: &Path, request: &SaveRequest) -> Result<SaveResult, SaveSettingsError> {
    save_section(path, &SettingsWriteLock::default(), request)
}

/// Loads the view, lets `edit` change the section's form and saves it.
fn save_edited(
    path: &Path,
    name: &str,
    edit: impl FnOnce(&mut SectionInput),
) -> Result<SaveResult, SaveSettingsError> {
    let mut request = request(&read_view(path).unwrap(), name);
    edit(&mut request.section);
    save(path, &request)
}

macro_rules! edit {
    ($variant:ident, |$form:ident| $body:block) => {
        |section: &mut SectionInput| {
            let SectionInput::$variant($form) = section else {
                panic!("wrong section");
            };
            $body
        }
    };
}

/// Every loader's result, in a comparable form.
fn everything(path: &Path) -> String {
    let gitlab = load_gitlab_settings(path).map(|config| {
        config.map(|c| {
            let user = c.user.map(|u| u.as_str().to_string());
            (c.host.as_str().to_string(), user)
        })
    });
    let filter = load_gitlab_filter(path).map(|f| {
        (
            (f.projects.include, f.projects.exclude),
            (f.labels.include, f.labels.exclude),
            (f.milestones.include, f.milestones.exclude),
            f.closed_days,
        )
    });
    let github = load_github_settings(path).map(|config| {
        config.map(|c| {
            let repos = c.repos.iter().map(|r| r.as_str().to_string());
            repos.collect::<Vec<_>>()
        })
    });
    let first = (
        gitlab,
        load_gitlab_refresh_minutes(path),
        filter,
        github,
        load_github_refresh_minutes(path),
    );
    let second = (
        load_todo_limit(path),
        load_work_time_settings(path),
        load_reminder_settings(path),
        load_notice_settings(path),
        load_autostart_settings(path),
    );
    format!("{first:?} {second:?}")
}

#[test]
fn an_unchanged_form_saves_to_the_same_settings_in_every_section() {
    for text in [FILLED, r#"{"version":1}"#] {
        for name in SECTIONS {
            let (_dir, path) = setup(text);
            let before = everything(&path);
            let result = save_edited(&path, name, |_| {}).unwrap();
            assert_eq!(everything(&path), before, "{name}");
            assert_eq!(result.view.file, FileState::Present);
            assert!(result.autostart.is_none());
            // The returned view is what the next load gives.
            assert_eq!(
                serde_json::to_value(&result.view).unwrap(),
                serde_json::to_value(read_view(&path).unwrap()).unwrap(),
                "{name}"
            );
        }
    }
}

#[test]
fn the_view_shows_normalised_values() {
    let (_dir, path) = setup(FILLED);
    let view = serde_json::to_value(read_view(&path).unwrap()).unwrap();
    assert_eq!(view["file"], "present");
    let sections = &view["sections"];
    assert_eq!(
        sections["gitlab"]["form"],
        json!({
            "host": "gitlab.example.com",
            "user": "Invented.User",
            "refreshMinutes": "5",
            "filter": {
                "projects": {"include": ["acme/backend"], "exclude": ["acme/backend/legacy"]},
                "labels": {"include": [], "exclude": ["blocked"]},
                "milestones": {"include": ["sprint 42"], "exclude": []},
                "closedDays": "7"
            }
        })
    );
    assert_eq!(
        sections["workTime"]["form"],
        json!({"holidays": false, "dailyNormHours": "7.5"})
    );
    assert_eq!(
        sections["reminders"]["form"],
        json!({
            "longRunningOn": false,
            "longRunningHours": "10",
            "notRunningAfterOn": true,
            "notRunningAfter": "08:30"
        })
    );
}

#[test]
fn a_missing_file_shows_the_defaults_and_is_not_created() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let view = serde_json::to_value(read_view(&path).unwrap()).unwrap();
    assert!(!path.exists());
    assert_eq!(view["file"], "missing");
    let sections = &view["sections"];
    let defaults = [
        ("github", json!({"repos": [], "refreshMinutes": "10"})),
        ("todo", json!({"limit": "5"})),
        ("workTime", json!({"holidays": true, "dailyNormHours": "8"})),
        (
            "reminders",
            json!({
                "longRunningOn": true,
                "longRunningHours": "10",
                "notRunningAfterOn": true,
                "notRunningAfter": "09:00"
            }),
        ),
        (
            "notices",
            json!({"newAssignments": true, "sourceProblems": true, "daySummary": true}),
        ),
        ("autostart", json!({"enabled": false})),
    ];
    for (name, form) in defaults {
        assert_eq!(sections[name]["form"], form, "{name}");
        assert_eq!(sections[name]["invalidBlocks"], json!([]), "{name}");
    }
    assert_eq!(
        sections["gitlab"]["fingerprints"],
        json!({"gitlab": "absent", "gitlabRefresh": "absent", "gitlabFilter": "absent"})
    );
}

#[test]
fn a_save_changes_only_its_blocks_and_keeps_the_order() {
    let (_dir, path) = setup(FILLED);
    let before = on_disk(&path);
    let edit = edit!(Todo, |form| {
        form.limit = " 3 ".into();
    });
    save_edited(&path, "todo", edit).unwrap();
    let after = on_disk(&path);
    assert_eq!(after["todo"], json!({"limit": 3}));
    assert_eq!(keys(&after), keys(&before));
    for key in keys(&before).iter().filter(|k| *k != "todo") {
        assert_eq!(after[key], before[key], "{key}");
    }
}

#[test]
fn defaults_leave_their_keys_out() {
    let (_dir, path) = setup(FILLED);
    let edit = edit!(WorkTime, |form| {
        form.holidays = true;
        form.daily_norm_hours = "7,5".into();
    });
    save_edited(&path, "workTime", edit).unwrap();
    assert_eq!(on_disk(&path)["workTime"], json!({"dailyNormHours": 7.5}));

    let edit = edit!(Reminders, |form| {
        form.long_running_on = true;
        form.long_running_hours = String::new();
        form.not_running_after_on = false;
    });
    save_edited(&path, "reminders", edit).unwrap();
    assert_eq!(
        on_disk(&path)["reminders"],
        json!({"notRunningAfter": false})
    );

    let edit = edit!(Gitlab, |form| {
        form.user = String::new();
        form.refresh_minutes = String::new();
        form.filter = FilterForm::default();
    });
    save_edited(&path, "gitlab", edit).unwrap();
    let after = on_disk(&path);
    assert_eq!(after["gitlab"], json!({"host": "gitlab.example.com"}));
    assert!(after.get("gitlabRefresh").is_none());
    assert!(after.get("gitlabFilter").is_none());
}

#[test]
fn an_empty_host_or_empty_repos_removes_the_block() {
    let (_dir, path) = setup(FILLED);
    let edit = edit!(Gitlab, |form| {
        form.host = String::new();
    });
    save_edited(&path, "gitlab", edit).unwrap();
    let edit = edit!(Github, |form| {
        form.repos.clear();
    });
    save_edited(&path, "github", edit).unwrap();
    let after = on_disk(&path);
    assert_eq!(after["githubRefresh"], json!({"minutes": 14}));
    assert_eq!(
        keys(&after),
        [
            "spaces",
            "version",
            "gitlabRefresh",
            "gitlabFilter",
            "githubRefresh",
            "todo",
            "workTime",
            "reminders",
            "notices",
            "autostart",
            "mine"
        ]
    );
}

#[test]
fn a_changed_block_is_a_conflict_and_nothing_is_written() {
    // Changed, removed and added since the view was loaded.
    for (before, after) in [
        (r#","todo":{"limit":3}"#, r#","todo":{"limit":4}"#),
        (r#","todo":{"limit":3}"#, ""),
        ("", r#","todo":{"limit":4}"#),
    ] {
        let (_dir, path) = setup(&format!(r#"{{"version":1{before}}}"#));
        let request = request(&read_view(&path).unwrap(), "todo");
        let edited = format!(r#"{{"version":1{after}}}"#);
        fs::write(&path, &edited).unwrap();
        assert_eq!(
            save(&path, &request).unwrap_err(),
            SaveSettingsError::Conflict {
                blocks: vec!["todo".to_string()]
            }
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), edited);
    }
}

#[test]
fn a_change_to_another_section_is_not_a_conflict() {
    let (_dir, path) = setup(r#"{"version":1,"todo":{"limit":3}}"#);
    let request = request(&read_view(&path).unwrap(), "todo");
    fs::write(&path, r#"{"version":1,"todo":{"limit":3},"autostart":{}}"#).unwrap();
    save(&path, &request).unwrap();
    assert_eq!(on_disk(&path)["autostart"], json!({}));
}

#[test]
fn bad_fields_are_all_reported_and_nothing_is_written() {
    let (_dir, path) = setup(FILLED);
    let edit = edit!(Gitlab, |form| {
        form.host = format!("{MARKER} x");
        form.refresh_minutes = MARKER.into();
        form.filter.closed_days = "31".into();
        form.filter.projects.include = vec!["acme/ok".into(), format!("/{MARKER}")];
    });
    let err = save_edited(&path, "gitlab", edit).unwrap_err();
    let SaveSettingsError::Invalid { fields } = &err else {
        panic!("expected Invalid, got {err:?}");
    };
    let mut places: Vec<(&str, Option<usize>)> =
        fields.iter().map(|f| (f.field.as_str(), f.index)).collect();
    places.sort();
    assert_eq!(
        places,
        [
            ("gitlab.host", None),
            ("gitlabFilter.closedDays", None),
            ("gitlabFilter.projects.include", Some(1)),
            ("gitlabRefresh.minutes", None),
        ]
    );
    assert_no_marker(&err);
    assert_eq!(fs::read_to_string(&path).unwrap(), FILLED);
}

/// The only field `err` reports.
fn only_field(err: SaveSettingsError) -> String {
    let SaveSettingsError::Invalid { mut fields } = err else {
        panic!("expected Invalid, got {err:?}");
    };
    assert_eq!(fields.len(), 1, "{fields:?}");
    fields.remove(0).field
}

#[test]
fn numbers_that_do_not_parse_or_fit_are_field_errors() {
    let (_dir, path) = setup(FILLED);
    for limit in ["abc", "5,5", "5.5", "NaN", "inf", "0", "21", "-1"] {
        let edit = edit!(Todo, |form| {
            form.limit = limit.into();
        });
        let err = save_edited(&path, "todo", edit).unwrap_err();
        assert_eq!(only_field(err), "todo.limit", "{limit}");
    }
    for hours in ["x", "7.33", "11"] {
        let edit = edit!(WorkTime, |form| {
            form.daily_norm_hours = hours.into();
        });
        let err = save_edited(&path, "workTime", edit).unwrap_err();
        assert_eq!(only_field(err), "workTime.dailyNormHours", "{hours}");
    }
    let edit = edit!(Reminders, |form| {
        form.long_running_on = true;
        form.long_running_hours = "ten".into();
    });
    let err = save_edited(&path, "reminders", edit).unwrap_err();
    assert_eq!(only_field(err), "reminders.longRunningHours");
    let edit = edit!(Github, |form| {
        form.refresh_minutes = "15".into();
    });
    let err = save_edited(&path, "github", edit).unwrap_err();
    assert_eq!(only_field(err), "githubRefresh.minutes");
    assert_eq!(fs::read_to_string(&path).unwrap(), FILLED);
}

#[test]
fn an_invalid_block_on_disk_needs_a_confirmation() {
    let text = format!(
        r#"{{"version":1,"gitlab":{{"hots":"{MARKER}"}},"gitlabFilter":"{MARKER}","todo":{{"limit":3}}}}"#
    );
    let (_dir, path) = setup(&text);
    let view = read_view(&path).unwrap();
    assert_eq!(
        view.sections.gitlab.invalid_blocks,
        ["gitlab", "gitlabFilter"]
    );
    assert_eq!(view.sections.gitlab.form.host, "");
    assert!(view.sections.todo.invalid_blocks.is_empty());

    let mut request = request(&view, "gitlab");
    let SectionInput::Gitlab(form) = &mut request.section else {
        panic!("wrong section");
    };
    form.host = "gitlab.example.com".into();
    let err = save(&path, &request).unwrap_err();
    assert_eq!(
        err,
        SaveSettingsError::InvalidOnDisk {
            blocks: vec!["gitlab".to_string(), "gitlabFilter".to_string()]
        }
    );
    assert_no_marker(&err);
    assert_eq!(fs::read_to_string(&path).unwrap(), text);

    request.replace_invalid = true;
    let result = save(&path, &request).unwrap();
    assert!(result.view.sections.gitlab.invalid_blocks.is_empty());
    let after = on_disk(&path);
    assert_eq!(after["gitlab"], json!({"host": "gitlab.example.com"}));
    assert!(after.get("gitlabFilter").is_none());
    assert_eq!(after["todo"], json!({"limit": 3}));
}

#[test]
fn a_missing_file_is_created_on_save() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    save_edited(&path, "todo", |_| {}).unwrap();
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "{\n  \"version\": 1,\n  \"todo\": {\n    \"limit\": 5\n  }\n}\n"
    );
}

#[test]
fn a_missing_config_dir_creates_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing").join("settings.json");
    let err = save_edited(&path, "todo", |_| {}).unwrap_err();
    assert_eq!(err, SaveSettingsError::ConfigDirMissing);
    assert!(!dir.path().join("missing").exists());
}

#[test]
fn a_broken_file_is_never_overwritten() {
    let (_dir, path) = setup(r#"{"version":1}"#);
    let request = request(&read_view(&path).unwrap(), "todo");
    for text in [
        "{ not json".to_string(),
        r#"{"version":2}"#.to_string(),
        r#"{"todo":{}}"#.to_string(),
        format!(r#"["{MARKER}"]"#),
    ] {
        fs::write(&path, &text).unwrap();
        assert_eq!(
            read_view(&path).unwrap_err(),
            SettingsViewError::FileInvalid,
            "{text}"
        );
        assert_eq!(
            save(&path, &request).unwrap_err(),
            SaveSettingsError::FileInvalid,
            "{text}"
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), text);
    }
}

#[test]
fn a_failed_write_is_io_and_leaves_the_file_alone() {
    let (dir, path) = setup(FILLED);
    // A folder where the temp file should go.
    fs::create_dir(dir.path().join("settings.json.tmp")).unwrap();
    let err = save_edited(&path, "todo", |_| {}).unwrap_err();
    assert_eq!(err, SaveSettingsError::Io);
    assert_eq!(fs::read_to_string(&path).unwrap(), FILLED);
}

fn assert_no_marker(error: &SaveSettingsError) {
    let json = serde_json::to_string(error).unwrap();
    let shown = format!("{error:?} {error} {json}");
    assert!(
        !shown.to_lowercase().contains(&MARKER.to_lowercase()),
        "leak: {shown}"
    );
}

#[test]
fn errors_serialize_as_kind_and_detail() {
    let blocks = vec!["todo".to_string()];
    for (error, want) in [
        (
            SaveSettingsError::ConfigDirUnresolved,
            json!({"kind": "ConfigDirUnresolved"}),
        ),
        (
            SaveSettingsError::ConfigDirMissing,
            json!({"kind": "ConfigDirMissing"}),
        ),
        (
            SaveSettingsError::FileInvalid,
            json!({"kind": "FileInvalid"}),
        ),
        (
            SaveSettingsError::Conflict {
                blocks: blocks.clone(),
            },
            json!({"kind": "Conflict", "detail": {"blocks": ["todo"]}}),
        ),
        (
            SaveSettingsError::InvalidOnDisk { blocks },
            json!({"kind": "InvalidOnDisk", "detail": {"blocks": ["todo"]}}),
        ),
        (
            SaveSettingsError::Invalid {
                fields: vec![FieldError::new("todo.limit", "Zła wartość.")],
            },
            json!({"kind": "Invalid", "detail": {"fields": [
                {"field": "todo.limit", "index": null, "message": "Zła wartość."}
            ]}}),
        ),
        (SaveSettingsError::Io, json!({"kind": "Io"})),
    ] {
        assert_eq!(serde_json::to_value(&error).unwrap(), want);
    }
    for (error, kind) in [
        (
            SettingsViewError::ConfigDirUnresolved,
            "ConfigDirUnresolved",
        ),
        (SettingsViewError::FileInvalid, "FileInvalid"),
        (SettingsViewError::Io, "Io"),
    ] {
        assert_eq!(serde_json::to_value(&error).unwrap(), json!({"kind": kind}));
    }
}

/// The command logs only `section.name()` and the error kind; a request that is
/// ever formatted still shows no repo or filter value.
#[test]
fn the_request_deserializes_from_the_ui_shape_and_hides_values_in_debug() {
    let request: SaveRequest = serde_json::from_value(json!({
        "section": {"section": "github", "form": {"repos": [MARKER], "refreshMinutes": "5"}},
        "fingerprints": {"github": "absent", "githubRefresh": "absent"},
        "replaceInvalid": true
    }))
    .unwrap();
    assert_eq!(request.section.name(), "github");
    assert!(request.replace_invalid);
    assert!(!format!("{request:?}").contains(MARKER));
}
