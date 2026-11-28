//! Drives `fetch_login` and `fetch_issue_comments` through a real process
//! (`fake-cli`, invented data).

use std::path::PathBuf;
use std::time::Duration;

use day_navigator_lib::cli::{CliRunner, GithubIssueNumber, GithubRepo};
use day_navigator_lib::github::GithubError;
use day_navigator_lib::github::fetch::{fetch_issue_comments, fetch_login};

const COMMENT_MARKER: &str = "LEAK-MARKER-GH-COMMENT-8d3a";
const LOGIN_MARKER: &str = "LEAK-MARKER-GH-LOGIN-6b1c";

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

fn repo() -> GithubRepo {
    GithubRepo::parse("octo-test/alpha").unwrap()
}

fn number() -> GithubIssueNumber {
    GithubIssueNumber::new(3).unwrap()
}

fn comments(
    scenario: &str,
) -> Result<Vec<day_navigator_lib::github::comments::GithubComment>, GithubError> {
    fetch_issue_comments(&runner(scenario), &repo(), number())
}

#[test]
fn login_comes_from_api_user() {
    let login = fetch_login(&runner("gh-hidden-none")).expect("login");
    assert_eq!(login.as_str(), LOGIN_MARKER);
    assert!(!format!("{login:?}").contains("LEAK"));
}

#[test]
fn a_failing_login_call_is_failed_without_content() {
    let err = fetch_login(&runner("gh-hidden-user-fails")).unwrap_err();
    assert_eq!(err, GithubError::Failed { exit_code: Some(2) });
    assert!(!format!("{err:?} {err} {}", err.message_pl()).contains("LEAK"));
}

#[test]
fn no_comments_is_an_empty_list() {
    assert_eq!(comments("gh-hidden-none"), Ok(vec![]));
}

#[test]
fn another_authors_comment_is_read() {
    let got = comments("gh-hidden-other").expect("comments");
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].author.as_deref(), Some("octo-other-4e7a"));
    let dbg = format!("{got:?}");
    assert!(!dbg.contains("octo-other"));
    assert!(!dbg.contains(COMMENT_MARKER));
}

#[test]
fn own_comment_keeps_the_login_in_another_letter_case() {
    let got = comments("gh-hidden-own").expect("comments");
    assert_eq!(got.len(), 1);
    let author = got[0].author.as_deref().unwrap();
    assert_ne!(author, LOGIN_MARKER);
    assert!(author.eq_ignore_ascii_case(LOGIN_MARKER));
}

#[test]
fn ghost_and_empty_login_authors_are_none() {
    let got = comments("gh-hidden-ghost").expect("comments");
    assert_eq!(got.len(), 2);
    assert!(got.iter().all(|c| c.author.is_none()));
}

#[test]
fn an_old_comment_keeps_its_time() {
    let got = comments("gh-hidden-old").expect("comments");
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].created_at.to_rfc3339(), "2026-01-05T09:00:00+00:00");
}

#[test]
fn garbled_json_is_response_invalid_with_no_leak() {
    let err = comments("gh-hidden-bad-json").unwrap_err();
    assert_eq!(err, GithubError::ResponseInvalid);
    let text = format!("{err:?} {err} {}", err.message_pl());
    assert!(!text.contains(COMMENT_MARKER));
    assert!(!text.contains("LEAK"));
}

#[test]
fn an_unreachable_server_is_unavailable() {
    assert_eq!(
        comments("gh-hidden-view-unavailable"),
        Err(GithubError::Unavailable)
    );
}

#[test]
fn an_unknown_route_fails_without_content() {
    let err = fetch_issue_comments(
        &runner("gh-issues"),
        &repo(),
        GithubIssueNumber::new(3).unwrap(),
    )
    .unwrap_err();
    assert!(matches!(err, GithubError::Failed { .. }), "{err:?}");
}
