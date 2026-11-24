//! Seen-assignment records against a real store with the production migrations.

mod common;

use chrono::NaiveDate;
use common::test_store_dir;
use day_navigator_lib::gitlab::ItemKind;
use day_navigator_lib::new_assignments::{FORGET_AFTER_MS, ItemKey, mark_view_seen, record_round};
use day_navigator_lib::store::{Store, StoreError, StoreHandle, schema};

const NOW: i64 = 1_800_000_000_000;
const DAY: i64 = 86_400_000;
const ACCOUNT_A: &str = "invented-account-a";
const ACCOUNT_B: &str = "invented-account-b";

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 23).unwrap()
}

fn issue(id: u64) -> ItemKey {
    ItemKey::new(ItemKind::Issue, id)
}

fn mr(id: u64) -> ItemKey {
    ItemKey::new(ItemKind::MergeRequest, id)
}

fn user_version(store: &Store) -> i64 {
    store
        .read::<_, StoreError>(|c| Ok(c.query_row("PRAGMA user_version", [], |r| r.get(0))?))
        .unwrap()
}

fn count(store: &Store, account: &str) -> i64 {
    store
        .read::<_, StoreError>(|c| {
            Ok(c.query_row(
                "SELECT count(*) FROM seen_assignments WHERE account = ?1",
                [account],
                |r| r.get(0),
            )?)
        })
        .unwrap()
}

#[test]
fn new_migration_applies_over_the_previous_latest_version() {
    let (_tmp, dir) = test_store_dir();
    let n = schema::MIGRATIONS.len();
    // The store as it was just before the S-04 migration (later slices may
    // have appended more after it).
    let before = schema::MIGRATIONS
        .iter()
        .position(|m| m.sql.contains("CREATE TABLE seen_assignments"))
        .expect("the S-04 migration");
    {
        let mut old = Store::open(&dir, &schema::MIGRATIONS[..before], today()).unwrap();
        assert_eq!(user_version(&old), before as i64);
        old.write::<_, StoreError>(|tx| {
            tx.execute_batch(
                "INSERT INTO work_entries(start_ms, end_ms, created_at_ms, updated_at_ms) \
                 VALUES (1000, 2000, 1000, 2000);",
            )?;
            Ok(())
        })
        .unwrap();
    }
    let mut store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
    assert_eq!(user_version(&store), n as i64);
    let entries: i64 = store
        .read::<_, StoreError>(|c| {
            Ok(c.query_row("SELECT count(*) FROM work_entries", [], |r| r.get(0))?)
        })
        .unwrap();
    assert_eq!(entries, 1);
    assert!(
        record_round(&mut store, ACCOUNT_A, &[issue(1)], NOW)
            .unwrap()
            .new
            .is_empty()
    );
}

#[test]
fn seed_then_new_then_reopen_has_no_repeat() {
    let (_tmp, dir) = test_store_dir();
    {
        let mut store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
        let seeded = record_round(&mut store, ACCOUNT_A, &[issue(1), mr(1)], NOW).unwrap();
        assert!(seeded.new.is_empty());
        let next = record_round(
            &mut store,
            ACCOUNT_A,
            &[issue(1), mr(1), issue(2)],
            NOW + 1_000,
        )
        .unwrap();
        assert_eq!(next.new, vec![issue(2)]);
    }
    let handle = StoreHandle::new(Some(dir));
    let again = handle
        .with_store(today(), |s| {
            record_round(s, ACCOUNT_A, &[issue(1), mr(1), issue(2)], NOW + 2_000)
        })
        .unwrap();
    assert!(again.new.is_empty());
}

#[test]
fn accounts_are_isolated_including_pruning() {
    let (_tmp, dir) = test_store_dir();
    let mut store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
    record_round(&mut store, ACCOUNT_A, &[issue(1)], NOW).unwrap();
    record_round(&mut store, ACCOUNT_B, &[issue(1), issue(2)], NOW).unwrap();

    let b_new = record_round(
        &mut store,
        ACCOUNT_B,
        &[issue(1), issue(2), issue(3)],
        NOW + 1,
    )
    .unwrap();
    assert_eq!(b_new.new, vec![issue(3)]);

    // A's round 31 days later with an empty list prunes only A's rows.
    let later = NOW + 31 * DAY;
    record_round(&mut store, ACCOUNT_A, &[], later).unwrap();
    assert_eq!(count(&store, ACCOUNT_A), 0);
    assert_eq!(count(&store, ACCOUNT_B), 3);

    // A's marker survives the pruning: its next item notifies.
    let a_new = record_round(&mut store, ACCOUNT_A, &[issue(9)], later + 1).unwrap();
    assert_eq!(a_new.new, vec![issue(9)]);
}

