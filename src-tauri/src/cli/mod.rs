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

/// Why a GitHub repository name was refused. Carries no value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid github repository")]
pub struct RepoError;

/// A GitHub repository `owner/name`. Owner: 1-39 characters from `[A-Za-z0-9-]`,
/// not starting or ending with `-`. Name: 1-100 characters from `[A-Za-z0-9._-]`,
/// not `.` or `..`. The owner never starts with `-`, so the value can never be
/// read as a flag. Debug is opaque so a repo name never reaches a log.
#[derive(Clone, PartialEq, Eq)]
pub struct GithubRepo(String);

impl GithubRepo {
    pub fn parse(input: &str) -> Result<GithubRepo, RepoError> {
        let (owner, name) = input.split_once('/').ok_or(RepoError)?;
        let owner_ok = (1..=39).contains(&owner.len())
            && !owner.starts_with('-')
            && !owner.ends_with('-')
            && owner
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-');
        let name_ok = (1..=100).contains(&name.len())
            && name != "."
            && name != ".."
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'));
        if !owner_ok || !name_ok {
            return Err(RepoError);
        }
        Ok(GithubRepo(input.to_string()))
    }

    /// The name as written in settings (original case).
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Lowercase form: GitHub owners and repositories are case-insensitive.
    pub fn key(&self) -> String {
        self.0.to_ascii_lowercase()
    }
}

impl std::fmt::Debug for GithubRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("GithubRepo(..)")
    }
}

/// The configured GitHub repositories, in settings order (1..=50, no duplicates).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubConfig {
    pub repos: Vec<GithubRepo>,
}

/// `--limit` of the assigned-issues call; a repo that returns this many is
/// treated as cut off.
pub const GH_ISSUE_LIMIT: usize = 500;

fn gh_assigned_issues_args(repo: &GithubRepo) -> Vec<String> {
    [
        "issue",
        "list",
        "-R",
        repo.as_str(),
        "--assignee",
        "@me",
        "--state",
        "open",
        "--limit",
        "500",
        "--json",
        "number,title,url,updatedAt,labels",
    ]
    .iter()
    .map(|a| a.to_string())
    .collect()
}

/// Fixed endpoints for the two assigned-items calls without a user (GET: the
/// query string is part of the endpoint, no `-f` / `-F` fields).
pub const ASSIGNED_ISSUES_ENDPOINT: &str =
    "issues?scope=assigned_to_me&state=opened&with_labels_details=true&per_page=100";
pub const ASSIGNED_MRS_ENDPOINT: &str =
    "merge_requests?scope=assigned_to_me&state=opened&with_labels_details=true&per_page=100";

/// Endpoint shape with a user: `<PREFIX><login><SUFFIX>`.
pub const ISSUES_USER_PREFIX: &str = "issues?scope=all&state=opened&assignee_username=";
pub const MRS_USER_PREFIX: &str = "merge_requests?scope=all&state=opened&assignee_username=";
pub const USER_SUFFIX: &str = "&with_labels_details=true&per_page=100";

/// The open merge requests awaiting the user's review (GET, read-only).
pub const REVIEW_MRS_ENDPOINT: &str =
    "merge_requests?scope=reviews_for_me&state=opened&with_labels_details=true&per_page=100";
pub const REVIEW_MRS_USER_PREFIX: &str = "merge_requests?scope=all&state=opened&reviewer_username=";

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

/// The date-only `updated_after` value of the recently-closed calls. Built only
/// from a calendar date, so it always renders as `YYYY-MM-DD`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdatedAfter(chrono::NaiveDate);

impl UpdatedAfter {
    pub fn new(date: chrono::NaiveDate) -> UpdatedAfter {
        UpdatedAfter(date)
    }

    /// `YYYY-MM-DD`.
    pub fn render(&self) -> String {
        self.0.format("%Y-%m-%d").to_string()
    }
}

/// Endpoint shapes of the recently-closed calls: `<resource>?scope=assigned_to_me
/// &state=<S>&updated_after=<D>&with_labels_details=true&per_page=100` without a user, and
/// `<resource>?scope=all&state=<S>&assignee_username=<login>&updated_after=<D>
/// &with_labels_details=true&per_page=100` with one. Issues use `state=closed`, merge requests `state=all`.
pub const CLOSED_ISSUES_FIXED_PREFIX: &str =
    "issues?scope=assigned_to_me&state=closed&updated_after=";
pub const CLOSED_MRS_FIXED_PREFIX: &str =
    "merge_requests?scope=assigned_to_me&state=all&updated_after=";
pub const CLOSED_ISSUES_USER_PREFIX: &str = "issues?scope=all&state=closed&assignee_username=";
pub const CLOSED_MRS_USER_PREFIX: &str = "merge_requests?scope=all&state=all&assignee_username=";
pub const CLOSED_USER_MIDDLE: &str = "&updated_after=";
pub const CLOSED_SUFFIX: &str = "&with_labels_details=true&per_page=100";

fn closed_args(
    fixed_prefix: &str,
    user_prefix: &str,
    host: &GitlabHost,
    user: &Option<GitlabUser>,
    after: &UpdatedAfter,
) -> Vec<String> {
    let date = after.render();
    let endpoint = match user {
        Some(u) => format!(
            "{user_prefix}{}{CLOSED_USER_MIDDLE}{date}{CLOSED_SUFFIX}",
            u.as_str()
        ),
        None => format!("{fixed_prefix}{date}{CLOSED_SUFFIX}"),
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
/// Also a merge request's project id and iid (the MR notes call).
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

/// One GitHub issue number: a non-zero decimal that can only render as digits
/// in an argument. Debug is opaque so a number never reaches a log.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct GithubIssueNumber(u64);

impl GithubIssueNumber {
    pub fn new(number: u64) -> Option<GithubIssueNumber> {
        (number > 0).then_some(GithubIssueNumber(number))
    }

    pub fn get(&self) -> u64 {
        self.0
    }
}

impl std::fmt::Debug for GithubIssueNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("GithubIssueNumber(..)")
    }
}

