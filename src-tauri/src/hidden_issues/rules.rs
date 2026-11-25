//! Pure resurfacing rules (FR-003, US-02): no store, clock or process. `now_ms`
//! is a parameter; every instant in a record is UTC epoch milliseconds or the
//! RFC 3339 text exactly as GitLab sent it.

use std::collections::HashMap;

use chrono::{DateTime, FixedOffset};

use crate::cli::GitlabUser;
use crate::gitlab::notes::IssueNote;
use crate::gitlab::{ChangeReason, GitlabItem};

/// One settling comment check after a new baseline: 60 s GitLab touch throttle
/// plus a skew margin.
pub const SETTLE_MS: i64 = 120_000;
/// An issue absent from the list this long loses its record.
pub const PRUNE_AFTER_MS: i64 = 30 * 86_400_000;

/// A stored hide record. `changed_reason == None` means the issue is hidden;
/// a reason means it is visible and carries a mark.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HiddenRecord {
    pub issue_id: u64,
    pub project_id: u64,
    pub iid: u64,
    /// Sorted, deduplicated `key::value` labels.
    pub scoped_labels: Vec<String>,
    pub baseline_updated_at: String,
    pub settle_after_ms: Option<i64>,
    pub hidden_at_ms: i64,
    pub absent_since_ms: Option<i64>,
    pub changed_reason: Option<ChangeReason>,
    pub changed_at_ms: Option<i64>,
}

impl HiddenRecord {
    pub fn is_hidden(&self) -> bool {
        self.changed_reason.is_none()
    }

    /// The baseline as an instant; `None` when the stored text is unreadable.
    pub fn baseline(&self) -> Option<DateTime<FixedOffset>> {
        DateTime::parse_from_rfc3339(&self.baseline_updated_at).ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentVerdict {
    NewFromOthers,
    NothingNew,
}

/// What a round decided for one record. `hidden_at_ms` guards against a record
/// that was re-hidden while the round ran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transition {
    Resurface {
        issue_id: u64,
        hidden_at_ms: i64,
        reason: ChangeReason,
    },
    AdvanceBaseline {
        issue_id: u64,
        hidden_at_ms: i64,
        updated_at: String,
        settle_after_ms: Option<i64>,
    },
    MarkAbsent {
        issue_id: u64,
    },
    Prune {
        issue_id: u64,
    },
    ClearAbsent {
        issue_id: u64,
    },
}

/// Labels containing `::`, sorted and deduplicated.
pub fn scoped_labels<'a>(names: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut out: Vec<String> = names
        .into_iter()
        .filter(|l| l.contains("::"))
        .map(str::to_string)
        .collect();
    out.sort();
    out.dedup();
    out
}

/// The scoped label set differs from the one stored at hide time.
pub fn status_changed(record: &HiddenRecord, item: &GitlabItem) -> bool {
    record.scoped_labels != scoped_labels(item.label_names())
}

/// The listed `updated_at` is strictly later than the baseline. Unreadable
/// timestamps count as moved, so the check errs towards looking.
fn updated_moved(record: &HiddenRecord, item: &GitlabItem) -> bool {
    match (
        record.baseline(),
        DateTime::parse_from_rfc3339(&item.updated_at).ok(),
    ) {
        (Some(base), Some(listed)) => listed > base,
        _ => true,
    }
}

/// A hidden issue needs its notes read when its `updated_at` moved past the
/// baseline, or when its single settling check is due.
pub fn needs_comment_check(record: &HiddenRecord, item: &GitlabItem, now_ms: i64) -> bool {
    if !record.is_hidden() {
        return false;
    }
    updated_moved(record, item) || record.settle_after_ms.is_some_and(|due| now_ms >= due)
}

/// A note counts when it is not a system note, is strictly newer than the
/// baseline and is not by the user (ASCII case-insensitive). Without a user,
/// any author counts.
pub fn comment_verdict(
    notes: &[IssueNote],
    baseline: DateTime<FixedOffset>,
    user: Option<&GitlabUser>,
) -> CommentVerdict {
    let new = notes.iter().any(|n| {
        !n.system
            && n.created_at > baseline
            && user.is_none_or(|u| !n.author.eq_ignore_ascii_case(u.as_str()))
    });
    if new {
        CommentVerdict::NewFromOthers
    } else {
        CommentVerdict::NothingNew
    }
}

/// Decides every transition of one successful round. `checks` holds the notes
/// outcome per issue id; a failed check (`Err`) yields nothing for that issue
/// beyond what needs no notes (status, returned).
pub fn plan_round(
    records: &[HiddenRecord],
    listed: &[GitlabItem],
    checks: &HashMap<u64, Result<CommentVerdict, ()>>,
    now_ms: i64,
) -> Vec<Transition> {
    let mut out = Vec::new();
    for rec in records {
        let id = rec.issue_id;
        let Some(item) = listed.iter().find(|i| i.id == id) else {
            match rec.absent_since_ms {
                None => out.push(Transition::MarkAbsent { issue_id: id }),
                Some(since) if now_ms - since >= PRUNE_AFTER_MS => {
                    out.push(Transition::Prune { issue_id: id })
                }
                Some(_) => {}
            }
            continue;
        };
        if !rec.is_hidden() {
            if rec.absent_since_ms.is_some() {
                out.push(Transition::ClearAbsent { issue_id: id });
            }
            continue;
        }
        let verdict = checks.get(&id);
        let reason = if status_changed(rec, item) {
            Some(ChangeReason::Status)
        } else if matches!(verdict, Some(Ok(CommentVerdict::NewFromOthers))) {
            Some(ChangeReason::Comment)
        } else if rec.absent_since_ms.is_some() {
            Some(ChangeReason::Returned)
        } else {
            None
        };
        if let Some(reason) = reason {
            out.push(Transition::Resurface {
                issue_id: id,
                hidden_at_ms: rec.hidden_at_ms,
                reason,
            });
        } else if matches!(verdict, Some(Ok(CommentVerdict::NothingNew))) {
            let moved = updated_moved(rec, item);
            out.push(Transition::AdvanceBaseline {
                issue_id: id,
                hidden_at_ms: rec.hidden_at_ms,
                updated_at: item.updated_at.clone(),
                settle_after_ms: moved.then_some(now_ms + SETTLE_MS),
            });
        }
    }
    out
}
