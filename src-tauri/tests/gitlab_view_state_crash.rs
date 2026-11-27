//! Durability: killing the writer at any moment leaves a consistent store with
//! exactly one row, no lost commit and no torn row.

mod common;

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use chrono::NaiveDate;
use common::crash::{KillAt, MAX_WRITER_COMMITS, WRITER_DIR_ENV, run_writer_until};
use common::test_store_dir;
use day_navigator_lib::gitlab_view_state::{GitlabViewState, ViewKind, load, save};
use day_navigator_lib::store::{Store, StoreError, schema};

const WRITER: &str = "writer_process";

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 27).unwrap()
}

/// Cycles through the 6 combinations so consecutive states differ.
fn state_of(n: u64) -> GitlabViewState {
    let kind = [ViewKind::All, ViewKind::Issue, ViewKind::MergeRequest][(n % 3) as usize];
    GitlabViewState {
        kind,
        show_hidden: (n / 3) % 2 == 1,
    }
}

/// Child side: runs only when the parent test re-launches this binary.
#[test]
#[ignore = "writer process for crash_leaves_one_consistent_row"]
fn writer_process() {
    let Some(dir) = std::env::var_os(WRITER_DIR_ENV) else {
        return;
    };
    let mut store = Store::open(Path::new(&dir), schema::MIGRATIONS, today()).unwrap();
    for n in 1..=MAX_WRITER_COMMITS {
        save(&mut store, state_of(n)).unwrap();
        let mut out = std::io::stdout().lock();
        writeln!(out, "committed {n}").unwrap();
        out.flush().unwrap();
    }
}

#[test]
fn crash_leaves_one_consistent_row() {
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
        KillAt::AfterCommits(8),
        KillAt::AfterDelay(Duration::from_millis(250)),
    ];
    let mut previous = GitlabViewState::default();
    for (round, kill) in kills.into_iter().enumerate() {
        let last = run_writer_until(WRITER, &dir, kill);

        let store = Store::open(&dir, schema::MIGRATIONS, today())
            .unwrap_or_else(|e| panic!("round {round} ({kill:?}): open failed: {e:?}"));
        let check: String = store
            .read::<_, StoreError>(|c| Ok(c.query_row("PRAGMA quick_check", [], |r| r.get(0))?))
            .unwrap();
        assert_eq!(check, "ok", "round {round} ({kill:?})");
        let rows: i64 = store
            .read::<_, StoreError>(|c| {
                Ok(c.query_row("SELECT count(*) FROM gitlab_view_state", [], |r| r.get(0))?)
            })
            .unwrap();
        assert!(rows <= 1, "round {round} ({kill:?}): {rows} rows");

        let loaded = load(&store).unwrap();
        if last >= 1 {
            assert!(
                loaded == state_of(last) || loaded == state_of(last + 1),
                "round {round} ({kill:?}): reported {last}, loaded {loaded:?}"
            );
            assert_eq!(rows, 1);
        } else {
            assert!(
                loaded == previous || loaded == state_of(1),
                "round {round} ({kill:?}): nothing reported, loaded {loaded:?}"
            );
        }
        previous = loaded;
    }
}
