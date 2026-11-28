//! The field errors of every block validator: where they point, that all bad
//! fields are reported, and that none repeats the offending value.

use serde_json::{Value, json};

use super::autostart::validate_autostart;
use super::work_time::validate_work_time;
use super::*;

const MARKER: &str = "LEAK-MARKER-SETTINGS";

/// Field path and list index of every error, sorted. Also checks that no error
/// carries the marker (in any case: filter entries are lowercased).
fn places<T>(result: Result<T, Vec<FieldError>>) -> Vec<(String, Option<usize>)> {
    let Err(errors) = result else {
        panic!("expected field errors");
    };
    for error in &errors {
        assert!(!error.message.is_empty());
        let shown = format!("{error:?} {}", serde_json::to_string(error).unwrap());
        assert!(
            !shown.to_lowercase().contains(&MARKER.to_lowercase()),
            "leak: {shown}"
        );
    }
    let mut out: Vec<_> = errors.into_iter().map(|e| (e.field, e.index)).collect();
    out.sort();
    out
}

fn at(field: &str) -> (String, Option<usize>) {
    (field.to_string(), None)
}

fn entry(field: &str, index: usize) -> (String, Option<usize>) {
    (field.to_string(), Some(index))
}

/// Every validator, run on one block, with only its errors kept.
fn all_validators(block: &Value) -> Vec<(&'static str, Result<(), Vec<FieldError>>)> {
    vec![
        ("gitlab", validate_gitlab(block).map(drop)),
        ("github", validate_github(block).map(drop)),
        (
            "gitlabRefresh",
            validate_refresh(block, "gitlabRefresh").map(drop),
        ),
        (
            "githubRefresh",
            validate_refresh(block, "githubRefresh").map(drop),
        ),
        ("gitlabFilter", validate_gitlab_filter(block).map(drop)),
        ("todo", validate_todo(block).map(drop)),
        ("reminders", validate_reminders(block).map(drop)),
        ("workTime", validate_work_time(block).map(drop)),
        ("autostart", validate_autostart(block).map(drop)),
        ("notices", validate_notices(block).map(drop)),
    ]
}

#[test]
fn a_non_object_block_is_one_error_on_the_block_key() {
    for block in [json!(MARKER), json!([MARKER]), json!(5), json!(true)] {
        for (key, result) in all_validators(&block) {
            assert_eq!(places(result), [at(key)], "{key}");
        }
    }
}

#[test]
fn an_unknown_key_is_one_error_on_the_block_key() {
    let block = json!({ MARKER: MARKER, "other": 1 });
    for (key, result) in all_validators(&block) {
        assert_eq!(places(result), [at(key)], "{key}");
    }
}

#[test]
fn a_null_block_is_valid_everywhere() {
    for (key, result) in all_validators(&Value::Null) {
        assert!(result.is_ok(), "{key}");
    }
}

#[test]
fn gitlab_reports_host_and_user_together() {
    let bad = format!("{MARKER} x");
    let block = json!({"host": bad, "user": bad});
    assert_eq!(
        places(validate_gitlab(&block)),
        [at("gitlab.host"), at("gitlab.user")]
    );
    assert_eq!(places(validate_gitlab(&json!({}))), [at("gitlab.host")]);
}

#[test]
fn github_reports_every_bad_entry_with_its_index() {
    let block = json!({"repos": ["octo-org/alpha", MARKER, 5, "octo-org/beta", "a/b/c"]});
    assert_eq!(
        places(validate_github(&block)),
        [
            entry("github.repos", 1),
            entry("github.repos", 2),
            entry("github.repos", 4)
        ]
    );
    for repos in [json!(MARKER), Value::Null] {
        assert_eq!(
            places(validate_github(&json!({ "repos": repos }))),
            [at("github.repos")]
        );
    }
    let mut many: Vec<String> = (0..51).map(|i| format!("octo-org/r{i}")).collect();
    many.push(MARKER.to_string());
    assert_eq!(
        places(validate_github(&json!({ "repos": many }))),
        [at("github.repos"), entry("github.repos", 51)]
    );
}

#[test]
fn refresh_reports_the_minutes_of_its_own_block() {
    for key in ["gitlabRefresh", "githubRefresh"] {
        assert_eq!(
            places(validate_refresh(&json!({"minutes": MARKER}), key)),
            [at(&format!("{key}.minutes"))]
        );
    }
}

#[test]
fn filter_reports_every_bad_field_and_entry() {
    let long = format!("{MARKER}{}", "a".repeat(300));
    let block = json!({
        "projects": {
            "include": ["acme/backend", "acme/api", "acme/web", format!("/{MARKER}")],
            "exclude": MARKER,
        },
        "labels": {
            "include": [long, "ok", format!("{MARKER}\u{7}")],
            "exclude": ["  ", 7],
        },
        "milestones": MARKER,
        "closedDays": MARKER,
    });
    assert_eq!(
        places(validate_gitlab_filter(&block)),
        [
            at("gitlabFilter.closedDays"),
            entry("gitlabFilter.labels.exclude", 0),
            entry("gitlabFilter.labels.exclude", 1),
            entry("gitlabFilter.labels.include", 0),
            entry("gitlabFilter.labels.include", 2),
            at("gitlabFilter.milestones"),
            at("gitlabFilter.projects.exclude"),
            entry("gitlabFilter.projects.include", 3),
        ]
    );
}

#[test]
fn filter_reports_a_dimension_with_an_unknown_key_and_a_long_list() {
    let block = json!({
        "labels": { MARKER: [] },
        "milestones": {"exclude": vec!["m"; 101]},
    });
    assert_eq!(
        places(validate_gitlab_filter(&block)),
        [
            at("gitlabFilter.labels"),
            at("gitlabFilter.milestones.exclude")
        ]
    );
}

#[test]
fn todo_reports_the_limit() {
    for limit in [json!(MARKER), json!(0), json!(21)] {
        assert_eq!(
            places(validate_todo(&json!({ "limit": limit }))),
            [at("todo.limit")]
        );
    }
}

#[test]
fn reminders_report_both_fields() {
    let block = json!({"longRunningHours": MARKER, "notRunningAfter": MARKER});
    assert_eq!(
        places(validate_reminders(&block)),
        [
            at("reminders.longRunningHours"),
            at("reminders.notRunningAfter")
        ]
    );
}

#[test]
fn work_time_reports_both_fields() {
    let block = json!({"holidays": MARKER, "dailyNormHours": MARKER});
    assert_eq!(
        places(validate_work_time(&block)),
        [at("workTime.dailyNormHours"), at("workTime.holidays")]
    );
}

#[test]
fn autostart_reports_enabled() {
    assert_eq!(
        places(validate_autostart(&json!({"enabled": MARKER}))),
        [at("autostart.enabled")]
    );
}

#[test]
fn notices_report_every_field() {
    let block = json!({"newAssignments": MARKER, "sourceProblems": 0, "daySummary": [MARKER]});
    assert_eq!(
        places(validate_notices(&block)),
        [
            at("notices.daySummary"),
            at("notices.newAssignments"),
            at("notices.sourceProblems")
        ]
    );
}

#[test]
fn field_error_serializes_in_camel_case() {
    let error = FieldError::at("github.repos", 2, "Zła pozycja.");
    assert_eq!(
        serde_json::to_value(&error).unwrap(),
        json!({"field": "github.repos", "index": 2, "message": "Zła pozycja."})
    );
}
