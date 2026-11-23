//! Runs the two assigned-items calls for one configured GitLab. The only GitLab
//! code that touches processes; blocking like the runner, so callers keep it
//! off the main thread.

use super::{AssignedItems, GitlabError, parse_issues, parse_merge_requests};
use crate::cli::{CliCall, CliRunner, GitlabConfig};

/// Merge requests first, then issues. The first failure ends the round and the
/// second call is not started: no partial list. With a configured user the
/// calls list that user's items instead of the logged-in account's.
pub fn fetch_assigned(
    runner: &CliRunner,
    config: &GitlabConfig,
) -> Result<AssignedItems, GitlabError> {
    let host = &config.host;
    let mrs_out = runner.run(&CliCall::GlabAssignedMergeRequests(
        host.clone(),
        config.user.clone(),
    ))?;
    let merge_requests = parse_merge_requests(&mrs_out, host)?;
    let issues_out = runner.run(&CliCall::GlabAssignedIssues(
        host.clone(),
        config.user.clone(),
    ))?;
    let issues = parse_issues(&issues_out, host)?;
    Ok(AssignedItems {
        merge_requests,
        issues,
    })
}
