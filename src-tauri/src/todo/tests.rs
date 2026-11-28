//! Rules of the "To do" list on invented days and data.

use chrono::{NaiveDate, NaiveDateTime};
use tempfile::TempDir;

use super::*;
use crate::events::EventView;
use crate::own_tasks::{self, OwnTaskInput};
use crate::store::schema;

fn day(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

fn at(d: &str, h: u32, m: u32) -> NaiveDateTime {
    day(d).and_hms_opt(h, m, 0).unwrap()
}

const D1: &str = "2026-11-23";
const D2: &str = "2026-11-24";
const D3: &str = "2026-11-25";
/// Invented `gitlab::account_key` of the account the test lists come from.
const ACC: &str = "account-a";

fn open() -> (TempDir, Store) {
    let tmp = tempfile::tempdir().unwrap();
    let store = Store::open(&tmp.path().join("data"), schema::MIGRATIONS, day(D1)).unwrap();
    (tmp, store)
}

fn own(store: &mut Store, title: &str) -> i64 {
    let input = OwnTaskInput {
        space: Space::Work,
        title: title.into(),
        note: String::new(),
        due: None,
        priority: None,
    };
    own_tasks::add(store, &input, at(D1, 7, 0)).unwrap().id
}

fn gl(kind: ItemKind, n: u32) -> GitlabItem {
    let sep = if kind == ItemKind::Issue { '#' } else { '!' };
    GitlabItem {
        kind,
        id: u64::from(n),
        project_id: 1,
        iid: u64::from(n),
        changed: None,
        milestone: None,
        is_new: false,
        hidden_at_ms: None,
        reference: format!("invented/project{sep}{n}"),
        title: format!("Pozycja testowa {n}"),
        web_url: Some(format!(
            "https://gitlab.example.invalid/invented/project/{n}"
        )),
        updated_at: "2026-11-20T10:00:00+01:00".into(),
        labels: vec![],
        draft: false,
        review_requested: false,
    }
}

fn assigned(items: &[&GitlabItem]) -> AssignedItems {
    let mut a = AssignedItems::default();
    for i in items {
        match i.kind {
            ItemKind::Issue => a.issues.push((*i).clone()),
            ItemKind::MergeRequest => a.merge_requests.push((*i).clone()),
        }
    }
    a
}

fn list_on(store: &mut Store, d: &str, limit: u32) -> TodoList {
    list(store, day(d), at(d, 9, 0), Ok(limit)).unwrap()
}

fn titles(l: &TodoList) -> Vec<String> {
    l.items
        .iter()
        .map(|i| match &i.entry {
            TodoEntry::Own { title, .. }
            | TodoEntry::Gitlab { title, .. }
            | TodoEntry::Github { title, .. } => title.clone(),
        })
        .collect()
}

fn pick_own(store: &mut Store, task: i64, d: &str, h: u32, limit: u32) -> Result<i64, TodoError> {
    add_own(store, task, day(d), at(d, h, 0), Ok(limit))
}

fn pick_gl(
    store: &mut Store,
    item: &GitlabItem,
    d: &str,
    h: u32,
    limit: u32,
) -> Result<i64, TodoError> {
    add_gitlab(store, ACC, item, day(d), at(d, h, 0), Ok(limit))
}

fn tick(store: &mut Store, id: i64, ticked: bool, d: &str, h: u32) -> Result<(), TodoError> {
    set_ticked(store, id, ticked, day(d), at(d, h, 30))
}

fn done_at(store: &Store, task: i64) -> Option<String> {
    store
        .read::<_, StoreError>(|c| {
            Ok(
                c.query_row("SELECT done_at FROM own_tasks WHERE id = ?1", [task], |r| {
                    r.get(0)
                })?,
            )
        })
        .unwrap()
}

#[test]
fn open_entries_carry_over_and_are_marked() {
    let (_t, mut s) = open();
    let a = own(&mut s, "Zadanie A");
    pick_own(&mut s, a, D1, 8, 5).unwrap();
    let l = list_on(&mut s, D1, 5);
    assert!(!l.items[0].carried_over);
    let l = list_on(&mut s, D2, 5);
    assert_eq!(titles(&l), ["Zadanie A"]);
    assert!(l.items[0].carried_over);
    assert_eq!(l.items[0].picked_on, D1);
    assert_eq!(l.open_count, 1);
}

#[test]
fn ticked_today_shows_ticked_yesterday_is_closed() {
    let (_t, mut s) = open();
    let a = own(&mut s, "Zadanie A");
    let b = own(&mut s, "Zadanie B");
    let ia = pick_own(&mut s, a, D1, 8, 5).unwrap();
    pick_own(&mut s, b, D1, 9, 5).unwrap();
    tick(&mut s, ia, true, D1, 10).unwrap();

    let l = list_on(&mut s, D1, 5);
    assert_eq!(titles(&l), ["Zadanie B", "Zadanie A"]);
    assert!(l.items[1].ticked);
    assert_eq!(l.open_count, 1);

    let l = list_on(&mut s, D2, 5);
    assert_eq!(titles(&l), ["Zadanie B"]);
}

#[test]
fn reopened_task_after_roll_over_does_not_come_back() {
    let (_t, mut s) = open();
    let a = own(&mut s, "Zadanie A");
    let ia = pick_own(&mut s, a, D1, 8, 5).unwrap();
    tick(&mut s, ia, true, D1, 10).unwrap();
    assert!(list_on(&mut s, D2, 5).items.is_empty());
    own_tasks::set_done(&mut s, a, false, at(D2, 11, 0)).unwrap();
    assert!(list_on(&mut s, D2, 5).items.is_empty());
    assert!(list_on(&mut s, D3, 5).items.is_empty());
    // The task is open again, so it can be picked anew.
    pick_own(&mut s, a, D3, 8, 5).unwrap();
    assert_eq!(list_on(&mut s, D3, 5).items.len(), 1);
}

#[test]
fn own_tick_is_done_at_both_ways() {
    let (_t, mut s) = open();
    let a = own(&mut s, "Zadanie A");
    let ia = pick_own(&mut s, a, D1, 8, 5).unwrap();
    tick(&mut s, ia, true, D1, 10).unwrap();
    assert_eq!(done_at(&s, a).as_deref(), Some("2026-11-23T10:30:00"));
    // Ticking again keeps the first time.
    tick(&mut s, ia, true, D1, 12).unwrap();
    assert_eq!(done_at(&s, a).as_deref(), Some("2026-11-23T10:30:00"));
    tick(&mut s, ia, false, D1, 13).unwrap();
    assert_eq!(done_at(&s, a), None);

    // Done in "Moje zadania" shows ticked here.
    own_tasks::set_done(&mut s, a, true, at(D1, 14, 0)).unwrap();
    let l = list_on(&mut s, D1, 5);
    assert!(l.items[0].ticked);
    own_tasks::set_done(&mut s, a, false, at(D1, 15, 0)).unwrap();
    assert!(!list_on(&mut s, D1, 5).items[0].ticked);
}

#[test]
fn gitlab_user_tick_and_untick() {
    let (_t, mut s) = open();
    let item = gl(ItemKind::MergeRequest, 1);
    let id = pick_gl(&mut s, &item, D1, 8, 5).unwrap();
    tick(&mut s, id, true, D1, 10).unwrap();
    let l = list_on(&mut s, D1, 5);
    assert!(l.items[0].ticked);
    assert!(matches!(
        l.items[0].entry,
        TodoEntry::Gitlab {
            closed_in_source: false,
            ..
        }
    ));
    tick(&mut s, id, false, D1, 11).unwrap();
    assert!(!list_on(&mut s, D1, 5).items[0].ticked);
}

#[test]
fn roll_over_runs_inside_every_write_without_a_prior_list() {
    let (_t, mut s) = open();
    let item = gl(ItemKind::Issue, 7);
    let other = gl(ItemKind::Issue, 8);
    let id = pick_gl(&mut s, &item, D1, 8, 5).unwrap();
    tick(&mut s, id, true, D1, 10).unwrap();

    // add: the item ticked yesterday can be picked again today.
    let again = pick_gl(&mut s, &item, D2, 8, 5).unwrap();
    assert_ne!(again, id);

    // tick: yesterday's ticked entry is closed, so it cannot be changed.
    let (_t2, mut s2) = open();
    let id2 = pick_gl(&mut s2, &item, D1, 8, 5).unwrap();
    tick(&mut s2, id2, true, D1, 10).unwrap();
    assert!(matches!(
        tick(&mut s2, id2, false, D2, 8),
        Err(TodoError::NotFound)
    ));

    // remove: same.
    assert!(matches!(
        remove(&mut s2, id2, day(D2), at(D2, 8, 0)),
        Err(TodoError::NotFound)
    ));

    // reconcile: the closed entry is not ticked again, an open one is.
    let open_id = pick_gl(&mut s2, &other, D2, 9, 5).unwrap();
    let n = reconcile_gitlab(&mut s2, ACC, &assigned(&[]), day(D2), at(D2, 10, 0)).unwrap();
    assert_eq!(n, 1);
    let l = list_on(&mut s2, D2, 5);
    assert_eq!(l.items.len(), 1);
    assert_eq!(l.items[0].id, open_id);
}

#[test]
fn reopen_command_pattern_keeps_closed_entry_closed() {
    // set_own_task_done will run roll_over_in then set_done_in in one write.
    let (_t, mut s) = open();
    let a = own(&mut s, "Zadanie A");
    let ia = pick_own(&mut s, a, D1, 8, 5).unwrap();
    tick(&mut s, ia, true, D1, 10).unwrap();
    s.write::<_, TodoError>(|tx| {
        roll_over_in(tx, day(D2))?;
        own_tasks::set_done_in(tx, a, false, at(D2, 8, 0))?;
        Ok(())
    })
    .unwrap();
    assert!(list_on(&mut s, D2, 5).items.is_empty());
}

#[test]
fn limit_counts_open_entries_only() {
    let (_t, mut s) = open();
    let tasks: Vec<i64> = (1..=8)
        .map(|n| own(&mut s, &format!("Zadanie {n}")))
        .collect();
    let ids: Vec<i64> = tasks[..5]
        .iter()
        .map(|t| pick_own(&mut s, *t, D1, 8, 5).unwrap())
        .collect();
    assert!(matches!(
        pick_own(&mut s, tasks[5], D1, 9, 5),
        Err(TodoError::LimitReached { limit: 5 })
    ));
    tick(&mut s, ids[0], true, D1, 10).unwrap();
    tick(&mut s, ids[1], true, D1, 10).unwrap();
    pick_own(&mut s, tasks[5], D1, 11, 5).unwrap();
    pick_own(&mut s, tasks[6], D1, 11, 5).unwrap();
    assert!(matches!(
        pick_own(&mut s, tasks[7], D1, 12, 5),
        Err(TodoError::LimitReached { limit: 5 })
    ));
    let l = list_on(&mut s, D1, 5);
    assert_eq!(l.open_count, 5);
    assert_eq!(l.items.len(), 7);
}

#[test]
fn lowered_limit_refuses_adds_without_dropping_entries() {
    let (_t, mut s) = open();
    let tasks: Vec<i64> = (1..=4)
        .map(|n| own(&mut s, &format!("Zadanie {n}")))
        .collect();
    for t in &tasks[..3] {
        pick_own(&mut s, *t, D1, 8, 5).unwrap();
    }
    assert!(matches!(
        pick_own(&mut s, tasks[3], D1, 9, 2),
        Err(TodoError::LimitReached { limit: 2 })
    ));
    let l = list_on(&mut s, D1, 2);
    assert_eq!(l.items.len(), 3);
    assert_eq!(l.open_count, 3);
    assert_eq!(l.limit, Some(2));
}

#[test]
fn untick_and_remove_are_allowed_above_the_limit() {
    let (_t, mut s) = open();
    let a = own(&mut s, "Zadanie A");
    let b = own(&mut s, "Zadanie B");
    let ia = pick_own(&mut s, a, D1, 8, 5).unwrap();
    let ib = pick_own(&mut s, b, D1, 8, 5).unwrap();
    tick(&mut s, ia, true, D1, 9).unwrap();
    // Limit lowered to 1 while 1 open: untick pushes the open count to 2.
    tick(&mut s, ia, false, D1, 10).unwrap();
    assert_eq!(list_on(&mut s, D1, 1).open_count, 2);
    remove(&mut s, ib, day(D1), at(D1, 11, 0)).unwrap();
    assert_eq!(list_on(&mut s, D1, 1).open_count, 1);
}

#[test]
fn settings_error_is_returned_by_adds_and_shown_by_list() {
    let (_t, mut s) = open();
    let a = own(&mut s, "Zadanie A");
    let item = gl(ItemKind::Issue, 1);
    let r = add_own(
        &mut s,
        a,
        day(D1),
        at(D1, 8, 0),
        Err(TodoError::SettingsInvalid),
    );
    assert!(matches!(r, Err(TodoError::SettingsInvalid)));
    let r = add_gitlab(
        &mut s,
        ACC,
        &item,
        day(D1),
        at(D1, 8, 0),
        Err(TodoError::ConfigDirUnresolved),
    );
    assert!(matches!(r, Err(TodoError::ConfigDirUnresolved)));
    let l = list(
        &mut s,
        day(D1),
        at(D1, 9, 0),
        Err(TodoError::SettingsInvalid),
    )
    .unwrap();
    assert_eq!(l.limit, None);
    assert!(matches!(l.limit_error, Some(TodoError::SettingsInvalid)));
    assert!(l.items.is_empty());
}

#[test]
fn duplicate_live_entry_is_refused_and_repick_follows_remove_or_close() {
    let (_t, mut s) = open();
    let a = own(&mut s, "Zadanie A");
    let item = gl(ItemKind::MergeRequest, 3);
    let ia = pick_own(&mut s, a, D1, 8, 5).unwrap();
    let ig = pick_gl(&mut s, &item, D1, 8, 5).unwrap();
    assert!(matches!(
        pick_own(&mut s, a, D1, 9, 5),
        Err(TodoError::AlreadyOnList)
    ));
    assert!(matches!(
        pick_gl(&mut s, &item, D1, 9, 5),
        Err(TodoError::AlreadyOnList)
    ));
    // A GitLab entry ticked today is still on the list.
    tick(&mut s, ig, true, D1, 10).unwrap();
    assert!(matches!(
        pick_gl(&mut s, &item, D1, 11, 5),
        Err(TodoError::AlreadyOnList)
    ));

    remove(&mut s, ia, day(D1), at(D1, 12, 0)).unwrap();
    pick_own(&mut s, a, D1, 13, 5).unwrap();
    // Close by roll-over, then re-pick.
    pick_gl(&mut s, &item, D2, 8, 5).unwrap();
    assert_eq!(list_on(&mut s, D2, 5).items.len(), 2);
}

#[test]
fn picking_a_done_or_unknown_own_task_is_refused() {
    let (_t, mut s) = open();
    let a = own(&mut s, "Zadanie A");
    own_tasks::set_done(&mut s, a, true, at(D1, 8, 0)).unwrap();
    assert!(matches!(
        pick_own(&mut s, a, D1, 9, 5),
        Err(TodoError::TaskDone)
    ));
    assert!(matches!(
        pick_own(&mut s, 999, D1, 9, 5),
        Err(TodoError::NotFound)
    ));
    assert!(list_on(&mut s, D1, 5).items.is_empty());
}

#[test]
fn remove_keeps_own_done_state_and_hides_the_entry() {
    let (_t, mut s) = open();
    let a = own(&mut s, "Zadanie A");
    let ia = pick_own(&mut s, a, D1, 8, 5).unwrap();
    tick(&mut s, ia, true, D1, 9).unwrap();
    remove(&mut s, ia, day(D1), at(D1, 10, 0)).unwrap();
    assert!(list_on(&mut s, D1, 5).items.is_empty());
    assert!(done_at(&s, a).is_some());
    assert!(matches!(
        remove(&mut s, ia, day(D1), at(D1, 11, 0)),
        Err(TodoError::NotFound)
    ));
}

#[test]
fn items_are_ordered_open_first_by_pick_then_ticked() {
    let (_t, mut s) = open();
    let a = own(&mut s, "Zadanie A");
    let b = own(&mut s, "Zadanie B");
    let g = gl(ItemKind::Issue, 5);
    let ia = pick_own(&mut s, a, D1, 8, 5).unwrap();
    pick_gl(&mut s, &g, D1, 9, 5).unwrap();
    pick_own(&mut s, b, D1, 10, 5).unwrap();
    assert_eq!(
        titles(&list_on(&mut s, D1, 5)),
        ["Zadanie A", "Pozycja testowa 5", "Zadanie B"]
    );
    tick(&mut s, ia, true, D1, 11).unwrap();
    assert_eq!(
        titles(&list_on(&mut s, D1, 5)),
        ["Pozycja testowa 5", "Zadanie B", "Zadanie A"]
    );
}

#[test]
fn own_title_is_read_live() {
    let (_t, mut s) = open();
    let a = own(&mut s, "Zadanie A");
    pick_own(&mut s, a, D1, 8, 5).unwrap();
    let input = OwnTaskInput {
        space: Space::Private,
        title: "Zadanie A po zmianie".into(),
        note: String::new(),
        due: None,
        priority: None,
    };
    own_tasks::edit(&mut s, a, &input, at(D1, 9, 0)).unwrap();
    let l = list_on(&mut s, D1, 5);
    assert!(matches!(
        &l.items[0].entry,
        TodoEntry::Own { space: Space::Private, title, .. } if title == "Zadanie A po zmianie"
    ));
}

#[test]
fn reconcile_ticks_missing_and_refreshes_present() {
    let (_t, mut s) = open();
    let kept = gl(ItemKind::MergeRequest, 1);
    let gone = gl(ItemKind::Issue, 2);
    pick_gl(&mut s, &kept, D1, 8, 5).unwrap();
    pick_gl(&mut s, &gone, D1, 9, 5).unwrap();

    let mut renamed = kept.clone();
    renamed.title = "Nowy tytul testowy".into();
    renamed.web_url = Some("https://gitlab.example.invalid/invented/project/new".into());
    let n = reconcile_gitlab(&mut s, ACC, &assigned(&[&renamed]), day(D1), at(D1, 10, 0)).unwrap();
    assert_eq!(n, 2);

    let l = list_on(&mut s, D1, 5);
    assert_eq!(titles(&l), ["Nowy tytul testowy", "Pozycja testowa 2"]);
    assert!(!l.items[0].ticked);
    assert!(l.items[1].ticked);
    assert!(matches!(
        &l.items[0].entry,
        TodoEntry::Gitlab { web_url: Some(u), .. } if u.ends_with("/new")
    ));
    assert!(matches!(
        l.items[1].entry,
        TodoEntry::Gitlab {
            closed_in_source: true,
            ..
        }
    ));
}

#[test]
fn reconcile_keeps_a_picked_issue_the_user_hid_open() {
    let (_t, mut s) = open();
    let item = gl(ItemKind::Issue, 1);
    pick_gl(&mut s, &item, D1, 8, 5).unwrap();
    // The round's partitioned list: the issue is hidden, not gone.
    let mut items = assigned(&[]);
    items.hidden_issues.push(item.clone());

    let n = reconcile_gitlab(&mut s, ACC, &items, day(D1), at(D1, 9, 0)).unwrap();

    assert_eq!(n, 0);
    assert!(!list_on(&mut s, D1, 5).items[0].ticked);
}

#[test]
fn reconcile_keeps_a_picked_review_only_merge_request_and_ticks_it_once_gone() {
    let (_t, mut s) = open();
    let item = gl(ItemKind::MergeRequest, 1);
    pick_gl(&mut s, &item, D1, 8, 5).unwrap();
    // Only in the review list: present, title and URL refreshed from it.
    let mut fresh = item.clone();
    fresh.title = "Nowy tytul review".into();
    fresh.web_url = Some("https://gitlab.example.invalid/invented/project/rev".into());
    let mut items = assigned(&[]);
    items.review_requests.push(fresh);

    let n = reconcile_gitlab(&mut s, ACC, &items, day(D1), at(D1, 9, 0)).unwrap();

    assert_eq!(n, 1);
    let l = list_on(&mut s, D1, 5);
    assert_eq!(titles(&l), ["Nowy tytul review"]);
    assert!(!l.items[0].ticked);
    assert!(matches!(
        &l.items[0].entry,
        TodoEntry::Gitlab { web_url: Some(u), .. } if u.ends_with("/rev")
    ));

    // In neither list any more (also: removed as reviewer): ticked.
    let n = reconcile_gitlab(&mut s, ACC, &assigned(&[]), day(D1), at(D1, 10, 0)).unwrap();
    assert_eq!(n, 1);
    assert!(list_on(&mut s, D1, 5).items[0].ticked);
}

#[test]
fn reconcile_count_is_zero_when_nothing_changes() {
    let (_t, mut s) = open();
    let item = gl(ItemKind::Issue, 1);
    pick_gl(&mut s, &item, D1, 8, 5).unwrap();
    let n = reconcile_gitlab(&mut s, ACC, &assigned(&[&item]), day(D1), at(D1, 9, 0)).unwrap();
    assert_eq!(n, 0);
    let n = reconcile_gitlab(&mut s, ACC, &assigned(&[&item]), day(D1), at(D1, 9, 5)).unwrap();
    assert_eq!(n, 0);
}

#[test]
fn reconcile_leaves_ticked_removed_closed_and_own_alone() {
    let (_t, mut s) = open();
    let a = own(&mut s, "Zadanie A");
    pick_own(&mut s, a, D1, 8, 5).unwrap();
    let ticked = gl(ItemKind::Issue, 1);
    let removed = gl(ItemKind::Issue, 2);
    let it = pick_gl(&mut s, &ticked, D1, 8, 5).unwrap();
    let ir = pick_gl(&mut s, &removed, D1, 8, 5).unwrap();
    tick(&mut s, it, true, D1, 9).unwrap();
    remove(&mut s, ir, day(D1), at(D1, 9, 0)).unwrap();

    let n = reconcile_gitlab(&mut s, ACC, &assigned(&[]), day(D1), at(D1, 10, 0)).unwrap();
    assert_eq!(n, 0);
    let l = list_on(&mut s, D1, 5);
    assert_eq!(l.items.len(), 2);
    // The user tick was not turned into a source tick.
    let g = l.items.iter().find(|i| i.id == it).unwrap();
    assert!(matches!(
        g.entry,
        TodoEntry::Gitlab {
            closed_in_source: false,
            ..
        }
    ));
    // The own entry is still open.
    assert!(l.items.iter().any(|i| !i.ticked));
}

#[test]
fn empty_successful_list_ticks_every_open_gitlab_entry() {
    let (_t, mut s) = open();
    for n in 1..=3 {
        pick_gl(&mut s, &gl(ItemKind::MergeRequest, n), D1, 8, 5).unwrap();
    }
    let n = reconcile_gitlab(&mut s, ACC, &assigned(&[]), day(D1), at(D1, 10, 0)).unwrap();
    assert_eq!(n, 3);
    assert_eq!(list_on(&mut s, D1, 5).open_count, 0);
}

#[test]
fn same_reference_with_other_kind_is_a_different_item() {
    let (_t, mut s) = open();
    let mr = gl(ItemKind::MergeRequest, 1);
    let mut issue = gl(ItemKind::Issue, 1);
    issue.reference = mr.reference.clone();
    pick_gl(&mut s, &mr, D1, 8, 5).unwrap();
    pick_gl(&mut s, &issue, D1, 9, 5).unwrap();
    let n = reconcile_gitlab(&mut s, ACC, &assigned(&[&mr]), day(D1), at(D1, 10, 0)).unwrap();
    assert_eq!(n, 1);
}

#[test]
fn reconcile_leaves_entries_of_another_account_alone() {
    let (_t, mut s) = open();
    let item = gl(ItemKind::Issue, 1);
    pick_gl(&mut s, &item, D1, 8, 5).unwrap();
    // A successful list of another account (a changed host or user, or a typo
    // in the user) says nothing about this entry.
    let n = reconcile_gitlab(&mut s, "account-b", &assigned(&[]), day(D1), at(D1, 9, 0)).unwrap();
    assert_eq!(n, 0);
    assert_eq!(list_on(&mut s, D1, 5).open_count, 1);
    // Its own account's list still closes it.
    let n = reconcile_gitlab(&mut s, ACC, &assigned(&[]), day(D1), at(D1, 10, 0)).unwrap();
    assert_eq!(n, 1);
    assert_eq!(list_on(&mut s, D1, 5).open_count, 0);
}

#[test]
fn entry_picked_before_accounts_were_recorded_is_reconciled_by_any_account() {
    let (_t, mut s) = open();
    let id = pick_gl(&mut s, &gl(ItemKind::Issue, 1), D1, 8, 5).unwrap();
    s.write::<_, StoreError>(|tx| {
        tx.execute(
            "UPDATE todo_items SET gitlab_account = NULL WHERE id = ?1",
            [id],
        )?;
        Ok(())
    })
    .unwrap();
    let n = reconcile_gitlab(&mut s, "account-b", &assigned(&[]), day(D1), at(D1, 9, 0)).unwrap();
    assert_eq!(n, 1);
}

#[test]
fn tick_on_a_later_day_is_closed_when_the_clock_moves_back() {
    let (_t, mut s) = open();
    let item = gl(ItemKind::Issue, 1);
    let a = own(&mut s, "Zadanie A");
    let ig = pick_gl(&mut s, &item, D1, 8, 5).unwrap();
    let io = pick_own(&mut s, a, D1, 8, 5).unwrap();
    // Ticked while the clock said D2, then the clock is back on D1.
    tick(&mut s, ig, true, D2, 9).unwrap();
    tick(&mut s, io, true, D2, 9).unwrap();
    let l = list_on(&mut s, D1, 5);
    assert!(l.items.is_empty());
    // Closed, so nothing hidden blocks a re-pick.
    pick_gl(&mut s, &item, D1, 10, 5).unwrap();
    assert_eq!(titles(&list_on(&mut s, D1, 5)), ["Pozycja testowa 1"]);
}

fn gh(repo: &str, n: u64) -> GithubIssue {
    GithubIssue {
        repo: repo.into(),
        number: n,
        reference: format!("{}#{n}", repo.to_lowercase()),
        title: format!("Zgloszenie testowe {n}"),
        web_url: Some(format!("https://github.com/{repo}/issues/{n}")),
        updated_at: "2026-11-20T10:00:00Z".into(),
        labels: vec![],
        changed: None,
        hidden_at_ms: None,
    }
}

fn pick_gh(
    store: &mut Store,
    issue: &GithubIssue,
    d: &str,
    h: u32,
    limit: u32,
) -> Result<i64, TodoError> {
    add_github(store, issue, day(d), at(d, h, 0), Ok(limit))
}

fn loaded<'a>(repo: &str, issues: &'a [GithubIssue]) -> (String, &'a [GithubIssue]) {
    (repo.to_string(), issues)
}

