//! Own tasks against a real store with the production migrations.

mod common;

use chrono::{NaiveDate, NaiveDateTime};
use common::test_store_dir;
use day_navigator_lib::own_tasks::{self, OwnTaskError, OwnTaskInput, Priority};
use day_navigator_lib::space::Space;
use day_navigator_lib::store::{Store, StoreError, schema};

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 23).unwrap()
}

fn at(h: u32, m: u32) -> NaiveDateTime {
    today().and_hms_opt(h, m, 0).unwrap()
}

fn open(dir: &std::path::Path) -> Store {
    Store::open(dir, schema::MIGRATIONS, today()).unwrap()
}

fn input(title: &str) -> OwnTaskInput {
    OwnTaskInput {
        space: Space::Work,
        title: title.into(),
        note: "Pierwsza linia\r\nDruga linia".into(),
        due: Some("2026-12-01".into()),
        priority: Some(Priority::High),
    }
}

fn count(store: &Store) -> i64 {
    store
        .read::<_, StoreError>(|c| {
            Ok(c.query_row("SELECT count(*) FROM own_tasks", [], |r| r.get(0))?)
        })
        .unwrap()
}

#[test]
fn production_migrations_apply_on_a_fresh_store() {
    let (_tmp, dir) = test_store_dir();
    let store = open(&dir);
    let version: i64 = store
        .read::<_, StoreError>(|c| Ok(c.query_row("PRAGMA user_version", [], |r| r.get(0))?))
        .unwrap();
    assert_eq!(
        version,
        i64::from(schema::MIGRATIONS.last().unwrap().version)
    );
    assert_eq!(count(&store), 0);
}

#[test]
fn add_list_edit_done_reopen_round_trip() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);

    let added = own_tasks::add(&mut store, &input("  Zadanie testowe "), at(8, 0)).unwrap();
    assert_eq!(added.title, "Zadanie testowe");
    assert_eq!(added.note, "Pierwsza linia\nDruga linia");
    assert_eq!(added.space, Space::Work);
    assert_eq!(added.due.as_deref(), Some("2026-12-01"));
    assert_eq!(added.priority, Some(Priority::High));
    assert_eq!(added.created_at, "2026-11-23T08:00:00");
    assert_eq!(added.updated_at, "2026-11-23T08:00:00");
    assert_eq!(added.done_at, None);

    assert_eq!(own_tasks::list(&store).unwrap(), vec![added.clone()]);

    let replacement = OwnTaskInput {
        space: Space::Private,
        title: "Zmieniony tytul".into(),
        note: String::new(),
        due: None,
        priority: None,
    };
    let edited = own_tasks::edit(&mut store, added.id, &replacement, at(9, 0)).unwrap();
    assert_eq!(edited.id, added.id);
    assert_eq!(edited.space, Space::Private);
    assert_eq!(edited.title, "Zmieniony tytul");
    assert_eq!(edited.note, "");
    assert_eq!(edited.due, None);
    assert_eq!(edited.priority, None);
    assert_eq!(edited.created_at, "2026-11-23T08:00:00");
    assert_eq!(edited.updated_at, "2026-11-23T09:00:00");

    let done = own_tasks::set_done(&mut store, added.id, true, at(10, 0)).unwrap();
    assert_eq!(done.done_at.as_deref(), Some("2026-11-23T10:00:00"));
    assert_eq!(done.updated_at, "2026-11-23T10:00:00");

    let reopened = own_tasks::set_done(&mut store, added.id, false, at(11, 0)).unwrap();
    assert_eq!(reopened.done_at, None);
    assert_eq!(reopened.updated_at, "2026-11-23T11:00:00");
    assert_eq!(own_tasks::list(&store).unwrap(), vec![reopened]);
}

