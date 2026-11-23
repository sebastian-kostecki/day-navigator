//! The one place that names the external `glab` / `gh` calls the app may make.
//!
//! `CliCall` is a closed list of read-only calls with fixed arguments: a write
//! call or a string-built argument cannot be expressed. `CliError` carries the
//! tool and, for `Failed`, the exit code, never stderr or a hostname.
//! Nothing here starts a process or imports `tauri`.

pub mod classify;
pub mod runner;

use serde::Serialize;

pub use classify::classify;
pub use runner::CliRunner;

/// The two CLIs the app talks to. Serialised as `"glab"` / `"gh"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Tool {
    Glab,
    Gh,
}

impl Tool {
    pub fn program(self) -> &'static str {
        match self {
            Tool::Glab => "glab",
            Tool::Gh => "gh",
        }
    }
}

impl std::fmt::Display for Tool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.program())
    }
}

/// Why a hostname was refused. Carries no value, so a host never reaches a log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid hostname")]
pub struct HostError;

/// A bare, lowercased hostname (`gitlab.example.com`): no scheme, port, path or
/// whitespace. The only value from outside that reaches a `glab` argument.
#[derive(Clone, PartialEq, Eq)]
pub struct GitlabHost(String);

impl GitlabHost {
    pub fn parse(input: &str) -> Result<GitlabHost, HostError> {
        if input.is_empty() || input.len() > 253 {
            return Err(HostError);
        }
        let lowered = input.to_ascii_lowercase();
        let mut labels = 0;
        for label in lowered.split('.') {
            labels += 1;
            let ok_len = (1..=63).contains(&label.len());
            let ok_chars = label
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
            if !ok_len || !ok_chars || label.starts_with('-') || label.ends_with('-') {
                return Err(HostError);
            }
        }
        if labels < 2 {
            return Err(HostError);
        }
        Ok(GitlabHost(lowered))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for GitlabHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("GitlabHost(..)")
    }
}

/// Fixed endpoints for the two assigned-items calls (GET: the query string is
/// part of the endpoint, no `-f` / `-F` fields).
pub const ASSIGNED_ISSUES_ENDPOINT: &str = "issues?scope=assigned_to_me&state=opened&per_page=100";
pub const ASSIGNED_MRS_ENDPOINT: &str =
    "merge_requests?scope=assigned_to_me&state=opened&per_page=100";

/// Closed list of allowed calls. Adding one is a reviewed change in this file;
/// the tests below guard that every argument list is read-only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCall {
    GlabVersion,
    GlabAuthStatus,
    GhVersion,
    GhAuthStatus,
    GlabAssignedIssues(GitlabHost),
    GlabAssignedMergeRequests(GitlabHost),
}

impl CliCall {
    /// The parameterless calls (the real-CLI smoke test runs these).
    pub const ALL: &'static [CliCall] = &[
        CliCall::GlabVersion,
        CliCall::GlabAuthStatus,
        CliCall::GhVersion,
        CliCall::GhAuthStatus,
    ];

    /// Every variant once, the GitLab ones with an invented host.
    #[cfg(any(test, feature = "test-support"))]
    #[doc(hidden)]
    pub fn examples() -> Vec<CliCall> {
        let host = GitlabHost::parse("gitlab.example.invalid").expect("valid example host");
        let mut all = CliCall::ALL.to_vec();
        all.push(CliCall::GlabAssignedIssues(host.clone()));
        all.push(CliCall::GlabAssignedMergeRequests(host));
        all
    }

    pub fn tool(&self) -> Tool {
        match self {
            CliCall::GlabVersion
            | CliCall::GlabAuthStatus
            | CliCall::GlabAssignedIssues(_)
            | CliCall::GlabAssignedMergeRequests(_) => Tool::Glab,
            CliCall::GhVersion | CliCall::GhAuthStatus => Tool::Gh,
        }
    }

    pub fn args(&self) -> Vec<&str> {
        match self {
            CliCall::GlabVersion => vec!["version"],
            CliCall::GlabAuthStatus | CliCall::GhAuthStatus => vec!["auth", "status"],
            CliCall::GhVersion => vec!["--version"],
            CliCall::GlabAssignedIssues(host) => vec![
                "api",
                ASSIGNED_ISSUES_ENDPOINT,
                "--paginate",
                "--hostname",
                host.as_str(),
            ],
            CliCall::GlabAssignedMergeRequests(host) => vec![
                "api",
                ASSIGNED_MRS_ENDPOINT,
                "--paginate",
                "--hostname",
                host.as_str(),
            ],
        }
    }

    /// Stable key for logs.
    pub fn name(&self) -> &'static str {
        match self {
            CliCall::GlabVersion => "glab:version",
            CliCall::GlabAuthStatus => "glab:auth-status",
            CliCall::GhVersion => "gh:version",
            CliCall::GhAuthStatus => "gh:auth-status",
            CliCall::GlabAssignedIssues(_) => "glab:assigned-issues",
            CliCall::GlabAssignedMergeRequests(_) => "glab:assigned-mrs",
        }
    }
}

