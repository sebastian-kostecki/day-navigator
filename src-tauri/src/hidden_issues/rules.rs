//! Pure resurfacing rules (FR-003, US-02): no store, clock or process. `now_ms`
//! is a parameter; every instant in a record is UTC epoch milliseconds or the
//! RFC 3339 text exactly as the source sent it.
//!
//! The rules are source-neutral: a listed item is a `HideItem`, a stored record
//! a `HideRecord` and a comment a `Comment`. GitLab issues and GitHub issues
//! share this one copy.

use std::collections::HashMap;

use chrono::{DateTime, FixedOffset};

use super::items::HiddenItemRecord;
use crate::github::GithubIssue;
use crate::gitlab::notes::IssueNote;
use crate::gitlab::{ChangeReason, GitlabItem, ItemKind};

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

/// A listed item as the rules see it: a key within its scope, the listed
/// `updated_at` text and a status fingerprint.
pub trait HideItem {
    fn hide_id(&self) -> u64;
    fn updated_at(&self) -> &str;
    /// What the item is hidden under; a change brings it back. Empty when the
    /// source has no such signal.
    fn status_fingerprint(&self) -> Vec<String>;
}

/// A stored hide record as the rules see it.
pub trait HideRecord {
    fn item_id(&self) -> u64;
    fn fingerprint(&self) -> &[String];
    fn baseline_updated_at(&self) -> &str;
    fn settle_after_ms(&self) -> Option<i64>;
    fn hidden_at_ms(&self) -> i64;
    fn absent_since_ms(&self) -> Option<i64>;
    fn changed_reason(&self) -> Option<ChangeReason>;

    fn is_hidden(&self) -> bool {
        self.changed_reason().is_none()
    }

    /// The baseline as an instant; `None` when the stored text is unreadable.
    fn baseline(&self) -> Option<DateTime<FixedOffset>> {
        DateTime::parse_from_rfc3339(self.baseline_updated_at()).ok()
    }
}

impl HideItem for GitlabItem {
    fn hide_id(&self) -> u64 {
        self.id
    }
    fn updated_at(&self) -> &str {
        &self.updated_at
    }
    /// Scoped labels bring an issue back; labels never bring a merge request
    /// back (S-07).
    fn status_fingerprint(&self) -> Vec<String> {
        match self.kind {
            ItemKind::Issue => scoped_labels(self.label_names()),
            ItemKind::MergeRequest => Vec::new(),
        }
    }
}

impl HideItem for GithubIssue {
    fn hide_id(&self) -> u64 {
        self.number
    }
    fn updated_at(&self) -> &str {
        &self.updated_at
    }
    /// Labels never bring a GitHub issue back.
    fn status_fingerprint(&self) -> Vec<String> {
        Vec::new()
    }
}

impl HideRecord for HiddenRecord {
    fn item_id(&self) -> u64 {
        self.issue_id
    }
    fn fingerprint(&self) -> &[String] {
        &self.scoped_labels
    }
    fn baseline_updated_at(&self) -> &str {
        &self.baseline_updated_at
    }
    fn settle_after_ms(&self) -> Option<i64> {
        self.settle_after_ms
    }
    fn hidden_at_ms(&self) -> i64 {
        self.hidden_at_ms
    }
    fn absent_since_ms(&self) -> Option<i64> {
        self.absent_since_ms
    }
    fn changed_reason(&self) -> Option<ChangeReason> {
        self.changed_reason
    }
}

impl HideRecord for HiddenItemRecord {
    fn item_id(&self) -> u64 {
        self.item_id
    }
    fn fingerprint(&self) -> &[String] {
        &self.status_fingerprint
    }
    fn baseline_updated_at(&self) -> &str {
        &self.baseline_updated_at
    }
    fn settle_after_ms(&self) -> Option<i64> {
        self.settle_after_ms
    }
    fn hidden_at_ms(&self) -> i64 {
        self.hidden_at_ms
    }
    fn absent_since_ms(&self) -> Option<i64> {
        self.absent_since_ms
    }
    fn changed_reason(&self) -> Option<ChangeReason> {
        self.changed_reason
    }
}

/// One comment as the rules see it. `author` is `None` for a deleted account
/// or an unknown login: that is someone else than the user.
#[derive(Clone, PartialEq, Eq)]
pub struct Comment {
    pub author: Option<String>,
    /// `false` for a system note: it never counts.
    pub counts: bool,
    pub created_at: DateTime<FixedOffset>,
}

