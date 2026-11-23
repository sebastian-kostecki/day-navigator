//! Hidden GitLab issues (S-03): the hide records in the app data store and the
//! rules that bring an issue back.
//!
//! No `tauri` import and no clock: `now_ms` is a parameter. Every mutation is
//! one `Store::write`, so a refusal changes nothing. A record holds ids,
//! scoped labels and timestamps only: no title, URL, reference or comment text.
//! Phase 3 wires this into the refresh round and the commands.

pub mod error;
pub mod rules;

#[cfg(test)]
mod tests;

use rusqlite::{Connection, Row};

use crate::cli::GitlabHost;
use crate::gitlab::{ChangeReason, GitlabItem, ItemKind};
use crate::store::{Store, StoreError};
pub use error::HiddenIssueError;
pub use rules::{
    CommentVerdict, HiddenRecord, PRUNE_AFTER_MS, SETTLE_MS, Transition, comment_verdict,
    needs_comment_check, plan_round, scoped_labels, status_changed,
};

const COLUMNS: &str = "issue_id, project_id, iid, scoped_labels, baseline_updated_at, \
     settle_after_ms, hidden_at_ms, absent_since_ms, changed_reason, changed_at_ms";

fn reason_text(r: ChangeReason) -> &'static str {
    match r {
        ChangeReason::Status => "status",
        ChangeReason::Comment => "comment",
        ChangeReason::Returned => "returned",
    }
}

fn corrupt() -> HiddenIssueError {
    HiddenIssueError::Store(StoreError::Damaged {
        reason: "hidden issue record is unreadable".into(),
    })
}

/// Reads one row; `None` when a column holds something the rules cannot use.
fn read_row(r: &Row<'_>) -> rusqlite::Result<Option<HiddenRecord>> {
    let labels: String = r.get(3)?;
    let reason: Option<String> = r.get(8)?;
    let changed_reason = match reason.as_deref() {
        None => None,
        Some("status") => Some(ChangeReason::Status),
        Some("comment") => Some(ChangeReason::Comment),
        Some("returned") => Some(ChangeReason::Returned),
        Some(_) => return Ok(None),
    };
    let Ok(scoped_labels) = serde_json::from_str::<Vec<String>>(&labels) else {
        return Ok(None);
    };
    Ok(Some(HiddenRecord {
        issue_id: r.get::<_, i64>(0)? as u64,
        project_id: r.get::<_, i64>(1)? as u64,
        iid: r.get::<_, i64>(2)? as u64,
        scoped_labels,
        baseline_updated_at: r.get(4)?,
        settle_after_ms: r.get(5)?,
        hidden_at_ms: r.get(6)?,
        absent_since_ms: r.get(7)?,
        changed_reason,
        changed_at_ms: r.get(9)?,
    }))
}

fn records(conn: &Connection, host: &GitlabHost) -> Result<Vec<HiddenRecord>, HiddenIssueError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM hidden_issues WHERE host = ?1 ORDER BY issue_id"
    ))?;
    let rows = stmt.query_map([host.as_str()], read_row)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?.ok_or_else(corrupt)?);
    }
    Ok(out)
}

/// The host's records: hidden ones and ones carrying a mark.
pub fn list(store: &Store, host: &GitlabHost) -> Result<Vec<HiddenRecord>, HiddenIssueError> {
    store.read(|c| records(c, host))
}

/// Hides an issue with a fresh snapshot; replaces a marked record. One
/// settling comment check is scheduled.
pub fn hide(
    store: &mut Store,
    host: &GitlabHost,
    item: &GitlabItem,
    now_ms: i64,
) -> Result<(), HiddenIssueError> {
    if item.kind != ItemKind::Issue {
        return Err(HiddenIssueError::NotInList);
    }
    let labels = serde_json::to_string(&scoped_labels(&item.labels)).map_err(|_| corrupt())?;
    store.write(|tx| {
        tx.execute(
            "INSERT INTO hidden_issues(host, issue_id, project_id, iid, scoped_labels, \
                 baseline_updated_at, settle_after_ms, hidden_at_ms, absent_since_ms, \
                 changed_reason, changed_at_ms) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, NULL, NULL) \
             ON CONFLICT(host, issue_id) DO UPDATE SET \
                 project_id = excluded.project_id, iid = excluded.iid, \
                 scoped_labels = excluded.scoped_labels, \
                 baseline_updated_at = excluded.baseline_updated_at, \
                 settle_after_ms = excluded.settle_after_ms, \
                 hidden_at_ms = excluded.hidden_at_ms, absent_since_ms = NULL, \
                 changed_reason = NULL, changed_at_ms = NULL",
            (
                host.as_str(),
                item.id as i64,
                item.project_id as i64,
                item.iid as i64,
                &labels,
                &item.updated_at,
                now_ms + SETTLE_MS,
                now_ms,
            ),
        )?;
        Ok(())
    })
}

