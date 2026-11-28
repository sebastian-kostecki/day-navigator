//! Durability: killing the writer at any moment never loses an acknowledged
//! pick or tick and never leaves a half-applied one.

mod common;

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use chrono::{NaiveDate, NaiveDateTime};
use common::crash::{KillAt, MAX_WRITER_COMMITS, WRITER_DIR_ENV, run_writer_until};
use common::test_store_dir;
use day_navigator_lib::github::GithubIssue;
use day_navigator_lib::gitlab::{AssignedItems, GitlabItem, ItemKind};
use day_navigator_lib::own_tasks::{self, OwnTaskInput};
use day_navigator_lib::space::Space;
use day_navigator_lib::store::{Store, StoreError, schema};
use day_navigator_lib::todo;

const WRITER: &str = "writer_process";
/// Invented `gitlab::account_key` of the account the writer's lists come from.
const ACC: &str = "account-a";

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 23).unwrap()
}

fn now() -> NaiveDateTime {
    today().and_hms_opt(8, 0, 0).unwrap()
}

fn gitlab_item(n: i64) -> GitlabItem {
    GitlabItem {
        kind: ItemKind::Issue,
        id: n as u64,
        project_id: 1,
        iid: n as u64,
        changed: None,
        milestone: None,
        is_new: false,
        hidden_at_ms: None,
        reference: format!("invented/project#{n}"),
        title: format!("Pozycja testowa {n}"),
        web_url: Some(format!("https://gitlab.example.invalid/invented/{n}")),
        updated_at: "2026-11-20T10:00:00+01:00".into(),
        labels: vec![],
        draft: false,
        review_requested: false,
    }
}

fn github_issue(n: i64) -> GithubIssue {
    GithubIssue {
        repo: "octo-test/alpha".into(),
        number: n as u64,
        reference: format!("octo-test/alpha#{n}"),
        title: format!("Zgloszenie testowe {n}"),
        web_url: Some(format!("https://github.com/octo-test/alpha/issues/{n}")),
        updated_at: "2026-11-20T10:00:00Z".into(),
        labels: vec![],
        changed: None,
        hidden_at_ms: None,
    }
}

/// Child side: runs only when the parent test re-launches this binary. Each
/// round picks and ticks an own task, then picks a GitLab item and a GitHub
/// issue and reconciles both away; the round number is reported after all
/// writes were acknowledged.
#[test]
#[ignore = "writer process for crash_never_loses_or_half_applies_a_pick_or_tick"]
fn writer_process() {
    let Some(dir) = std::env::var_os(WRITER_DIR_ENV) else {
        return;
    };
    let mut store = Store::open(Path::new(&dir), schema::MIGRATIONS, today()).unwrap();
    // The GitHub pick is the last pick of a round, so its row count is the
    // number of rounds fully picked; the earlier picks may be one ahead after a
    // kill.
    let mut n = todo_rows(&store, "github");
    for _ in 0..MAX_WRITER_COMMITS {
        n += 1;
        let input = OwnTaskInput {
            space: Space::Private,
            title: format!("Zadanie testowe {n}"),
            note: String::new(),
            due: None,
            priority: None,
        };
        let task = own_tasks::add(&mut store, &input, now()).unwrap().id;
        let entry = todo::add_own(&mut store, task, today(), now(), Ok(u32::MAX)).unwrap();
        todo::set_ticked(&mut store, entry, true, today(), now()).unwrap();
        // A kill after this pick but before the GitHub one leaves the item
        // on the list; the next run reaches the same number again.
        match todo::add_gitlab(
            &mut store,
            ACC,
            &gitlab_item(n),
            today(),
            now(),
            Ok(u32::MAX),
        ) {
            Ok(_) | Err(todo::TodoError::AlreadyOnList) => {}
            Err(e) => panic!("add_gitlab failed: {e:?}"),
        }
        todo::reconcile_gitlab(&mut store, ACC, &AssignedItems::default(), today(), now()).unwrap();
        todo::add_github(&mut store, &github_issue(n), today(), now(), Ok(u32::MAX)).unwrap();
        todo::reconcile_github(
            &mut store,
            &[("octo-test/alpha".to_string(), &[])],
            today(),
            now(),
        )
        .unwrap();
        let mut out = std::io::stdout().lock();
        writeln!(out, "committed {n}").unwrap();
        out.flush().unwrap();
    }
}

fn todo_rows(store: &Store, source: &str) -> i64 {
    store
        .read::<_, StoreError>(|c| {
            Ok(c.query_row(
                "SELECT count(*) FROM todo_items WHERE source = ?1",
                [source],
                |r| r.get(0),
            )?)
        })
        .unwrap()
}