/// How a call ended when it did not produce data. No variant carries stderr.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all_fields = "camelCase")]
pub enum CliError {
    #[error("{tool} is unavailable (no network or server not responding)")]
    Unavailable { tool: Tool },
    #[error("{tool} is logged out")]
    LoggedOut { tool: Tool },
    #[error("{tool} was not found")]
    ToolMissing { tool: Tool },
    #[error("{tool} failed ({})", exit_code_text(.exit_code))]
    Failed { tool: Tool, exit_code: Option<i32> },
}

fn exit_code_text(exit_code: &Option<i32>) -> String {
    match exit_code {
        Some(code) => format!("exit code {code}"),
        None => "no exit code".to_string(),
    }
}

impl CliError {
    /// Variant name, for logs.
    pub fn kind(&self) -> &'static str {
        match self {
            CliError::Unavailable { .. } => "Unavailable",
            CliError::LoggedOut { .. } => "LoggedOut",
            CliError::ToolMissing { .. } => "ToolMissing",
            CliError::Failed { .. } => "Failed",
        }
    }

    /// Fixed Polish description; never includes stderr or a hostname.
    pub fn message_pl(&self) -> String {
        match self {
            CliError::Unavailable { tool: Tool::Glab } => {
                "GitLab niedostępny (brak VPN, sieci lub serwer nie odpowiada)".to_string()
            }
            CliError::Unavailable { tool: Tool::Gh } => {
                "GitHub niedostępny (brak sieci lub serwer nie odpowiada)".to_string()
            }
            CliError::LoggedOut { tool } => {
                format!("Nie zalogowano w {tool} — uruchom {tool} auth login")
            }
            CliError::ToolMissing { tool } => format!("Nie znaleziono programu {tool}"),
            CliError::Failed {
                tool,
                exit_code: Some(code),
            } => format!("{tool} zakończył się błędem (kod {code})"),
            CliError::Failed {
                tool,
                exit_code: None,
            } => format!("{tool} zakończył się błędem (bez kodu)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VARIANT_COUNT: usize = 6;

    fn example_host() -> GitlabHost {
        GitlabHost::parse("gitlab.example.invalid").unwrap()
    }

    /// Exhaustive on purpose (no `_` arm): a new variant breaks the build here.
    /// The payload is ignored.
    fn ordinal(c: &CliCall) -> usize {
        match c {
            CliCall::GlabVersion => 0,
            CliCall::GlabAuthStatus => 1,
            CliCall::GhVersion => 2,
            CliCall::GhAuthStatus => 3,
            CliCall::GlabAssignedIssues(_) => 4,
            CliCall::GlabAssignedMergeRequests(_) => 5,
        }
    }

    #[test]
    fn examples_list_every_variant_once() {
        let examples = CliCall::examples();
        assert_eq!(examples.len(), VARIANT_COUNT);
        let mut seen = [false; VARIANT_COUNT];
        for c in &examples {
            let o = ordinal(c);
            assert!(o < VARIANT_COUNT, "ordinal out of range for {c:?}");
            assert!(!seen[o], "{c:?} listed twice");
            seen[o] = true;
        }
        assert!(seen.iter().all(|s| *s));
        for c in CliCall::ALL {
            assert!(
                !matches!(
                    c,
                    CliCall::GlabAssignedIssues(_) | CliCall::GlabAssignedMergeRequests(_)
                ),
                "ALL must stay parameterless"
            );
        }
    }

    const WRITE_FLAGS: &[&str] = &[
        "-X",
        "--method",
        "-f",
        "-F",
        "--field",
        "--raw-field",
        "--input",
        "-d",
        "--data",
    ];
    const BAD_CHARS: &str = "&|<>^%\"`;$";
    const ALLOWED_ENDPOINTS: &[&str] = &[ASSIGNED_ISSUES_ENDPOINT, ASSIGNED_MRS_ENDPOINT];

    /// The read-only guard on one argument list.
    fn check_read_only(args: &[&str]) -> Result<(), String> {
        if args.is_empty() {
            return Err("no arguments".into());
        }
        let simple = matches!(args[0], "version" | "--version") || args == ["auth", "status"];
        let gitlab_api = args.len() == 5
            && args[0] == "api"
            && ALLOWED_ENDPOINTS.contains(&args[1])
            && args[2] == "--paginate"
            && args[3] == "--hostname"
            && GitlabHost::parse(args[4]).is_ok();
        if !simple && !gitlab_api {
            return Err("not a known read-only shape".into());
        }
        for (i, a) in args.iter().enumerate() {
            if WRITE_FLAGS.contains(a) {
                return Err(format!("write flag {a}"));
            }
            let is_endpoint = gitlab_api && i == 1;
            if !is_endpoint
                && a.chars()
                    .any(|c| c.is_whitespace() || BAD_CHARS.contains(c))
            {
                return Err(format!("argument {a:?} has a forbidden character"));
            }
        }
        Ok(())
    }

    #[test]
    fn every_call_is_read_only() {
        for call in CliCall::examples() {
            if let Err(why) = check_read_only(&call.args()) {
                panic!("{call:?}: {why}");
            }
        }
    }

    #[test]
    fn guard_rejects_writes_other_endpoints_and_bad_hosts() {
        let ep = ASSIGNED_ISSUES_ENDPOINT;
        let host = "gitlab.example.invalid";
        assert!(check_read_only(&["api", ep, "--paginate", "--hostname", host]).is_ok());
        let bad: &[&[&str]] = &[
            &["api", ep, "--paginate", "--hostname", host, "-X", "POST"],
            &["api", ep, "--paginate", "--hostname", host, "-f"],
            &["api", "projects", "--paginate", "--hostname", host],
            &["api", "issues?scope=all", "--paginate", "--hostname", host],
            &[
                "api",
                ep,
                "--paginate",
                "--hostname",
                "https://gitlab.example.invalid",
            ],
            &[
                "api",
                ep,
                "--paginate",
                "--hostname",
                "gitlab.example.invalid:8443",
            ],
            &["api", ep, "--paginate", "--hostname", "gitlab"],
            &["api", ep, "--hostname", host],
            &["issue", "create"],
        ];
        for args in bad {
            assert!(check_read_only(args).is_err(), "{args:?}");
        }
    }

    #[test]
    fn gitlab_calls_have_exact_arguments_and_names() {
        let issues = CliCall::GlabAssignedIssues(example_host());
        let mrs = CliCall::GlabAssignedMergeRequests(example_host());
        assert_eq!(
            issues.args(),
            [
                "api",
                "issues?scope=assigned_to_me&state=opened&per_page=100",
                "--paginate",
                "--hostname",
                "gitlab.example.invalid"
            ]
        );
        assert_eq!(
            mrs.args(),
            [
                "api",
                "merge_requests?scope=assigned_to_me&state=opened&per_page=100",
                "--paginate",
                "--hostname",
                "gitlab.example.invalid"
            ]
        );
        assert_eq!(issues.name(), "glab:assigned-issues");
        assert_eq!(mrs.name(), "glab:assigned-mrs");
        assert_eq!(issues.tool(), Tool::Glab);
        assert_eq!(mrs.tool(), Tool::Glab);
    }

    #[test]
    fn host_accepts_bare_hostnames_and_lowercases() {
        for ok in [
            "gitlab.example.com",
            "a.b",
            "git-lab.example.invalid",
            "x1.y2.z3",
        ] {
            assert_eq!(GitlabHost::parse(ok).unwrap().as_str(), ok);
        }
        assert_eq!(
            GitlabHost::parse("GitLab.Example.COM").unwrap().as_str(),
            "gitlab.example.com"
        );
        let longest_label = format!("{}.example.invalid", "a".repeat(63));
        assert!(GitlabHost::parse(&longest_label).is_ok());
    }

    #[test]
    fn host_refuses_everything_else() {
        let too_long_label = format!("{}.example.invalid", "a".repeat(64));
        let too_long = format!("{}invalid", "a.".repeat(130));
        let bad = [
            "",
            "gitlab",
            "https://gitlab.example.invalid",
            "gitlab.example.invalid:8443",
            "gitlab.example.invalid/path",
            "gitlab.example.invalid ",
            " gitlab.example.invalid",
            "git lab.example.invalid",
            "-gitlab.example.invalid",
            "gitlab-.example.invalid",
            "gitlab..example.invalid",
            ".example.invalid",
            "gitlab.example.invalid.",
            "git_lab.example.invalid",
            "gitlab.exämple.invalid",
            "user@gitlab.example.invalid",
            too_long_label.as_str(),
            too_long.as_str(),
        ];
        for b in bad {
            assert!(GitlabHost::parse(b).is_err(), "{b:?}");
        }
    }

    #[test]
    fn host_debug_and_error_are_opaque() {
        let host = example_host();
        assert_eq!(format!("{host:?}"), "GitlabHost(..)");
        let call = CliCall::GlabAssignedIssues(host);
        assert!(!format!("{call:?}").contains("example.invalid"));
        let err = GitlabHost::parse("bad host.example.invalid").unwrap_err();
        assert!(!format!("{err:?}").contains("example.invalid"));
        assert!(!err.to_string().contains("example.invalid"));
    }

    #[test]
    fn tools_programs_and_names_are_stable() {
        assert_eq!(Tool::Glab.program(), "glab");
        assert_eq!(Tool::Gh.program(), "gh");
        assert_eq!(CliCall::GlabVersion.tool(), Tool::Glab);
        assert_eq!(CliCall::GhAuthStatus.tool(), Tool::Gh);
        assert_eq!(CliCall::GhAuthStatus.name(), "gh:auth-status");
        assert_eq!(CliCall::GlabVersion.name(), "glab:version");
    }

    #[test]
    fn failed_serializes_with_kind_and_camel_case_detail() {
        let err = CliError::Failed {
            tool: Tool::Glab,
            exit_code: Some(2),
        };
        assert_eq!(
            serde_json::to_string(&err).unwrap(),
            r#"{"kind":"Failed","detail":{"tool":"glab","exitCode":2}}"#
        );
    }

    #[test]
    fn failed_display_shows_a_plain_exit_code() {
        let failed = |exit_code| CliError::Failed {
            tool: Tool::Gh,
            exit_code,
        };
        assert_eq!(failed(Some(2)).to_string(), "gh failed (exit code 2)");
        assert_eq!(failed(None).to_string(), "gh failed (no exit code)");
    }

    #[test]
    fn kind_and_polish_messages() {
        let glab = Tool::Glab;
        let gh = Tool::Gh;
        let failed = |tool, exit_code| CliError::Failed { tool, exit_code };
        assert_eq!(CliError::Unavailable { tool: glab }.kind(), "Unavailable");
        assert_eq!(CliError::LoggedOut { tool: gh }.kind(), "LoggedOut");
        assert_eq!(CliError::ToolMissing { tool: gh }.kind(), "ToolMissing");
        assert_eq!(failed(gh, None).kind(), "Failed");
        assert_eq!(
            CliError::Unavailable { tool: glab }.message_pl(),
            "GitLab niedostępny (brak VPN, sieci lub serwer nie odpowiada)"
        );
        assert_eq!(
            CliError::Unavailable { tool: gh }.message_pl(),
            "GitHub niedostępny (brak sieci lub serwer nie odpowiada)"
        );
        assert_eq!(
            CliError::LoggedOut { tool: glab }.message_pl(),
            "Nie zalogowano w glab — uruchom glab auth login"
        );
        assert_eq!(
            CliError::ToolMissing { tool: gh }.message_pl(),
            "Nie znaleziono programu gh"
        );
        assert_eq!(
            failed(glab, Some(2)).message_pl(),
            "glab zakończył się błędem (kod 2)"
        );
        assert_eq!(
            failed(gh, None).message_pl(),
            "gh zakończył się błędem (bez kodu)"
        );
    }
}
