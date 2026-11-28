//! GitHub seen-set tables (migration v16) against a real store with the
//! production migrations. Invented repos (`octo-test/*`) and ids only.

mod common;

use chrono::NaiveDate;
use common::test_store_dir;
use day_navigator_lib::github_seen::{RepoRound, SeenKey, mark_view_seen, record_round, view_mark};
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

const NOW: i64 = 1_800_000_000_000;
const DAY: i64 = 86_400_000;
const ALPHA: &str = "octo-test/alpha";
const BETA: &str = "octo-test/beta";

fn round(repo: &str, numbers: &[u64]) -> RepoRound {
    RepoRound {
        repo: repo.to_string(),
        numbers: numbers.to_vec(),
    }
}

fn active(repos: &[&str]) -> Vec<String> {
    repos.iter().map(|r| r.to_string()).collect()
}

/// The fresh keys as (reference, first instant), so a failure names them.
fn refs(fresh: &[(SeenKey, i64)]) -> Vec<(String, i64)> {
    fresh.iter().map(|(k, f)| (k.reference(), *f)).collect()
}

/// (issue number, first seen, last seen) of one repo, by number.
fn issue_rows(store: &Store, repo: &str) -> Vec<(i64, i64, i64)> {
    store
        .read::<_, StoreError>(|c| {
            let mut stmt = c.prepare(
                "SELECT issue_number, first_seen_ms, last_seen_ms FROM github_seen_issues
                 WHERE repo = ?1 ORDER BY issue_number",
            )?;
            let rows = stmt
                .query_map([repo], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, i64>(2)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })
        .unwrap()
}

#[test]
fn first_round_seeds_silently_even_with_a_mark() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    mark_view_seen(&mut store, NOW - DAY, &[]).unwrap();

    let fresh = record_round(&mut store, &[round(ALPHA, &[1, 2])], &active(&[ALPHA]), NOW).unwrap();
    assert!(fresh.is_empty());
    assert_eq!(scalar(&store, "SELECT COUNT(*) FROM github_seen_repos"), 1);
    assert_eq!(issue_rows(&store, ALPHA), vec![(1, 0, NOW), (2, 0, NOW)]);

    let fresh = record_round(
        &mut store,
        &[round(ALPHA, &[1, 2])],
        &active(&[ALPHA]),
        NOW + 1000,
    )
    .unwrap();
    assert!(fresh.is_empty());
}

#[test]
fn a_repo_seeded_without_a_mark_is_never_fresh_under_an_older_mark() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    record_round(&mut store, &[round(ALPHA, &[1])], &active(&[ALPHA]), NOW).unwrap();

    let both = active(&[ALPHA, BETA]);
    let fresh = record_round(
        &mut store,
        &[round(ALPHA, &[1]), round(BETA, &[5, 6])],
        &both,
        NOW + 1000,
    )
    .unwrap();
    assert!(fresh.is_empty());

    // A mark older than beta's seeding round, as a visit ending mid-round saves.
    mark_view_seen(&mut store, NOW, &[]).unwrap();
    let fresh = record_round(
        &mut store,
        &[round(ALPHA, &[1]), round(BETA, &[5, 6])],
        &both,
        NOW + 2000,
    )
    .unwrap();
    assert!(fresh.is_empty());
    assert_eq!(
        issue_rows(&store, BETA),
        vec![(5, 0, NOW + 2000), (6, 0, NOW + 2000)]
    );
}

#[test]
fn no_mark_means_nothing_is_fresh() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    record_round(&mut store, &[round(ALPHA, &[1])], &active(&[ALPHA]), NOW).unwrap();

    let fresh = record_round(
        &mut store,
        &[round(ALPHA, &[1, 2])],
        &active(&[ALPHA]),
        NOW + 1000,
    )
    .unwrap();
    assert!(fresh.is_empty());
}

