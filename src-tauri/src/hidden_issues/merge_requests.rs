//! GitLab merge request hide records (S-07): rows of `hidden_items` with
//! `source = gitlab`, `kind = mergeRequest` and the GitLab host as scope. Also
//! the two GitLab kinds' records read together (`GitlabHidden`) and the split
//! of both merge request lists by those records. `tauri`-free and clock-free.

use super::items::{
    HiddenItemKey, HiddenItemRecord, HiddenKind, HiddenSource, NewHiddenItem, list_items,
};
use super::rules::{HideItem, HideRecord};
use super::{HiddenIssueError, HiddenRecord, list};
use crate::cli::GitlabHost;
use crate::gitlab::GitlabItem;
use crate::store::Store;

/// The GitLab hide records of one host, both kinds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitlabHidden {
    pub issues: Vec<HiddenRecord>,
    pub merge_requests: Vec<HiddenItemRecord>,
}

/// The merge request records of one host. Rows of other hosts are not returned.
pub fn list_mr_records(
    store: &Store,
    host: &GitlabHost,
) -> Result<Vec<HiddenItemRecord>, HiddenIssueError> {
    Ok(
        list_items(store, HiddenSource::Gitlab, HiddenKind::MergeRequest)?
            .into_iter()
            .filter(|r| r.scope == host.as_str())
            .collect(),
    )
}

/// Both GitLab kinds' records of one host; the first read error wins. Both
/// halves fail together on purpose: one store, one notice. A failed merge
/// request read therefore also drops the issue records of that call (the
/// callers keep the list they already show and surface the failure).
pub fn list_gitlab(store: &Store, host: &GitlabHost) -> Result<GitlabHidden, HiddenIssueError> {
    let issues = list(store, host)?;
    let merge_requests = list_mr_records(store, host)?;
    Ok(GitlabHidden {
        issues,
        merge_requests,
    })
}

/// A fresh hide record for a listed merge request. Its fingerprint is empty:
/// labels never bring a merge request back.
pub fn new_hidden_mr(host: &GitlabHost, item: &GitlabItem) -> NewHiddenItem {
    NewHiddenItem {
        source: HiddenSource::Gitlab,
        scope: host.as_str().to_string(),
        kind: HiddenKind::MergeRequest,
        item_id: item.id,
        project_id: Some(item.project_id),
        iid: Some(item.iid),
        status_fingerprint: item.status_fingerprint(),
        baseline_updated_at: item.updated_at.clone(),
    }
}

/// The key of one GitLab merge request record (global id).
pub fn mr_key(host: &GitlabHost, id: u64) -> HiddenItemKey {
    HiddenItemKey {
        source: HiddenSource::Gitlab,
        scope: host.as_str().to_string(),
        kind: HiddenKind::MergeRequest,
        item_id: id,
    }
}

/// The two merge request lists after the hide records were applied. `hidden`
/// holds the hidden assigned merge requests first, then the hidden review-only
/// ones, each in list order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MrPartition {
    pub merge_requests: Vec<GitlabItem>,
    pub review_requests: Vec<GitlabItem>,
    pub hidden: Vec<GitlabItem>,
}

enum Placed {
    Visible(GitlabItem),
    Hidden(GitlabItem),
}

fn place(item: &GitlabItem, records: &[HiddenItemRecord]) -> Placed {
    let record = records.iter().find(|r| r.item_id == item.id);
    match record {
        None => Placed::Visible(GitlabItem {
            changed: None,
            hidden_at_ms: None,
            ..item.clone()
        }),
        Some(r) if r.is_hidden() => Placed::Hidden(GitlabItem {
            hidden_at_ms: Some(r.hidden_at_ms),
            ..item.clone()
        }),
        Some(r) => Placed::Visible(GitlabItem {
            changed: r.changed_reason,
            hidden_at_ms: None,
            ..item.clone()
        }),
    }
}

/// Splits both merge request lists by the same records, matched by global id.
/// Marked records stay visible with their reason. Order is kept in all three
/// lists and every other field, `review_requested` included, is kept: a hidden
/// MR that awaits the user's review shows the "Do review" tag in the main
/// table whether it is assigned or review-only.
pub fn partition_merge_requests(
    merge_requests: &[GitlabItem],
    review_requests: &[GitlabItem],
    records: &[HiddenItemRecord],
) -> MrPartition {
    debug_assert!(
        merge_requests
            .iter()
            .all(|a| review_requests.iter().all(|b| a.id != b.id)),
        "the two merge request lists must not share an id"
    );
    let mut visible_assigned = Vec::new();
    let mut hidden_assigned = Vec::new();
    for item in merge_requests {
        match place(item, records) {
            Placed::Visible(i) => visible_assigned.push(i),
            Placed::Hidden(i) => hidden_assigned.push(i),
        }
    }
    let mut visible_review = Vec::new();
    let mut hidden_review = Vec::new();
    for item in review_requests {
        match place(item, records) {
            Placed::Visible(i) => visible_review.push(i),
            Placed::Hidden(i) => hidden_review.push(i),
        }
    }
    hidden_assigned.extend(hidden_review);
    MrPartition {
        merge_requests: visible_assigned,
        review_requests: visible_review,
        hidden: hidden_assigned,
    }
}
