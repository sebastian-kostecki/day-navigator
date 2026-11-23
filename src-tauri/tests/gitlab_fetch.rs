//! Drives `fetch_assigned` through a real process (`fake-cli`, invented data).

use std::path::PathBuf;
use std::time::Duration;

use day_navigator_lib::cli::{CliRunner, GitlabHost};
use day_navigator_lib::gitlab::fetch::fetch_assigned;
use day_navigator_lib::gitlab::{GitlabError, ItemKind};

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

fn host() -> GitlabHost {
    GitlabHost::parse("gitlab.example.invalid").unwrap()
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
    assert_eq!(widget.labels, ["backend", "review"]);
    assert_eq!(
        widget.web_url.as_deref(),
        Some("https://gitlab.example.invalid/acme/widgets/-/merge_requests/34")
    );
    assert!(!got.merge_requests[0].draft);

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