impl std::fmt::Debug for Comment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Comment")
            .field("counts", &self.counts)
            .finish_non_exhaustive()
    }
}

impl From<&IssueNote> for Comment {
    fn from(n: &IssueNote) -> Self {
        Comment {
            author: Some(n.author.clone()),
            counts: !n.system,
            created_at: n.created_at,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentVerdict {
    NewFromOthers,
    NothingNew,
}

/// What a round decided for one record. `hidden_at_ms` guards against a record
/// that was re-hidden while the round ran. `issue_id` is the item's key within
/// its scope (GitLab global id, GitHub issue number).
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

/// The status fingerprint differs from the one stored at hide time.
pub fn status_changed<R: HideRecord, I: HideItem>(record: &R, item: &I) -> bool {
    record.fingerprint() != item.status_fingerprint().as_slice()
}

/// The listed `updated_at` is strictly later than the baseline. Unreadable
/// timestamps count as moved, so the check errs towards looking.
fn updated_moved<R: HideRecord, I: HideItem>(record: &R, item: &I) -> bool {
    match (
        record.baseline(),
        DateTime::parse_from_rfc3339(item.updated_at()).ok(),
    ) {
        (Some(base), Some(listed)) => listed > base,
        _ => true,
    }
}

/// A hidden issue needs its comments read when its `updated_at` moved past the
/// baseline, or when its single settling check is due.
pub fn needs_comment_check<R: HideRecord, I: HideItem>(record: &R, item: &I, now_ms: i64) -> bool {
    if !record.is_hidden() {
        return false;
    }
    updated_moved(record, item) || record.settle_after_ms().is_some_and(|due| now_ms >= due)
}

/// A comment counts when it counts at all (not a system note), is strictly
/// newer than the baseline and is not by the user (ASCII case-insensitive). A
/// comment without an author is someone else's. Without a user, any author
/// counts.
pub fn comment_verdict(
    comments: &[Comment],
    baseline: DateTime<FixedOffset>,
    user: Option<&str>,
) -> CommentVerdict {
    let new = comments.iter().any(|c| {
        c.counts
            && c.created_at > baseline
            && match (user, c.author.as_deref()) {
                (Some(u), Some(a)) => !a.eq_ignore_ascii_case(u),
                _ => true,
            }
    });
    if new {
        CommentVerdict::NewFromOthers
    } else {
        CommentVerdict::NothingNew
    }
}

/// Decides every transition of one successful round. `checks` holds the
/// comments outcome per item key; a failed check (`Err`) yields nothing for
/// that item beyond what needs no comments (status, returned).
pub fn plan_round<R: HideRecord, I: HideItem>(
    records: &[R],
    listed: &[I],
    checks: &HashMap<u64, Result<CommentVerdict, ()>>,
    now_ms: i64,
) -> Vec<Transition> {
    let mut out = Vec::new();
    for rec in records {
        let id = rec.item_id();
        let Some(item) = listed.iter().find(|i| i.hide_id() == id) else {
            match rec.absent_since_ms() {
                None => out.push(Transition::MarkAbsent { issue_id: id }),
                Some(since) if now_ms - since >= PRUNE_AFTER_MS => {
                    out.push(Transition::Prune { issue_id: id })
                }
                Some(_) => {}
            }
            continue;
        };
        if !rec.is_hidden() {
            if rec.absent_since_ms().is_some() {
                out.push(Transition::ClearAbsent { issue_id: id });
            }
            continue;
        }
        let verdict = checks.get(&id);
        let reason = if status_changed(rec, item) {
            Some(ChangeReason::Status)
        } else if matches!(verdict, Some(Ok(CommentVerdict::NewFromOthers))) {
            Some(ChangeReason::Comment)
        } else if rec.absent_since_ms().is_some() {
            Some(ChangeReason::Returned)
        } else {
            None
        };
        if let Some(reason) = reason {
            out.push(Transition::Resurface {
                issue_id: id,
                hidden_at_ms: rec.hidden_at_ms(),
                reason,
            });
        } else if matches!(verdict, Some(Ok(CommentVerdict::NothingNew))) {
            let moved = updated_moved(rec, item);
            out.push(Transition::AdvanceBaseline {
                issue_id: id,
                hidden_at_ms: rec.hidden_at_ms(),
                updated_at: item.updated_at().to_string(),
                settle_after_ms: moved.then_some(now_ms + SETTLE_MS),
            });
        }
    }
    out
}
