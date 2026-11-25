//! Drives `fetch_assigned` through a real process (`fake-cli`, invented data).

use std::path::PathBuf;
use std::time::Duration;

use chrono::{DateTime, FixedOffset, NaiveDate};
use day_navigator_lib::cli::{
    CliRunner, GitlabConfig, GitlabHost, GitlabUser, IssueRef, UpdatedAfter,
};
use day_navigator_lib::gitlab::fetch::{fetch_assigned, fetch_issue_notes, fetch_recently_closed};
use day_navigator_lib::gitlab::notes::IssueNote;
use day_navigator_lib::gitlab::{GitlabError, GitlabLabel, ItemKind};

const MARKER: &str = "LEAK-MARKER-TITLE-7c1e";

fn fake() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_fake-cli"))
}

fn runner(scenario: &str) -> CliRunner {
    CliRunner::with_programs(
        fake(),
        fake(),
        Duration::from_secs(10),
        vec![("FAKE_CLI_SCENARIO".to_string(), scenario.to_string())],
    )
}

fn host() -> GitlabConfig {
    GitlabConfig {
        host: GitlabHost::parse("gitlab.example.invalid").unwrap(),
        user: None,
    }
}

fn refs(items: &[day_navigator_lib::gitlab::GitlabItem]) -> Vec<&str> {
    items.iter().map(|i| i.reference.as_str()).collect()
}

#[test]
fn assigned_data_is_parsed_sorted_and_foreign_urls_are_dropped() {
    let got = fetch_assigned(&runner("gitlab-assigned"), &host()).expect("data");

    // Newest first; the tie on updated_at is ordered by reference.
    assert_eq!(
        refs(&got.merge_requests),
        ["acme/widgets!12", "acme/widgets!34", "acme/gizmo!7"]
    );
    assert_eq!(refs(&got.issues), ["acme/widgets#12", "acme/gizmo#5"]);

    assert!(
        got.merge_requests
            .iter()
            .all(|i| i.kind == ItemKind::MergeRequest)
    );
    assert!(got.issues.iter().all(|i| i.kind == ItemKind::Issue));

    let widget = &got.merge_requests[1];
    assert_eq!(widget.title, "Add widget parser");
    assert!(widget.draft);
    // Bare-name labels (an older response shape) carry no colours.
    assert_eq!(
        widget.labels,
        ["backend", "review"].map(|name| GitlabLabel {
            name: name.to_string(),
            color: None,
            text_color: None,
        })
    );
    assert_eq!(
        widget.web_url.as_deref(),
        Some("https://gitlab.example.invalid/acme/widgets/-/merge_requests/34")
    );
    assert!(!got.merge_requests[0].draft);

    // Detailed labels: validated lowercase colours, or none when one is invalid.
    let gizmo = &got.issues[1];
    assert_eq!(
        gizmo.labels,
        [
            GitlabLabel {
                name: "bug".into(),
                color: Some("#d9534f".into()),
                text_color: Some("#ffffff".into()),
            },
            GitlabLabel {
                name: "workflow::doing".into(),
                color: Some("#5843ad".into()),
                text_color: Some("#ffffff".into()),
            },
        ]
    );
    assert_eq!(
        got.issues[0].labels,
        [GitlabLabel {
            name: "needs-triage".into(),
            color: None,
            text_color: None,
        }]
    );

    // The link on another host is not kept.
    assert!(got.merge_requests[2].web_url.is_none());
    assert!(got.issues.iter().all(|i| i.web_url.is_some()));
}

#[test]
fn empty_arrays_give_two_empty_lists() {
    let got = fetch_assigned(&runner("gitlab-empty"), &host()).expect("data");
    assert!(got.merge_requests.is_empty());
    assert!(got.issues.is_empty());
}

#[test]
fn bad_json_is_response_invalid_and_leaks_nothing() {
    let err = fetch_assigned(&runner("gitlab-bad-json"), &host()).unwrap_err();
    assert_eq!(err, GitlabError::ResponseInvalid);
    let shown = [
        format!("{err:?}"),
        err.to_string(),
        serde_json::to_string(&err).unwrap(),
        err.message_pl(),
    ];
    for text in shown {
        assert!(!text.contains(MARKER), "{text}");
        assert!(!text.contains("example.invalid"), "{text}");
    }
}

#[test]
fn failures_map_to_their_gitlab_errors() {
    let cases = [
        ("glab-no-network", GitlabError::Unavailable),
        ("glab-logged-out", GitlabError::LoggedOut),
        ("unknown-error", GitlabError::Failed { exit_code: Some(2) }),
    ];
    for (scenario, expected) in cases {
        assert_eq!(
            fetch_assigned(&runner(scenario), &host()),
            Err(expected),
            "{scenario}"
        );
    }
}

#[test]
fn second_call_failure_is_unavailable_with_no_partial_result() {
    let got = fetch_assigned(&runner("gitlab-mrs-ok-issues-no-network"), &host());
    assert_eq!(got, Err(GitlabError::Unavailable));
}