#[test]
fn github_duplicate_is_refused_case_insensitively_until_removed_or_closed() {
    let (_t, mut s) = open();
    let id = pick_gh(&mut s, &gh("Octo-Test/Alpha", 1), D1, 8, 5).unwrap();
    // Same issue, repo written in another case: the reference is lowercase.
    assert!(matches!(
        pick_gh(&mut s, &gh("octo-test/alpha", 1), D1, 9, 5),
        Err(TodoError::AlreadyOnList)
    ));
    remove(&mut s, id, day(D1), at(D1, 9, 0)).unwrap();
    pick_gh(&mut s, &gh("octo-test/alpha", 1), D1, 10, 5).unwrap();
    assert_eq!(list_on(&mut s, D1, 5).items.len(), 1);
}

#[test]
fn github_and_gitlab_entries_with_the_same_reference_coexist() {
    let (_t, mut s) = open();
    let mut lab = gl(ItemKind::Issue, 1);
    lab.reference = "a/b#1".into();
    pick_gl(&mut s, &lab, D1, 8, 5).unwrap();
    pick_gh(&mut s, &gh("a/b", 1), D1, 9, 5).unwrap();
    assert_eq!(list_on(&mut s, D1, 5).items.len(), 2);
    // A GitHub reconcile never touches the GitLab entry.
    let n = reconcile_github(&mut s, &[loaded("a/b", &[])], day(D1), at(D1, 10, 0)).unwrap();
    assert_eq!(n, 1);
    let l = list_on(&mut s, D1, 5);
    assert_eq!(l.open_count, 1);
    // And a GitLab reconcile never touches the GitHub entry.
    let n = reconcile_gitlab(&mut s, ACC, &assigned(&[]), day(D1), at(D1, 11, 0)).unwrap();
    assert_eq!(n, 1);
    assert_eq!(list_on(&mut s, D1, 5).open_count, 0);
}

