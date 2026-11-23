//! Reconciliation of hidden issues in one successful GitLab round, split in two
//! `tauri`-free halves so the refresh thread can keep the process calls outside
//! its lock:
//!
//! - `check_round` reads the records and runs the due notes calls (no lock held);
//! - `commit_round` plans the transitions, applies them in one store write and
//!   partitions the full list from the rows that write returned.
//!
//! Logging: kinds and plain counts only.

use std::collections::HashMap;

use chrono::Local;

use super::{
    CommentVerdict, HiddenIssueError, HiddenRecord, apply_round, comment_verdict, list,
    needs_comment_check, partition, plan_round, status_changed,
};
use crate::cli::{CliRunner, GitlabConfig, GitlabHost, IssueRef};
use crate::gitlab::fetch::fetch_issue_notes;
use crate::gitlab::{AssignedItems, GitlabError, GitlabItem};
use crate::store::StoreHandle;

/// What `check_round` found: the records read before the notes calls and one
/// verdict per checked issue.
#[derive(Debug)]
pub struct RoundChecks {
    records: Result<Vec<HiddenRecord>, HiddenIssueError>,
    checks: HashMap<u64, Result<CommentVerdict, ()>>,
    failed: u32,
}

/// What the round shows: the partitioned issues, how many comment checks
/// failed, and the store failure (if any) that made the list unpartitioned.
#[derive(Debug)]
pub struct RoundOutcome {
    pub visible_issues: Vec<GitlabItem>,
    pub hidden_issues: Vec<GitlabItem>,
    pub check_failed: u32,
    pub store_error: Option<HiddenIssueError>,
}

/// A failure after which more notes calls are pointless this round (no VPN,
/// logged out, no `glab`): a VPN drop costs one timeout, not one per issue.
fn stops_round(e: &GitlabError) -> bool {
    matches!(
        e,
        GitlabError::Unavailable | GitlabError::LoggedOut | GitlabError::ToolMissing
    )
}

/// Reads the records and runs the notes calls of the hidden issues that are due.
/// Holds no lock. After the first unavailable, logged-out or missing-tool failure
/// no further call starts and every remaining due issue counts as failed. A store
/// read failure is carried in the result and skips the calls.
pub fn check_round(
    store: &StoreHandle,
    runner: &CliRunner,
    config: &GitlabConfig,
    full: &AssignedItems,
    now_ms: i64,
) -> RoundChecks {
    let today = Local::now().date_naive();
    let records = match store.with_store(today, |s| list(s, &config.host)) {
        Ok(records) => records,
        Err(e) => {
            return RoundChecks {
                records: Err(e),
                checks: HashMap::new(),
                failed: 0,
            };
        }
    };
    let mut checks = HashMap::new();
    let mut failed = 0u32;
    let mut stopped = false;
    for rec in &records {
        let Some(item) = full.issues.iter().find(|i| i.id == rec.issue_id) else {
            continue;
        };
        // A scoped-label change resurfaces the issue whatever the comments say.
        if status_changed(rec, item) || !needs_comment_check(rec, item, now_ms) {
            continue;
        }
        // An unreadable baseline cannot be compared: a failed check, never a guess.
        let verdict = match rec.baseline() {
            Some(baseline) if !stopped => {
                let issue = IssueRef {
                    project_id: item.project_id,
                    iid: item.iid,
                };
                match fetch_issue_notes(runner, &config.host, issue) {
                    Ok(notes) => Ok(comment_verdict(&notes, baseline, config.user.as_ref())),
                    Err(e) => {
                        if stops_round(&e) {
                            stopped = true;
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
        checks.insert(rec.issue_id, verdict);
    }
    if failed > 0 {
        log::debug!("gitlab: hidden comment checks failed: {failed}");
    }
    RoundChecks {
        records: Ok(records),
        checks,
        failed,
    }
}

/// Plans and applies the round's transitions in one store write, then splits
/// the full issue list from the rows that write returned. When the store cannot
/// be read or written the list is returned unpartitioned with `store_error` set:
/// never an empty list in place of a failed source.
pub fn commit_round(
    store: &StoreHandle,
    host: &GitlabHost,
    full: &AssignedItems,
    checks: RoundChecks,
    now_ms: i64,
) -> RoundOutcome {
    let unpartitioned = |error: HiddenIssueError| {
        log::warn!("gitlab: hidden state unavailable: {}", error.kind());
        RoundOutcome {
            visible_issues: full
                .issues
                .iter()
                .map(|i| GitlabItem {
                    changed: None,
                    ..i.clone()
                })
                .collect(),
            hidden_issues: Vec::new(),
            check_failed: 0,
            store_error: Some(error),
        }
    };
    let RoundChecks {
        records,
        checks: verdicts,
        failed,
    } = checks;
    let records = match records {
        Ok(records) => records,
        Err(e) => return unpartitioned(e),
    };
    let transitions = plan_round(&records, &full.issues, &verdicts, now_ms);
    let today = Local::now().date_naive();
    let rows = match store.with_store(today, |s| apply_round(s, host, &transitions, now_ms)) {
        Ok(rows) => rows,
        Err(e) => return unpartitioned(e),
    };
    let (visible_issues, hidden_issues) = partition(&full.issues, &rows);
    RoundOutcome {
        visible_issues,
        hidden_issues,
        check_failed: failed,
        store_error: None,
    }
}
