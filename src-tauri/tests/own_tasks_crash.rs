//! Durability: killing the writer at any moment never loses a reported own
//! task and never leaves a partial row.

mod common;

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use chrono::NaiveDate;
use common::crash::{KillAt, MAX_WRITER_COMMITS, WRITER_DIR_ENV, run_writer_until};
use common::test_store_dir;
use day_navigator_lib::own_tasks::{self, OwnTaskInput, Priority};
use day_navigator_lib::space::Space;
use day_navigator_lib::store::{Store, StoreError, schema};

const WRITER: &str = "writer_process";

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 23).unwrap()
}

fn title(n: i64) -> String {
    format!("Zadanie testowe {n}")
}

fn note(n: i64) -> String {
    format!("Notatka {n}\nDruga linia {n}")
}

fn due(n: i64) -> String {
    format!("2026-12-{:02}", n % 28 + 1)
}

fn space(n: i64) -> Space {
    if n % 2 == 0 {
        Space::Work
    } else {
        Space::Private
    }
}

fn priority(n: i64) -> Option<Priority> {
    match n % 4 {
        0 => None,
        1 => Some(Priority::High),
        2 => Some(Priority::Medium),
        _ => Some(Priority::Low),
    }
}

/// Child side: runs only when the parent test re-launches this binary.
#[test]
#[ignore = "writer process for crash_never_loses_or_half_applies_an_own_task"]
fn writer_process() {
    let Some(dir) = std::env::var_os(WRITER_DIR_ENV) else {
        return;
    };
    let mut store = Store::open(Path::new(&dir), schema::MIGRATIONS, today()).unwrap();
    let mut n = own_tasks::list(&store).unwrap().len() as i64;
    for _ in 0..MAX_WRITER_COMMITS {
        n += 1;
        let input = OwnTaskInput {
            space: space(n),
            title: title(n),
            note: note(n),
            due: Some(due(n)),
            priority: priority(n),
        };
        let now = today().and_hms_opt(8, 0, 0).unwrap();
        own_tasks::add(&mut store, &input, now).unwrap();
        let mut out = std::io::stdout().lock();
        writeln!(out, "committed {n}").unwrap();
        out.flush().unwrap();
    }
}

#[test]
fn crash_never_loses_or_half_applies_an_own_task() {
    let (_tmp, dir) = test_store_dir();
    let kills = [
        KillAt::AfterCommits(1),
        KillAt::AfterDelay(Duration::from_millis(150)),
        KillAt::AfterCommits(5),
        KillAt::AfterDelay(Duration::from_millis(500)),
        KillAt::AfterCommits(20),
        KillAt::AfterDelay(Duration::from_millis(900)),
        KillAt::AfterCommits(3),
        KillAt::AfterDelay(Duration::from_millis(333)),
    ];
    for (round, kill) in kills.into_iter().enumerate() {
        let last = run_writer_until(WRITER, &dir, kill);

        let store = Store::open(&dir, schema::MIGRATIONS, today())
            .unwrap_or_else(|e| panic!("round {round} ({kill:?}): open failed: {e:?}"));
        let check: String = store
            .read::<_, StoreError>(|c| Ok(c.query_row("PRAGMA quick_check", [], |r| r.get(0))?))
            .unwrap();
        assert_eq!(check, "ok", "round {round} ({kill:?})");

        let mut tasks = own_tasks::list(&store).unwrap();
        tasks.sort_by_key(|t| t.id);
        assert!(
            tasks.len() as u64 >= last,
            "round {round} ({kill:?}): lost commit, {} rows < reported {last}",
            tasks.len()
        );
        for (i, t) in tasks.iter().enumerate() {
            let n = i as i64 + 1;
            assert_eq!(t.id, n, "round {round} ({kill:?}): gap in ids");
            assert_eq!(t.title, title(n), "round {round} ({kill:?})");
            assert_eq!(t.note, note(n), "round {round} ({kill:?})");
            assert_eq!(t.space, space(n), "round {round} ({kill:?})");
            assert_eq!(t.due.as_deref(), Some(due(n).as_str()));
            assert_eq!(t.priority, priority(n));
            assert_eq!(t.created_at, "2026-11-23T08:00:00");
            assert_eq!(t.updated_at, t.created_at);
            assert_eq!(t.done_at, None);
        }
    }
}
