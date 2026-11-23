//! Runs the two assigned-items calls for one host. The only GitLab code that
//! touches processes; blocking like the runner, so callers keep it off the
//! main thread.

use super::{AssignedItems, GitlabError, parse_issues, parse_merge_requests};
use crate::cli::{CliCall, CliRunner, GitlabHost};

/// Merge requests first, then issues. The first failure ends the round and the
/// second call is not started: no partial list.
pub fn fetch_assigned(runner: &CliRunner, host: &GitlabHost) -> Result<AssignedItems, GitlabError> {
    let mrs_out = runner.run(&CliCall::GlabAssignedMergeRequests(host.clone()))?;
    let merge_requests = parse_merge_requests(&mrs_out, host)?;
    let issues_out = runner.run(&CliCall::GlabAssignedIssues(host.clone()))?;
    let issues = parse_issues(&issues_out, host)?;
    Ok(AssignedItems {
        merge_requests,
        issues,
    })
}