#[test]
fn missing_program_is_tool_missing() {
    let dir = tempfile::tempdir().unwrap();
    let r = CliRunner::with_programs(
        dir.path().join("no-glab"),
        dir.path().join("no-gh"),
        Duration::from_secs(5),
        Vec::new(),
    );
    assert_eq!(fetch_assigned(&r, &host()), Err(GitlabError::ToolMissing));
}

fn host_with_user() -> GitlabConfig {
    GitlabConfig {
        user: Some(GitlabUser::parse("invented.user").unwrap()),
        ..host()
    }
}

#[test]
fn with_a_user_both_endpoint_forms_are_served_and_parsed() {
    let with = fetch_assigned(&runner("gitlab-assigned"), &host_with_user()).expect("data");
    let without = fetch_assigned(&runner("gitlab-assigned"), &host()).expect("data");
    assert_eq!(with, without);
    assert_eq!(refs(&with.issues), ["acme/widgets#12", "acme/gizmo#5"]);
}

#[test]
fn with_a_user_failures_never_show_the_user_or_host() {
    let cases = [
        "gitlab-bad-json",
        "glab-no-network",
        "glab-logged-out",
        "unknown-error",
        "gitlab-mrs-ok-issues-no-network",
    ];
    for scenario in cases {
        let err = fetch_assigned(&runner(scenario), &host_with_user()).unwrap_err();
        let shown = [
            format!("{err:?}"),
            err.to_string(),
            serde_json::to_string(&err).unwrap(),
            err.message_pl(),
        ];
        for text in shown {
            assert!(!text.contains("invented"), "{scenario}: {text}");
            assert!(!text.contains("example.invalid"), "{scenario}: {text}");
        }
    }
}

// ----------------------------------------------------------- recently closed

fn closed_cutoff() -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339("2026-03-01T00:00:00Z").unwrap()
}

fn after() -> UpdatedAfter {
    UpdatedAfter::new(NaiveDate::from_ymd_opt(2026, 2, 28).unwrap())
}

fn closed(
    scenario: &str,
    config: &GitlabConfig,
) -> Result<Vec<day_navigator_lib::gitlab::ClosedItem>, GitlabError> {
    fetch_recently_closed(&runner(scenario), config, after(), closed_cutoff())
}

#[test]
fn recently_closed_returns_the_expected_items_newest_closing_first() {
    let closed_items = closed("gitlab-assigned", &host()).expect("data");
    // Each item carries the closing instant that ordered it.
    let times: Vec<DateTime<FixedOffset>> = closed_items
        .iter()
        .map(|c| DateTime::parse_from_rfc3339(&c.closed_at).unwrap())
        .collect();
    assert!(times.windows(2).all(|w| w[0] >= w[1]));
    assert!(times.iter().all(|t| *t >= closed_cutoff()));
    let got: Vec<_> = closed_items.into_iter().map(|c| c.item).collect();
    // The open MR and the issue closed before the cutoff are dropped.
    assert_eq!(
        refs(&got),
        ["acme/widgets#20", "acme/widgets!40", "acme/gizmo!41"]
    );
    assert_eq!(got[0].kind, ItemKind::Issue);
    assert_eq!(got[1].kind, ItemKind::MergeRequest);
    assert_eq!(got[1].title, "Merge widget cache");
    assert!(got.iter().all(|i| i.web_url.is_some()));
}

#[test]
fn recently_closed_with_a_user_is_served_the_same() {
    let with = closed("gitlab-assigned", &host_with_user()).expect("data");
    let without = closed("gitlab-assigned", &host()).expect("data");
    assert_eq!(with, without);
}