#[test]
fn a_present_item_after_a_long_gap_gives_no_repeat() {
    let (_tmp, dir) = test_store_dir();
    let mut store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
    record_round(&mut store, ACCOUNT_A, &[issue(1)], NOW).unwrap();
    let after_gap = NOW + 31 * DAY;
    let again = record_round(&mut store, ACCOUNT_A, &[issue(1)], after_gap).unwrap();
    assert!(again.new.is_empty());
    assert_eq!(count(&store, ACCOUNT_A), 1);
}

#[test]
fn an_absent_item_is_forgotten_after_thirty_days_and_notifies_on_return() {
    let (_tmp, dir) = test_store_dir();
    let mut store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
    record_round(&mut store, ACCOUNT_A, &[issue(1), issue(2)], NOW).unwrap();
    // Just short of 30 days: kept, so its return would be silent.
    record_round(
        &mut store,
        ACCOUNT_A,
        &[issue(2)],
        NOW + FORGET_AFTER_MS - 1,
    )
    .unwrap();
    assert_eq!(count(&store, ACCOUNT_A), 2);
    // Exactly 30 days since issue 1 was last seen: forgotten.
    record_round(&mut store, ACCOUNT_A, &[issue(2)], NOW + FORGET_AFTER_MS).unwrap();
    assert_eq!(count(&store, ACCOUNT_A), 1);
    let back = record_round(
        &mut store,
        ACCOUNT_A,
        &[issue(1), issue(2)],
        NOW + FORGET_AFTER_MS + 1,
    )
    .unwrap();
    assert_eq!(back.new, vec![issue(1)]);
}

fn mark_of(store: &Store, account: &str) -> Option<i64> {
    store
        .read::<_, StoreError>(|c| {
            Ok(c.query_row(
                "SELECT seen_until_ms FROM gitlab_view_marks WHERE account = ?1",
                [account],
                |r| r.get(0),
            )
            .ok())
        })
        .unwrap()
}

#[test]
fn view_marks_migration_keeps_seen_assignment_rows() {
    let (_tmp, dir) = test_store_dir();
    let before = schema::MIGRATIONS
        .iter()
        .position(|m| m.sql.contains("CREATE TABLE gitlab_view_marks"))
        .expect("the S-02 migration");
    assert_eq!(before, 8, "v9 follows v8");
    {
        let mut old = Store::open(&dir, &schema::MIGRATIONS[..before], today()).unwrap();
        assert_eq!(user_version(&old), 8);
        // record_round needs the v9 table, so write the v8-era rows directly.
        old.write::<_, StoreError>(|tx| {
            tx.execute_batch(
                "INSERT INTO assignment_accounts(account, seeded_at_ms) \
                 VALUES ('invented-account-a', 1000); \
                 INSERT INTO seen_assignments(account, kind, item_id, first_seen_ms, last_seen_ms) \
                 VALUES ('invented-account-a', 'issue', 1, 1000, 1000), \
                        ('invented-account-a', 'mergeRequest', 2, 1000, 1000);",
            )?;
            Ok(())
        })
        .unwrap();
        assert_eq!(count(&old, ACCOUNT_A), 2);
    }
    let mut store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
    assert_eq!(user_version(&store), 9);
    assert_eq!(count(&store, ACCOUNT_A), 2);
    assert_eq!(mark_of(&store, ACCOUNT_A), None);
    // No mark yet after the upgrade: nothing is fresh, and the old rows seed
    // nothing new either.
    let round = record_round(&mut store, ACCOUNT_A, &[issue(1), mr(2)], 2_000).unwrap();
    assert!(round.new.is_empty());
    assert!(round.fresh.is_empty());
}

#[test]
fn seeding_round_gives_empty_fresh() {
    let (_tmp, dir) = test_store_dir();
    let mut store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
    let seeded = record_round(&mut store, ACCOUNT_A, &[issue(1), mr(1)], NOW).unwrap();
    assert!(seeded.new.is_empty());
    assert!(seeded.fresh.is_empty());
}

#[test]
fn seeding_round_after_an_earlier_mark_gives_empty_fresh() {
    // A mark saved before the account's first stored round (e.g. its first
    // seen-set write failed) must not turn the whole seeded list NEW.
    let (_tmp, dir) = test_store_dir();
    let mut store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
    mark_view_seen(&mut store, ACCOUNT_A, NOW - 1_000).unwrap();
    let seeded = record_round(&mut store, ACCOUNT_A, &[issue(1), mr(1)], NOW).unwrap();
    assert!(seeded.new.is_empty());
    assert!(seeded.fresh.is_empty());
}