#[test]
fn an_issue_first_seen_after_the_mark_is_fresh_until_the_mark_passes_it() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    let alpha = active(&[ALPHA]);
    record_round(&mut store, &[round(ALPHA, &[1])], &alpha, NOW).unwrap();
    mark_view_seen(&mut store, NOW, &[]).unwrap();

    let fresh = record_round(&mut store, &[round(ALPHA, &[1, 2])], &alpha, NOW + 1000).unwrap();
    assert_eq!(
        refs(&fresh),
        vec![("octo-test/alpha#2".to_string(), NOW + 1000)]
    );

    let fresh = record_round(&mut store, &[round(ALPHA, &[1, 2])], &alpha, NOW + 2000).unwrap();
    assert_eq!(
        refs(&fresh),
        vec![("octo-test/alpha#2".to_string(), NOW + 1000)]
    );

    mark_view_seen(&mut store, NOW + 2000, &[]).unwrap();
    let fresh = record_round(&mut store, &[round(ALPHA, &[1, 2])], &alpha, NOW + 3000).unwrap();
    assert!(fresh.is_empty());
}

#[test]
fn a_repo_added_later_seeds_silently_under_an_existing_mark() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    record_round(&mut store, &[round(ALPHA, &[1])], &active(&[ALPHA]), NOW).unwrap();
    mark_view_seen(&mut store, NOW, &[]).unwrap();

    let both = active(&[ALPHA, BETA]);
    let fresh = record_round(
        &mut store,
        &[round(ALPHA, &[1]), round(BETA, &[7])],
        &both,
        NOW + 1000,
    )
    .unwrap();
    assert!(fresh.is_empty());

    let fresh = record_round(
        &mut store,
        &[round(ALPHA, &[1]), round(BETA, &[7])],
        &both,
        NOW + 2000,
    )
    .unwrap();
    assert!(fresh.is_empty());
}

#[test]
fn a_failed_repo_is_neither_recorded_nor_forgotten() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    let both = active(&[ALPHA, BETA]);
    record_round(
        &mut store,
        &[round(ALPHA, &[1]), round(BETA, &[4])],
        &both,
        NOW,
    )
    .unwrap();

    // Beta failed this round: it is not in `loaded`, but still configured.
    record_round(&mut store, &[round(ALPHA, &[1])], &both, NOW + 31 * DAY).unwrap();
    assert_eq!(issue_rows(&store, BETA), vec![(4, 0, NOW)]);
}

#[test]
fn an_absent_issue_is_forgotten_after_30_days_and_is_fresh_again() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    let alpha = active(&[ALPHA]);
    record_round(&mut store, &[round(ALPHA, &[1, 2])], &alpha, NOW).unwrap();
    mark_view_seen(&mut store, NOW, &[]).unwrap();

    record_round(&mut store, &[round(ALPHA, &[1])], &alpha, NOW + 31 * DAY).unwrap();
    assert_eq!(issue_rows(&store, ALPHA), vec![(1, 0, NOW + 31 * DAY)]);

    let fresh = record_round(&mut store, &[round(ALPHA, &[1, 2])], &alpha, NOW + 32 * DAY).unwrap();
    assert_eq!(
        refs(&fresh),
        vec![("octo-test/alpha#2".to_string(), NOW + 32 * DAY)]
    );
}

#[test]
fn an_issue_back_within_30_days_keeps_its_first_instant() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    let alpha = active(&[ALPHA]);
    record_round(&mut store, &[round(ALPHA, &[1, 2])], &alpha, NOW).unwrap();
    mark_view_seen(&mut store, NOW, &[]).unwrap();

    record_round(&mut store, &[round(ALPHA, &[1])], &alpha, NOW + 10 * DAY).unwrap();
    let fresh = record_round(&mut store, &[round(ALPHA, &[1, 2])], &alpha, NOW + 20 * DAY).unwrap();
    assert!(fresh.is_empty());
}

#[test]
fn a_repo_removed_from_settings_is_pruned_and_seeds_silently_when_added_again() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    let both = active(&[ALPHA, BETA]);
    record_round(
        &mut store,
        &[round(ALPHA, &[1]), round(BETA, &[9])],
        &both,
        NOW,
    )
    .unwrap();

    record_round(
        &mut store,
        &[round(ALPHA, &[1])],
        &active(&[ALPHA]),
        NOW + 1000,
    )
    .unwrap();
    assert_eq!(
        scalar(
            &store,
            "SELECT COUNT(*) FROM github_seen_repos WHERE repo = 'octo-test/beta'"
        ),
        0
    );
    assert_eq!(
        scalar(
            &store,
            "SELECT COUNT(*) FROM github_seen_issues WHERE repo = 'octo-test/beta'"
        ),
        0
    );

    mark_view_seen(&mut store, NOW + 1000, &[]).unwrap();
    let fresh = record_round(
        &mut store,
        &[round(ALPHA, &[1]), round(BETA, &[9])],
        &both,
        NOW + 2000,
    )
    .unwrap();
    assert!(fresh.is_empty());
}

