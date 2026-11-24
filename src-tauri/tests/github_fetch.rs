//! Drives `fetch_assigned` through a real process (`fake-cli`, invented data).

use std::path::PathBuf;
use std::time::Duration;

use day_navigator_lib::cli::{CliRunner, GithubConfig, GithubRepo};
use day_navigator_lib::github::GithubError;
use day_navigator_lib::github::fetch::{RepoResult, fetch_assigned};

const MARKER: &str = "LEAK-MARKER-TITLE-7c1e";

fn fake() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_fake-cli"))
}

fn runner_with(scenario: &str, extra: Vec<(String, String)>) -> CliRunner {
    let mut env = vec![("FAKE_CLI_SCENARIO".to_string(), scenario.to_string())];
    env.extend(extra);
    CliRunner::with_programs(fake(), fake(), Duration::from_secs(10), env)
}

fn runner(scenario: &str) -> CliRunner {
    runner_with(scenario, Vec::new())
}

fn config(repos: &[&str]) -> GithubConfig {
    GithubConfig {
        repos: repos
            .iter()
            .map(|r| GithubRepo::parse(r).unwrap())
            .collect(),
    }
}

fn outcome<'a>(
    results: &'a [RepoResult],
    repo: &str,
) -> &'a Result<Vec<day_navigator_lib::github::GithubIssue>, GithubError> {
    &results
        .iter()
        .find(|r| r.repo.as_str() == repo)
        .expect("repo result")
        .outcome
}

#[test]
fn alpha_and_beta_load_in_settings_order() {
    let got = fetch_assigned(
        &runner("gh-issues"),
        &config(&["octo-test/beta", "octo-test/alpha"]),
    )
    .expect("data");
    let order: Vec<&str> = got.iter().map(|r| r.repo.as_str()).collect();
    assert_eq!(order, ["octo-test/beta", "octo-test/alpha"]);
    assert_eq!(outcome(&got, "octo-test/beta"), &Ok(vec![]));
    let alpha = outcome(&got, "octo-test/alpha").as_ref().unwrap();
    // Newest first.
    let refs: Vec<&str> = alpha.iter().map(|i| i.reference.as_str()).collect();
    assert_eq!(refs, ["octo-test/alpha#11", "octo-test/alpha#3"]);
    assert_eq!(alpha[0].title, "Widget crashes on start");
    assert_eq!(alpha[0].labels, ["bug", "needs-triage"]);
    assert_eq!(
        alpha[0].web_url.as_deref(),
        Some("https://github.com/octo-test/alpha/issues/11")
    );
}

#[test]
fn a_missing_repo_fails_alone() {
    let got = fetch_assigned(
        &runner("gh-issues"),
        &config(&["octo-test/alpha", "octo-test/missing", "octo-test/beta"]),
    )
    .expect("round continues");
    assert_eq!(got.len(), 3);
    assert!(outcome(&got, "octo-test/alpha").is_ok());
    assert_eq!(
        outcome(&got, "octo-test/missing"),
        &Err(GithubError::Failed { exit_code: Some(1) })
    );
    assert!(outcome(&got, "octo-test/beta").is_ok());
}

#[test]
fn garbled_output_is_response_invalid_for_that_repo_and_leaks_nothing() {
    let got = fetch_assigned(
        &runner("gh-issues"),
        &config(&["octo-test/garbled", "octo-test/alpha"]),
    )
    .expect("round continues");
    let err = outcome(&got, "octo-test/garbled").as_ref().unwrap_err();
    assert_eq!(err, &GithubError::ResponseInvalid);
    let shown = [
        format!("{err:?}"),
        err.to_string(),
        serde_json::to_string(err).unwrap(),
        err.message_pl(),
    ];
    for text in shown {
        assert!(!text.contains(MARKER), "{text}");
        assert!(!text.contains("octo-test"), "{text}");
    }
    assert!(outcome(&got, "octo-test/alpha").is_ok());
}

