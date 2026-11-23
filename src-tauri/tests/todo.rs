//! "To do" list against a real store with the production migrations.

mod common;

use chrono::{NaiveDate, NaiveDateTime};
use common::test_store_dir;
use day_navigator_lib::github::GithubIssue;
use day_navigator_lib::gitlab::{AssignedItems, GitlabItem, ItemKind};
use day_navigator_lib::own_tasks::{self, OwnTaskInput};
use day_navigator_lib::space::Space;
use day_navigator_lib::store::{Store, StoreError, schema};
use day_navigator_lib::todo::{self, TodoEntry, TodoError};
use rusqlite::types::Value;

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
        id: u64::from(n),
        project_id: 1,
        iid: u64::from(n),
        changed: None,
        milestone: None,
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
            "todo_items_live_github",
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
        hidden_issues: vec![],
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

fn gh(repo: &str, n: u64) -> GithubIssue {
    GithubIssue {
        repo: repo.into(),
        number: n,
        reference: format!("{}#{n}", repo.to_lowercase()),
        title: format!("Zgloszenie testowe {n}"),
        web_url: Some(format!("https://github.com/{repo}/issues/{n}")),
        updated_at: "2026-11-20T10:00:00Z".into(),
    }
}

fn todo_rows(store: &Store) -> Vec<Vec<Value>> {
    store
        .read::<_, StoreError>(|c| {
            let mut stmt = c.prepare(
                "SELECT id, source, own_task_id, gitlab_kind, gitlab_ref, gitlab_title, \
                        gitlab_url, gitlab_account, picked_on, picked_at, ticked_at, \
                        ticked_by, removed_at, closed_at \
                 FROM todo_items ORDER BY id",
            )?;
            let rows = stmt
                .query_map([], |r| (0..14).map(|i| r.get::<_, Value>(i)).collect())?
                .collect::<Result<Vec<Vec<Value>>, _>>()?;
            Ok(rows)
        })
        .unwrap()
}

fn exec(store: &mut Store, sql: &str) -> Result<(), StoreError> {
    store.write::<_, StoreError>(|tx| {
        tx.execute_batch(sql)?;
        Ok(())
    })
}

