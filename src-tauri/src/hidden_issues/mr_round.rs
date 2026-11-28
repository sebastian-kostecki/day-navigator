//! One refresh round's hidden-merge-request reconciliation (S-07), the merge
//! request counterpart of `round.rs`: `check_mr_round` reads the MR records and
//! runs the due notes calls (no lock held); `commit_mr_round` plans the
//! transitions, applies them in one store write (`hidden_items`) and partitions
//! both merge request lists from the rows that write returned.
//!
//! Logging: kinds and plain counts only.

use std::collections::HashMap;

use chrono::Local;

use super::items::{HiddenItemRecord, HiddenKind, HiddenSource, apply_items_round};
use super::merge_requests::{list_mr_records, partition_merge_requests};
use super::round::{check_order, stops_round};
use super::rules::HideRecord;
use super::{
    Comment, CommentVerdict, HiddenIssueError, comment_verdict, needs_comment_check, plan_round,
    status_changed,
};
use crate::cli::{CliRunner, GitlabConfig, GitlabHost, GitlabUser, IssueRef};
use crate::gitlab::fetch::fetch_merge_request_notes;
use crate::gitlab::{AssignedItems, GitlabItem};
use crate::store::StoreHandle;

/// What `check_mr_round` found: the records read before the notes calls, one
/// verdict per checked merge request, and the merge request whose failed call
/// stopped the round (if any).
pub struct MrRoundChecks {
    records: Result<Vec<HiddenItemRecord>, HiddenIssueError>,
    checks: HashMap<u64, Result<CommentVerdict, ()>>,
    stopped_at: Option<u64>,
}

impl std::fmt::Debug for MrRoundChecks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MrRoundChecks")
            .field("records_ok", &self.records.is_ok())
            .field("checks", &self.checks.len())
            .field("stopped", &self.stopped_at.is_some())
            .finish()
    }
}

impl MrRoundChecks {
    /// The merge request whose notes call stopped the round. The next round
    /// passes it to `check_mr_round` as `resume_after`.
    pub fn stopped_at(&self) -> Option<u64> {
        self.stopped_at
    }
}

/// What the round shows: both partitioned merge request lists, the hidden
/// assigned ones first in `hidden_merge_requests` with the hidden review-only
/// ones after them, how many comment checks failed, and the store failure (if
/// any) that made the lists unpartitioned.
#[derive(Debug)]
pub struct MrRoundOutcome {
    pub merge_requests: Vec<GitlabItem>,
    pub review_requests: Vec<GitlabItem>,
    pub hidden_merge_requests: Vec<GitlabItem>,
    pub check_failed: u32,
    pub store_error: Option<HiddenIssueError>,
}

/// Whether the merge request notes calls may run this round. They are skipped
/// after the issue checks stopped this round (no VPN, logged out, no `glab`),
/// so a VPN drop costs one timeout per round, not two. Never two rounds in a
/// row: one issue call that keeps failing must not starve the MR checks.
pub fn mr_calls_allowed(issue_checks_stopped: bool, skipped_last_round: bool) -> bool {
    !issue_checks_stopped || skipped_last_round
}

/// The listed merge requests a record can match: assigned, then review-only.
fn listed_mrs(full: &AssignedItems) -> impl Iterator<Item = &GitlabItem> {
    full.merge_requests
        .iter()
        .chain(full.review_requests.iter())
}

