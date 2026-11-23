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

/// Why a GitLab login was refused. Carries no value, so a login never reaches a log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid gitlab user")]
pub struct UserError;

/// A GitLab login (`first.last`): 1-255 characters from `[A-Za-z0-9._-]`, not
/// starting with `-` or `.`. Reaches a `glab` argument only inside a query value.
#[derive(Clone, PartialEq, Eq)]
pub struct GitlabUser(String);

impl GitlabUser {
    pub fn parse(input: &str) -> Result<GitlabUser, UserError> {
        if input.is_empty() || input.len() > 255 {
            return Err(UserError);
        }
        let ok_chars = input
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'));
        if !ok_chars || input.starts_with('-') || input.starts_with('.') {
            return Err(UserError);
        }
        Ok(GitlabUser(input.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for GitlabUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("GitlabUser(..)")
    }
}

/// The configured GitLab: a host and, optionally, the login whose items to list
/// (needed when `glab` is logged in as a bot, whose own assigned list is empty).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitlabConfig {
    pub host: GitlabHost,
    pub user: Option<GitlabUser>,
}

/// Fixed endpoints for the two assigned-items calls without a user (GET: the
/// query string is part of the endpoint, no `-f` / `-F` fields).
pub const ASSIGNED_ISSUES_ENDPOINT: &str = "issues?scope=assigned_to_me&state=opened&per_page=100";
pub const ASSIGNED_MRS_ENDPOINT: &str =
    "merge_requests?scope=assigned_to_me&state=opened&per_page=100";

/// Endpoint shape with a user: `<PREFIX><login><SUFFIX>`.
pub const ISSUES_USER_PREFIX: &str = "issues?scope=all&state=opened&assignee_username=";
pub const MRS_USER_PREFIX: &str = "merge_requests?scope=all&state=opened&assignee_username=";
pub const USER_SUFFIX: &str = "&per_page=100";

fn assigned_args(
    fixed: &str,
    prefix: &str,
    host: &GitlabHost,
    user: &Option<GitlabUser>,
) -> Vec<String> {
    let endpoint = match user {
        Some(u) => format!("{prefix}{}{USER_SUFFIX}", u.as_str()),
        None => fixed.to_string(),
    };
    vec![
        "api".to_string(),
        endpoint,
        "--paginate".to_string(),
        "--hostname".to_string(),
        host.as_str().to_string(),
    ]
}

/// One issue, by the numbers GitLab sent (project id and in-project number).
/// Built only from parsed JSON numbers, so it can only render as digits in a
/// path. Debug is opaque so ids never reach a log.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct IssueRef {
    pub project_id: u64,
    pub iid: u64,
}

impl std::fmt::Debug for IssueRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("IssueRef(..)")
    }
}

/// Endpoint of an issue's notes, newest first: `projects/<id>/issues/<iid>` +
/// this suffix. Numeric segments only (`glab api` expands `:id` placeholders
/// from the working directory's repo).
pub const ISSUE_NOTES_PREFIX: &str = "projects/";
pub const ISSUE_NOTES_MIDDLE: &str = "/issues/";
pub const ISSUE_NOTES_SUFFIX: &str = "/notes?sort=desc&order_by=created_at&per_page=100";

fn issue_notes_args(host: &GitlabHost, issue: &IssueRef) -> Vec<String> {
    vec![
        "api".to_string(),
        format!(
            "{ISSUE_NOTES_PREFIX}{}{ISSUE_NOTES_MIDDLE}{}{ISSUE_NOTES_SUFFIX}",
            issue.project_id, issue.iid
        ),
        "--paginate".to_string(),
        "--hostname".to_string(),
        host.as_str().to_string(),
    ]
}

/// Closed list of allowed calls. Adding one is a reviewed change in this file;
/// the tests below guard that every argument list is read-only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCall {
    GlabVersion,
    GlabAuthStatus,
    GhVersion,
    GhAuthStatus,
    GlabAssignedIssues(GitlabHost, Option<GitlabUser>),
    GlabAssignedMergeRequests(GitlabHost, Option<GitlabUser>),
    /// GET of one issue's notes, newest first (read-only).
    GlabIssueNotes(GitlabHost, IssueRef),
}