/// `todo_items` was rebuilt for GitHub rows (v8): a v7 store with own and
/// GitLab rows in every state opens at v8 with each row and column unchanged,
/// and the old unique indexes still refuse duplicates.
#[test]
fn v8_rebuild_keeps_every_row_and_index_of_a_v7_store() {
    let (_tmp, dir) = test_store_dir();
    {
        let mut v7 = Store::open(&dir, &schema::MIGRATIONS[..7], day("2026-11-23")).unwrap();
        exec(
            &mut v7,
            "INSERT INTO own_tasks(space, title, created_at, updated_at) VALUES \
               ('work','Zadanie testowe 1','2026-11-23T07:00:00','2026-11-23T07:00:00'), \
               ('private','Zadanie testowe 2','2026-11-23T07:00:00','2026-11-23T07:00:00'); \
             INSERT INTO todo_items (source, own_task_id, picked_on, picked_at) VALUES \
               ('own', 1, '2026-11-22', '2026-11-22T08:00:00'); \
             INSERT INTO todo_items (source, own_task_id, picked_on, picked_at, removed_at) VALUES \
               ('own', 2, '2026-11-22', '2026-11-22T08:30:00', '2026-11-22T09:00:00'); \
             INSERT INTO todo_items (source, gitlab_kind, gitlab_ref, gitlab_title, gitlab_url, \
                 gitlab_account, picked_on, picked_at) VALUES \
               ('gitlab','issue','invented/p#1','Pozycja 1','https://gitlab.example.invalid/1', \
                'account-a','2026-11-23','2026-11-23T08:00:00'); \
             INSERT INTO todo_items (source, gitlab_kind, gitlab_ref, gitlab_title, \
                 picked_on, picked_at, ticked_at, ticked_by) VALUES \
               ('gitlab','mergeRequest','invented/p!2','Pozycja 2', \
                '2026-11-23','2026-11-23T08:10:00','2026-11-23T09:00:00','user'); \
             INSERT INTO todo_items (source, gitlab_kind, gitlab_ref, gitlab_title, gitlab_account, \
                 picked_on, picked_at, ticked_at, ticked_by) VALUES \
               ('gitlab','issue','invented/p#3','Pozycja 3','account-b', \
                '2026-11-23','2026-11-23T08:20:00','2026-11-23T09:30:00','source'); \
             INSERT INTO todo_items (source, gitlab_kind, gitlab_ref, gitlab_title, gitlab_account, \
                 picked_on, picked_at, removed_at) VALUES \
               ('gitlab','issue','invented/p#1','Pozycja 1 stara','account-a', \
                '2026-11-21','2026-11-21T08:00:00','2026-11-21T09:00:00'); \
             INSERT INTO todo_items (source, gitlab_kind, gitlab_ref, gitlab_title, \
                 picked_on, picked_at, closed_at) VALUES \
               ('gitlab','issue','invented/p#1','Pozycja 1 starsza', \
                '2026-11-20','2026-11-20T08:00:00','2026-11-21T00:00:00');",
        )
        .unwrap();
    }
    let before = {
        let v7 = Store::open(&dir, &schema::MIGRATIONS[..7], day("2026-11-23")).unwrap();
        todo_rows(&v7)
    };
    assert_eq!(before.len(), 7);

    let mut store = open(&dir);
    assert_eq!(todo_rows(&store), before);
    // The rebuilt table still refuses what the v3 indexes refused.
    for sql in [
        "INSERT INTO todo_items (source, own_task_id, picked_on, picked_at) \
         VALUES ('own', 1, '2026-11-23', '2026-11-23T10:00:00')",
        "INSERT INTO todo_items (source, gitlab_kind, gitlab_ref, gitlab_title, picked_on, picked_at) \
         VALUES ('gitlab','issue','invented/p#1','t','2026-11-23','2026-11-23T10:00:00')",
    ] {
        assert!(exec(&mut store, sql).is_err(), "{sql}");
    }
    // The id counter carries on and the new source is accepted.
    let id = todo::add_github(
        &mut store,
        &gh("Octo-Test/Alpha", 1),
        day("2026-11-23"),
        at("2026-11-23", 10),
        Ok(9),
    )
    .unwrap();
    assert!(id > 7);
    assert_eq!(&todo_rows(&store)[..7], &before[..]);
}

#[test]
fn table_check_refuses_inconsistent_github_rows() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    let bad = [
        // github without a reference
        "INSERT INTO todo_items (source, github_title, picked_on, picked_at) \
         VALUES ('github','t','2026-11-23','2026-11-23T08:00:00')",
        // github with a tick but no ticked_by
        "INSERT INTO todo_items (source, github_ref, github_title, picked_on, picked_at, ticked_at) \
         VALUES ('github','o/r#1','t','2026-11-23','2026-11-23T08:00:00','2026-11-23T09:00:00')",
        // github carrying gitlab columns
        "INSERT INTO todo_items (source, github_ref, github_title, gitlab_ref, picked_on, picked_at) \
         VALUES ('github','o/r#1','t','x','2026-11-23','2026-11-23T08:00:00')",
        // github carrying a gitlab account
        "INSERT INTO todo_items (source, github_ref, github_title, gitlab_account, picked_on, picked_at) \
         VALUES ('github','o/r#1','t','acct','2026-11-23','2026-11-23T08:00:00')",
        // gitlab carrying github columns
        "INSERT INTO todo_items (source, gitlab_kind, gitlab_ref, gitlab_title, github_ref, \
            picked_on, picked_at) \
         VALUES ('gitlab','issue','p#1','t','o/r#1','2026-11-23','2026-11-23T08:00:00')",
        // own carrying github columns
        "INSERT INTO own_tasks(space, title, created_at, updated_at) \
         VALUES ('work','t','2026-11-23T07:00:00','2026-11-23T07:00:00'); \
         INSERT INTO todo_items (source, own_task_id, github_ref, picked_on, picked_at) \
         VALUES ('own',1,'o/r#1','2026-11-23','2026-11-23T08:00:00')",
    ];
    for sql in bad {
        assert!(exec(&mut store, sql).is_err(), "{sql}");
    }
}

