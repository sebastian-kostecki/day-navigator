//! Durability: killing the writer at any moment never loses a reported hide
//! record and never leaves a half-applied one.

mod common;

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use chrono::NaiveDate;
use common::crash::{KillAt, MAX_WRITER_COMMITS, WRITER_DIR_ENV, run_writer_until};
use common::test_store_dir;
use day_navigator_lib::cli::GitlabHost;
use day_navigator_lib::gitlab::{GitlabItem, ItemKind};
use day_navigator_lib::hidden_issues;
use day_navigator_lib::store::{Store, StoreError, schema};

const WRITER: &str = "writer_process";
const BASE_MS: i64 = 1_800_000_000_000;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 23).unwrap()
}

fn host() -> GitlabHost {
    GitlabHost::parse("gitlab.example.invalid").unwrap()
}

fn labels(n: u64) -> Vec<String> {
    vec![format!("workflow::step{}", n % 5), format!("plain{n}")]
}

fn updated(n: u64) -> String {
    format!("2026-03-02T10:{:02}:00Z", n % 60)
}

fn item(n: u64) -> GitlabItem {
    GitlabItem {
        kind: ItemKind::Issue,
        id: n,
        project_id: 1_000 + n,
        iid: n + 10,
        reference: format!("acme/widgets#{n}"),
        title: format!("Invented issue {n}"),
        web_url: None,
        updated_at: updated(n),
        labels: labels(n),
        draft: false,
        changed: None,
    }
}

/// Child side: runs only when the parent test re-launches this binary.
#[test]
#[ignore = "writer process for crash_never_loses_or_half_applies_a_hide_record"]
fn writer_process() {
    let Some(dir) = std::env::var_os(WRITER_DIR_ENV) else {
        return;
    };
    let mut store = Store::open(Path::new(&dir), schema::MIGRATIONS, today()).unwrap();
    let mut n = hidden_issues::list(&store, &host()).unwrap().len() as u64;
    for _ in 0..MAX_WRITER_COMMITS {
        n += 1;
        hidden_issues::hide(&mut store, &host(), &item(n), BASE_MS + n as i64).unwrap();
        let mut out = std::io::stdout().lock();
        writeln!(out, "committed {n}").unwrap();
        out.flush().unwrap();
    }
}

#[test]
fn crash_never_loses_or_half_applies_a_hide_record() {
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

        let mut rows = hidden_issues::list(&store, &host()).unwrap();
        rows.sort_by_key(|r| r.issue_id);
        assert!(
            rows.len() as u64 >= last,
            "round {round} ({kill:?}): lost commit, {} rows < reported {last}",
            rows.len()
        );
        for (i, r) in rows.iter().enumerate() {
            let n = i as u64 + 1;
            assert_eq!(r.issue_id, n, "round {round} ({kill:?}): gap in ids");
            assert_eq!(r.project_id, 1_000 + n);
            assert_eq!(r.iid, n + 10);
            assert_eq!(r.scoped_labels, [format!("workflow::step{}", n % 5)]);
            assert_eq!(r.baseline_updated_at, updated(n));
            assert_eq!(r.hidden_at_ms, BASE_MS + n as i64);
            assert_eq!(r.settle_after_ms, Some(BASE_MS + n as i64 + 120_000));
            assert!(r.is_hidden());
            assert_eq!(r.absent_since_ms, None);
        }
    }
}
