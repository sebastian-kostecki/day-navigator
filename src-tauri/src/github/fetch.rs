//! Runs one `gh issue list` per configured repository. The only GitHub code
//! that touches processes; blocking like the runner, so callers keep it off the
//! main thread.

use super::{GithubError, GithubIssue, parse_issues};
use crate::cli::{CliCall, CliRunner, GH_ISSUE_LIMIT, GithubConfig, GithubRepo};

/// The outcome of one repository in a round.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoResult {
    pub repo: GithubRepo,
    pub outcome: Result<Vec<GithubIssue>, GithubError>,
}

/// Fetches the repos in settings order. `Unavailable`, `LoggedOut` and
/// `ToolMissing` hit every later call the same way, so the first one ends the
/// round as a whole-source `Err` and no further call is started. `Failed`,
/// `ResponseInvalid` and a list that hit the call's limit (`Truncated`) belong
/// to that repo only; the loop goes on.
pub fn fetch_assigned(
    runner: &CliRunner,
    config: &GithubConfig,
) -> Result<Vec<RepoResult>, GithubError> {
    let mut results = Vec::with_capacity(config.repos.len());
    for repo in &config.repos {
        let outcome = match runner.run(&CliCall::GhAssignedIssues(repo.clone())) {
            Ok(stdout) => parse_issues(repo, &stdout).and_then(|issues| {
                if issues.len() >= GH_ISSUE_LIMIT {
                    Err(GithubError::Truncated)
                } else {
                    Ok(issues)
                }
            }),
            Err(e) => match GithubError::from(e) {
                whole @ (GithubError::Unavailable
                | GithubError::LoggedOut
                | GithubError::ToolMissing) => return Err(whole),
                per_repo => Err(per_repo),
            },
        };
        results.push(RepoResult {
            repo: repo.clone(),
            outcome,
        });
    }
    Ok(results)
}
