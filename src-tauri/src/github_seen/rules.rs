//! The NEW rule of one GitHub repo for one round: seeding, insert instant,
//! 30-day forget and fresh keys. No store, clock or process.

use std::collections::HashSet;
use std::fmt;

pub use crate::new_assignments::FORGET_AFTER_MS;

/// Identity of one GitHub issue in the seen set: the lowercase `owner/name`
/// repo key and the issue number.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SeenKey {
    pub repo: String,
    pub number: u64,
}

impl SeenKey {
    pub fn new(repo: impl Into<String>, number: u64) -> Self {
        SeenKey {
            repo: repo.into(),
            number,
        }
    }

    /// The same text as `GithubIssue::reference`: `owner/name#number`.
    pub fn reference(&self) -> String {
        format!("{}#{}", self.repo, self.number)
    }
}

impl fmt::Debug for SeenKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SeenKey(..)")
    }
}

/// One stored seen issue of a repo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredIssue {
    pub number: u64,
    pub first_seen_ms: i64,
    pub last_seen_ms: i64,
}

/// What one successful round does to one repo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoPlan {
    pub seed: bool,
    pub insert_first_seen_ms: i64,
    pub forget: Vec<u64>,
    pub fresh: Vec<(u64, i64)>,
}

/// Plans one repo's round. `insert_first_seen_ms` is `0` for a seeding round
/// (at or below every mark, so a seeded key is never fresh), else `now_ms`.
pub fn plan_repo(
    seeded: bool,
    stored: &[StoredIssue],
    current: &[u64],
    mark: Option<i64>,
    now_ms: i64,
) -> RepoPlan {
    let seed = !seeded;
    let insert_first_seen_ms = if seed { 0 } else { now_ms };

    let current_set: HashSet<u64> = current.iter().copied().collect();
    let forget: Vec<u64> = stored
        .iter()
        .filter(|s| !current_set.contains(&s.number))
        .filter(|s| now_ms - s.last_seen_ms >= FORGET_AFTER_MS)
        .map(|s| s.number)
        .collect();

    let fresh = match mark {
        Some(mark) if !seed => {
            let mut seen: HashSet<u64> = HashSet::new();
            let mut out = Vec::new();
            for &number in current {
                if !seen.insert(number) {
                    continue;
                }
                let first = stored
                    .iter()
                    .find(|s| s.number == number)
                    .map_or(insert_first_seen_ms, |s| s.first_seen_ms);
                if first > mark {
                    out.push((number, first));
                }
            }
            out
        }
        _ => Vec::new(),
    };

    RepoPlan {
        seed,
        insert_first_seen_ms,
        forget,
        fresh,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_000;

    fn stored(number: u64, first: i64, last: i64) -> StoredIssue {
        StoredIssue {
            number,
            first_seen_ms: first,
            last_seen_ms: last,
        }
    }

    #[test]
    fn seeding_round_is_silent_and_stores_at_instant_zero() {
        for mark in [Some(500), None, Some(2000)] {
            let plan = plan_repo(false, &[], &[1, 2], mark, NOW);
            assert!(plan.seed);
            assert_eq!(plan.insert_first_seen_ms, 0);
            assert!(plan.fresh.is_empty());
            assert!(plan.forget.is_empty());
        }
    }

    #[test]
    fn without_a_mark_nothing_is_fresh() {
        let plan = plan_repo(true, &[], &[1, 2], None, NOW);
        assert!(!plan.seed);
        assert!(plan.fresh.is_empty());
        assert_eq!(plan.insert_first_seen_ms, NOW);
    }

    #[test]
    fn first_seen_after_the_mark_is_fresh_and_at_the_mark_is_not() {
        let st = [stored(1, 500, 900), stored(2, 501, 900)];
        let plan = plan_repo(true, &st, &[1, 2, 3], Some(500), NOW);
        assert_eq!(plan.fresh, vec![(2, 501), (3, NOW)]);
        assert!(plan.forget.is_empty());
    }

    #[test]
    fn a_stored_issue_keeps_its_first_instant() {
        let st = [stored(1, 100, 900)];
        let plan = plan_repo(true, &st, &[1], Some(50), NOW);
        assert_eq!(plan.fresh, vec![(1, 100)]);
        assert!(plan.forget.is_empty());
    }

    #[test]
    fn absent_issue_is_forgotten_after_thirty_days_only() {
        let now = 1_800_000_000_000_i64;
        let st = [
            stored(1, 0, now - FORGET_AFTER_MS),
            stored(2, 0, now - FORGET_AFTER_MS + 1),
            stored(3, 0, now - 2 * FORGET_AFTER_MS),
        ];
        let plan = plan_repo(true, &st, &[3], None, now);
        assert_eq!(plan.forget, vec![1]);
        assert!(plan.fresh.is_empty());
    }

    #[test]
    fn repeated_numbers_are_fresh_once_in_input_order() {
        let plan = plan_repo(true, &[], &[3, 2, 3], Some(0), NOW);
        assert_eq!(plan.fresh, vec![(3, NOW), (2, NOW)]);
    }

    #[test]
    fn seen_key_debug_is_opaque_and_reference_is_key_hash_number() {
        let key = SeenKey::new("octo-test/alpha", 7);
        assert_eq!(format!("{key:?}"), "SeenKey(..)");
        assert_eq!(key.reference(), "octo-test/alpha#7");
    }
}