fn gh_issue_comments_args(repo: &GithubRepo, number: &GithubIssueNumber) -> Vec<String> {
    [
        "issue",
        "view",
        &number.get().to_string(),
        "-R",
        repo.as_str(),
        "--json",
        "comments",
    ]
    .iter()
    .map(|a| a.to_string())
    .collect()
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

/// The same shape for one merge request's notes: `projects/<id>/merge_requests/<iid>`
/// + the shared suffix.
pub const MR_NOTES_MIDDLE: &str = "/merge_requests/";

fn mr_notes_args(host: &GitlabHost, mr: &IssueRef) -> Vec<String> {
    vec![
        "api".to_string(),
        format!(
            "{ISSUE_NOTES_PREFIX}{}{MR_NOTES_MIDDLE}{}{ISSUE_NOTES_SUFFIX}",
            mr.project_id, mr.iid
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
    /// GET of the assigned issues closed since a date (read-only).
    GlabRecentlyClosedIssues(GitlabHost, Option<GitlabUser>, UpdatedAfter),
    /// GET of the assigned merge requests, any state, updated since a date
    /// (read-only; the parser keeps the closed and merged ones).
    GlabRecentlyClosedMergeRequests(GitlabHost, Option<GitlabUser>, UpdatedAfter),
    /// GET of the open merge requests awaiting the user's review (read-only).
    GlabReviewMergeRequests(GitlabHost, Option<GitlabUser>),
    /// GET of one issue's notes, newest first (read-only).
    GlabIssueNotes(GitlabHost, IssueRef),
    /// `gh issue list` of one repository: open issues assigned to the account
    /// (read-only).
    GhAssignedIssues(GithubRepo),
    /// `gh api user`: the login of the logged-in account (read-only GET).
    GhUser,
    /// `gh issue view <N> -R <repo> --json comments`: one issue's comments
    /// (read-only).
    GhIssueComments(GithubRepo, GithubIssueNumber),
    /// GET of one merge request's notes, newest first (read-only).
    GlabMergeRequestNotes(GitlabHost, IssueRef),
}

impl CliCall {
    /// The parameterless calls (the real-CLI smoke test runs these).
    pub const ALL: &'static [CliCall] = &[
        CliCall::GlabVersion,
        CliCall::GlabAuthStatus,
        CliCall::GhVersion,
        CliCall::GhAuthStatus,
        CliCall::GhUser,
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
        all.push(CliCall::GlabAssignedMergeRequests(
            host.clone(),
            Some(user.clone()),
        ));
        all.push(CliCall::GlabReviewMergeRequests(host.clone(), None));
        all.push(CliCall::GlabReviewMergeRequests(
            host.clone(),
            Some(user.clone()),
        ));
        let after = UpdatedAfter::new(
            chrono::NaiveDate::from_ymd_opt(2026, 3, 1).expect("valid example date"),
        );
        all.push(CliCall::GlabRecentlyClosedIssues(host.clone(), None, after));
        all.push(CliCall::GlabRecentlyClosedMergeRequests(
            host.clone(),
            None,
            after,
        ));
        all.push(CliCall::GlabRecentlyClosedIssues(
            host.clone(),
            Some(user.clone()),
            after,
        ));
        all.push(CliCall::GlabRecentlyClosedMergeRequests(
            host.clone(),
            Some(user),
            after,
        ));
        all.push(CliCall::GlabIssueNotes(
            host.clone(),
            IssueRef {
                project_id: 77,
                iid: 12,
            },
        ));
        all.push(CliCall::GlabMergeRequestNotes(
            host,
            IssueRef {
                project_id: 77,
                iid: 34,
            },
        ));
        all.push(CliCall::GhAssignedIssues(
            GithubRepo::parse("octo-test/alpha").expect("valid example repo"),
        ));
        all.push(CliCall::GhIssueComments(
            GithubRepo::parse("octo-test/alpha").expect("valid example repo"),
            GithubIssueNumber::new(12).expect("valid example number"),
        ));
        all
    }

    pub fn tool(&self) -> Tool {
        match self {
            CliCall::GlabVersion
            | CliCall::GlabAuthStatus
            | CliCall::GlabAssignedIssues(..)
            | CliCall::GlabAssignedMergeRequests(..)
            | CliCall::GlabRecentlyClosedIssues(..)
            | CliCall::GlabRecentlyClosedMergeRequests(..)
            | CliCall::GlabReviewMergeRequests(..)
            | CliCall::GlabIssueNotes(..)
            | CliCall::GlabMergeRequestNotes(..) => Tool::Glab,
            CliCall::GhVersion
            | CliCall::GhAuthStatus
            | CliCall::GhAssignedIssues(..)
            | CliCall::GhUser
            | CliCall::GhIssueComments(..) => Tool::Gh,
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
            CliCall::GlabRecentlyClosedIssues(host, user, after) => closed_args(
                CLOSED_ISSUES_FIXED_PREFIX,
                CLOSED_ISSUES_USER_PREFIX,
                host,
                user,
                after,
            ),
            CliCall::GlabRecentlyClosedMergeRequests(host, user, after) => closed_args(
                CLOSED_MRS_FIXED_PREFIX,
                CLOSED_MRS_USER_PREFIX,
                host,
                user,
                after,
            ),
            CliCall::GlabReviewMergeRequests(host, user) => {
                assigned_args(REVIEW_MRS_ENDPOINT, REVIEW_MRS_USER_PREFIX, host, user)
            }
            CliCall::GlabIssueNotes(host, issue) => issue_notes_args(host, issue),
            CliCall::GlabMergeRequestNotes(host, mr) => mr_notes_args(host, mr),
            CliCall::GhAssignedIssues(repo) => gh_assigned_issues_args(repo),
            CliCall::GhUser => vec!["api".to_string(), "user".to_string()],
            CliCall::GhIssueComments(repo, number) => gh_issue_comments_args(repo, number),
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
            CliCall::GlabRecentlyClosedIssues(..) => "glab:closed-issues",
            CliCall::GlabRecentlyClosedMergeRequests(..) => "glab:closed-mrs",
            CliCall::GlabReviewMergeRequests(..) => "glab:review-mrs",
            CliCall::GlabIssueNotes(..) => "glab:issue-notes",
            CliCall::GlabMergeRequestNotes(..) => "glab:mr-notes",
            CliCall::GhAssignedIssues(..) => "gh:assigned-issues",
            CliCall::GhUser => "gh:user",
            CliCall::GhIssueComments(..) => "gh:issue-comments",
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

    const VARIANT_COUNT: usize = 14;

    const EXAMPLE_COUNT: usize = 19;

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
            CliCall::GhAssignedIssues(..) => 7,
            CliCall::GlabRecentlyClosedIssues(..) => 8,
            CliCall::GlabRecentlyClosedMergeRequests(..) => 9,
            CliCall::GhUser => 10,
            CliCall::GhIssueComments(..) => 11,
            CliCall::GlabReviewMergeRequests(..) => 12,
            CliCall::GlabMergeRequestNotes(..) => 13,
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
        assert_eq!(seen, [1, 1, 1, 1, 2, 2, 1, 1, 2, 2, 1, 1, 2, 1]);
        let with_user = examples
            .iter()
            .filter(|c| {
                matches!(
                    c,
                    CliCall::GlabAssignedIssues(_, Some(_))
                        | CliCall::GlabAssignedMergeRequests(_, Some(_))
                        | CliCall::GlabRecentlyClosedIssues(_, Some(_), _)
                        | CliCall::GlabRecentlyClosedMergeRequests(_, Some(_), _)
                        | CliCall::GlabReviewMergeRequests(_, Some(_))
                )
            })
            .count();
        assert_eq!(with_user, 5);
        for c in CliCall::ALL {
            assert!(
                !matches!(
                    c,
                    CliCall::GlabAssignedIssues(..)
                        | CliCall::GlabAssignedMergeRequests(..)
                        | CliCall::GlabRecentlyClosedIssues(..)
                        | CliCall::GlabRecentlyClosedMergeRequests(..)
                        | CliCall::GlabReviewMergeRequests(..)
                        | CliCall::GlabIssueNotes(..)
                        | CliCall::GhAssignedIssues(..)
                        | CliCall::GhIssueComments(..)
                        | CliCall::GlabMergeRequestNotes(..)
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
    const ALLOWED_ENDPOINTS: &[&str] = &[
        ASSIGNED_ISSUES_ENDPOINT,
        ASSIGNED_MRS_ENDPOINT,
        REVIEW_MRS_ENDPOINT,
    ];

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

    /// Exactly `projects/<D>/issues/<D>/notes?sort=desc&order_by=created_at&per_page=100`
    /// or the same with `merge_requests/<D>` in place of `issues/<D>`.
    fn notes_endpoint_allowed(endpoint: &str) -> bool {
        const SUFFIX: &str = "/notes?sort=desc&order_by=created_at&per_page=100";
        endpoint
            .strip_prefix("projects/")
            .and_then(|rest| rest.strip_suffix(SUFFIX))
            .is_some_and(|ids| {
                ["/issues/", "/merge_requests/"].iter().any(|middle| {
                    ids.split_once(middle).is_some_and(|(project, iid)| {
                        guard_number_ok(project) && guard_number_ok(iid)
                    })
                })
            })
    }

    /// `YYYY-MM-DD`, written out here on purpose (the guard must not trust
    /// `UpdatedAfter`): ten ASCII characters, digits and two dashes.
    fn guard_date_ok(date: &str) -> bool {
        date.len() == 10
            && date.bytes().enumerate().all(|(i, b)| match i {
                4 | 7 => b == b'-',
                _ => b.is_ascii_digit(),
            })
    }

    /// The recently-closed shapes, spelled out in full: the fixed form
    /// `<R>?scope=assigned_to_me&state=<S>&updated_after=<D>&with_labels_details=true&per_page=100` and
    /// the user form `<R>?scope=all&state=<S>&assignee_username=<login>&updated_after=<D>&with_labels_details=true&per_page=100`,
    /// with `issues` + `closed` or `merge_requests` + `all`.
    fn closed_endpoint_allowed(endpoint: &str) -> bool {
        const FORMS: [(&str, &str); 2] = [("issues", "closed"), ("merge_requests", "all")];
        FORMS.iter().any(|(resource, state)| {
            let fixed = format!("{resource}?scope=assigned_to_me&state={state}&updated_after=");
            let user = format!("{resource}?scope=all&state={state}&assignee_username=");
            let fixed_ok = endpoint
                .strip_prefix(fixed.as_str())
                .and_then(|rest| rest.strip_suffix("&with_labels_details=true&per_page=100"))
                .is_some_and(guard_date_ok);
            let user_ok = endpoint
                .strip_prefix(user.as_str())
                .and_then(|rest| rest.strip_suffix("&with_labels_details=true&per_page=100"))
                .and_then(|rest| rest.split_once("&updated_after="))
                .is_some_and(|(login, date)| guard_login_ok(login) && guard_date_ok(date));
            fixed_ok || user_ok
        })
    }

    /// A fixed endpoint, the notes shape, a recently-closed shape, or
    /// `<prefix><valid login><suffix>` with one query and exactly its four
    /// fixed parameters.
    fn endpoint_allowed(endpoint: &str) -> bool {
        if ALLOWED_ENDPOINTS.contains(&endpoint)
            || notes_endpoint_allowed(endpoint)
            || closed_endpoint_allowed(endpoint)
        {
            return true;
        }
        let shape_ok = endpoint.matches('?').count() == 1
            && endpoint.matches('&').count() == 4
            && !endpoint
                .chars()
                .any(|c| c.is_whitespace() || c.is_control() || "#%:/+".contains(c));
        shape_ok
            && [ISSUES_USER_PREFIX, MRS_USER_PREFIX, REVIEW_MRS_USER_PREFIX]
                .iter()
                .any(|prefix| {
                    endpoint
                        .strip_prefix(prefix)
                        .and_then(|rest| rest.strip_suffix(USER_SUFFIX))
                        .is_some_and(guard_login_ok)
                })
    }

    /// The repo allow-list, written out here on purpose: the guard must not
    /// trust `GithubRepo::parse`, so a looser validator fails this test.
    fn guard_repo_ok(repo: &str) -> bool {
        let Some((owner, name)) = repo.split_once('/') else {
            return false;
        };
        (1..=39).contains(&owner.len())
            && !owner.starts_with('-')
            && !owner.ends_with('-')
            && owner
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            && (1..=100).contains(&name.len())
            && name != "."
            && name != ".."
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    }

    /// Everything after the repo in the one allowed `gh issue list` shape.
    const GH_ISSUES_TAIL: [&str; 8] = [
        "--assignee",
        "@me",
        "--state",
        "open",
        "--limit",
        "500",
        "--json",
        "number,title,url,updatedAt,labels",
    ];

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
        let gh_issues = args.len() == 12
            && args[..3] == ["issue", "list", "-R"]
            && guard_repo_ok(args[3])
            && args[4..] == GH_ISSUES_TAIL;
        let gh_user = args == ["api", "user"];
        let gh_comments = args.len() == 7
            && args[0] == "issue"
            && args[1] == "view"
            && guard_number_ok(args[2])
            && args[3] == "-R"
            && guard_repo_ok(args[4])
            && args[5] == "--json"
            && args[6] == "comments";
        if !simple && !gitlab_api && !gh_issues && !gh_user && !gh_comments {
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
        let good_review = format!("{REVIEW_MRS_USER_PREFIX}invented.user{USER_SUFFIX}");
        assert!(check_read_only(&["api", &good_review, "--paginate", "--hostname", host]).is_ok());

        let mut bad_endpoints = vec![
            // other endpoint with the same shape
            format!("projects?scope=all&state=opened&assignee_username=a{USER_SUFFIX}"),
            // missing suffix
            format!("{ISSUES_USER_PREFIX}invented.user"),
            // extra parameter
            format!("{ISSUES_USER_PREFIX}invented.user&state=closed{USER_SUFFIX}"),
            // empty user
            format!("{ISSUES_USER_PREFIX}{USER_SUFFIX}"),
            // review form with an extra parameter
            format!("{REVIEW_MRS_USER_PREFIX}invented.user&state=closed{USER_SUFFIX}"),
        ];
        for bad_user in [
            "a&b", "a/b", "a b", "-a", ".a", "a?b", "a=b", "a%20b", "a#b", ":id", "a:b", "a+b",
            "a,b", "a[b]",
        ] {
            bad_endpoints.push(format!("{ISSUES_USER_PREFIX}{bad_user}{USER_SUFFIX}"));
            bad_endpoints.push(format!("{MRS_USER_PREFIX}{bad_user}{USER_SUFFIX}"));
            bad_endpoints.push(format!("{REVIEW_MRS_USER_PREFIX}{bad_user}{USER_SUFFIX}"));
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
        /// Every `IssueRef` the code can build passes the independent guard
        /// (`parse_items` refuses zero ids, so the range starts at 1).
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
                format!("projects/{project_id}/issues/{iid}/notes?{junk}"),
            ];
            for ep in &cases {
                proptest::prop_assert!(!guard_accepts_endpoint(ep), "{:?}", ep);
            }
        }
    }

    const MR_NOTES_SUFFIX: &str = "/notes?sort=desc&order_by=created_at&per_page=100";

    fn mr_notes_passes_the_guard(project_id: u64, iid: u64) -> bool {
        let call = CliCall::GlabMergeRequestNotes(example_host(), IssueRef { project_id, iid });
        let args = call.args();
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        check_read_only(&args).is_ok()
    }

    #[test]
    fn mr_notes_call_has_exact_arguments_name_and_tool() {
        let call = CliCall::GlabMergeRequestNotes(
            example_host(),
            IssueRef {
                project_id: 77,
                iid: 34,
            },
        );
        assert_eq!(
            call.args(),
            [
                "api",
                "projects/77/merge_requests/34/notes?sort=desc&order_by=created_at&per_page=100",
                "--paginate",
                "--hostname",
                "gitlab.example.invalid"
            ]
        );
        assert_eq!(call.name(), "glab:mr-notes");
        assert_eq!(call.tool(), Tool::Glab);
        assert!(!format!("{call:?}").contains("77"));
        assert!(!format!("{call:?}").contains("example.invalid"));
    }

    #[test]
    fn guard_accepts_the_mr_notes_shape_and_rejects_near_misses() {
        let ok = format!("projects/77/merge_requests/34{MR_NOTES_SUFFIX}");
        assert!(guard_accepts_endpoint(&ok));
        let bad = [
            format!("projects/77/merge_requests/0{MR_NOTES_SUFFIX}"),
            format!("projects/77/merge_requests/034{MR_NOTES_SUFFIX}"),
            format!("projects/077/merge_requests/34{MR_NOTES_SUFFIX}"),
            format!("projects//merge_requests/34{MR_NOTES_SUFFIX}"),
            format!("projects/77/merge_request/34{MR_NOTES_SUFFIX}"),
            format!("projects/77/merge_requests/34/extra{MR_NOTES_SUFFIX}"),
            "projects/77/merge_requests/34/notes/5?sort=desc&order_by=created_at&per_page=100"
                .to_string(),
            format!("projects/77/merge_requests/34/issues/2{MR_NOTES_SUFFIX}"),
            format!("projects/77/issues/1/merge_requests/34{MR_NOTES_SUFFIX}"),
            "projects/77/merge_requests/34/notes?sort=asc&order_by=created_at&per_page=100"
                .to_string(),
            "projects/77/merge_requests/34/notes?sort=desc&order_by=created_at".to_string(),
            format!("projects/77/merge_requests/34{MR_NOTES_SUFFIX}&x=1"),
        ];
        for ep in &bad {
            assert!(!guard_accepts_endpoint(ep), "{ep:?}");
        }
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

    proptest::proptest! {
        /// Every merge request `IssueRef` the code can build passes the guard.
        #[test]
        fn every_mr_ref_passes_the_guard(project_id in 1u64.., iid in 1u64..) {
            proptest::prop_assert!(mr_notes_passes_the_guard(project_id, iid));
        }
    }

    fn gh_args(repo: &str) -> Vec<String> {
        let mut args: Vec<String> = ["issue", "list", "-R", repo]
            .iter()
            .map(|a| a.to_string())
            .collect();
        args.extend(GH_ISSUES_TAIL.iter().map(|a| a.to_string()));
        args
    }

    fn gh_guard(args: &[String]) -> Result<(), String> {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        check_read_only(&args)
    }

    #[test]
    fn gh_assigned_issues_has_exact_arguments_name_and_tool() {
        let call = CliCall::GhAssignedIssues(GithubRepo::parse("Octo-Test/Alpha").unwrap());
        assert_eq!(
            call.args(),
            [
                "issue",
                "list",
                "-R",
                "Octo-Test/Alpha",
                "--assignee",
                "@me",
                "--state",
                "open",
                "--limit",
                "500",
                "--json",
                "number,title,url,updatedAt,labels"
            ]
        );
        assert_eq!(call.name(), "gh:assigned-issues");
        assert_eq!(call.tool(), Tool::Gh);
        assert_eq!(GH_ISSUE_LIMIT, 500);
        assert!(!format!("{call:?}").contains("Octo"));
        assert!(gh_guard(&call.args()).is_ok());
    }

    #[test]
    fn guard_rejects_bad_repos_and_altered_gh_shapes() {
        assert!(gh_guard(&gh_args("octo-test/alpha")).is_ok());
        for bad in ["-X/foo", "a/b c", "a/b&c", "a", "a/b/c", "a/", "/b", "a/.."] {
            assert!(gh_guard(&gh_args(bad)).is_err(), "{bad:?}");
        }
        // Every position other than the repo must match the literal.
        for i in (0..12).filter(|i| *i != 3) {
            let mut args = gh_args("octo-test/alpha");
            args[i] = "other".to_string();
            assert!(gh_guard(&args).is_err(), "position {i}");
        }
        let mut wrong_state = gh_args("octo-test/alpha");
        wrong_state[7] = "all".to_string();
        assert!(gh_guard(&wrong_state).is_err());
        for flag in WRITE_FLAGS.iter().chain(&["--web", "--body", "close"]) {
            let mut extra = gh_args("octo-test/alpha");
            extra.push(flag.to_string());
            assert!(gh_guard(&extra).is_err(), "{flag}");
        }
        let mut shorter = gh_args("octo-test/alpha");
        shorter.pop();
        assert!(gh_guard(&shorter).is_err());
        assert!(check_read_only(&["issue", "create"]).is_err());
        assert!(check_read_only(&["issue", "list", "-R", "octo-test/alpha"]).is_err());
    }

    #[test]
    fn gh_user_has_exact_arguments_name_and_tool() {
        let call = CliCall::GhUser;
        assert_eq!(call.args(), ["api", "user"]);
        assert_eq!(call.name(), "gh:user");
        assert_eq!(call.tool(), Tool::Gh);
        assert!(CliCall::ALL.contains(&CliCall::GhUser));
        assert!(gh_guard(&call.args()).is_ok());
    }

    #[test]
    fn guard_rejects_near_misses_of_gh_user() {
        let bad: &[&[&str]] = &[
            &["api", "user", "extra"],
            &["api", "users"],
            &["api", "user/emails"],
            &["api", "user", "-X", "GET"],
            &["api", "user", "-X", "POST"],
            &["api", "user", "--method", "PATCH"],
            &["api"],
            &["user", "api"],
        ];
        for args in bad {
            assert!(check_read_only(args).is_err(), "{args:?}");
        }
    }

    fn gh_view_args(number: &str, repo: &str) -> Vec<String> {
        ["issue", "view", number, "-R", repo, "--json", "comments"]
            .iter()
            .map(|a| a.to_string())
            .collect()
    }

    #[test]
    fn gh_issue_comments_has_exact_arguments_name_and_tool() {
        let call = CliCall::GhIssueComments(
            GithubRepo::parse("Octo-Test/Alpha").unwrap(),
            GithubIssueNumber::new(12).unwrap(),
        );
        assert_eq!(call.args(), gh_view_args("12", "Octo-Test/Alpha"));
        assert_eq!(call.name(), "gh:issue-comments");
        assert_eq!(call.tool(), Tool::Gh);
        assert!(!format!("{call:?}").contains("Octo"));
        assert!(!format!("{call:?}").contains("12"));
        assert!(gh_guard(&call.args()).is_ok());
        assert!(GithubIssueNumber::new(0).is_none());
        assert_eq!(GithubIssueNumber::new(7).unwrap().get(), 7);
        assert_eq!(
            format!("{:?}", GithubIssueNumber::new(7).unwrap()),
            "GithubIssueNumber(..)"
        );
    }

    #[test]
    fn guard_rejects_near_misses_of_gh_issue_view() {
        assert!(gh_guard(&gh_view_args("12", "octo-test/alpha")).is_ok());
        for bad_number in ["012", "0", "1a", "-1", "", "1 2", "1.5"] {
            assert!(
                gh_guard(&gh_view_args(bad_number, "octo-test/alpha")).is_err(),
                "{bad_number:?}"
            );
        }
        for bad_repo in ["-X/foo", "a/b c", "a", "a/.."] {
            assert!(
                gh_guard(&gh_view_args("12", bad_repo)).is_err(),
                "{bad_repo:?}"
            );
        }
        // Every literal position must match.
        for i in [0, 1, 3, 5, 6] {
            let mut args = gh_view_args("12", "octo-test/alpha");
            args[i] = "other".to_string();
            assert!(gh_guard(&args).is_err(), "position {i}");
        }
        let near: &[&[&str]] = &[
            &[
                "issue",
                "view",
                "12",
                "-R",
                "octo-test/alpha",
                "--json",
                "comments",
                "--web",
            ],
            &["issue", "view", "12", "-R", "octo-test/alpha", "--web"],
            &[
                "issue",
                "view",
                "12",
                "-R",
                "octo-test/alpha",
                "--json",
                "body",
            ],
            &[
                "issue",
                "view",
                "12",
                "-R",
                "octo-test/alpha",
                "--json",
                "comments,body",
            ],
            &["issue", "view", "12", "-R", "octo-test/alpha", "--comments"],
            &["issue", "view", "12", "--json", "comments"],
            &[
                "issue",
                "view",
                "12",
                "--json",
                "comments",
                "-R",
                "octo-test/alpha",
            ],
            &[
                "issue",
                "view",
                "-R",
                "octo-test/alpha",
                "12",
                "--json",
                "comments",
            ],
            &[
                "issue",
                "view",
                "12",
                "-R",
                "octo-test/alpha",
                "--json",
                "comments",
                "-X",
                "POST",
            ],
        ];
        for args in near {
            assert!(check_read_only(args).is_err(), "{args:?}");
        }
    }

    #[test]
    fn repo_accepts_valid_forms_and_refuses_everything_else() {
        let long_owner = format!("{}/x", "a".repeat(39));
        let long_name = format!("o/{}", "n".repeat(100));
        for ok in [
            "octo-test/alpha",
            "Octo/Repo",
            "a/b",
            "a-b/c.d_e-f",
            "a/.hidden",
            "a/..x",
            "a/-x",
            long_owner.as_str(),
            long_name.as_str(),
        ] {
            assert_eq!(GithubRepo::parse(ok).unwrap().as_str(), ok);
        }
        assert_eq!(GithubRepo::parse("Octo/Repo").unwrap().key(), "octo/repo");
        let too_long_owner = format!("{}/x", "a".repeat(40));
        let too_long_name = format!("o/{}", "n".repeat(101));
        let bad = [
            "",
            "a",
            "a/",
            "/b",
            "a/b/c",
            "-a/b",
            "a-/b",
            "a_b/c",
            "a.b/c",
            "a/.",
            "a/..",
            "a /b",
            "a/b ",
            " a/b",
            "a/b c",
            "a/b\n",
            "a/b&c",
            "a/b;c",
            "a/b@c",
            "ä/b",
            "a/ł",
            too_long_owner.as_str(),
            too_long_name.as_str(),
        ];
        for b in bad {
            assert!(GithubRepo::parse(b).is_err(), "{b:?}");
        }
    }

    #[test]
    fn repo_debug_and_error_are_opaque() {
        let repo = GithubRepo::parse("octo-test/alpha").unwrap();
        assert_eq!(format!("{repo:?}"), "GithubRepo(..)");
        let config = GithubConfig { repos: vec![repo] };
        assert!(!format!("{config:?}").contains("octo"));
        let err = GithubRepo::parse("octo test/alpha").unwrap_err();
        assert!(!format!("{err:?}").contains("octo"));
        assert!(!err.to_string().contains("octo"));
    }

    proptest::proptest! {
        /// No value `GithubRepo::parse` accepts starts with `-` or carries a
        /// forbidden character, and every accepted value passes the guard.
        #[test]
        fn accepted_repos_are_safe_arguments(
            s in "[a-zA-Z0-9._/ &|<>^%\"`;$@:?#+,=-]{0,30}"
        ) {
            if let Ok(repo) = GithubRepo::parse(&s) {
                let v = repo.as_str();
                proptest::prop_assert!(!v.starts_with('-'));
                proptest::prop_assert!(
                    !v.chars().any(|c| c.is_whitespace() || BAD_CHARS.contains(c))
                );
                proptest::prop_assert!(
                    gh_guard(&CliCall::GhAssignedIssues(repo.clone()).args()).is_ok()
                );
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
                "issues?scope=all&state=opened&assignee_username=invented.user&with_labels_details=true&per_page=100",
                "--paginate",
                "--hostname",
                "gitlab.example.invalid"
            ]
        );
        assert_eq!(
            mrs_user.args(),
            [
                "api",
                "merge_requests?scope=all&state=opened&assignee_username=invented.user&with_labels_details=true&per_page=100",
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
                "issues?scope=assigned_to_me&state=opened&with_labels_details=true&per_page=100",
                "--paginate",
                "--hostname",
                "gitlab.example.invalid"
            ]
        );
        assert_eq!(
            mrs.args(),
            [
                "api",
                "merge_requests?scope=assigned_to_me&state=opened&with_labels_details=true&per_page=100",
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

    fn example_after() -> UpdatedAfter {
        UpdatedAfter::new(chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap())
    }

    #[test]
    fn closed_calls_have_exact_arguments_names_and_tool() {
        let after = example_after();
        assert_eq!(after.render(), "2026-03-01");
        let cases = [
            (
                CliCall::GlabRecentlyClosedIssues(example_host(), None, after),
                "issues?scope=assigned_to_me&state=closed&updated_after=2026-03-01&with_labels_details=true&per_page=100",
                "glab:closed-issues",
            ),
            (
                CliCall::GlabRecentlyClosedIssues(example_host(), Some(example_user()), after),
                "issues?scope=all&state=closed&assignee_username=invented.user&updated_after=2026-03-01&with_labels_details=true&per_page=100",
                "glab:closed-issues",
            ),
            (
                CliCall::GlabRecentlyClosedMergeRequests(example_host(), None, after),
                "merge_requests?scope=assigned_to_me&state=all&updated_after=2026-03-01&with_labels_details=true&per_page=100",
                "glab:closed-mrs",
            ),
            (
                CliCall::GlabRecentlyClosedMergeRequests(
                    example_host(),
                    Some(example_user()),
                    after,
                ),
                "merge_requests?scope=all&state=all&assignee_username=invented.user&updated_after=2026-03-01&with_labels_details=true&per_page=100",
                "glab:closed-mrs",
            ),
        ];
        for (call, endpoint, name) in cases {
            assert_eq!(
                call.args(),
                [
                    "api",
                    endpoint,
                    "--paginate",
                    "--hostname",
                    "gitlab.example.invalid"
                ]
            );
            assert_eq!(call.name(), name);
            assert_eq!(call.tool(), Tool::Glab);
            assert!(!format!("{call:?}").contains("example.invalid"));
            assert!(!format!("{call:?}").contains("invented"));
            assert!(guard_accepts_endpoint(endpoint), "{endpoint}");
        }
    }

    #[test]
    fn guard_accepts_the_closed_shapes_and_rejects_near_misses() {
        let fixed_issues = "issues?scope=assigned_to_me&state=closed&updated_after=";
        let fixed_mrs = "merge_requests?scope=assigned_to_me&state=all&updated_after=";
        let user_issues =
            "issues?scope=all&state=closed&assignee_username=invented.user&updated_after=";
        let user_mrs =
            "merge_requests?scope=all&state=all&assignee_username=invented.user&updated_after=";
        for prefix in [fixed_issues, fixed_mrs, user_issues, user_mrs] {
            assert!(guard_accepts_endpoint(&format!(
                "{prefix}2026-03-01&with_labels_details=true&per_page=100"
            )));
            for bad_date in [
                "2026-03-01T00:00:00Z",
                "2026-03-01T00%3A00",
                "2026-03-0",
                "2026-3-01",
                "2026/03/01",
                "2026-03-0a",
                "20260301xx",
                "",
                "2026-03-01 ",
                "2026-03-01&x=1",
            ] {
                assert!(
                    !guard_accepts_endpoint(&format!(
                        "{prefix}{bad_date}&with_labels_details=true&per_page=100"
                    )),
                    "{prefix}{bad_date}"
                );
            }
            // missing or altered tail
            assert!(!guard_accepts_endpoint(&format!("{prefix}2026-03-01")));
            assert!(!guard_accepts_endpoint(&format!(
                "{prefix}2026-03-01&per_page=50"
            )));
            assert!(!guard_accepts_endpoint(&format!(
                "{prefix}2026-03-01&with_labels_details=true&per_page=100&extra=1"
            )));
            assert!(!guard_accepts_endpoint(&format!(
                "{prefix}2026-03-01&extra=1&with_labels_details=true&per_page=100"
            )));
        }
        // missing updated_after
        assert!(!guard_accepts_endpoint(
            "issues?scope=assigned_to_me&state=closed&with_labels_details=true&per_page=100"
        ));
        // wrong state for the resource, other scope or resource
        for ep in [
            "issues?scope=assigned_to_me&state=all&updated_after=2026-03-01&with_labels_details=true&per_page=100",
            "merge_requests?scope=assigned_to_me&state=closed&updated_after=2026-03-01&with_labels_details=true&per_page=100",
            "issues?scope=assigned_to_me&state=opened&updated_after=2026-03-01&with_labels_details=true&per_page=100",
            "issues?scope=created_by_me&state=closed&updated_after=2026-03-01&with_labels_details=true&per_page=100",
            "projects?scope=assigned_to_me&state=closed&updated_after=2026-03-01&with_labels_details=true&per_page=100",
            "issues?scope=all&state=closed&updated_after=2026-03-01&with_labels_details=true&per_page=100",
            "issues?scope=all&state=closed&assignee_username=&updated_after=2026-03-01&with_labels_details=true&per_page=100",
            "issues?scope=all&state=closed&assignee_username=a&b&updated_after=2026-03-01&with_labels_details=true&per_page=100",
            "issues?scope=all&state=closed&assignee_username=a:b&updated_after=2026-03-01&with_labels_details=true&per_page=100",
            "issues?scope=all&state=closed&assignee_username=-a&updated_after=2026-03-01&with_labels_details=true&per_page=100",
        ] {
            assert!(!guard_accepts_endpoint(ep), "{ep}");
        }
        // The date goes through the guard's own check, not the type's.
        for flag in WRITE_FLAGS {
            assert!(
                check_read_only(&[
                    "api",
                    &format!("{fixed_issues}2026-03-01&with_labels_details=true&per_page=100"),
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

    proptest::proptest! {
        /// Every date the type can render passes the independent guard.
        #[test]
        fn every_updated_after_passes_the_guard(
            y in 1000i32..=9999,
            m in 1u32..=12,
            d in 1u32..=28,
        ) {
            let after = UpdatedAfter::new(chrono::NaiveDate::from_ymd_opt(y, m, d).unwrap());
            for call in [
                CliCall::GlabRecentlyClosedIssues(example_host(), None, after),
                CliCall::GlabRecentlyClosedMergeRequests(example_host(), Some(example_user()), after),
            ] {
                let args = call.args();
                let args: Vec<&str> = args.iter().map(String::as_str).collect();
                proptest::prop_assert!(check_read_only(&args).is_ok(), "{:?}", args);
            }
        }
    }

    #[test]
    fn guard_requires_the_exact_labels_details_parameter() {
        const GOOD: &str = "with_labels_details=true&per_page=100";
        let accepted = [
            format!("issues?scope=assigned_to_me&state=opened&{GOOD}"),
            format!("merge_requests?scope=assigned_to_me&state=opened&{GOOD}"),
            format!("issues?scope=all&state=opened&assignee_username=invented.user&{GOOD}"),
            format!("merge_requests?scope=all&state=opened&assignee_username=invented.user&{GOOD}"),
            format!("issues?scope=assigned_to_me&state=closed&updated_after=2026-03-01&{GOOD}"),
            format!(
                "merge_requests?scope=all&state=all&assignee_username=invented.user&updated_after=2026-03-01&{GOOD}"
            ),
        ];
        for ep in &accepted {
            assert!(guard_accepts_endpoint(ep), "{ep}");
        }
        let rejected = [
            // the old shape without the parameter
            "issues?scope=assigned_to_me&state=opened&per_page=100",
            "merge_requests?scope=assigned_to_me&state=opened&per_page=100",
            "issues?scope=all&state=opened&assignee_username=invented.user&per_page=100",
            "merge_requests?scope=all&state=opened&assignee_username=invented.user&per_page=100",
            "issues?scope=assigned_to_me&state=closed&updated_after=2026-03-01&per_page=100",
            "merge_requests?scope=all&state=all&assignee_username=invented.user&updated_after=2026-03-01&per_page=100",
            // a different value
            "issues?scope=assigned_to_me&state=opened&with_labels_details=false&per_page=100",
            "issues?scope=all&state=opened&assignee_username=invented.user&with_labels_details=false&per_page=100",
            "issues?scope=assigned_to_me&state=closed&updated_after=2026-03-01&with_labels_details=false&per_page=100",
            // an extra parameter next to it
            "issues?scope=assigned_to_me&state=opened&with_labels_details=true&extra=1&per_page=100",
            "issues?scope=all&state=opened&assignee_username=invented.user&with_labels_details=true&extra=1&per_page=100",
            "issues?scope=assigned_to_me&state=closed&updated_after=2026-03-01&with_labels_details=true&extra=1&per_page=100",
            // the parameter in another place
            "issues?scope=assigned_to_me&with_labels_details=true&state=opened&per_page=100",
            "issues?with_labels_details=true&scope=all&state=opened&assignee_username=invented.user&per_page=100",
            "issues?scope=assigned_to_me&state=closed&with_labels_details=true&updated_after=2026-03-01&per_page=100",
            "issues?scope=assigned_to_me&state=opened&per_page=100&with_labels_details=true",
            // the notes call takes no such parameter
            "projects/77/issues/12/notes?sort=desc&order_by=created_at&with_labels_details=true&per_page=100",
        ];
        for ep in rejected {
            assert!(!guard_accepts_endpoint(ep), "{ep}");
        }
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