#[test]
fn fresh_holds_keys_first_seen_after_the_mark() {
    let (_tmp, dir) = test_store_dir();
    let mut store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
    record_round(&mut store, ACCOUNT_A, &[issue(1)], NOW).unwrap();
    record_round(&mut store, ACCOUNT_A, &[issue(1), issue(2)], NOW + 10).unwrap();
    mark_view_seen(&mut store, ACCOUNT_A, NOW + 10).unwrap();

    // Issue 2 was first seen exactly at the mark: not fresh. MR 3 arrives later.
    let round = record_round(
        &mut store,
        ACCOUNT_A,
        &[issue(1), issue(2), mr(3)],
        NOW + 20,
    )
    .unwrap();
    assert_eq!(round.new, vec![mr(3)]);
    assert_eq!(round.fresh, vec![(mr(3), NOW + 20)]);

    // A later round still reports it, with its original first instant.
    let again = record_round(
        &mut store,
        ACCOUNT_A,
        &[issue(1), issue(2), mr(3)],
        NOW + 30,
    )
    .unwrap();
    assert!(again.new.is_empty());
    assert_eq!(again.fresh, vec![(mr(3), NOW + 20)]);
}

#[test]
fn mark_view_seen_never_moves_the_mark_back() {
    let (_tmp, dir) = test_store_dir();
    let mut store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
    mark_view_seen(&mut store, ACCOUNT_A, NOW).unwrap();
    assert_eq!(mark_of(&store, ACCOUNT_A), Some(NOW));
    mark_view_seen(&mut store, ACCOUNT_A, NOW - 5).unwrap();
    assert_eq!(mark_of(&store, ACCOUNT_A), Some(NOW));
    mark_view_seen(&mut store, ACCOUNT_A, NOW + 5).unwrap();
    assert_eq!(mark_of(&store, ACCOUNT_A), Some(NOW + 5));
}

#[test]
fn a_mark_of_one_account_does_not_touch_another() {
    let (_tmp, dir) = test_store_dir();
    let mut store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
    record_round(&mut store, ACCOUNT_A, &[issue(1)], NOW).unwrap();
    record_round(&mut store, ACCOUNT_B, &[issue(1)], NOW).unwrap();
    mark_view_seen(&mut store, ACCOUNT_A, NOW + 100).unwrap();

    let a = record_round(&mut store, ACCOUNT_A, &[issue(1), issue(2)], NOW + 200).unwrap();
    let b = record_round(&mut store, ACCOUNT_B, &[issue(1), issue(2)], NOW + 200).unwrap();
    assert_eq!(a.fresh, vec![(issue(2), NOW + 200)]);
    assert!(b.fresh.is_empty(), "B has no mark");
    assert_eq!(mark_of(&store, ACCOUNT_B), None);
}

mod round {
    use std::path::PathBuf;
    use std::time::Duration;

    use super::*;
    use day_navigator_lib::cli::{CliRunner, GitlabConfig, GitlabHost};
    use day_navigator_lib::gitlab::fetch::fetch_assigned;
    use day_navigator_lib::new_assignments::keys_of;

    fn fake() -> PathBuf {
        PathBuf::from(env!("CARGO_BIN_EXE_fake-cli"))
    }

    #[test]
    fn new_assignments_fake_cli_round_seeds_then_returns_a_deleted_key() {
        let runner = CliRunner::with_programs(
            fake(),
            fake(),
            Duration::from_secs(10),
            vec![(
                "FAKE_CLI_SCENARIO".to_string(),
                "gitlab-assigned".to_string(),
            )],
        );
        let config = GitlabConfig {
            host: GitlabHost::parse("gitlab.example.invalid").unwrap(),
            user: None,
        };
        let (_tmp, dir) = test_store_dir();
        let mut store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();

        let list = fetch_assigned(&runner, &config).expect("data");
        let keys = keys_of(&list);
        assert!(keys.len() > 1);

        let first = record_round(&mut store, ACCOUNT_A, &keys, NOW).unwrap();
        assert!(first.new.is_empty(), "the first round seeds silently");

        let gone = keys[0];
        store
            .write::<_, StoreError>(|tx| {
                let kind = match gone.kind {
                    ItemKind::Issue => "issue",
                    ItemKind::MergeRequest => "mergeRequest",
                };
                let n = tx.execute(
                    "DELETE FROM seen_assignments WHERE account = ?1 AND kind = ?2 AND item_id = ?3",
                    rusqlite::params![ACCOUNT_A, kind, gone.id as i64],
                )?;
                assert_eq!(n, 1);
                Ok(())
            })
            .unwrap();

        let again = fetch_assigned(&runner, &config).expect("data");
        let next = record_round(&mut store, ACCOUNT_A, &keys_of(&again), NOW + 1_000).unwrap();
        assert_eq!(next.new, vec![gone]);
    }
}
