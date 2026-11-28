//! New GitLab assignments (S-04): the durable seen set and the rules that say
//! which items of a round are new.
//!
//! No `tauri` import and no clock: `now_ms` is a parameter. A round is one
//! `Store::write`. The seen set holds the opaque account key, kind, numeric id
//! and two instants: no title, reference, URL or hostname.
//!
//! Accepted by design (no replay of suppressed toasts):
//! - Delivery is at most once: a round's keys are stored before its toasts
//!   show, so a toast the OS drops, or a quit in between, is not repeated.
//! - A key the session fallback toasted while the store failed, which leaves
//!   the list before the store recovers, is never stored; if it returns later
//!   it notifies again.
//! - Rows and the seeding markers of an account key no longer in use (after a
//!   host or `gitlab.user` change) are never pruned; they are a few rows.
//!
//! Review requests (S-06) share the seen set: a merge request is one key
//! whether it is in the assigned list or the review list, so moving between
//! them never notifies again. The review set has its own seeding flag, a
//! marker row `review_marker_key(account)` in `assignment_accounts` (no
//! schema change); no `seen_assignments` row ever uses it. Like the account
//! marker it is never pruned. The first round after the flag is missing takes
//! the already-open review-only keys in silently.

pub mod error;
pub mod notice;
pub mod rules;

#[cfg(test)]
mod tests;

use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};

use chrono::DateTime;
use rusqlite::OptionalExtension;

use crate::gitlab::{AssignedItems, GitlabItem, ItemKind};
use crate::store::{Store, StoreError};
pub use error::NewAssignmentError;
pub use notice::notices_for;
pub use rules::{
    FORGET_AFTER_MS, ItemKey, RoundPlan, RoundSeen, SeenRow, baseline_first_seen, fresh_since,
    plan_round, review_baseline,
};

fn kind_text(kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Issue => "issue",
        ItemKind::MergeRequest => "mergeRequest",
    }
}

fn kind_from_text(text: &str) -> Option<ItemKind> {
    match text {
        "issue" => Some(ItemKind::Issue),
        "mergeRequest" => Some(ItemKind::MergeRequest),
        _ => None,
    }
}

fn all_items(items: &AssignedItems) -> impl Iterator<Item = &GitlabItem> {
    items
        .merge_requests
        .iter()
        .chain(&items.issues)
        .chain(&items.hidden_issues)
        .chain(&items.review_requests)
        .chain(&items.hidden_merge_requests)
}

fn valid_id(item: &GitlabItem) -> bool {
    i64::try_from(item.id).is_ok_and(|id| id > 0)
}

/// The keys of the review-only list, under the same id filter as `keys_of`.
pub fn review_only_keys(items: &AssignedItems) -> Vec<ItemKey> {
    items
        .review_requests
        .iter()
        .filter(|i| valid_id(i))
        .map(|i| ItemKey::new(i.kind, i.id))
        .collect()
}

/// The marker row in `assignment_accounts` that says the account's review set
/// is seeded. The account key is 64 hex characters, so the suffix cannot
/// collide with a real key; it stays opaque and no `seen_assignments` row uses
/// it.
pub fn review_marker_key(account: &str) -> String {
    format!("{account}:reviews")
}

/// The full assigned list: merge requests, visible issues and hidden issues.
/// An id outside `1..=i64::MAX` cannot be stored (a positive SQLite INTEGER):
/// GitLab never sends one, and skipping it keeps one bad item from failing
/// the whole round's write.
pub fn keys_of(items: &AssignedItems) -> Vec<ItemKey> {
    all_items(items)
        .filter(|i| valid_id(i))
        .map(|i| ItemKey::new(i.kind, i.id))
        .collect()
}

/// The items behind `keys`, one per key even when the fetched list repeats an
/// item (a paginated list can, when an item changes mid-fetch), newest
/// `updated_at` first across both kinds; ties keep the order of `keys`.
pub fn items_for<'a>(items: &'a AssignedItems, keys: &[ItemKey]) -> Vec<&'a GitlabItem> {
    let mut by_key: HashMap<ItemKey, &GitlabItem> = HashMap::new();
    for item in all_items(items) {
        by_key
            .entry(ItemKey::new(item.kind, item.id))
            .or_insert(item);
    }
    let mut taken = HashSet::new();
    let mut out: Vec<&GitlabItem> = keys
        .iter()
        .filter(|k| taken.insert(**k))
        .filter_map(|k| by_key.get(k).copied())
        .collect();
    // Parsed, not compared as text: GitLab may send different offsets.
    out.sort_by_key(|i| Reverse(DateTime::parse_from_rfc3339(&i.updated_at).ok()));
    out
}