#[test]
fn github_limit_counts_open_entries_of_every_source() {
    let (_t, mut s) = open();
    let a = own(&mut s, "Zadanie A");
    pick_own(&mut s, a, D1, 8, 2).unwrap();
    let id = pick_gh(&mut s, &gh("o/r", 1), D1, 9, 2).unwrap();
    assert!(matches!(
        pick_gh(&mut s, &gh("o/r", 2), D1, 10, 2),
        Err(TodoError::LimitReached { limit: 2 })
    ));
    tick(&mut s, id, true, D1, 11).unwrap();
    pick_gh(&mut s, &gh("o/r", 2), D1, 12, 2).unwrap();
}

#[test]
fn github_reconcile_ticks_only_missing_entries_of_loaded_repos() {
    let (_t, mut s) = open();
    let kept = gh("Octo-Test/Alpha", 1);
    let gone = gh("Octo-Test/Alpha", 2);
    let failed = gh("octo-test/beta", 1);
    let removed_from_settings = gh("octo-test/gamma", 1);
    for i in [&kept, &gone, &failed, &removed_from_settings] {
        pick_gh(&mut s, i, D1, 8, 9).unwrap();
    }

    let mut renamed = kept.clone();
    renamed.title = "Nowy tytul testowy".into();
    renamed.web_url = Some("https://github.com/Octo-Test/Alpha/issues/1?new".into());
    let alpha = [renamed];
    // Only alpha loaded: beta failed this round, gamma is no longer configured.
    let n = reconcile_github(
        &mut s,
        &[loaded("octo-test/alpha", &alpha)],
        day(D1),
        at(D1, 10, 0),
    )
    .unwrap();
    assert_eq!(n, 2);

    let l = list_on(&mut s, D1, 9);
    assert_eq!(l.open_count, 3);
    let by_ref = |r: &str| {
        l.items
            .iter()
            .find(|i| matches!(&i.entry, TodoEntry::Github { reference, .. } if reference == r))
            .unwrap()
    };
    assert!(!by_ref("octo-test/alpha#1").ticked);
    assert!(matches!(
        &by_ref("octo-test/alpha#1").entry,
        TodoEntry::Github { title, web_url: Some(u), .. }
            if title == "Nowy tytul testowy" && u.ends_with("?new")
    ));
    assert!(by_ref("octo-test/alpha#2").ticked);
    assert!(matches!(
        by_ref("octo-test/alpha#2").entry,
        TodoEntry::Github {
            closed_in_source: true,
            ..
        }
    ));
    assert!(!by_ref("octo-test/beta#1").ticked);
    assert!(!by_ref("octo-test/gamma#1").ticked);

    // Nothing changed since: a second round counts zero.
    let n = reconcile_github(
        &mut s,
        &[loaded("octo-test/alpha", &alpha)],
        day(D1),
        at(D1, 11, 0),
    )
    .unwrap();
    assert_eq!(n, 0);
    // No repo loaded: nothing is touched.
    assert_eq!(
        reconcile_github(&mut s, &[], day(D1), at(D1, 12, 0)).unwrap(),
        0
    );
}

