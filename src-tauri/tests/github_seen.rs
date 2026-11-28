//! GitHub seen-set tables (migration v16) against a real store with the
//! production migrations. Invented repos (`octo-test/*`) and ids only.

mod common;

use chrono::NaiveDate;
use common::test_store_dir;
use day_navigator_lib::store::{Store, StoreError, schema};

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 28).unwrap()
}

fn open(dir: &std::path::Path) -> Store {
    Store::open(dir, schema::MIGRATIONS, today()).unwrap()
}

fn scalar(store: &Store, sql: &'static str) -> i64 {
    store
        .read::<_, StoreError>(|c| Ok(c.query_row(sql, [], |r| r.get(0))?))
        .unwrap()
}

fn columns(store: &Store, table: &'static str) -> Vec<String> {
    store
        .read::<_, StoreError>(|c| {
            let sql = format!("PRAGMA table_info({table})");
            let mut stmt = c.prepare(&sql)?;
            let names = stmt
                .query_map([], |r| r.get::<_, String>(1))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(names)
        })
        .unwrap()
}

fn exec(store: &mut Store, sql: &'static str) -> Result<(), StoreError> {
    store.write::<_, StoreError>(|tx| {
        tx.execute(sql, [])?;
        Ok(())
    })
}

#[test]
fn v15_database_upgrades_to_the_github_seen_tables_and_keeps_its_data() {
    let (_tmp, dir) = test_store_dir();
    let pos = schema::MIGRATIONS
        .iter()
        .position(|m| m.sql.contains("CREATE TABLE github_seen_repos"))
        .expect("the GitHub seen migration");
    assert_eq!(schema::MIGRATIONS[pos].version, 16);
    {
        let mut old = Store::open(&dir, &schema::MIGRATIONS[..pos], today()).unwrap();
        assert_eq!(scalar(&old, "PRAGMA user_version"), 15);
        old.write::<_, StoreError>(|tx| {
            tx.execute(
                "INSERT INTO days_off(day, created_at_ms) VALUES ('2026-11-12', 1000)",
                [],
            )?;
            Ok(())
        })
        .unwrap();
    }
    let store = open(&dir);
    assert_eq!(
        scalar(&store, "PRAGMA user_version"),
        schema::MIGRATIONS.len() as i64
    );
    assert_eq!(scalar(&store, "SELECT count(*) FROM days_off"), 1);
    assert_eq!(scalar(&store, "SELECT count(*) FROM github_seen_repos"), 0);
    assert_eq!(scalar(&store, "SELECT count(*) FROM github_seen_issues"), 0);
    assert_eq!(scalar(&store, "SELECT count(*) FROM github_view_mark"), 0);
}

#[test]
fn github_seen_tables_refuse_invalid_rows() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    let refused = [
        "INSERT INTO github_seen_repos(repo, seeded_at_ms) VALUES ('', 1)",
        "INSERT INTO github_seen_repos(repo, seeded_at_ms) VALUES ('Octo-Test/Alpha', 1)",
        "INSERT INTO github_seen_issues(repo, issue_number, first_seen_ms, last_seen_ms) VALUES ('octo-test/alpha', 1, 1, 1)",
        "INSERT INTO github_seen_repos(repo, seeded_at_ms) VALUES ('octo-test/alpha', 1); INSERT INTO github_seen_issues(repo, issue_number, first_seen_ms, last_seen_ms) VALUES ('octo-test/alpha', 0, 1, 1)",
        "INSERT INTO github_view_mark(id, seen_until_ms) VALUES (2, 1)",
    ];
    for sql in &refused[..2] {
        let err = exec(&mut store, sql).unwrap_err();
        assert!(matches!(err, StoreError::Sql { .. }), "{sql}: {err:?}");
    }
    // Issue whose repo has no seeding row: the foreign key refuses it.
    let err = exec(&mut store, refused[2]).unwrap_err();
    assert!(matches!(err, StoreError::Sql { .. }), "fk: {err:?}");
    // Issue number 0: the CHECK refuses it (the repo row is valid).
    let err = store
        .write::<_, StoreError>(|tx| {
            tx.execute_batch(refused[3])?;
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(err, StoreError::Sql { .. }), "check: {err:?}");
    let err = exec(&mut store, refused[4]).unwrap_err();
    assert!(matches!(err, StoreError::Sql { .. }), "mark: {err:?}");

    // A valid repo, issue and mark are accepted.
    exec(
        &mut store,
        "INSERT INTO github_seen_repos(repo, seeded_at_ms) VALUES ('octo-test/beta', 1)",
    )
    .unwrap();
    exec(
        &mut store,
        "INSERT INTO github_seen_issues(repo, issue_number, first_seen_ms, last_seen_ms) VALUES ('octo-test/beta', 5, 1, 1)",
    )
    .unwrap();
    exec(
        &mut store,
        "INSERT INTO github_view_mark(id, seen_until_ms) VALUES (1, 1)",
    )
    .unwrap();
}

#[test]
fn github_seen_tables_hold_only_keys_and_instants() {
    let (_tmp, dir) = test_store_dir();
    let store = open(&dir);
    assert_eq!(
        columns(&store, "github_seen_repos"),
        ["repo", "seeded_at_ms"]
    );
    assert_eq!(
        columns(&store, "github_seen_issues"),
        ["repo", "issue_number", "first_seen_ms", "last_seen_ms"]
    );
    assert_eq!(columns(&store, "github_view_mark"), ["id", "seen_until_ms"]);
}