/// Sets `is_new` on merge requests, review requests and visible issues from
/// `fresh`; hidden issues and hidden merge requests are never new.
pub fn mark_fresh(items: &mut AssignedItems, fresh: &HashSet<ItemKey>) {
    for item in items
        .merge_requests
        .iter_mut()
        .chain(&mut items.issues)
        .chain(&mut items.review_requests)
    {
        item.is_new = fresh.contains(&ItemKey::new(item.kind, item.id));
    }
    for item in items
        .hidden_issues
        .iter_mut()
        .chain(&mut items.hidden_merge_requests)
    {
        item.is_new = false;
    }
}

/// Moves the account's seen-until mark to `seen_until_ms`, never back: the
/// caller passes the instant of the round that was shown, not the clock.
pub fn mark_view_seen(
    store: &mut Store,
    account: &str,
    seen_until_ms: i64,
) -> Result<(), NewAssignmentError> {
    store.write::<_, NewAssignmentError>(|tx| {
        tx.execute(
            "INSERT INTO gitlab_view_marks (account, seen_until_ms) VALUES (?1, ?2)
             ON CONFLICT (account)
             DO UPDATE SET seen_until_ms = max(seen_until_ms, excluded.seen_until_ms)",
            rusqlite::params![account, seen_until_ms],
        )?;
        Ok(())
    })
}

