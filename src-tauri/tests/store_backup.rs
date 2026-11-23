mod common;

use std::fs;
use std::path::Path;

use chrono::{Days, NaiveDate};
use common::test_store_dir;
use day_navigator_lib::store::{DB_FILE, Migration, Store, StoreError};
use rusqlite::Connection;

const V1: Migration = Migration {
    version: 1,
    sql: "CREATE TABLE items(id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE);",
};

fn day(offset: u64) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 1)
        .unwrap()
        .checked_add_days(Days::new(offset))
        .unwrap()
}

fn backup_name(date: NaiveDate) -> String {
    format!("day-navigator-{date}.db")
}

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

fn insert(store: &mut Store, name: &str) {
    store
        .write::<_, StoreError>(|tx| {
            tx.execute("INSERT INTO items(name) VALUES (?1)", [name])?;
            Ok(())
        })
        .unwrap();
}

#[test]
fn nine_days_keep_the_seven_newest() {
    let (_tmp, dir) = test_store_dir();
    for i in 0..9 {
        drop(Store::open(&dir, &[V1], day(i)).unwrap());
    }
    let expected: Vec<String> = (2..9).map(|i| backup_name(day(i))).collect();
    assert_eq!(names(&dir.join("backups")), expected);
}

#[test]
fn second_call_on_same_date_adds_nothing() {
    let (_tmp, dir) = test_store_dir();
    let mut store = Store::open(&dir, &[V1], day(0)).unwrap();
    let before = names(&dir.join("backups"));
    assert_eq!(before, vec![backup_name(day(0))]);
    store.backup_if_due(day(0));
    assert_eq!(names(&dir.join("backups")), before);
    store.backup_if_due(day(1));
    assert_eq!(names(&dir.join("backups")).len(), 2);
}

#[test]
fn backup_is_a_valid_database_with_the_same_rows() {
    let (_tmp, dir) = test_store_dir();
    let mut store = Store::open(&dir, &[V1], day(0)).unwrap();
    insert(&mut store, "alpha");
    insert(&mut store, "beta");
    store.backup_if_due(day(1));

    let c = Connection::open(dir.join("backups").join(backup_name(day(1)))).unwrap();
    let check: String = c.query_row("PRAGMA quick_check", [], |r| r.get(0)).unwrap();
    assert_eq!(check, "ok");
    let mut stmt = c.prepare("SELECT name FROM items ORDER BY id").unwrap();
    let rows: Vec<String> = stmt
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(rows, vec!["alpha", "beta"]);
    let v: i64 = c
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(v, 1);
}

#[test]
fn stale_tmp_is_removed_and_unrelated_file_stays() {
    let (_tmp, dir) = test_store_dir();
    drop(Store::open(&dir, &[V1], day(0)).unwrap());
    let backups = dir.join("backups");
    fs::write(backups.join(".day-navigator-2026-10-01.db.tmp"), b"junk").unwrap();
    fs::write(backups.join("notes.txt"), b"mine").unwrap();
    drop(Store::open(&dir, &[V1], day(1)).unwrap());
    assert_eq!(
        names(&backups),
        vec![
            backup_name(day(0)),
            backup_name(day(1)),
            "notes.txt".to_string()
        ]
    );
    assert_eq!(fs::read(backups.join("notes.txt")).unwrap(), b"mine");
}

#[test]
fn failing_backup_does_not_block_open_and_is_not_retried_on_same_date() {
    let (_tmp, dir) = test_store_dir();
    fs::create_dir_all(&dir).unwrap();
    let backups = dir.join("backups");
    fs::write(&backups, b"i am a file, not a folder").unwrap();

    let mut store = Store::open(&dir, &[V1], day(0)).unwrap();
    store.check().unwrap();

    // Make a backup possible, then call again for the same date: no new attempt.
    fs::remove_file(&backups).unwrap();
    fs::create_dir(&backups).unwrap();
    store.backup_if_due(day(0));
    assert!(names(&backups).is_empty());

    // A later date is attempted again.
    store.backup_if_due(day(1));
    assert_eq!(names(&backups), vec![backup_name(day(1))]);
}

#[test]
fn newer_schema_store_writes_no_backup() {
    let (_tmp, dir) = test_store_dir();
    fs::create_dir_all(&dir).unwrap();
    {
        let c = Connection::open(dir.join(DB_FILE)).unwrap();
        c.execute_batch("CREATE TABLE items(id INTEGER); PRAGMA user_version = 5;")
            .unwrap();
    }
    let r = Store::open(&dir, &[V1], day(0));
    assert!(matches!(r, Err(StoreError::NewerSchema { .. })));
    assert!(!dir.join("backups").exists());
}

#[test]
fn damaged_store_writes_no_backup() {
    let (_tmp, dir) = test_store_dir();
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join(DB_FILE),
        "this is plainly not a database, just some invented text ".repeat(20),
    )
    .unwrap();
    let r = Store::open(&dir, &[V1], day(0));
    assert!(matches!(r, Err(StoreError::Damaged { .. })));
    assert!(!dir.join("backups").exists());
}
