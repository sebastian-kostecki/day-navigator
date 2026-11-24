//! Pure rules for new-assignment detection: no store, clock or process.
//! `now_ms` is a parameter; instants are UTC epoch milliseconds.

use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use crate::gitlab::ItemKind;

/// An absent id is forgotten after this long (30 days).
pub const FORGET_AFTER_MS: i64 = 30 * 86_400_000;

/// One assigned item: its kind and GitLab's global id. Issue ids and MR ids
/// are separate sequences, so the kind is part of the key. Debug is opaque so
/// ids never reach a log.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ItemKey {
    pub kind: ItemKind,
    pub id: u64,
}

impl ItemKey {
    pub fn new(kind: ItemKind, id: u64) -> Self {
        ItemKey { kind, id }
    }

    fn rank(&self) -> u8 {
        match self.kind {
            ItemKind::Issue => 0,
            ItemKind::MergeRequest => 1,
        }
    }
}

impl std::fmt::Debug for ItemKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ItemKey(..)")
    }
}

impl Hash for ItemKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.rank().hash(state);
        self.id.hash(state);
    }
}

impl PartialOrd for ItemKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ItemKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.rank(), self.id).cmp(&(other.rank(), other.id))
    }
}

/// A stored seen row of one account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeenRow {
    pub key: ItemKey,
    pub last_seen_ms: i64,
}

/// What one successful round decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundPlan {
    /// Keys to notify: empty when the account is not seeded yet.
    pub new: Vec<ItemKey>,
    /// The account has no seeding marker yet: record it.
    pub seed: bool,
    /// Stored keys absent from the current list for 30 days or more.
    pub forget: Vec<ItemKey>,
}

/// What one recorded round returns: the keys to toast and the keys that are
/// new to the view, each with its `first_seen_ms`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundSeen {
    /// Keys to notify (see `RoundPlan::new`).
    pub new: Vec<ItemKey>,
    /// `fresh_since` of the round's keys and the account's seen-until mark.
    pub fresh: Vec<(ItemKey, i64)>,
}

/// The keys that are new to the view (S-02): `first_seen_ms` strictly after the
/// account's seen-until `mark`. Without a mark nothing is fresh. Input order is
/// kept; a repeated key is returned once.
pub fn fresh_since(seen: &[(ItemKey, i64)], mark: Option<i64>) -> Vec<(ItemKey, i64)> {
    let Some(mark) = mark else {
        return Vec::new();
    };
    let mut taken = HashSet::new();
    seen.iter()
        .filter(|(key, first_seen_ms)| *first_seen_ms > mark && taken.insert(*key))
        .copied()
        .collect()
}

pub fn plan_round(seeded: bool, stored: &[SeenRow], current: &[ItemKey], now_ms: i64) -> RoundPlan {
    let known: HashSet<ItemKey> = stored.iter().map(|r| r.key).collect();
    let present: HashSet<ItemKey> = current.iter().copied().collect();

    let mut new = Vec::new();
    if seeded {
        let mut taken = HashSet::new();
        for key in current {
            if !known.contains(key) && taken.insert(*key) {
                new.push(*key);
            }
        }
    }

    let forget = stored
        .iter()
        .filter(|r| !present.contains(&r.key) && now_ms - r.last_seen_ms >= FORGET_AFTER_MS)
        .map(|r| r.key)
        .collect();

    RoundPlan {
        new,
        seed: !seeded,
        forget,
    }
}