#[test]
fn github_reconcile_keeps_the_stored_url_when_the_fresh_one_was_dropped() {
    let (_t, mut s) = open();
    pick_gh(&mut s, &gh("octo-test/alpha", 1), D1, 8, 5).unwrap();
    // The issue is still listed, but its URL failed the check (say, a rename).
    let mut no_url = gh("octo-test/alpha", 1);
    no_url.web_url = None;
    let alpha = [no_url];
    let n = reconcile_github(
        &mut s,
        &[loaded("octo-test/alpha", &alpha)],
        day(D1),
        at(D1, 10, 0),
    )
    .unwrap();
    assert_eq!(n, 0);
    let l = list_on(&mut s, D1, 5);
    assert!(matches!(
        &l.items[0].entry,
        TodoEntry::Github { web_url: Some(u), .. }
            if u == "https://github.com/octo-test/alpha/issues/1"
    ));

    // A title change still refreshes, and the stored URL survives it.
    let mut retitled = alpha[0].clone();
    retitled.title = "Nowy tytul testowy".into();
    let n = reconcile_github(
        &mut s,
        &[loaded("octo-test/alpha", &[retitled])],
        day(D1),
        at(D1, 11, 0),
    )
    .unwrap();
    assert_eq!(n, 1);
    let l = list_on(&mut s, D1, 5);
    assert!(matches!(
        &l.items[0].entry,
        TodoEntry::Github { title, web_url: Some(_), .. } if title == "Nowy tytul testowy"
    ));
}

