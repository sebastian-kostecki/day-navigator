//! "To do" list against a real store with the production migrations.

mod common;

use chrono::{NaiveDate, NaiveDateTime};
use common::test_store_dir;
use day_navigator_lib::gitlab::{AssignedItems, GitlabItem, ItemKind};
use day_navigator_lib::own_tasks::{self, OwnTaskInput};
use day_navigator_lib::space::Space;
use day_navigator_lib::store::{Store, StoreError, schema};
use day_navigator_lib::todo::{self, TodoEntry, TodoError};

/// Invented `gitlab::account_key` of the account the test lists come from.
const ACC: &str = "account-a";

fn day(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

fn at(d: &str, h: u32) -> NaiveDateTime {
    day(d).and_hms_opt(h, 0, 0).unwrap()
}

fn open(dir: &std::path::Path) -> Store {
    Store::open(dir, schema::MIGRATIONS, day("2026-11-23")).unwrap()
}

fn task(store: &mut Store, title: &str) -> i64 {
    let input = OwnTaskInput {
        space: Space::Private,
        title: title.into(),
        note: String::new(),
        due: None,
        priority: None,
    };
    own_tasks::add(store, &input, at("2026-11-23", 7))
        .unwrap()
        .id
}

fn item(n: u32) -> GitlabItem {
    GitlabItem {
        kind: ItemKind::MergeRequest,
        reference: format!("invented/project!{n}"),
        title: format!("Zmiana testowa {n}"),
        web_url: Some(format!("https://gitlab.example.invalid/invented/{n}")),
        updated_at: "2026-11-20T10:00:00+01:00".into(),
        labels: vec![],
        draft: false,
    }
}

#[test]
fn migration_v3_applies_on_a_fresh_store_with_its_table_and_indexes() {
    let (_tmp, dir) = test_store_dir();
    let store = open(&dir);
    let names: Vec<String> = store
        .read::<_, StoreError>(|c| {
            let mut stmt = c.prepare(
                "SELECT name FROM sqlite_master WHERE tbl_name = 'todo_items' ORDER BY name",
            )?;
            Ok(stmt
                .query_map([], |r| r.get(0))?
                .collect::<Result<Vec<_>, _>>()?)
        })
        .unwrap();
    assert_eq!(
        names,
        [
            "todo_items",
            "todo_items_live_gitlab",
            "todo_items_live_own"
        ]
    );
}

#[test]
fn table_check_refuses_inconsistent_rows() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    let bad = [
        // own without a task
        "INSERT INTO todo_items (source, picked_on, picked_at) VALUES ('own','2026-11-23','2026-11-23T08:00:00')",
        // gitlab with a tick but no ticked_by
        "INSERT INTO todo_items (source, gitlab_kind, gitlab_ref, gitlab_title, picked_on, picked_at, ticked_at) \
         VALUES ('gitlab','issue','invented/p#1','t','2026-11-23','2026-11-23T08:00:00','2026-11-23T09:00:00')",
        // gitlab without a reference
        "INSERT INTO todo_items (source, gitlab_kind, gitlab_title, picked_on, picked_at) \
         VALUES ('gitlab','issue','t','2026-11-23','2026-11-23T08:00:00')",
    ];
    for sql in bad {
        let r = store.write::<_, StoreError>(|tx| Ok(tx.execute(sql, [])?));
        assert!(r.is_err(), "{sql}");
    }
}

#[test]
fn full_day_flow_survives_reopen() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    let own_id = task(&mut store, "Zadanie testowe");
    let d1 = "2026-11-23";
    let e_own = todo::add_own(&mut store, own_id, day(d1), at(d1, 8), Ok(5)).unwrap();
    let e_gl = todo::add_gitlab(&mut store, ACC, &item(1), day(d1), at(d1, 9), Ok(5)).unwrap();
    todo::set_ticked(&mut store, e_own, true, day(d1), at(d1, 10)).unwrap();
    drop(store);

    let mut store = open(&dir);
    let l = todo::list(&mut store, day(d1), at(d1, 11), Ok(5)).unwrap();
    assert_eq!(l.open_count, 1);
    assert_eq!(l.items.len(), 2);
    assert_eq!(l.items[0].id, e_gl);
    assert!(l.items[1].ticked);

    // Ticking the own entry is the own task's done state.
    let tasks = own_tasks::list(&store).unwrap();
    assert!(
        tasks
            .iter()
            .find(|t| t.id == own_id)
            .unwrap()
            .done_at
            .is_some()
    );

    // Next day: the ticked entry is gone, the open one carries over.
    let d2 = "2026-11-24";
    let l = todo::list(&mut store, day(d2), at(d2, 8), Ok(5)).unwrap();
    assert_eq!(l.items.len(), 1);
    assert!(l.items[0].carried_over);
    assert!(matches!(l.items[0].entry, TodoEntry::Gitlab { .. }));
}

#[test]
fn reconcile_flow_against_a_real_store() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    let d1 = "2026-11-23";
    todo::add_gitlab(&mut store, ACC, &item(1), day(d1), at(d1, 8), Ok(5)).unwrap();
    todo::add_gitlab(&mut store, ACC, &item(2), day(d1), at(d1, 9), Ok(5)).unwrap();
    let mut still = item(1);
    still.title = "Zmiana testowa 1 po zmianie".into();
    let list = AssignedItems {
        merge_requests: vec![still],
        issues: vec![],
    };
    let n = todo::reconcile_gitlab(&mut store, ACC, &list, day(d1), at(d1, 10)).unwrap();
    assert_eq!(n, 2);
    let n = todo::reconcile_gitlab(&mut store, ACC, &list, day(d1), at(d1, 11)).unwrap();
    assert_eq!(n, 0);
    let l = todo::list(&mut store, day(d1), at(d1, 12), Ok(5)).unwrap();
    assert_eq!(l.open_count, 1);
    assert!(l.items[1].ticked);
}

#[test]
fn limit_and_duplicates_against_a_real_store() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    let d1 = "2026-11-23";
    let a = task(&mut store, "Zadanie A");
    todo::add_own(&mut store, a, day(d1), at(d1, 8), Ok(1)).unwrap();
    assert!(matches!(
        todo::add_own(&mut store, a, day(d1), at(d1, 9), Ok(1)),
        Err(TodoError::AlreadyOnList)
    ));
    assert!(matches!(
        todo::add_gitlab(&mut store, ACC, &item(1), day(d1), at(d1, 9), Ok(1)),
        Err(TodoError::LimitReached { limit: 1 })
    ));
}