impl CliCall {
    /// The parameterless calls (the real-CLI smoke test runs these).
    pub const ALL: &'static [CliCall] = &[
        CliCall::GlabVersion,
        CliCall::GlabAuthStatus,
        CliCall::GhVersion,
        CliCall::GhAuthStatus,
    ];

    /// Every variant, the GitLab ones with an invented host, without and with
    /// an invented user.
    #[cfg(any(test, feature = "test-support"))]
    #[doc(hidden)]
    pub fn examples() -> Vec<CliCall> {
        let host = GitlabHost::parse("gitlab.example.invalid").expect("valid example host");
        let user = GitlabUser::parse("invented.user").expect("valid example user");
        let mut all = CliCall::ALL.to_vec();
        all.push(CliCall::GlabAssignedIssues(host.clone(), None));
        all.push(CliCall::GlabAssignedMergeRequests(host.clone(), None));
        all.push(CliCall::GlabAssignedIssues(
            host.clone(),
            Some(user.clone()),
        ));
        all.push(CliCall::GlabAssignedMergeRequests(host.clone(), Some(user)));
        all.push(CliCall::GlabIssueNotes(
            host,
            IssueRef {
                project_id: 77,
                iid: 12,
            },
        ));
        all
    }

    pub fn tool(&self) -> Tool {
        match self {
            CliCall::GlabVersion
            | CliCall::GlabAuthStatus
            | CliCall::GlabAssignedIssues(..)
            | CliCall::GlabAssignedMergeRequests(..)
            | CliCall::GlabIssueNotes(..) => Tool::Glab,
            CliCall::GhVersion | CliCall::GhAuthStatus => Tool::Gh,
        }
    }

    pub fn args(&self) -> Vec<String> {
        match self {
            CliCall::GlabVersion => vec!["version".to_string()],
            CliCall::GlabAuthStatus | CliCall::GhAuthStatus => {
                vec!["auth".to_string(), "status".to_string()]
            }
            CliCall::GhVersion => vec!["--version".to_string()],
            CliCall::GlabAssignedIssues(host, user) => {
                assigned_args(ASSIGNED_ISSUES_ENDPOINT, ISSUES_USER_PREFIX, host, user)
            }
            CliCall::GlabAssignedMergeRequests(host, user) => {
                assigned_args(ASSIGNED_MRS_ENDPOINT, MRS_USER_PREFIX, host, user)
            }
            CliCall::GlabIssueNotes(host, issue) => issue_notes_args(host, issue),
        }
    }

    /// Stable key for logs.
    pub fn name(&self) -> &'static str {
        match self {
            CliCall::GlabVersion => "glab:version",
            CliCall::GlabAuthStatus => "glab:auth-status",
            CliCall::GhVersion => "gh:version",
            CliCall::GhAuthStatus => "gh:auth-status",
            CliCall::GlabAssignedIssues(..) => "glab:assigned-issues",
            CliCall::GlabAssignedMergeRequests(..) => "glab:assigned-mrs",
            CliCall::GlabIssueNotes(..) => "glab:issue-notes",
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

    const VARIANT_COUNT: usize = 7;

    const EXAMPLE_COUNT: usize = 9;

    fn example_host() -> GitlabHost {
        GitlabHost::parse("gitlab.example.invalid").unwrap()
    }

    fn example_user() -> GitlabUser {
        GitlabUser::parse("invented.user").unwrap()
    }

    /// Exhaustive on purpose (no `_` arm): a new variant breaks the build here.
    /// The payload is ignored.
    fn ordinal(c: &CliCall) -> usize {
        match c {
            CliCall::GlabVersion => 0,
            CliCall::GlabAuthStatus => 1,
            CliCall::GhVersion => 2,
            CliCall::GhAuthStatus => 3,
            CliCall::GlabAssignedIssues(..) => 4,
            CliCall::GlabAssignedMergeRequests(..) => 5,
            CliCall::GlabIssueNotes(..) => 6,
        }
    }

    #[test]
    fn examples_cover_every_variant_with_and_without_a_user() {
        let examples = CliCall::examples();
        assert_eq!(examples.len(), EXAMPLE_COUNT);
        let mut seen = [0usize; VARIANT_COUNT];
        for c in &examples {
            let o = ordinal(c);
            assert!(o < VARIANT_COUNT, "ordinal out of range for {c:?}");
            seen[o] += 1;
        }
        assert_eq!(seen, [1, 1, 1, 1, 2, 2, 1]);
        let with_user = examples
            .iter()
            .filter(|c| {
                matches!(
                    c,
                    CliCall::GlabAssignedIssues(_, Some(_))
                        | CliCall::GlabAssignedMergeRequests(_, Some(_))
                )
            })
            .count();
        assert_eq!(with_user, 2);
        for c in CliCall::ALL {
            assert!(
                !matches!(
                    c,
                    CliCall::GlabAssignedIssues(..)
                        | CliCall::GlabAssignedMergeRequests(..)
                        | CliCall::GlabIssueNotes(..)
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

    /// The login allow-list, written out here on purpose: the guard must not
    /// trust `GitlabUser::parse`, so a looser validator fails this test.
    fn guard_login_ok(login: &str) -> bool {
        (1..=255).contains(&login.len())
            && !login.starts_with(['-', '.'])
            && login
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    }

    /// Decimal id for the notes path, written out here on purpose (the guard
    /// must not trust `IssueRef`): 1-20 ASCII digits, no leading zero.
    fn guard_number_ok(s: &str) -> bool {
        (1..=20).contains(&s.len()) && !s.starts_with('0') && s.bytes().all(|b| b.is_ascii_digit())
    }

    /// Exactly `projects/<D>/issues/<D>/notes?sort=desc&order_by=created_at&per_page=100`.
    fn notes_endpoint_allowed(endpoint: &str) -> bool {
        const SUFFIX: &str = "/notes?sort=desc&order_by=created_at&per_page=100";
        endpoint
            .strip_prefix("projects/")
            .and_then(|rest| rest.strip_suffix(SUFFIX))
            .and_then(|ids| ids.split_once("/issues/"))
            .is_some_and(|(project, iid)| guard_number_ok(project) && guard_number_ok(iid))
    }

    /// A fixed endpoint, the notes shape, or `<prefix><valid login><suffix>`
    /// with one query and exactly its three fixed parameters.
    fn endpoint_allowed(endpoint: &str) -> bool {
        if ALLOWED_ENDPOINTS.contains(&endpoint) || notes_endpoint_allowed(endpoint) {
            return true;
        }
        let shape_ok = endpoint.matches('?').count() == 1
            && endpoint.matches('&').count() == 3
            && !endpoint
                .chars()
                .any(|c| c.is_whitespace() || c.is_control() || "#%:/+".contains(c));
        shape_ok
            && [ISSUES_USER_PREFIX, MRS_USER_PREFIX].iter().any(|prefix| {
                endpoint
                    .strip_prefix(prefix)
                    .and_then(|rest| rest.strip_suffix(USER_SUFFIX))
                    .is_some_and(guard_login_ok)
            })
    }

    /// The read-only guard on one argument list.
    fn check_read_only(args: &[&str]) -> Result<(), String> {
        if args.is_empty() {
            return Err("no arguments".into());
        }
        let simple = matches!(args[0], "version" | "--version") || args == ["auth", "status"];
        let gitlab_api = args.len() == 5
            && args[0] == "api"
            && endpoint_allowed(args[1])
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
            let args = call.args();
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            if let Err(why) = check_read_only(&args) {
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
    fn guard_accepts_the_user_form_and_rejects_bad_users_and_endpoints() {
        let host = "gitlab.example.invalid";
        let good = format!("{ISSUES_USER_PREFIX}invented.user{USER_SUFFIX}");
        assert!(check_read_only(&["api", &good, "--paginate", "--hostname", host]).is_ok());
        let good_mr = format!("{MRS_USER_PREFIX}invented.user{USER_SUFFIX}");
        assert!(check_read_only(&["api", &good_mr, "--paginate", "--hostname", host]).is_ok());

        let mut bad_endpoints = vec![
            // other endpoint with the same shape
            format!("projects?scope=all&state=opened&assignee_username=a{USER_SUFFIX}"),
            // missing suffix
            format!("{ISSUES_USER_PREFIX}invented.user"),
            // extra parameter
            format!("{ISSUES_USER_PREFIX}invented.user&state=closed{USER_SUFFIX}"),
            // empty user
            format!("{ISSUES_USER_PREFIX}{USER_SUFFIX}"),
        ];
        for bad_user in [
            "a&b", "a/b", "a b", "-a", ".a", "a?b", "a=b", "a%20b", "a#b", ":id", "a:b", "a+b",
            "a,b", "a[b]",
        ] {
            bad_endpoints.push(format!("{ISSUES_USER_PREFIX}{bad_user}{USER_SUFFIX}"));
            bad_endpoints.push(format!("{MRS_USER_PREFIX}{bad_user}{USER_SUFFIX}"));
        }
        for ep in &bad_endpoints {
            assert!(
                check_read_only(&["api", ep, "--paginate", "--hostname", host]).is_err(),
                "{ep:?}"
            );
        }
        for flag in WRITE_FLAGS {
            assert!(
                check_read_only(&["api", &good, "--paginate", "--hostname", host, flag]).is_err(),
                "{flag}"
            );
        }
        // The forbidden-character check still covers every other argument.
        assert!(
            check_read_only(&["api", &good, "--paginate", "--hostname", "a&b.invalid"]).is_err()
        );
    }

    #[test]
    fn user_accepts_logins_and_refuses_everything_else() {
        let longest = "a".repeat(255);
        for ok in [
            "invented.user",
            "a",
            "A_b-c.9",
            "x-",
            "_x",
            longest.as_str(),
        ] {
            assert_eq!(GitlabUser::parse(ok).unwrap().as_str(), ok);
        }
        let too_long = "a".repeat(256);
        let bad = [
            "",
            "-a",
            ".a",
            "a&b",
            "a/b",
            "a b",
            " a",
            "a ",
            "a?b",
            "a=b",
            "a@b",
            "a#b",
            "a:b",
            ":user",
            "a+b",
            "a,b",
            "a[b]",
            "ä",
            "a\nb",
            too_long.as_str(),
        ];
        for b in bad {
            assert!(GitlabUser::parse(b).is_err(), "{b:?}");
        }
    }

    /// Whether both user-form calls built from `login` pass the guard.
    /// `None` when `GitlabUser::parse` refuses the login.
    fn user_calls_pass_the_guard(login: &str) -> Option<bool> {
        let user = GitlabUser::parse(login).ok()?;
        Some(
            [
                CliCall::GlabAssignedIssues(example_host(), Some(user.clone())),
                CliCall::GlabAssignedMergeRequests(example_host(), Some(user)),
            ]
            .iter()
            .all(|call| {
                let args = call.args();
                let args: Vec<&str> = args.iter().map(String::as_str).collect();
                check_read_only(&args).is_ok()
            }),
        )
    }

    /// Every character `GitlabUser::parse` accepts must pass the independent
    /// guard, so widening the validator by one character breaks this test.
    #[test]
    fn every_char_the_validator_accepts_passes_the_guard() {
        for c in (0u8..=127).map(char::from).chain(['ä', 'ł', '\u{2028}']) {
            for login in [format!("a{c}b"), c.to_string()] {
                if let Some(passes) = user_calls_pass_the_guard(&login) {
                    assert!(passes, "{login:?}");
                }
            }
        }
    }

    proptest::proptest! {
        /// Mostly valid logins with at most one odd character in the middle.
        #[test]
        fn every_accepted_login_passes_the_guard(
            s in "[a-zA-Z0-9]{1,6}[._#:+&?/%=, -]?[a-zA-Z0-9._-]{0,6}"
        ) {
            if let Some(passes) = user_calls_pass_the_guard(&s) {
                proptest::prop_assert!(passes, "{:?}", s);
            }
        }
    }

    const NOTES_SUFFIX: &str = "/notes?sort=desc&order_by=created_at&per_page=100";

    fn notes_passes_the_guard(project_id: u64, iid: u64) -> bool {
        let call = CliCall::GlabIssueNotes(example_host(), IssueRef { project_id, iid });
        let args = call.args();
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        check_read_only(&args).is_ok()
    }

    fn guard_accepts_endpoint(endpoint: &str) -> bool {
        check_read_only(&[
            "api",
            endpoint,
            "--paginate",
            "--hostname",
            "gitlab.example.invalid",
        ])
        .is_ok()
    }

    #[test]
    fn notes_call_has_exact_arguments_name_and_tool() {
        let call = CliCall::GlabIssueNotes(
            example_host(),
            IssueRef {
                project_id: 77,
                iid: 12,
            },
        );
        assert_eq!(
            call.args(),
            [
                "api",
                "projects/77/issues/12/notes?sort=desc&order_by=created_at&per_page=100",
                "--paginate",
                "--hostname",
                "gitlab.example.invalid"
            ]
        );
        assert_eq!(call.name(), "glab:issue-notes");
        assert_eq!(call.tool(), Tool::Glab);
        assert!(!format!("{call:?}").contains("77"));
        assert!(!format!("{call:?}").contains("example.invalid"));
    }

    #[test]
    fn guard_accepts_the_notes_shape_and_rejects_near_misses() {
        let ok = format!("projects/77/issues/12{NOTES_SUFFIX}");
        assert!(guard_accepts_endpoint(&ok));
        let bad = [
            // extra segment
            format!("projects/77/issues/12/extra{NOTES_SUFFIX}"),
            format!("projects/77/foo/issues/12{NOTES_SUFFIX}"),
            format!("projects/77/issues/12/{NOTES_SUFFIX}"),
            // placeholders and non-numbers
            format!("projects/:id/issues/12{NOTES_SUFFIX}"),
            format!("projects/77/issues/:iid{NOTES_SUFFIX}"),
            format!("projects/a/issues/12{NOTES_SUFFIX}"),
            format!("projects//issues/12{NOTES_SUFFIX}"),
            format!("projects/77/issues/{NOTES_SUFFIX}"),
            format!("projects/-1/issues/12{NOTES_SUFFIX}"),
            format!("projects/077/issues/12{NOTES_SUFFIX}"),
            format!("projects/77/issues/012{NOTES_SUFFIX}"),
            format!("projects/{}/issues/12{NOTES_SUFFIX}", "1".repeat(21)),
            // other resources
            format!("projects/77/merge_requests/12{NOTES_SUFFIX}"),
            "projects/77/issues/12/discussions?per_page=100".to_string(),
            format!("groups/77/issues/12{NOTES_SUFFIX}"),
            format!("/projects/77/issues/12{NOTES_SUFFIX}"),
            // other queries
            "projects/77/issues/12/notes".to_string(),
            "projects/77/issues/12/notes?sort=asc&order_by=created_at&per_page=100".to_string(),
            "projects/77/issues/12/notes?sort=desc&order_by=updated_at&per_page=100".to_string(),
            "projects/77/issues/12/notes?sort=desc&order_by=created_at&per_page=50".to_string(),
            format!("projects/77/issues/12{NOTES_SUFFIX}&extra=1"),
            format!("projects/77/issues/12{NOTES_SUFFIX} "),
        ];
        for ep in &bad {
            assert!(!guard_accepts_endpoint(ep), "{ep:?}");
        }
        assert!(check_read_only(&["api", &ok, "--hostname", "gitlab.example.invalid"]).is_err());
        for flag in WRITE_FLAGS {
            assert!(
                check_read_only(&[
                    "api",
                    &ok,
                    "--paginate",
                    "--hostname",
                    "gitlab.example.invalid",
                    flag
                ])
                .is_err(),
                "{flag}"
            );
        }
    }

    /// Every character, in either id position, passes the guard only when it
    /// is an ASCII digit (and not a leading zero).
    #[test]
    fn notes_guard_accepts_only_digits_in_the_ids() {
        for c in (0u8..=127).map(char::from).chain(['ä', 'ł', '\u{2028}']) {
            let in_project = format!("projects/1{c}2/issues/3{NOTES_SUFFIX}");
            let in_iid = format!("projects/1/issues/2{c}3{NOTES_SUFFIX}");
            let alone = format!("projects/{c}/issues/{c}{NOTES_SUFFIX}");
            let digit = c.is_ascii_digit();
            assert_eq!(guard_accepts_endpoint(&in_project), digit, "{c:?}");
            assert_eq!(guard_accepts_endpoint(&in_iid), digit, "{c:?}");
            assert_eq!(
                guard_accepts_endpoint(&alone),
                c.is_ascii_digit() && c != '0',
                "{c:?}"
            );
        }
    }

    proptest::proptest! {
        /// Every `IssueRef` the code can build passes the independent guard.
        #[test]
        fn every_issue_ref_passes_the_guard(project_id in 1u64.., iid in 1u64..) {
            proptest::prop_assert!(notes_passes_the_guard(project_id, iid));
        }

        /// A near miss built from real numbers never passes.
        #[test]
        fn near_misses_of_a_valid_notes_endpoint_fail(
            project_id in 1u64..,
            iid in 1u64..,
            junk in "[a-z:/?&=.-]{1,5}",
        ) {
            let cases = [
                format!("projects/{project_id}/issues/{iid}/{junk}{NOTES_SUFFIX}"),
                format!("projects/{project_id}{junk}/issues/{iid}{NOTES_SUFFIX}"),
                format!("projects/{project_id}/issues/{iid}{junk}{NOTES_SUFFIX}"),
                format!("projects/{project_id}/merge_requests/{iid}{NOTES_SUFFIX}"),
                format!("projects/{project_id}/issues/{iid}/notes?{junk}"),
            ];
            for ep in &cases {
                proptest::prop_assert!(!guard_accepts_endpoint(ep), "{:?}", ep);
            }
        }
    }

    #[test]
    fn user_debug_and_error_are_opaque() {
        let user = example_user();
        assert_eq!(format!("{user:?}"), "GitlabUser(..)");
        let call = CliCall::GlabAssignedIssues(example_host(), Some(user.clone()));
        let config = GitlabConfig {
            host: example_host(),
            user: Some(user),
        };
        for shown in [format!("{call:?}"), format!("{config:?}")] {
            assert!(!shown.contains("invented"), "{shown}");
            assert!(!shown.contains("example.invalid"), "{shown}");
        }
        let err = GitlabUser::parse("invented user").unwrap_err();
        assert!(!format!("{err:?}").contains("invented"));
        assert!(!err.to_string().contains("invented"));
    }

    #[test]
    fn gitlab_calls_have_exact_arguments_and_names() {
        let issues = CliCall::GlabAssignedIssues(example_host(), None);
        let mrs = CliCall::GlabAssignedMergeRequests(example_host(), None);
        let issues_user = CliCall::GlabAssignedIssues(example_host(), Some(example_user()));
        let mrs_user = CliCall::GlabAssignedMergeRequests(example_host(), Some(example_user()));
        assert_eq!(
            issues_user.args(),
            [
                "api",
                "issues?scope=all&state=opened&assignee_username=invented.user&per_page=100",
                "--paginate",
                "--hostname",
                "gitlab.example.invalid"
            ]
        );
        assert_eq!(
            mrs_user.args(),
            [
                "api",
                "merge_requests?scope=all&state=opened&assignee_username=invented.user&per_page=100",
                "--paginate",
                "--hostname",
                "gitlab.example.invalid"
            ]
        );
        assert_eq!(issues_user.name(), "glab:assigned-issues");
        assert_eq!(mrs_user.name(), "glab:assigned-mrs");
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
        let call = CliCall::GlabAssignedIssues(host, None);
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
