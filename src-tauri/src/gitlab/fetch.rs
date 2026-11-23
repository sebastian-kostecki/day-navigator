//! Runs the two assigned-items calls for one configured GitLab. The only GitLab
//! code that touches processes; blocking like the runner, so callers keep it
//! off the main thread.

use super::notes::{IssueNote, parse_notes};
use super::{AssignedItems, GitlabError, parse_issues, parse_merge_requests};
use crate::cli::{CliCall, CliRunner, GitlabConfig, GitlabHost, IssueRef};

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
        hidden_issues: Vec::new(),
    })
}

/// One issue's notes, newest first, read in full across pages. Read-only; the
/// caller decides what a failure means (the list round does not depend on it).
pub fn fetch_issue_notes(
    runner: &CliRunner,
    host: &GitlabHost,
    issue: IssueRef,
) -> Result<Vec<IssueNote>, GitlabError> {
    let out = runner.run(&CliCall::GlabIssueNotes(host.clone(), issue))?;
    parse_notes(&out)
}