/// Reads the MR records and runs the notes calls of the hidden merge requests
/// that are due. Holds no lock. After the first unavailable, logged-out or
/// missing-tool failure no further call starts and every remaining due check
/// counts as failed. With `calls_allowed == false` no call starts at all: every
/// due check fails and `stopped_at` keeps `resume_after`, so the cursor survives
/// a skipped round. A store read failure is carried in the result and skips the
/// calls.
pub fn check_mr_round(
    store: &StoreHandle,
    runner: &CliRunner,
    config: &GitlabConfig,
    full: &AssignedItems,
    now_ms: i64,
    resume_after: Option<u64>,
    calls_allowed: bool,
) -> MrRoundChecks {
    let today = Local::now().date_naive();
    let records = match store.with_store(today, |s| list_mr_records(s, &config.host)) {
        Ok(records) => records,
        Err(e) => {
            return MrRoundChecks {
                records: Err(e),
                checks: HashMap::new(),
                stopped_at: None,
            };
        }
    };
    let mut checks = HashMap::new();
    let mut failed = 0u32;
    let mut stopped_at = None;
    for rec in check_order(&records, resume_after) {
        let Some(item) = listed_mrs(full).find(|i| i.id == rec.item_id) else {
            continue;
        };
        // A status change (the fingerprint of an MR is empty) never fires here.
        if status_changed(rec, item) || !needs_comment_check(rec, item, now_ms) {
            continue;
        }
        // An unreadable baseline cannot be compared: a failed check, never a guess.
        let verdict = match rec.baseline() {
            Some(baseline) if calls_allowed && stopped_at.is_none() => {
                let mr = IssueRef {
                    project_id: item.project_id,
                    iid: item.iid,
                };
                match fetch_merge_request_notes(runner, &config.host, mr) {
                    Ok(notes) => {
                        let comments: Vec<Comment> = notes.iter().map(Comment::from).collect();
                        Ok(comment_verdict(
                            &comments,
                            baseline,
                            config.user.as_ref().map(GitlabUser::as_str),
                        ))
                    }
                    Err(e) => {
                        if stops_round(&e) {
                            stopped_at = Some(rec.item_id);
                        }
                        Err(())
                    }
                }
            }
            _ => Err(()),
        };
        if verdict.is_err() {
            failed += 1;
        }
        checks.insert(rec.item_id, verdict);
    }
    if failed > 0 {
        log::debug!("gitlab: hidden mr comment checks failed: {failed}");
    }
    MrRoundChecks {
        records: Ok(records),
        checks,
        stopped_at: if calls_allowed {
            stopped_at
        } else {
            resume_after
        },
    }
}

/// Plans and applies the round's merge request transitions in one store write,
/// then splits both lists from the rows that write returned, for this host only.
/// When the store cannot be read or written both lists are returned as listed
/// (unpartitioned) with `store_error` set: never an empty list in place of a
/// failed source.
pub fn commit_mr_round(
    store: &StoreHandle,
    host: &GitlabHost,
    full: &AssignedItems,
    checks: MrRoundChecks,
    now_ms: i64,
) -> MrRoundOutcome {
    let unpartitioned = |error: HiddenIssueError| {
        log::warn!("gitlab: hidden mr state unavailable: {}", error.kind());
        let as_listed = |items: &[GitlabItem]| -> Vec<GitlabItem> {
            items
                .iter()
                .map(|i| GitlabItem {
                    changed: None,
                    hidden_at_ms: None,
                    ..i.clone()
                })
                .collect()
        };
        MrRoundOutcome {
            merge_requests: as_listed(&full.merge_requests),
            review_requests: as_listed(&full.review_requests),
            hidden_merge_requests: Vec::new(),
            check_failed: 0,
            store_error: Some(error),
        }
    };
    let MrRoundChecks {
        records,
        checks: verdicts,
        ..
    } = checks;
    let records = match records {
        Ok(records) => records,
        Err(e) => return unpartitioned(e),
    };
    let listed: Vec<GitlabItem> = listed_mrs(full).cloned().collect();
    let transitions = plan_round(&records, &listed, &verdicts, now_ms);
    let today = Local::now().date_naive();
    let host_key = host.as_str().to_string();
    let planned = vec![(host_key.clone(), transitions)];
    let active = vec![host_key.clone()];
    let rows = match store.with_store(today, |s| {
        apply_items_round(
            s,
            HiddenSource::Gitlab,
            HiddenKind::MergeRequest,
            &planned,
            &active,
            now_ms,
        )
    }) {
        Ok(rows) => rows,
        Err(e) => return unpartitioned(e),
    };
    let rows: Vec<HiddenItemRecord> = rows.into_iter().filter(|r| r.scope == host_key).collect();
    let partition = partition_merge_requests(&full.merge_requests, &full.review_requests, &rows);
    // Only merge requests still hidden after the write count: one that came
    // back without notes (status, returned) is visible and needs no notice.
    let check_failed = verdicts
        .iter()
        .filter(|(id, v)| v.is_err() && rows.iter().any(|r| r.item_id == **id && r.is_hidden()))
        .count() as u32;
    MrRoundOutcome {
        merge_requests: partition.merge_requests,
        review_requests: partition.review_requests,
        hidden_merge_requests: partition.hidden,
        check_failed,
        store_error: None,
    }
}