fn scalar(store: &Store, sql: &str) -> i64 {
    store
        .read::<_, StoreError>(|c| Ok(c.query_row(sql, [], |r| r.get(0))?))
        .unwrap()
}

#[test]
fn crash_never_loses_or_half_applies_a_pick_or_tick() {
    let (_tmp, dir) = test_store_dir();
    let kills = [
        KillAt::AfterCommits(1),
        KillAt::AfterDelay(Duration::from_millis(150)),
        KillAt::AfterCommits(5),
        KillAt::AfterDelay(Duration::from_millis(500)),
        KillAt::AfterCommits(15),
        KillAt::AfterDelay(Duration::from_millis(900)),
        KillAt::AfterCommits(3),
        KillAt::AfterDelay(Duration::from_millis(333)),
    ];
    for (round, kill) in kills.into_iter().enumerate() {
        let last = run_writer_until(WRITER, &dir, kill);
        let ctx = format!("round {round} ({kill:?})");

        let store = Store::open(&dir, schema::MIGRATIONS, today())
            .unwrap_or_else(|e| panic!("{ctx}: open failed: {e:?}"));
        let check: String = store
            .read::<_, StoreError>(|c| Ok(c.query_row("PRAGMA quick_check", [], |r| r.get(0))?))
            .unwrap();
        assert_eq!(check, "ok", "{ctx}");

        // Every acknowledged round has its own pick ticked and its GitLab
        // entry ticked by the source.
        let own = todo_rows(&store, "own");
        let gitlab = todo_rows(&store, "gitlab");
        assert!(own as u64 >= last, "{ctx}: lost own pick ({own} < {last})");
        assert!(
            gitlab as u64 >= last,
            "{ctx}: lost gitlab pick ({gitlab} < {last})"
        );
        let own_ticked = scalar(
            &store,
            "SELECT count(*) FROM todo_items t JOIN own_tasks o ON o.id = t.own_task_id \
             WHERE o.done_at IS NOT NULL",
        );
        assert!(own_ticked as u64 >= last, "{ctx}: lost own tick");
        let source_ticked = scalar(
            &store,
            "SELECT count(*) FROM todo_items WHERE ticked_by = 'source'",
        );
        assert!(source_ticked as u64 >= last, "{ctx}: lost reconcile tick");

        // Never half-applied.
        assert_eq!(
            scalar(
                &store,
                "SELECT count(*) FROM todo_items WHERE (ticked_at IS NULL) != (ticked_by IS NULL)"
            ),
            0,
            "{ctx}: half tick"
        );
        assert_eq!(
            scalar(
                &store,
                "SELECT count(*) FROM (SELECT own_task_id FROM todo_items \
                 WHERE own_task_id IS NOT NULL AND removed_at IS NULL AND closed_at IS NULL \
                 GROUP BY own_task_id HAVING count(*) > 1)"
            ),
            0,
            "{ctx}: duplicate live own entry"
        );
        assert_eq!(
            scalar(
                &store,
                "SELECT count(*) FROM (SELECT 1 FROM todo_items \
                 WHERE gitlab_ref IS NOT NULL AND removed_at IS NULL AND closed_at IS NULL \
                 GROUP BY gitlab_kind, gitlab_ref HAVING count(*) > 1)"
            ),
            0,
            "{ctx}: duplicate live gitlab entry"
        );
        assert_eq!(
            scalar(
                &store,
                "SELECT count(*) FROM (SELECT 1 FROM todo_items \
                 WHERE github_ref IS NOT NULL AND removed_at IS NULL AND closed_at IS NULL \
                 GROUP BY github_ref HAVING count(*) > 1)"
            ),
            0,
            "{ctx}: duplicate live github entry"
        );
        // Every acknowledged round picked a GitHub issue and ticked it by the
        // source; a source tick never exists without its entry.
        let github = todo_rows(&store, "github");
        assert!(
            github as u64 >= last,
            "{ctx}: lost github pick ({github} < {last})"
        );
        let github_ticked = scalar(
            &store,
            "SELECT count(*) FROM todo_items WHERE source = 'github' AND ticked_by = 'source'",
        );
        assert!(
            github_ticked as u64 >= last,
            "{ctx}: lost github reconcile tick"
        );
        // A pick is committed in one write: every own entry points at a task.
        assert_eq!(
            scalar(
                &store,
                "SELECT count(*) FROM todo_items t LEFT JOIN own_tasks o ON o.id = t.own_task_id \
                 WHERE t.source = 'own' AND o.id IS NULL"
            ),
            0,
            "{ctx}: dangling own entry"
        );
    }
}