#[test]
fn recently_closed_failures_classify_like_the_open_calls() {
    let cases = [
        ("glab-no-network", GitlabError::Unavailable),
        ("glab-logged-out", GitlabError::LoggedOut),
        ("unknown-error", GitlabError::Failed { exit_code: Some(2) }),
        ("gitlab-closed-issues-no-network", GitlabError::Unavailable),
        ("gitlab-bad-json", GitlabError::ResponseInvalid),
    ];
    for (scenario, expected) in cases {
        for config in [host(), host_with_user()] {
            let err = closed(scenario, &config).unwrap_err();
            assert_eq!(err, expected, "{scenario}");
            let shown = [
                format!("{err:?}"),
                err.to_string(),
                serde_json::to_string(&err).unwrap(),
                err.message_pl(),
            ];
            for text in shown {
                assert!(!text.contains(MARKER), "{scenario}: {text}");
                assert!(!text.contains("invented"), "{scenario}: {text}");
                assert!(!text.contains("example.invalid"), "{scenario}: {text}");
            }
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let r = CliRunner::with_programs(
        dir.path().join("no-glab"),
        dir.path().join("no-gh"),
        Duration::from_secs(5),
        Vec::new(),
    );
    assert_eq!(
        fetch_recently_closed(&r, &host(), after(), closed_cutoff()),
        Err(GitlabError::ToolMissing)
    );
}

// ------------------------------------------------------------------ notes

fn gitlab_host() -> GitlabHost {
    GitlabHost::parse("gitlab.example.invalid").unwrap()
}

/// The issue that has notes in the fake GitLab (project 102, iid 12).
const WITH_NOTES: IssueRef = IssueRef {
    project_id: 102,
    iid: 12,
};

fn notes(scenario: &str, issue: IssueRef) -> Result<Vec<IssueNote>, GitlabError> {
    fetch_issue_notes(&runner(scenario), &gitlab_host(), issue)
}

fn authors(notes: &[IssueNote]) -> Vec<&str> {
    notes.iter().map(|n| n.author.as_str()).collect()
}

#[test]
fn assigned_items_carry_their_ids() {
    let got = fetch_assigned(&runner("gitlab-assigned"), &host()).expect("data");
    let widget = got
        .issues
        .iter()
        .find(|i| i.reference == "acme/widgets#12")
        .unwrap();
    assert_eq!((widget.id, widget.project_id, widget.iid), (5002, 102, 12));
    let mr = got
        .merge_requests
        .iter()
        .find(|i| i.reference == "acme/widgets!34")
        .unwrap();
    assert_eq!((mr.id, mr.project_id, mr.iid), (6002, 102, 34));
}

#[test]
fn notes_by_another_user_are_parsed_newest_first() {
    let got = notes("gitlab-notes-other", WITH_NOTES).expect("notes");
    assert_eq!(authors(&got), ["other.invented", "invented.user"]);
    assert!(got.iter().all(|n| !n.system));
    assert!(got[0].created_at > got[1].created_at);
}

#[test]
fn own_comments_system_notes_and_no_notes_are_all_readable() {
    let own = notes("gitlab-notes-own", WITH_NOTES).expect("notes");
    assert_eq!(authors(&own), ["Invented.User", "invented.user"]);
    let system = notes("gitlab-notes-system", WITH_NOTES).expect("notes");
    assert_eq!(system.len(), 1);
    assert!(system[0].system);
    assert!(notes("gitlab-notes-none", WITH_NOTES).unwrap().is_empty());
}

#[test]
fn any_other_issue_gets_an_empty_array() {
    let other = IssueRef {
        project_id: 101,
        iid: 5,
    };
    for scenario in ["gitlab-notes-other", "gitlab-notes-own", "gitlab-two-pages"] {
        assert!(notes(scenario, other).unwrap().is_empty(), "{scenario}");
    }
}

#[test]
fn two_concatenated_notes_pages_are_read_in_full() {
    let got = notes("gitlab-two-pages", WITH_NOTES).expect("notes");
    assert_eq!(authors(&got), ["invented.user", "other.invented"]);
}

#[test]
fn two_concatenated_issue_pages_are_read_in_full() {
    let got = fetch_assigned(&runner("gitlab-two-pages"), &host()).expect("data");
    assert_eq!(refs(&got.issues), ["acme/widgets#12", "acme/gizmo#5"]);
    assert_eq!(got.merge_requests.len(), 3);
}

#[test]
fn the_list_succeeds_while_notes_are_unavailable() {
    let r = runner("gitlab-notes-unavailable");
    assert!(fetch_assigned(&r, &host()).is_ok());
    assert_eq!(
        fetch_issue_notes(&r, &gitlab_host(), WITH_NOTES),
        Err(GitlabError::Unavailable)
    );
}

#[test]
fn bad_notes_json_is_response_invalid_and_leaks_nothing() {
    let err = notes("gitlab-notes-bad-json", WITH_NOTES).unwrap_err();
    assert_eq!(err, GitlabError::ResponseInvalid);
    let shown = [
        format!("{err:?}"),
        err.to_string(),
        serde_json::to_string(&err).unwrap(),
        err.message_pl(),
    ];
    for text in shown {
        assert!(!text.contains("LEAK-MARKER-NOTE-4b2d"), "{text}");
        assert!(!text.contains("example.invalid"), "{text}");
    }
}

#[test]
fn notes_failures_map_to_their_gitlab_errors() {
    let cases = [
        ("glab-no-network", GitlabError::Unavailable),
        ("glab-logged-out", GitlabError::LoggedOut),
        ("unknown-error", GitlabError::Failed { exit_code: Some(2) }),
    ];
    for (scenario, expected) in cases {
        assert_eq!(notes(scenario, WITH_NOTES), Err(expected), "{scenario}");
    }
    let dir = tempfile::tempdir().unwrap();
    let r = CliRunner::with_programs(
        dir.path().join("no-glab"),
        dir.path().join("no-gh"),
        Duration::from_secs(5),
        Vec::new(),
    );
    assert_eq!(
        fetch_issue_notes(&r, &gitlab_host(), WITH_NOTES),
        Err(GitlabError::ToolMissing)
    );
}
