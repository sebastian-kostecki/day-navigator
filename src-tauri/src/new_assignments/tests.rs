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

mod notices {
    use crate::gitlab::{GitlabItem, ItemKind};
    use crate::new_assignments::notices_for;

    pub(super) fn item(kind: ItemKind, id: u64, title: &str) -> GitlabItem {
        GitlabItem {
            kind,
            id,
            project_id: 1,
            iid: id,
            reference: match kind {
                ItemKind::Issue => format!("acme/invented#{id}"),
                ItemKind::MergeRequest => format!("acme/invented!{id}"),
            },
            title: title.to_string(),
            web_url: Some("https://secret-host.example.invalid/acme/invented/-/issues/1".into()),
            updated_at: "2026-03-03T08:00:00.000Z".into(),
            labels: vec![],
            draft: false,
            changed: None,
            milestone: None,
            is_new: false,
            hidden_at_ms: None,
            review_requested: false,
        }
    }

    fn issues(n: u64) -> Vec<GitlabItem> {
        (1..=n)
            .map(|i| item(ItemKind::Issue, i, &format!("Title {i}")))
            .collect()
    }

    fn refs(items: &[GitlabItem]) -> Vec<&GitlabItem> {
        items.iter().collect()
    }

    #[test]
    fn no_items_give_no_notice() {
        assert!(notices_for(&[]).is_empty());
    }

    #[test]
    fn one_issue_and_one_mr_get_their_own_titles() {
        let items = [
            item(ItemKind::Issue, 1, "Fix parser"),
            item(ItemKind::MergeRequest, 2, "Add widget"),
        ];
        let n = notices_for(&refs(&items));
        assert_eq!(n.len(), 2);
        assert_eq!(n[0].title, "Nawigator dnia — nowe zgłoszenie");
        assert_eq!(n[0].body, "acme/invented#1 — Fix parser");
        assert_eq!(n[1].title, "Nawigator dnia — nowy MR");
        assert_eq!(n[1].body, "acme/invented!2 — Add widget");
    }

    #[test]
    fn three_items_give_three_separate_notices_in_order() {
        let items = issues(3);
        let n = notices_for(&refs(&items));
        assert_eq!(n.len(), 3);
        assert_eq!(n[2].body, "acme/invented#3 — Title 3");
    }

    #[test]
    fn four_items_give_one_combined_notice() {
        let items = issues(4);
        let n = notices_for(&refs(&items));
        assert_eq!(n.len(), 1);
        assert_eq!(n[0].title, "Nawigator dnia — nowe przypisania (4)");
        assert_eq!(
            n[0].body,
            "acme/invented#1 — Title 1\nacme/invented#2 — Title 2\nacme/invented#3 — Title 3\ni 1 więcej"
        );
    }

    #[test]
    fn separate_title_is_cut_at_100_chars_on_a_char_boundary() {
        let title = format!("{}ąlong-tail", "a".repeat(99));
        let items = [item(ItemKind::Issue, 1, &title)];
        let body = &notices_for(&refs(&items))[0].body;
        assert_eq!(body, &format!("acme/invented#1 — {}ą…", "a".repeat(99)));
        // Exactly 100 characters is not cut.
        let exact = "ż".repeat(100);
        let items = [item(ItemKind::Issue, 1, &exact)];
        assert_eq!(
            notices_for(&refs(&items))[0].body,
            format!("acme/invented#1 — {exact}")
        );
    }

    #[test]
    fn combined_line_title_is_cut_at_40_chars() {
        let long = format!("{}łlong-tail", "b".repeat(39));
        let mut items = issues(4);
        items[0].title = long;
        let n = notices_for(&refs(&items));
        let first = n[0].body.lines().next().unwrap();
        assert_eq!(first, format!("acme/invented#1 — {}ł…", "b".repeat(39)));
        assert!(n[0].body.ends_with("i 1 więcej"));
    }

    #[test]
    fn combined_line_reference_is_cut_at_60_chars() {
        let long_ref = format!("{}#1", "g/".repeat(40));
        let mut items = issues(4);
        items[0].reference = long_ref.clone();
        let n = notices_for(&refs(&items));
        let first = n[0].body.lines().next().unwrap();
        let cut: String = long_ref.chars().take(60).collect();
        assert_eq!(first, format!("{cut}… — Title 1"));
        assert_eq!(n[0].body.lines().count(), 4);
        // A separate notice keeps the whole reference.
        let n = notices_for(&refs(&items[..1]));
        assert!(n[0].body.starts_with(&format!("{long_ref} — ")));
    }

    #[test]
    fn control_characters_become_spaces() {
        let mut it = item(ItemKind::Issue, 1, "Line\none\u{7}two\r\n");
        it.reference = "acme/in\tvented#1".into();
        let items = [it];
        let n = notices_for(&refs(&items));
        assert_eq!(n[0].body, "acme/in vented#1 — Line one two  ");
        assert_eq!(n[0].body.lines().count(), 1);
    }

