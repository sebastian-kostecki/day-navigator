//! The remembered GitLab view state against a real store with the production
//! migrations. Invented values only.

mod common;

use chrono::NaiveDate;
use common::test_store_dir;
use day_navigator_lib::gitlab_view_state::{GitlabViewState, ViewKind, load, save};
use day_navigator_lib::store::{Store, StoreError, schema};

const KINDS: [ViewKind; 3] = [ViewKind::All, ViewKind::Issue, ViewKind::MergeRequest];

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 27).unwrap()
}

fn scalar(store: &Store, sql: &'static str) -> i64 {
    store
        .read::<_, StoreError>(|c| Ok(c.query_row(sql, [], |r| r.get(0))?))
        .unwrap()
}

#[test]
fn empty_store_loads_the_defaults() {
    let (_tmp, dir) = test_store_dir();
    let store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
    assert_eq!(
        load(&store).unwrap(),
        GitlabViewState {
            kind: ViewKind::All,
            show_hidden: false
        }
    );
    assert_eq!(scalar(&store, "SELECT count(*) FROM gitlab_view_state"), 0);
}

#[test]
fn save_then_load_round_trips_every_value() {
    let (_tmp, dir) = test_store_dir();
    for kind in KINDS {
        for show_hidden in [false, true] {
            let view = GitlabViewState { kind, show_hidden };
            {
                let mut store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
                save(&mut store, view).unwrap();
                assert_eq!(load(&store).unwrap(), view);
            }
            let store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
            assert_eq!(load(&store).unwrap(), view);
        }
    }
}

#[test]
fn save_keeps_exactly_one_row() {
    let (_tmp, dir) = test_store_dir();
    let mut store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
    for kind in KINDS {
        save(
            &mut store,
            GitlabViewState {
                kind,
                show_hidden: true,
            },
        )
        .unwrap();
    }
    assert_eq!(scalar(&store, "SELECT count(*) FROM gitlab_view_state"), 1);
}

#[test]
fn the_table_refuses_invalid_rows() {
    let (_tmp, dir) = test_store_dir();
    let mut store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
    for sql in [
        "INSERT INTO gitlab_view_state (id, kind, show_hidden) VALUES (1, 'epic', 0)",
        "INSERT INTO gitlab_view_state (id, kind, show_hidden) VALUES (1, 'all', 2)",
        "INSERT INTO gitlab_view_state (id, kind, show_hidden) VALUES (2, 'all', 0)",
    ] {
        let result = store.write::<_, StoreError>(|tx| {
            tx.execute(sql, [])?;
            Ok(())
        });
        assert!(result.is_err(), "{sql}");
    }
    assert_eq!(scalar(&store, "SELECT count(*) FROM gitlab_view_state"), 0);
}

#[test]
fn v13_database_upgrades_to_the_view_state_table() {
    let (_tmp, dir) = test_store_dir();
    let pos = schema::MIGRATIONS
        .iter()
        .position(|m| m.sql.contains("CREATE TABLE gitlab_view_state"))
        .expect("the view state migration");
    assert_eq!(schema::MIGRATIONS[pos].version, 14);
    {
        let mut old = Store::open(&dir, &schema::MIGRATIONS[..pos], today()).unwrap();
        assert_eq!(scalar(&old, "PRAGMA user_version"), 13);
        old.write::<_, StoreError>(|tx| {
            tx.execute(
                "INSERT INTO days_off(day, created_at_ms) VALUES ('2026-11-12', 1000)",
                [],
            )?;
            Ok(())
        })
        .unwrap();
    }
    let store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
    assert_eq!(
        scalar(&store, "PRAGMA user_version"),
        schema::MIGRATIONS.len() as i64
    );
    assert_eq!(scalar(&store, "SELECT count(*) FROM days_off"), 1);
    assert_eq!(load(&store).unwrap(), GitlabViewState::default());
}