#[test]
fn the_mark_never_moves_back() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    mark_view_seen(&mut store, 500, &[]).unwrap();
    mark_view_seen(&mut store, 400, &[]).unwrap();
    assert_eq!(view_mark(&store).unwrap(), Some(500));
    mark_view_seen(&mut store, 600, &[]).unwrap();
    assert_eq!(view_mark(&store).unwrap(), Some(600));
}

#[test]
fn the_seen_set_survives_a_reopen() {
    let (_tmp, dir) = test_store_dir();
    let alpha = active(&[ALPHA]);
    {
        let mut store = open(&dir);
        record_round(&mut store, &[round(ALPHA, &[1])], &alpha, NOW).unwrap();
        mark_view_seen(&mut store, NOW, &[]).unwrap();
    }
    let mut store = open(&dir);
    let fresh = record_round(&mut store, &[round(ALPHA, &[1, 2])], &alpha, NOW + 1000).unwrap();
    assert_eq!(
        refs(&fresh),
        vec![("octo-test/alpha#2".to_string(), NOW + 1000)]
    );
}

#[test]
fn a_mark_saved_while_a_repo_failed_keeps_that_repos_fresh_issues_new() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    let both = active(&[ALPHA, BETA]);
    record_round(
        &mut store,
        &[round(ALPHA, &[1]), round(BETA, &[7])],
        &both,
        NOW,
    )
    .unwrap();
    mark_view_seen(&mut store, NOW, &[]).unwrap();
    // Beta#9 and alpha#2 appear; then beta fails in the round that is shown.
    record_round(
        &mut store,
        &[round(ALPHA, &[1, 2]), round(BETA, &[7, 9])],
        &both,
        NOW + 1000,
    )
    .unwrap();
    record_round(&mut store, &[round(ALPHA, &[1, 2])], &both, NOW + 2000).unwrap();
    mark_view_seen(&mut store, NOW + 2000, &[BETA.to_string()]).unwrap();
    assert_eq!(view_mark(&store).unwrap(), Some(NOW + 2000));

    // Loaded repo rows are untouched; the failed repo's fresh row moved just past the mark.
    assert_eq!(
        issue_rows(&store, ALPHA),
        vec![(1, 0, NOW + 2000), (2, NOW + 1000, NOW + 2000)]
    );
    assert_eq!(
        issue_rows(&store, BETA),
        vec![(7, 0, NOW + 1000), (9, NOW + 2001, NOW + 1000)]
    );
    let fresh = record_round(
        &mut store,
        &[round(ALPHA, &[1, 2]), round(BETA, &[7, 9])],
        &both,
        NOW + 3000,
    )
    .unwrap();
    assert_eq!(
        refs(&fresh),
        vec![("octo-test/beta#9".to_string(), NOW + 2001)]
    );
}

#[test]
fn a_failed_repo_changes_nothing_without_an_old_mark() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    let both = active(&[ALPHA, BETA]);
    record_round(&mut store, &[round(BETA, &[7])], &both, NOW).unwrap();
    record_round(&mut store, &[round(BETA, &[7, 9])], &both, NOW + 1000).unwrap();
    mark_view_seen(&mut store, NOW + 1000, &[BETA.to_string()]).unwrap();
    assert_eq!(
        issue_rows(&store, BETA),
        vec![(7, 0, NOW + 1000), (9, NOW + 1000, NOW + 1000)]
    );
}

#[test]
fn a_repo_key_repeated_in_one_round_does_not_fail_the_round() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    let alpha = active(&[ALPHA]);
    record_round(
        &mut store,
        &[round(ALPHA, &[1]), round(ALPHA, &[1])],
        &alpha,
        NOW,
    )
    .unwrap();
    assert_eq!(scalar(&store, "SELECT count(*) FROM github_seen_repos"), 1);
}
