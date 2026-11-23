//! New GitLab assignments (S-04): the durable seen set and the rules that say
//! which items of a round are new.
//!
//! No `tauri` import and no clock: `now_ms` is a parameter. A round is one
//! `Store::write`. The seen set holds the opaque account key, kind, numeric id
//! and two instants: no title, reference, URL or hostname.

pub mod error;
pub mod notice;
pub mod rules;

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};

use crate::gitlab::{AssignedItems, ItemKind};
use crate::store::{Store, StoreError};
pub use error::NewAssignmentError;
pub use notice::notices_for;
pub use rules::{FORGET_AFTER_MS, ItemKey, RoundPlan, SeenRow, plan_round};

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

/// The full assigned list: merge requests, visible issues and hidden issues.
pub fn keys_of(items: &AssignedItems) -> Vec<ItemKey> {
    items
        .merge_requests
        .iter()
        .chain(&items.issues)
        .chain(&items.hidden_issues)
        .map(|i| ItemKey::new(i.kind, i.id))
        .collect()
}

/// Records one successful round for `account` and returns the keys that are
/// new (empty on the account's first round, which seeds silently). One write:
/// read the seeded flag and rows, plan, insert the marker when seeding, upsert
/// every current key, then delete this account's stale absent rows. Rows of
/// other accounts are never touched.
pub fn record_round(
    store: &mut Store,
    account: &str,
    current: &[ItemKey],
    now_ms: i64,
) -> Result<Vec<ItemKey>, NewAssignmentError> {
    store.write::<_, NewAssignmentError>(|tx| {
        let seeded: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM assignment_accounts WHERE account = ?1)",
            [account],
            |r| r.get(0),
        )?;

        let mut stored = Vec::new();
        {
            let mut stmt = tx.prepare(
                "SELECT kind, item_id, last_seen_ms FROM seen_assignments WHERE account = ?1",
            )?;
            let rows = stmt.query_map([account], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })?;
            for row in rows {
                let (kind, id, last_seen_ms) = row?;
                let (Some(kind), Ok(id)) = (kind_from_text(&kind), u64::try_from(id)) else {
                    return Err(NewAssignmentError::Store(StoreError::Damaged {
                        reason: "seen assignment record is unreadable".into(),
                    }));
                };
                stored.push(SeenRow {
                    key: ItemKey::new(kind, id),
                    last_seen_ms,
                });
            }
        }

        let plan = plan_round(seeded, &stored, current, now_ms);

        if plan.seed {
            tx.execute(
                "INSERT INTO assignment_accounts (account, seeded_at_ms) VALUES (?1, ?2)",
                rusqlite::params![account, now_ms],
            )?;
        }
        {
            let mut upsert = tx.prepare(
                "INSERT INTO seen_assignments (account, kind, item_id, first_seen_ms, last_seen_ms)
                 VALUES (?1, ?2, ?3, ?4, ?4)
                 ON CONFLICT (account, kind, item_id)
                 DO UPDATE SET last_seen_ms = excluded.last_seen_ms",
            )?;
            for key in current {
                upsert.execute(rusqlite::params![
                    account,
                    kind_text(key.kind),
                    key.id as i64,
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
        Ok(plan.new)
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