    #[test]
    fn no_host_or_url_reaches_a_notice() {
        for count in [1, 4] {
            let items = issues(count);
            for n in notices_for(&refs(&items)) {
                for text in [&n.title, &n.body] {
                    assert!(!text.contains("secret-host"));
                    assert!(!text.contains("http"));
                }
            }
        }
    }
}

mod session {
    use crate::gitlab::ItemKind;
    use crate::new_assignments::{ItemKey, NewAssignmentError, SessionSeen};
    use crate::store::StoreError;

    fn k(id: u64) -> ItemKey {
        ItemKey::new(ItemKind::Issue, id)
    }

    fn fail() -> Result<Vec<ItemKey>, NewAssignmentError> {
        Err(NewAssignmentError::Store(StoreError::Damaged {
            reason: "invented".into(),
        }))
    }

    #[test]
    fn store_ok_with_nothing_in_session_passes_new_through() {
        let mut s = SessionSeen::new();
        assert_eq!(s.resolve("a", &[k(1), k(2)], Ok(vec![k(2)])), vec![k(2)]);
    }

    #[test]
    fn store_ok_overlapping_the_session_has_no_duplicate() {
        let mut s = SessionSeen::new();
        s.resolve("a", &[k(1)], Ok(vec![]));
        assert_eq!(s.resolve("a", &[k(1), k(2)], Ok(vec![k(2)])), vec![k(2)]);
        assert!(s.resolve("a", &[k(1), k(2)], Ok(vec![])).is_empty());
    }

    #[test]
    fn store_error_after_an_earlier_round_notifies_only_the_new_item() {
        let mut s = SessionSeen::new();
        s.resolve("a", &[k(1)], Ok(vec![]));
        assert_eq!(s.resolve("a", &[k(1), k(2)], fail()), vec![k(2)]);
        // Not repeated while the store keeps failing.
        assert!(s.resolve("a", &[k(1), k(2)], fail()).is_empty());
    }

    #[test]
    fn store_error_first_is_silent() {
        let mut s = SessionSeen::new();
        assert!(s.resolve("a", &[k(1), k(2)], fail()).is_empty());
        assert!(s.resolve("a", &[k(1), k(2)], fail()).is_empty());
    }

    #[test]
    fn store_error_first_then_ok_notifies_an_overnight_key_once() {
        let mut s = SessionSeen::new();
        assert!(s.resolve("a", &[k(1)], fail()).is_empty());
        assert_eq!(s.resolve("a", &[k(1), k(2)], Ok(vec![k(2)])), vec![k(2)]);
        assert!(s.resolve("a", &[k(1), k(2)], Ok(vec![])).is_empty());
    }

    #[test]
    fn fallback_toast_then_ok_returning_it_has_no_duplicate() {
        let mut s = SessionSeen::new();
        s.resolve("a", &[k(1)], Ok(vec![]));
        assert_eq!(s.resolve("a", &[k(1), k(2)], fail()), vec![k(2)]);
        // The store recovers and still reports 2 (plus 3) as new.
        assert_eq!(
            s.resolve("a", &[k(1), k(2), k(3)], Ok(vec![k(2), k(3)])),
            vec![k(3)]
        );
        // The suppression was one-shot.
        assert_eq!(
            s.resolve("a", &[k(1), k(2), k(3)], Ok(vec![k(2)])),
            vec![k(2)]
        );
    }

    #[test]
    fn accounts_do_not_share_session_memory() {
        let mut s = SessionSeen::new();
        s.resolve("a", &[k(1)], Ok(vec![]));
        // Account b has no earlier round: silent on error.
        assert!(s.resolve("b", &[k(1), k(2)], fail()).is_empty());
        assert_eq!(s.resolve("a", &[k(1), k(2)], fail()), vec![k(2)]);
        // b's recovery is unaffected by a's fallback toast.
        assert_eq!(s.resolve("b", &[k(1), k(2)], Ok(vec![k(2)])), vec![k(2)]);
    }
}

mod items {
    use super::notices::item;
    use crate::gitlab::{AssignedItems, GitlabItem, ItemKind};
    use crate::new_assignments::{ItemKey, items_for, keys_of};

    fn key(kind: ItemKind, id: u64) -> ItemKey {
        ItemKey::new(kind, id)
    }

    fn at(kind: ItemKind, id: u64, updated_at: &str) -> GitlabItem {
        let mut it = item(kind, id, &format!("Title {id}"));
        it.updated_at = updated_at.to_string();
        it
    }