#[test]
fn github_entries_flow_against_a_real_store() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    let d1 = "2026-11-23";
    let alpha = "Octo-Test/Alpha";
    let beta = "octo-test/beta";
    let e1 = todo::add_github(&mut store, &gh(alpha, 1), day(d1), at(d1, 8), Ok(5)).unwrap();
    assert!(matches!(
        todo::add_github(&mut store, &gh(alpha, 1), day(d1), at(d1, 9), Ok(5)),
        Err(TodoError::AlreadyOnList)
    ));
    todo::add_github(&mut store, &gh(alpha, 2), day(d1), at(d1, 9), Ok(5)).unwrap();
    todo::add_github(&mut store, &gh(beta, 1), day(d1), at(d1, 9), Ok(5)).unwrap();
    // A GitLab entry with the same-looking reference is another item.
    let mut same = item(1);
    same.reference = "octo-test/alpha#1".into();
    todo::add_gitlab(&mut store, ACC, &same, day(d1), at(d1, 9), Ok(5)).unwrap();

    // alpha loaded without #2 (and #1 renamed); beta failed, gamma was removed
    // from settings: neither is touched.
    let mut renamed = gh(alpha, 1);
    renamed.title = "Po zmianie".into();
    let alpha_issues = [renamed];
    let n = todo::reconcile_github(
        &mut store,
        &[("octo-test/alpha".to_string(), &alpha_issues[..])],
        day(d1),
        at(d1, 10),
    )
    .unwrap();
    assert_eq!(n, 2);
    let l = todo::list(&mut store, day(d1), at(d1, 11), Ok(5)).unwrap();
    assert_eq!(l.open_count, 3);
    let ticked: Vec<_> = l.items.iter().filter(|i| i.ticked).collect();
    assert_eq!(ticked.len(), 1);
    assert!(matches!(
        &ticked[0].entry,
        TodoEntry::Github { reference, closed_in_source: true, .. } if reference == "octo-test/alpha#2"
    ));
    assert!(l.items.iter().any(|i| matches!(
        &i.entry,
        TodoEntry::Github { title, .. } if title == "Po zmianie"
    )));
    assert!(l.items.iter().any(|i| matches!(
        &i.entry,
        TodoEntry::Gitlab { reference, closed_in_source: false, .. } if reference == "octo-test/alpha#1"
    )));

    // A user tick and untick work on a GitHub entry like on a GitLab one.
    todo::set_ticked(&mut store, e1, true, day(d1), at(d1, 12)).unwrap();
    todo::set_ticked(&mut store, e1, false, day(d1), at(d1, 12)).unwrap();

    // Next day: the source-ticked entry rolls over to closed, the rest carries.
    let d2 = "2026-11-24";
    let l = todo::list(&mut store, day(d2), at(d2, 8), Ok(5)).unwrap();
    assert_eq!(l.items.len(), 3);
    assert!(l.items.iter().all(|i| i.carried_over));
    // The closed issue can be picked again.
    todo::add_github(&mut store, &gh(alpha, 2), day(d2), at(d2, 9), Ok(5)).unwrap();
}

#[test]
fn github_picks_share_the_limit_with_the_other_sources() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    let d1 = "2026-11-23";
    let a = task(&mut store, "Zadanie A");
    todo::add_own(&mut store, a, day(d1), at(d1, 8), Ok(2)).unwrap();
    todo::add_gitlab(&mut store, ACC, &item(1), day(d1), at(d1, 9), Ok(2)).unwrap();
    assert!(matches!(
        todo::add_github(
            &mut store,
            &gh("octo-test/alpha", 1),
            day(d1),
            at(d1, 10),
            Ok(2)
        ),
        Err(TodoError::LimitReached { limit: 2 })
    ));
}