#[test]
fn github_reconcile_leaves_ticked_removed_closed_and_other_sources_alone() {
    let (_t, mut s) = open();
    let a = own(&mut s, "Zadanie A");
    pick_own(&mut s, a, D1, 8, 9).unwrap();
    pick_gl(&mut s, &gl(ItemKind::Issue, 1), D1, 8, 9).unwrap();
    let ticked = pick_gh(&mut s, &gh("o/r", 1), D1, 8, 9).unwrap();
    let removed = pick_gh(&mut s, &gh("o/r", 2), D1, 8, 9).unwrap();
    tick(&mut s, ticked, true, D1, 9).unwrap();
    remove(&mut s, removed, day(D1), at(D1, 9, 0)).unwrap();

    let n = reconcile_github(&mut s, &[loaded("o/r", &[])], day(D1), at(D1, 10, 0)).unwrap();
    assert_eq!(n, 0);
    let l = list_on(&mut s, D1, 9);
    assert_eq!(l.items.len(), 3);
    // The user tick stayed a user tick.
    assert!(l.items.iter().any(|i| i.id == ticked
        && matches!(
            i.entry,
            TodoEntry::Github {
                closed_in_source: false,
                ..
            }
        )));
    assert_eq!(l.open_count, 2);
}

#[test]
fn github_source_tick_rolls_over_to_closed_the_next_day() {
    let (_t, mut s) = open();
    pick_gh(&mut s, &gh("o/r", 1), D1, 8, 5).unwrap();
    reconcile_github(&mut s, &[loaded("o/r", &[])], day(D1), at(D1, 10, 0)).unwrap();
    assert_eq!(list_on(&mut s, D1, 5).items.len(), 1);
    let l = list_on(&mut s, D2, 5);
    assert!(l.items.is_empty());
    // Closed: the issue can be picked again (it came back).
    pick_gh(&mut s, &gh("o/r", 1), D2, 9, 5).unwrap();
}