#[test]
fn a_loaded_title_reaches_the_issue_list_and_nothing_else() {
    use day_navigator_lib::github::{GithubSnapshot, next_snapshot};
    use day_navigator_lib::health::probe::github_status;
    const TITLE_MARKER: &str = "LEAK-MARKER-GH-TITLE-5a8e";
    const LABEL_MARKER: &str = "LEAK-MARKER-GH-LABEL-3d9f";

    let cfg = config(&["octo-test/alpha", "octo-test/missing"]);
    let fetched = fetch_assigned(&runner("gh-issues"), &cfg);
    let (snap, _) = next_snapshot(&GithubSnapshot::starting(), None, Some(&cfg), fetched, 1);

    // The title is data: it is in the alpha list...
    let alpha = &snap.repos[0];
    assert!(alpha.issues.iter().any(|i| i.title.contains(TITLE_MARKER)));
    // A label is data too: it is in the alpha issue's labels...
    assert!(
        alpha
            .issues
            .iter()
            .any(|i| i.labels.iter().any(|l| l.contains(LABEL_MARKER)))
    );
    // ...and in nothing that reaches a log line, the tray or an error text.
    let status = github_status(&snap).expect("a status after a round");
    let error = snap.repos[1].error.as_ref().expect("missing repo error");
    let shown = [
        format!("{:?}", snap.state),
        serde_json::to_string(&snap.state).unwrap(),
        format!("{status:?}"),
        serde_json::to_string(&status).unwrap(),
        format!("{error:?}"),
    ];
    for text in shown {
        assert!(!text.contains(TITLE_MARKER), "{text}");
        assert!(!text.contains(LABEL_MARKER), "{text}");
        assert!(!text.contains("octo-test"), "{text}");
    }
}

#[test]
fn a_full_page_is_truncated_for_that_repo_and_never_a_list() {
    let got = fetch_assigned(
        &runner("gh-issues"),
        &config(&["octo-test/huge", "octo-test/beta"]),
    )
    .expect("round continues");
    assert_eq!(
        outcome(&got, "octo-test/huge"),
        &Err(GithubError::Truncated)
    );
    assert!(outcome(&got, "octo-test/beta").is_ok());
}

fn call_log(dir: &tempfile::TempDir) -> (PathBuf, Vec<(String, String)>) {
    let log = dir.path().join("calls.log");
    let env = vec![(
        "FAKE_CLI_CALL_LOG".to_string(),
        log.to_string_lossy().into_owned(),
    )];
    (log, env)
}

fn logged_repos(log: &std::path::Path) -> Vec<String> {
    std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .map(|l| {
            let args: Vec<&str> = l.split(' ').collect();
            let at = args.iter().position(|a| *a == "-R").expect("-R in call");
            args[at + 1].to_string()
        })
        .collect()
}

#[test]
fn a_whole_source_failure_stops_the_round_before_the_next_repo() {
    let cases = [
        ("octo-test/offline", GithubError::Unavailable),
        ("octo-test/logged-out", GithubError::LoggedOut),
    ];
    for (failing, expected) in cases {
        let dir = tempfile::tempdir().unwrap();
        let (log, env) = call_log(&dir);
        let got = fetch_assigned(
            &runner_with("gh-issues", env),
            &config(&["octo-test/alpha", failing, "octo-test/beta"]),
        );
        assert_eq!(got, Err(expected), "{failing}");
        assert_eq!(
            logged_repos(&log),
            ["octo-test/alpha", failing],
            "{failing}"
        );
    }
}

#[test]
fn whole_source_scenarios_fail_on_the_first_call() {
    let cases = [
        ("gh-logged-out", GithubError::LoggedOut),
        ("gh-no-network", GithubError::Unavailable),
    ];
    for (scenario, expected) in cases {
        let dir = tempfile::tempdir().unwrap();
        let (log, env) = call_log(&dir);
        let got = fetch_assigned(
            &runner_with(scenario, env),
            &config(&["octo-test/alpha", "octo-test/beta"]),
        );
        assert_eq!(got, Err(expected), "{scenario}");
        assert_eq!(logged_repos(&log), ["octo-test/alpha"], "{scenario}");
    }
}

#[test]
fn a_missing_gh_program_is_tool_missing() {
    let dir = tempfile::tempdir().unwrap();
    let r = CliRunner::with_programs(
        dir.path().join("no-glab"),
        dir.path().join("no-gh"),
        Duration::from_secs(5),
        Vec::new(),
    );
    assert_eq!(
        fetch_assigned(&r, &config(&["octo-test/alpha"])),
        Err(GithubError::ToolMissing)
    );
}

#[test]
fn the_call_log_records_glab_calls_too() {
    use day_navigator_lib::cli::CliCall;
    let dir = tempfile::tempdir().unwrap();
    let (log, env) = call_log(&dir);
    let _ = runner_with("data", env).run(&CliCall::GlabVersion);
    let text = std::fs::read_to_string(&log).unwrap();
    assert_eq!(text.trim(), "version");
}