/// Records one successful round for `account` and returns the keys that are
/// new (empty on the account's first round, which seeds silently) and the keys
/// that are fresh for the view (first seen after the account's seen-until
/// mark). One write: read the seeded flag, rows and mark, plan, insert the
/// marker when seeding, upsert every current key, then delete this account's
/// stale absent rows. Rows of other accounts are never touched.
///
/// `current` holds every key, including the review-only ones in `review_only`.
/// When the account is seeded but its review marker is missing, the review-only
/// keys not yet stored are a silent baseline: never new, never fresh, stored
/// with `first_seen_ms = min(mark, now)` and `last_seen_ms = now`.
pub fn record_round(
    store: &mut Store,
    account: &str,
    current: &[ItemKey],
    review_only: &[ItemKey],
    now_ms: i64,
) -> Result<RoundSeen, NewAssignmentError> {
    store.write::<_, NewAssignmentError>(|tx| {
        let seeded: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM assignment_accounts WHERE account = ?1)",
            [account],
            |r| r.get(0),
        )?;
        let review_marker = review_marker_key(account);
        let reviews_seeded: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM assignment_accounts WHERE account = ?1)",
            [&review_marker],
            |r| r.get(0),
        )?;

        let mut stored = Vec::new();
        let mut first_seen: HashMap<ItemKey, i64> = HashMap::new();
        {
            let mut stmt = tx.prepare(
                "SELECT kind, item_id, first_seen_ms, last_seen_ms FROM seen_assignments
                 WHERE account = ?1",
            )?;
            let rows = stmt.query_map([account], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, i64>(3)?,
                ))
            })?;
            for row in rows {
                let (kind, id, first_seen_ms, last_seen_ms) = row?;
                let (Some(kind), Ok(id)) = (kind_from_text(&kind), u64::try_from(id)) else {
                    return Err(NewAssignmentError::Store(StoreError::Damaged {
                        reason: "seen assignment record is unreadable".into(),
                    }));
                };
                let key = ItemKey::new(kind, id);
                first_seen.insert(key, first_seen_ms);
                stored.push(SeenRow { key, last_seen_ms });
            }
        }
        let mark: Option<i64> = tx
            .query_row(
                "SELECT seen_until_ms FROM gitlab_view_marks WHERE account = ?1",
                [account],
                |r| r.get(0),
            )
            .optional()?;

        let plan = plan_round(seeded, &stored, current, now_ms);
        let baseline = review_baseline(seeded, reviews_seeded, &stored, review_only);
        let baseline_set: HashSet<ItemKey> = baseline.iter().copied().collect();
        let baseline_first = baseline_first_seen(mark, now_ms);

        if plan.seed {
            tx.execute(
                "INSERT INTO assignment_accounts (account, seeded_at_ms) VALUES (?1, ?2)",
                rusqlite::params![account, now_ms],
            )?;
        }
        if !reviews_seeded {
            tx.execute(
                "INSERT INTO assignment_accounts (account, seeded_at_ms) VALUES (?1, ?2)",
                rusqlite::params![review_marker, now_ms],
            )?;
        }
        {
            let mut upsert = tx.prepare(
                "INSERT INTO seen_assignments (account, kind, item_id, first_seen_ms, last_seen_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (account, kind, item_id)
                 DO UPDATE SET last_seen_ms = excluded.last_seen_ms",
            )?;
            for key in current {
                upsert.execute(rusqlite::params![
                    account,
                    kind_text(key.kind),
                    key.id as i64,
                    if baseline_set.contains(key) {
                        baseline_first
                    } else {
                        now_ms
                    },
                    now_ms
                ])?;
            }
        }
        {
            let mut delete = tx.prepare(
                "DELETE FROM seen_assignments WHERE account = ?1 AND kind = ?2 AND item_id = ?3",
            )?;
            for key in &plan.forget {
                delete.execute(rusqlite::params![
                    account,
                    kind_text(key.kind),
                    key.id as i64
                ])?;
            }
        }
        // A stored key keeps its first instant (the upsert never changes it);
        // an inserted one has `now_ms`, or the baseline instant.
        let seen: Vec<(ItemKey, i64)> = current
            .iter()
            .map(|k| {
                let inserted = if baseline_set.contains(k) {
                    baseline_first
                } else {
                    now_ms
                };
                (*k, first_seen.get(k).copied().unwrap_or(inserted))
            })
            .collect();
        // The seeding round is a baseline for the view too: nothing is fresh,
        // even when a mark was saved before the account's first stored round.
        let fresh = if plan.seed {
            Vec::new()
        } else {
            fresh_since(&seen, mark)
        };
        Ok(RoundSeen {
            new: plan
                .new
                .into_iter()
                .filter(|k| !baseline_set.contains(k))
                .collect(),
            fresh,
        })
    })
}

#[derive(Debug, Default)]
struct AccountSeen {
    /// Keys of the last successful round.
    last: Option<HashSet<ItemKey>>,
    /// Keys the fallback toasted while the store failed.
    toasted_unstored: HashSet<ItemKey>,
}

/// Per-account session memory that keeps notifications working, without
/// repeats, while the store fails, and avoids a duplicate when it recovers.
#[derive(Debug, Default)]
pub struct SessionSeen {
    accounts: HashMap<String, AccountSeen>,
}

impl SessionSeen {
    pub fn new() -> Self {
        Self::default()
    }

    /// The keys to notify for `account` this round, given the store's answer.
    /// Store ok: its new keys minus those the fallback already toasted. Store
    /// error: the keys not in the last round (nothing on the session's first
    /// round, a silent seed). The account's `last` becomes `current`.
    pub fn resolve(
        &mut self,
        account: &str,
        current: &[ItemKey],
        stored: Result<Vec<ItemKey>, NewAssignmentError>,
    ) -> Vec<ItemKey> {
        let seen = self.accounts.entry(account.to_string()).or_default();
        let notify = match stored {
            Ok(new) => {
                let out = new
                    .into_iter()
                    .filter(|k| !seen.toasted_unstored.contains(k))
                    .collect();
                seen.toasted_unstored.clear();
                out
            }
            Err(_) => match &seen.last {
                Some(last) => {
                    let mut out = Vec::new();
                    for key in current {
                        if !last.contains(key) && !out.contains(key) {
                            out.push(*key);
                        }
                    }
                    seen.toasted_unstored.extend(out.iter().copied());
                    out
                }
                None => Vec::new(),
            },
        };
        seen.last = Some(current.iter().copied().collect());
        notify
    }
}