#[test]
fn list_is_sorted_for_display() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    let mut later = input("Pozniej");
    later.due = Some("2026-12-31".into());
    let mut none = input("Bez terminu");
    none.due = None;
    own_tasks::add(&mut store, &none, at(8, 0)).unwrap();
    own_tasks::add(&mut store, &later, at(8, 1)).unwrap();
    own_tasks::add(&mut store, &input("Wczesniej"), at(8, 2)).unwrap();
    let titles: Vec<String> = own_tasks::list(&store)
        .unwrap()
        .into_iter()
        .map(|t| t.title)
        .collect();
    assert_eq!(titles, ["Wczesniej", "Pozniej", "Bez terminu"]);
}

#[test]
fn data_survives_close_and_reopen() {
    let (_tmp, dir) = test_store_dir();
    let id = {
        let mut store = open(&dir);
        let t = own_tasks::add(&mut store, &input("Przetrwa"), at(8, 0)).unwrap();
        own_tasks::set_done(&mut store, t.id, true, at(9, 0)).unwrap();
        t.id
    };
    let store = open(&dir);
    let tasks = own_tasks::list(&store).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].id, id);
    assert_eq!(tasks[0].title, "Przetrwa");
    assert_eq!(tasks[0].done_at.as_deref(), Some("2026-11-23T09:00:00"));
}

#[test]
fn unknown_id_is_not_found() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    assert!(matches!(
        own_tasks::edit(&mut store, 42, &input("x"), at(8, 0)),
        Err(OwnTaskError::NotFound)
    ));
    assert!(matches!(
        own_tasks::set_done(&mut store, 42, true, at(8, 0)),
        Err(OwnTaskError::NotFound)
    ));
    assert!(matches!(
        own_tasks::set_done(&mut store, 42, false, at(8, 0)),
        Err(OwnTaskError::NotFound)
    ));
    assert_eq!(count(&store), 0);
}

#[test]
fn failed_validation_leaves_the_table_unchanged() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    let kept = own_tasks::add(&mut store, &input("Zostaje"), at(8, 0)).unwrap();

    let mut bad = input("   ");
    assert!(matches!(
        own_tasks::add(&mut store, &bad, at(9, 0)),
        Err(OwnTaskError::TitleEmpty)
    ));
    bad = input("Nowy");
    bad.due = Some("2026-02-30".into());
    assert!(matches!(
        own_tasks::add(&mut store, &bad, at(9, 0)),
        Err(OwnTaskError::InvalidDue)
    ));
    bad = input(&"a".repeat(201));
    assert!(matches!(
        own_tasks::edit(&mut store, kept.id, &bad, at(9, 0)),
        Err(OwnTaskError::TitleTooLong { max: 200 })
    ));
    bad = input("ok");
    bad.note = "n".repeat(2001);
    assert!(matches!(
        own_tasks::edit(&mut store, kept.id, &bad, at(9, 0)),
        Err(OwnTaskError::NoteTooLong { max: 2000 })
    ));

    assert_eq!(own_tasks::list(&store).unwrap(), vec![kept]);
}

#[test]
fn marking_done_twice_keeps_the_first_done_at() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    let t = own_tasks::add(&mut store, &input("Dwa razy"), at(8, 0)).unwrap();
    let first = own_tasks::set_done(&mut store, t.id, true, at(9, 0)).unwrap();
    let second = own_tasks::set_done(&mut store, t.id, true, at(10, 0)).unwrap();
    assert_eq!(first.done_at, second.done_at);
    assert_eq!(second.done_at.as_deref(), Some("2026-11-23T09:00:00"));
}

#[test]
fn check_constraints_back_up_the_rust_rules() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    let err = store.write::<_, StoreError>(|tx| {
        tx.execute(
            "INSERT INTO own_tasks (space, title, note, created_at, updated_at) \
             VALUES ('other', 't', '', 'a', 'a')",
            [],
        )?;
        Ok(())
    });
    assert!(matches!(err, Err(StoreError::Sql { .. })));
    assert_eq!(count(&store), 0);
}