fn delete_where(
    store: &mut Store,
    host: &GitlabHost,
    issue_id: u64,
    marked: bool,
) -> Result<(), HiddenIssueError> {
    store.write(|tx| {
        let sql = if marked {
            "DELETE FROM hidden_issues WHERE host = ?1 AND issue_id = ?2 \
             AND changed_reason IS NOT NULL"
        } else {
            "DELETE FROM hidden_issues WHERE host = ?1 AND issue_id = ?2 \
             AND changed_reason IS NULL"
        };
        if tx.execute(sql, (host.as_str(), issue_id as i64))? == 0 {
            return Err(HiddenIssueError::NotFound);
        }
        Ok(())
    })
}

/// Deletes a hidden record; anything else is `NotFound`.
pub fn unhide(store: &mut Store, host: &GitlabHost, issue_id: u64) -> Result<(), HiddenIssueError> {
    delete_where(store, host, issue_id, false)
}

/// Deletes a marked record; anything else is `NotFound`.
pub fn dismiss(
    store: &mut Store,
    host: &GitlabHost,
    issue_id: u64,
) -> Result<(), HiddenIssueError> {
    delete_where(store, host, issue_id, true)
}

/// Applies a round's transitions in one write. Each transition is applied only
/// if its row is still in the state the plan assumed (hidden with the same
/// `hidden_at_ms`, absent or not absent); otherwise it is skipped. Also deletes
/// the rows of every other host hidden `PRUNE_AFTER_MS` or more ago (their
/// absence is never judged). Returns the host's rows as they are after the write.
pub fn apply_round(
    store: &mut Store,
    host: &GitlabHost,
    transitions: &[Transition],
    now_ms: i64,
) -> Result<Vec<HiddenRecord>, HiddenIssueError> {
    let h = host.as_str();
    store.write(|tx| {
        for t in transitions {
            match t {
                Transition::Resurface {
                    issue_id,
                    hidden_at_ms,
                    reason,
                } => tx.execute(
                    "UPDATE hidden_issues SET changed_reason = ?1, changed_at_ms = ?2, \
                         absent_since_ms = NULL, settle_after_ms = NULL \
                     WHERE host = ?3 AND issue_id = ?4 AND changed_reason IS NULL \
                         AND hidden_at_ms = ?5",
                    (
                        reason_text(*reason),
                        now_ms,
                        h,
                        *issue_id as i64,
                        *hidden_at_ms,
                    ),
                )?,
                Transition::AdvanceBaseline {
                    issue_id,
                    hidden_at_ms,
                    updated_at,
                    settle_after_ms,
                } => tx.execute(
                    "UPDATE hidden_issues SET baseline_updated_at = ?1, settle_after_ms = ?2 \
                     WHERE host = ?3 AND issue_id = ?4 AND changed_reason IS NULL \
                         AND hidden_at_ms = ?5",
                    (
                        updated_at,
                        settle_after_ms,
                        h,
                        *issue_id as i64,
                        *hidden_at_ms,
                    ),
                )?,
                Transition::MarkAbsent { issue_id } => tx.execute(
                    "UPDATE hidden_issues SET absent_since_ms = ?1 \
                     WHERE host = ?2 AND issue_id = ?3 AND absent_since_ms IS NULL",
                    (now_ms, h, *issue_id as i64),
                )?,
                Transition::Prune { issue_id } => tx.execute(
                    "DELETE FROM hidden_issues \
                     WHERE host = ?1 AND issue_id = ?2 AND absent_since_ms IS NOT NULL \
                         AND ?3 - absent_since_ms >= ?4",
                    (h, *issue_id as i64, now_ms, PRUNE_AFTER_MS),
                )?,
                Transition::ClearAbsent { issue_id } => tx.execute(
                    "UPDATE hidden_issues SET absent_since_ms = NULL \
                     WHERE host = ?1 AND issue_id = ?2 AND absent_since_ms IS NOT NULL",
                    (h, *issue_id as i64),
                )?,
            };
        }
        tx.execute(
            "DELETE FROM hidden_issues WHERE host <> ?1 AND ?2 - hidden_at_ms >= ?3",
            (h, now_ms, PRUNE_AFTER_MS),
        )?;
        records(tx, host)
    })
}

/// Splits the full issue list into visible issues (marked ones carry their
/// reason) and hidden ones. Both keep the list's order. Partition from the rows
/// `apply_round` returned, never from records read before the notes calls.
pub fn partition(
    full_issues: &[GitlabItem],
    records: &[HiddenRecord],
) -> (Vec<GitlabItem>, Vec<GitlabItem>) {
    let mut visible = Vec::new();
    let mut hidden = Vec::new();
    for issue in full_issues {
        match records.iter().find(|r| r.issue_id == issue.id) {
            None => visible.push(GitlabItem {
                changed: None,
                ..issue.clone()
            }),
            Some(r) if r.is_hidden() => hidden.push(issue.clone()),
            Some(r) => visible.push(GitlabItem {
                changed: r.changed_reason,
                ..issue.clone()
            }),
        }
    }
    (visible, hidden)
}
