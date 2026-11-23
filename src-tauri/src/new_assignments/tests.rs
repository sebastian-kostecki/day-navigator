//! Rule tests with invented ids and instants.

use super::rules::*;
use crate::gitlab::ItemKind;

const DAY: i64 = 86_400_000;
const NOW: i64 = 1_800_000_000_000;

fn issue(id: u64) -> ItemKey {
    ItemKey::new(ItemKind::Issue, id)
}

fn mr(id: u64) -> ItemKey {
    ItemKey::new(ItemKind::MergeRequest, id)
}

fn row(key: ItemKey, last_seen_ms: i64) -> SeenRow {
    SeenRow { key, last_seen_ms }
}

#[test]
fn unseeded_account_seeds_silently() {
    let plan = plan_round(false, &[], &[issue(1), mr(2)], NOW);
    assert!(plan.seed);
    assert!(plan.new.is_empty());
    assert!(plan.forget.is_empty());
}

#[test]
fn seeded_account_reports_unknown_keys_in_current_order() {
    let stored = [row(issue(1), NOW - 1_000)];
    let plan = plan_round(true, &stored, &[mr(9), issue(1), issue(5)], NOW);
    assert!(!plan.seed);
    assert_eq!(plan.new, vec![mr(9), issue(5)]);
}

#[test]
fn duplicate_current_keys_notify_once() {
    let plan = plan_round(true, &[], &[issue(3), issue(3), mr(3), issue(3)], NOW);
    assert_eq!(plan.new, vec![issue(3), mr(3)]);
}

#[test]
fn issue_and_mr_with_the_same_id_are_distinct() {
    let stored = [row(issue(7), NOW)];
    let plan = plan_round(true, &stored, &[issue(7), mr(7)], NOW);
    assert_eq!(plan.new, vec![mr(7)]);
    assert_ne!(issue(7), mr(7));
    assert!(issue(7) < mr(7));
}

#[test]
fn forget_boundary_is_exactly_thirty_days() {
    let stored = [
        row(issue(1), NOW - FORGET_AFTER_MS),
        row(issue(2), NOW - FORGET_AFTER_MS + 1),
    ];
    let plan = plan_round(true, &stored, &[], NOW);
    assert_eq!(plan.forget, vec![issue(1)]);
    assert_eq!(FORGET_AFTER_MS, 30 * DAY);
}

#[test]
fn present_stale_key_is_kept_and_not_new() {
    let stored = [row(issue(1), NOW - 90 * DAY)];
    let plan = plan_round(true, &stored, &[issue(1)], NOW);
    assert!(plan.forget.is_empty());
    assert!(plan.new.is_empty());
}

#[test]
fn seeded_account_with_all_rows_forgotten_still_notifies() {
    // The marker outlives the rows: an empty stored set is not "never seeded".
    let plan = plan_round(true, &[], &[issue(4)], NOW);
    assert!(!plan.seed);
    assert_eq!(plan.new, vec![issue(4)]);
}

#[test]
fn key_debug_is_opaque() {
    assert_eq!(format!("{:?}", issue(123_456)), "ItemKey(..)");
}
