//! Drives every runner state through a real process (`fake-cli`, invented data).

use std::path::PathBuf;
use std::time::{Duration, Instant};

use day_navigator_lib::cli::{CliCall, CliError, CliRunner, Tool};

fn fake() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_fake-cli"))
}

fn runner_with(scenario: &str, timeout: Duration, extra: Vec<(String, String)>) -> CliRunner {
    let mut env = vec![("FAKE_CLI_SCENARIO".to_string(), scenario.to_string())];
    env.extend(extra);
    CliRunner::with_programs(fake(), fake(), timeout, env)
}

fn runner(scenario: &str) -> CliRunner {
    runner_with(scenario, Duration::from_secs(10), Vec::new())
}

fn tool_call(tool: Tool) -> CliCall {
    match tool {
        Tool::Glab => CliCall::GlabAuthStatus,
        Tool::Gh => CliCall::GhAuthStatus,
    }
}

#[test]
fn data_echoes_exact_arguments_for_every_call() {
    let r = runner("data");
    for call in CliCall::examples() {
        let out = r.run(&call).expect("data");
        let v: serde_json::Value = serde_json::from_str(out.trim()).expect("json");
        let argv: Vec<&str> = v["argv"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a.as_str().unwrap())
            .collect();
        assert_eq!(argv, call.args(), "{call:?}");
    }
}

#[test]
fn runs_in_the_neutral_temp_dir_not_the_app_dir() {
    let out = runner("data").run(&CliCall::GlabVersion).expect("data");
    let v: serde_json::Value = serde_json::from_str(out.trim()).expect("json");
    let cwd = PathBuf::from(v["cwd"].as_str().expect("cwd"));
    let canon = |p: &std::path::Path| std::fs::canonicalize(p).expect("canonicalize");
    assert_eq!(canon(&cwd), canon(&std::env::temp_dir()));
}

#[test]
fn lingering_grandchild_on_success_is_failed_without_exit_code_within_bound() {
    let started = Instant::now();
    let got = runner("data-with-lingering-child").run(&CliCall::GlabVersion);
    assert_eq!(
        got,
        Err(CliError::Failed {
            tool: Tool::Glab,
            exit_code: None
        })
    );
    // One shared 2 s bound for both readers, well before the grandchild exits (5 s).
    assert!(
        started.elapsed() < Duration::from_millis(4500),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn lingering_grandchild_on_failure_is_still_classified() {
    let got = runner("no-network-with-lingering-child").run(&CliCall::GlabAuthStatus);
    assert_eq!(got, Err(CliError::Unavailable { tool: Tool::Glab }));
}

#[test]
fn exit_zero_with_noise_is_data() {
    assert!(runner("data-with-noise").run(&CliCall::GlabVersion).is_ok());
}

#[test]
fn logged_out_no_network_and_503_map_to_their_states() {
    let logged_out = |tool| Err(CliError::LoggedOut { tool });
    let unavailable = |tool| Err(CliError::Unavailable { tool });
    let cases = [
        ("glab-logged-out", Tool::Glab, logged_out(Tool::Glab)),
        ("gh-logged-out", Tool::Gh, logged_out(Tool::Gh)),
        ("glab-no-network", Tool::Glab, unavailable(Tool::Glab)),
        ("gh-no-network", Tool::Gh, unavailable(Tool::Gh)),
        ("http-503", Tool::Glab, unavailable(Tool::Glab)),
    ];
    for (scenario, tool, expected) in cases {
        let got = runner(scenario).run(&tool_call(tool));
        assert_eq!(got, expected, "{scenario}");
    }
}

#[test]
fn unknown_error_is_failed_with_exit_code() {
    assert_eq!(
        runner("unknown-error").run(&CliCall::GlabAuthStatus),
        Err(CliError::Failed {
            tool: Tool::Glab,
            exit_code: Some(2)
        })
    );
}

#[test]
fn hang_times_out_as_unavailable_and_the_process_is_killed() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("marker");
    let r = runner_with(
        "hang",
        Duration::from_secs(1),
        vec![(
            "FAKE_CLI_MARKER".to_string(),
            marker.to_string_lossy().into_owned(),
        )],
    );
    let started = Instant::now();
    let got = r.run(&CliCall::GlabAuthStatus);
    assert!(started.elapsed() < Duration::from_secs(5));
    assert_eq!(got, Err(CliError::Unavailable { tool: Tool::Glab }));
    std::thread::sleep(Duration::from_secs(6));
    assert!(!marker.exists(), "process survived the kill");
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
    assert_eq!(
        r.run(&CliCall::GlabVersion),
        Err(CliError::ToolMissing { tool: Tool::Glab })
    );
    assert_eq!(
        r.run(&CliCall::GhVersion),
        Err(CliError::ToolMissing { tool: Tool::Gh })
    );
}

#[test]
fn big_output_is_failed_without_exit_code() {
    assert_eq!(
        runner("big-output").run(&CliCall::GlabVersion),
        Err(CliError::Failed {
            tool: Tool::Glab,
            exit_code: None
        })
    );
}

#[test]
fn non_utf8_output_is_failed_with_exit_zero() {
    assert_eq!(
        runner("non-utf8").run(&CliCall::GhVersion),
        Err(CliError::Failed {
            tool: Tool::Gh,
            exit_code: Some(0)
        })
    );
}

#[test]
fn errors_never_leak_stderr_or_hostnames() {
    let scenarios = [
        ("glab-logged-out", Tool::Glab),
        ("gh-logged-out", Tool::Gh),
        ("glab-no-network", Tool::Glab),
        ("gh-no-network", Tool::Gh),
        ("http-503", Tool::Glab),
        ("unknown-error", Tool::Glab),
        ("no-network-with-lingering-child", Tool::Glab),
    ];
    for (scenario, tool) in scenarios {
        let err = runner(scenario)
            .run(&tool_call(tool))
            .expect_err("failing scenario");
        let shown = [
            format!("{err:?}"),
            err.to_string(),
            serde_json::to_string(&err).unwrap(),
            err.message_pl(),
        ];
        for text in shown {
            assert!(!text.contains("example.invalid"), "{scenario}: {text}");
        }
    }
}
