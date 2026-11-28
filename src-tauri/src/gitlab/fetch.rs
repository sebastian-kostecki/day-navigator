//! Runs the two assigned-items calls for one configured GitLab. The only GitLab
//! code that touches processes; blocking like the runner, so callers keep it
//! off the main thread.

use std::cmp::Reverse;

use chrono::{DateTime, Duration, FixedOffset, Local, NaiveDate, TimeZone};

use super::notes::{IssueNote, parse_notes};
use super::{
    AssignedItems, ClosedItem, GitlabError, ItemKind, parse_issues, parse_merge_requests,
    parse_recently_closed, split_review_requests,
};
use crate::cli::{CliCall, CliRunner, GitlabConfig, GitlabHost, IssueRef, UpdatedAfter};

/// Merge requests, then issues, then the review list. The first failure ends
/// the round and the later calls are not started: no partial list. With a
/// configured user the calls list that user's items instead of the
/// logged-in account's.
pub fn fetch_assigned(
    runner: &CliRunner,
    config: &GitlabConfig,
) -> Result<AssignedItems, GitlabError> {
    let host = &config.host;
    let mrs_out = runner.run(&CliCall::GlabAssignedMergeRequests(
        host.clone(),
        config.user.clone(),
    ))?;
    let mut merge_requests = parse_merge_requests(&mrs_out, host)?;
    let issues_out = runner.run(&CliCall::GlabAssignedIssues(
        host.clone(),
        config.user.clone(),
    ))?;
    let issues = parse_issues(&issues_out, host)?;
    let reviews_out = runner.run(&CliCall::GlabReviewMergeRequests(
        host.clone(),
        config.user.clone(),
    ))?;
    let reviews = parse_merge_requests(&reviews_out, host)?;
    let review_requests = split_review_requests(&mut merge_requests, reviews);
    Ok(AssignedItems {
        merge_requests,
        issues,
        review_requests,
        hidden_issues: Vec::new(),
    })
}

/// The window of the "recently closed" section: local midnight `days` days ago
/// as the cutoff, and the day before that date as the server-side pre-filter, so
/// the UTC reading of a date-only `updated_after` never cuts an item off. `None`
/// only when the calendar cannot express the date.
pub fn closed_window(today: NaiveDate, days: u32) -> Option<(UpdatedAfter, DateTime<FixedOffset>)> {
    let date = today.checked_sub_signed(Duration::days(i64::from(days)))?;
    // A local midnight skipped by a clock change falls back to the next hour.
    let cutoff = (0..=3).find_map(|hour| {
        Local
            .from_local_datetime(&date.and_hms_opt(hour, 0, 0)?)
            .earliest()
    })?;
    let after = date.pred_opt()?;
    Some((UpdatedAfter::new(after), cutoff.fixed_offset()))
}

/// The assigned items closed (issues) or closed and merged (MRs) since
/// `cutoff`: merge requests first, then issues, the first failure ends it (no
/// partial list). Newest closing first, ties by kind (MRs first), then reference.
/// `after` only narrows the server-side query; the closing time decides and is
/// kept on each item for the row.
pub fn fetch_recently_closed(
    runner: &CliRunner,
    config: &GitlabConfig,
    after: UpdatedAfter,
    cutoff: DateTime<FixedOffset>,
) -> Result<Vec<ClosedItem>, GitlabError> {
    let host = &config.host;
    let mrs_out = runner.run(&CliCall::GlabRecentlyClosedMergeRequests(
        host.clone(),
        config.user.clone(),
        after,
    ))?;
    let mut closed = parse_recently_closed(&mrs_out, host, ItemKind::MergeRequest, cutoff)?;
    let issues_out = runner.run(&CliCall::GlabRecentlyClosedIssues(
        host.clone(),
        config.user.clone(),
        after,
    ))?;
    closed.extend(parse_recently_closed(
        &issues_out,
        host,
        ItemKind::Issue,
        cutoff,
    )?);
    closed.sort_by(|a, b| {
        (Reverse(a.0), kind_rank(a.1.kind), &a.1.reference).cmp(&(
            Reverse(b.0),
            kind_rank(b.1.kind),
            &b.1.reference,
        ))
    });
    Ok(closed
        .into_iter()
        .map(|(at, item)| ClosedItem {
            item,
            closed_at: at.to_rfc3339(),
        })
        .collect())
}

fn kind_rank(kind: ItemKind) -> u8 {
    match kind {
        ItemKind::MergeRequest => 0,
        ItemKind::Issue => 1,
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn the_window_starts_at_local_midnight_and_pre_filters_a_day_earlier() {
        let today = NaiveDate::from_ymd_opt(2026, 3, 10).unwrap();
        let (after, cutoff) = closed_window(today, 7).unwrap();
        // Cutoff: local midnight on 2026-03-03; pre-filter: 2026-03-02.
        assert_eq!(after.render(), "2026-03-02");
        let local = cutoff.with_timezone(&Local);
        assert_eq!(
            local.date_naive(),
            NaiveDate::from_ymd_opt(2026, 3, 3).unwrap()
        );
        assert_eq!((local.hour(), local.minute(), local.second()), (0, 0, 0));
    }

    #[test]
    fn the_window_of_one_day_reaches_back_to_yesterday() {
        let today = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let (after, cutoff) = closed_window(today, 1).unwrap();
        assert_eq!(after.render(), "2026-02-27");
        assert_eq!(
            cutoff.with_timezone(&Local).date_naive(),
            NaiveDate::from_ymd_opt(2026, 2, 28).unwrap()
        );
    }
}