#[test]
fn github_user_tick_rolls_over_and_untick_works() {
    let (_t, mut s) = open();
    let id = pick_gh(&mut s, &gh("o/r", 1), D1, 8, 5).unwrap();
    tick(&mut s, id, true, D1, 9).unwrap();
    assert_eq!(list_on(&mut s, D1, 5).open_count, 0);
    tick(&mut s, id, false, D1, 10).unwrap();
    assert_eq!(list_on(&mut s, D1, 5).open_count, 1);
    tick(&mut s, id, true, D1, 11).unwrap();
    assert!(list_on(&mut s, D2, 5).items.is_empty());
}

#[test]
fn github_list_json_shape() {
    let (_t, mut s) = open();
    pick_gh(&mut s, &gh("Octo-Test/Alpha", 4), D1, 9, 5).unwrap();
    let v = serde_json::to_value(list_on(&mut s, D1, 5)).unwrap();
    let e = &v["items"][0]["entry"];
    assert_eq!(e["kind"], "github");
    assert_eq!(e["reference"], "octo-test/alpha#4");
    assert_eq!(e["title"], "Zgloszenie testowe 4");
    assert_eq!(e["closedInSource"], false);
    assert!(e["webUrl"].is_string());
    let json = serde_json::to_string(&TodoError::NotInGithubList).unwrap();
    assert_eq!(json, r#"{"kind":"NotInGithubList"}"#);
}

#[test]
fn error_json_shape() {
    let json = serde_json::to_string(&TodoError::LimitReached { limit: 5 }).unwrap();
    assert_eq!(json, r#"{"kind":"LimitReached","detail":{"limit":5}}"#);
    let json = serde_json::to_string(&TodoError::AlreadyOnList).unwrap();
    assert_eq!(json, r#"{"kind":"AlreadyOnList"}"#);
    let json = serde_json::to_string(&TodoError::Store(StoreError::Busy)).unwrap();
    assert_eq!(json, r#"{"kind":"Store","detail":{"kind":"Busy"}}"#);
}

#[test]
fn list_json_shape() {
    let (_t, mut s) = open();
    let a = own(&mut s, "Zadanie A");
    pick_own(&mut s, a, D1, 8, 5).unwrap();
    pick_gl(&mut s, &gl(ItemKind::Issue, 4), D1, 9, 5).unwrap();
    let l = list(
        &mut s,
        day(D1),
        at(D1, 9, 0),
        Err(TodoError::SettingsInvalid),
    )
    .unwrap();
    let v = serde_json::to_value(&l).unwrap();
    assert_eq!(v["day"], D1);
    assert_eq!(v["limit"], serde_json::Value::Null);
    assert_eq!(v["limitError"]["kind"], "SettingsInvalid");
    assert_eq!(v["openCount"], 2);
    let first = &v["items"][0];
    assert!(first["id"].is_i64());
    assert_eq!(first["ticked"], false);
    assert_eq!(first["carriedOver"], false);
    assert_eq!(first["pickedOn"], D1);
    assert_eq!(first["pickedAt"], "2026-11-23T08:00:00");
    assert_eq!(first["entry"]["kind"], "own");
    assert_eq!(first["entry"]["taskId"], a);
    assert_eq!(first["entry"]["space"], "work");
    assert_eq!(first["entry"]["title"], "Zadanie A");
    let second = &v["items"][1]["entry"];
    assert_eq!(second["kind"], "gitlab");
    assert_eq!(second["itemKind"], "issue");
    assert_eq!(second["reference"], "invented/project#4");
    assert_eq!(second["title"], "Pozycja testowa 4");
    assert_eq!(second["closedInSource"], false);
    assert!(second["webUrl"].is_string());
    // The stored account never reaches the webview.
    assert!(v["items"][1].get("account").is_none() && second.get("account").is_none());
}

#[test]
fn errors_never_contain_titles_or_references() {
    let (_t, mut s) = open();
    let secret = gl(ItemKind::Issue, 99);
    pick_gl(&mut s, &secret, D1, 8, 1).unwrap();
    let errors = [
        pick_gl(&mut s, &secret, D1, 9, 5).unwrap_err(),
        pick_gl(&mut s, &gl(ItemKind::Issue, 98), D1, 9, 1).unwrap_err(),
        pick_own(&mut s, 999, D1, 9, 5).unwrap_err(),
        TodoError::SettingsInvalid,
        TodoError::NotInGitlabList,
        TodoError::NotInGithubList,
        pick_gh(&mut s, &gh("octo-test/alpha", 1), D1, 9, 1).unwrap_err(),
    ];
    for e in errors {
        let shown = format!("{e} {e:?} {}", serde_json::to_string(&e).unwrap());
        for leak in ["Pozycja", "invented", "example.invalid", "project"] {
            assert!(!shown.contains(leak), "leaked {leak} in {shown}");
        }
    }
}

// --- event log (S-08) ---

fn evs(store: &Store, d: &str) -> Vec<EventView> {
    events::list_today(store, day(d))
        .unwrap()
        .events
        .into_iter()
        .rev()
        .map(|e| e.event)
        .collect()
}

fn subject(source: ItemSource, reference: Option<&str>, title: &str) -> Subject {
    Subject {
        source,
        reference: reference.map(String::from),
        title: title.into(),
    }
}

#[test]
fn ticking_gitlab_stores_one_event_and_repeats_or_unticks_store_none() {
    let (_t, mut s) = open();
    let id = pick_gl(&mut s, &gl(ItemKind::Issue, 1), D1, 8, 5).unwrap();
    tick(&mut s, id, true, D1, 9).unwrap();
    tick(&mut s, id, true, D1, 10).unwrap();
    tick(&mut s, id, false, D1, 11).unwrap();
    assert_eq!(
        evs(&s, D1),
        [EventView::Ticked(subject(
            ItemSource::Gitlab,
            Some("invented/project#1"),
            "Pozycja testowa 1"
        ))]
    );
    // Ticking again after an untick is a new change.
    tick(&mut s, id, true, D1, 12).unwrap();
    assert_eq!(evs(&s, D1).len(), 2);
}

#[test]
fn ticking_github_stores_reference_and_title() {
    let (_t, mut s) = open();
    let id = pick_gh(&mut s, &gh("Octo-Test/Alpha", 1), D1, 8, 5).unwrap();
    tick(&mut s, id, true, D1, 9).unwrap();
    assert_eq!(
        evs(&s, D1),
        [EventView::Ticked(subject(
            ItemSource::Github,
            Some("octo-test/alpha#1"),
            "Zgloszenie testowe 1"
        ))]
    );
}

#[test]
fn ticking_own_stores_title_without_reference_and_not_when_already_done() {
    let (_t, mut s) = open();
    let task = own(&mut s, "Zadanie wlasne");
    let id = pick_own(&mut s, task, D1, 8, 5).unwrap();
    tick(&mut s, id, true, D1, 9).unwrap();
    tick(&mut s, id, true, D1, 10).unwrap();
    assert_eq!(
        evs(&s, D1),
        [EventView::Ticked(subject(
            ItemSource::Own,
            None,
            "Zadanie wlasne"
        ))]
    );
    tick(&mut s, id, false, D1, 11).unwrap();
    assert_eq!(evs(&s, D1).len(), 1);
}

#[test]
fn removing_stores_removed_and_a_refusal_stores_nothing() {
    let (_t, mut s) = open();
    let id = pick_gl(&mut s, &gl(ItemKind::MergeRequest, 3), D1, 8, 5).unwrap();
    assert!(matches!(
        remove(&mut s, 999, day(D1), at(D1, 9, 0)),
        Err(TodoError::NotFound)
    ));
    assert!(matches!(
        set_ticked(&mut s, 999, true, day(D1), at(D1, 9, 0)),
        Err(TodoError::NotFound)
    ));
    assert!(evs(&s, D1).is_empty());
    remove(&mut s, id, day(D1), at(D1, 9, 0)).unwrap();
    assert!(matches!(
        remove(&mut s, id, day(D1), at(D1, 9, 5)),
        Err(TodoError::NotFound)
    ));
    assert_eq!(
        evs(&s, D1),
        [EventView::Removed(subject(
            ItemSource::Gitlab,
            Some("invented/project!3"),
            "Pozycja testowa 3"
        ))]
    );
}

#[test]
fn reconcile_gitlab_stores_closed_in_source_but_not_for_a_refresh() {
    let (_t, mut s) = open();
    let kept = gl(ItemKind::MergeRequest, 1);
    let gone = gl(ItemKind::Issue, 2);
    pick_gl(&mut s, &kept, D1, 8, 5).unwrap();
    pick_gl(&mut s, &gone, D1, 9, 5).unwrap();
    let mut renamed = kept.clone();
    renamed.title = "Nowy tytul testowy".into();
    reconcile_gitlab(&mut s, ACC, &assigned(&[&renamed]), day(D1), at(D1, 10, 0)).unwrap();
    assert_eq!(
        evs(&s, D1),
        [EventView::ClosedInSource(subject(
            ItemSource::Gitlab,
            Some("invented/project#2"),
            "Pozycja testowa 2"
        ))]
    );
}

#[test]
fn reconcile_github_stores_closed_in_source_but_not_for_a_refresh() {
    let (_t, mut s) = open();
    let kept = gh("octo-test/alpha", 1);
    let gone = gh("octo-test/alpha", 2);
    pick_gh(&mut s, &kept, D1, 8, 5).unwrap();
    pick_gh(&mut s, &gone, D1, 9, 5).unwrap();
    let mut renamed = kept.clone();
    renamed.title = "Nowy tytul testowy".into();
    let issues = [renamed];
    reconcile_github(
        &mut s,
        &[loaded("octo-test/alpha", &issues)],
        day(D1),
        at(D1, 10, 0),
    )
    .unwrap();
    assert_eq!(
        evs(&s, D1),
        [EventView::ClosedInSource(subject(
            ItemSource::Github,
            Some("octo-test/alpha#2"),
            "Zgloszenie testowe 2"
        ))]
    );
}