    #[test]
    fn a_repeated_item_gives_one_toast_item() {
        let full = AssignedItems {
            merge_requests: vec![],
            issues: vec![
                at(ItemKind::Issue, 1, "2026-03-03T08:00:00Z"),
                at(ItemKind::Issue, 1, "2026-03-03T08:00:00Z"),
            ],
            hidden_issues: vec![],
            review_requests: Vec::new(),
        };
        let keys = [key(ItemKind::Issue, 1), key(ItemKind::Issue, 1)];
        assert_eq!(items_for(&full, &keys).len(), 1);
    }

    #[test]
    fn items_are_newest_first_across_kinds() {
        let full = AssignedItems {
            merge_requests: vec![
                at(ItemKind::MergeRequest, 7, "2026-03-01T09:00:00Z"),
                at(ItemKind::MergeRequest, 8, "2026-03-04T09:00:00Z"),
            ],
            // The same instant as MR 8 in another offset: a tie keeps key order.
            issues: vec![at(ItemKind::Issue, 7, "2026-03-04T10:00:00+01:00")],
            hidden_issues: vec![at(ItemKind::Issue, 9, "2026-03-02T09:00:00Z")],
            review_requests: Vec::new(),
        };
        let keys = [
            key(ItemKind::MergeRequest, 7),
            key(ItemKind::MergeRequest, 8),
            key(ItemKind::Issue, 7),
            key(ItemKind::Issue, 9),
        ];
        let order: Vec<_> = items_for(&full, &keys)
            .iter()
            .map(|i| (i.kind, i.id))
            .collect();
        assert_eq!(
            order,
            vec![
                (ItemKind::MergeRequest, 8),
                (ItemKind::Issue, 7),
                (ItemKind::Issue, 9),
                (ItemKind::MergeRequest, 7),
            ]
        );
    }

    #[test]
    fn a_key_without_an_item_is_skipped() {
        let full = AssignedItems::default();
        assert!(items_for(&full, &[key(ItemKind::Issue, 1)]).is_empty());
    }

    #[test]
    fn ids_that_cannot_be_stored_are_not_keyed() {
        let max = i64::MAX as u64;
        let full = AssignedItems {
            merge_requests: vec![at(ItemKind::MergeRequest, u64::MAX, "2026-03-01T09:00:00Z")],
            issues: vec![
                at(ItemKind::Issue, max, "2026-03-01T09:00:00Z"),
                at(ItemKind::Issue, max + 1, "2026-03-01T09:00:00Z"),
            ],
            hidden_issues: vec![],
            review_requests: Vec::new(),
        };
        assert_eq!(keys_of(&full), vec![key(ItemKind::Issue, max)]);
    }
}

#[test]
fn fresh_since_without_a_mark_gives_nothing() {
    let seen = [(issue(1), NOW), (mr(2), NOW + 1)];
    assert!(fresh_since(&seen, None).is_empty());
}

#[test]
fn fresh_since_is_strictly_after_the_mark() {
    let seen = [(issue(1), NOW - 1), (issue(2), NOW), (mr(3), NOW + 1)];
    assert_eq!(
        fresh_since(&seen, Some(NOW)),
        vec![(mr(3), NOW + 1)],
        "equal is not new, greater is"
    );
}

#[test]
fn fresh_since_keeps_input_order_and_drops_repeats() {
    let seen = [
        (mr(9), NOW + 5),
        (issue(1), NOW + 2),
        (mr(9), NOW + 5),
        (issue(4), NOW - 3),
    ];
    assert_eq!(
        fresh_since(&seen, Some(NOW)),
        vec![(mr(9), NOW + 5), (issue(1), NOW + 2)]
    );
}

mod flags {
    use std::collections::HashSet;

    use super::notices::item;
    use crate::gitlab::{AssignedItems, ItemKind};
    use crate::new_assignments::{ItemKey, mark_fresh};

    #[test]
    fn mark_fresh_flags_mrs_and_visible_issues_but_never_hidden() {
        let mut items = AssignedItems {
            merge_requests: vec![
                item(ItemKind::MergeRequest, 1, "a"),
                item(ItemKind::MergeRequest, 2, "b"),
            ],
            issues: vec![item(ItemKind::Issue, 1, "c"), item(ItemKind::Issue, 2, "d")],
            hidden_issues: vec![item(ItemKind::Issue, 3, "e")],
            review_requests: Vec::new(),
        };
        items.hidden_issues[0].is_new = true;
        let fresh: HashSet<ItemKey> = [
            ItemKey::new(ItemKind::MergeRequest, 1),
            ItemKey::new(ItemKind::Issue, 2),
            ItemKey::new(ItemKind::Issue, 3),
        ]
        .into_iter()
        .collect();
        mark_fresh(&mut items, &fresh);
        assert!(items.merge_requests[0].is_new);
        assert!(!items.merge_requests[1].is_new);
        assert!(!items.issues[0].is_new);
        assert!(items.issues[1].is_new);
        assert!(!items.hidden_issues[0].is_new);
    }
}
