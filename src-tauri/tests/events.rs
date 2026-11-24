//! Event records against a real store with the production migrations.

mod common;

use chrono::{NaiveDate, NaiveDateTime};
use common::test_store_dir;
use day_navigator_lib::events::{
    EventItem, EventView, ItemSource, NewEvent, Subject, list_today, record,
};
use day_navigator_lib::gitlab::ItemKind;
use day_navigator_lib::health::SourceId;
use day_navigator_lib::store::{Store, StoreError, schema};

fn day(d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, d).unwrap()
}

fn at(d: u32, h: u32, m: u32) -> NaiveDateTime {
    day(d).and_hms_opt(h, m, 0).unwrap()
}

fn open(dir: &std::path::Path) -> Store {
    Store::open(dir, schema::MIGRATIONS, day(23)).unwrap()
}

fn assigned() -> NewEvent {
    NewEvent::Assigned {
        item_kind: ItemKind::MergeRequest,
        reference: "acme/web!421".to_string(),
        title: "Wymyślony tytuł".to_string(),
    }
}

fn ticked() -> NewEvent {
    NewEvent::Ticked(Subject {
        source: ItemSource::Github,
        reference: Some("octo/demo#12".to_string()),
        title: "Wymyślony tytuł".to_string(),
    })
}

fn scalar(store: &Store, sql: &'static str) -> i64 {
    store
        .read::<_, StoreError>(|c| Ok(c.query_row(sql, [], |r| r.get(0))?))
        .unwrap()
}

fn count(store: &Store) -> i64 {
    scalar(store, "SELECT count(*) FROM events")
}

fn title_of(item: &EventItem) -> String {
    match &item.event {
        EventView::Assigned { title, .. } => title.clone(),
        EventView::Ticked(s) | EventView::Removed(s) | EventView::ClosedInSource(s) => {
            s.title.clone()
        }
        other => panic!("no title: {other:?}"),
    }
}

#[test]
fn upgrade_from_v9_keeps_rows_and_reaches_latest() {
    let (_tmp, dir) = test_store_dir();
    let before = schema::MIGRATIONS
        .iter()
        .position(|m| m.sql.contains("CREATE TABLE events"))
        .expect("the events migration");
    assert_eq!(before, 9);
    {
        let mut old = Store::open(&dir, &schema::MIGRATIONS[..before], day(23)).unwrap();
        old.write::<_, StoreError>(|tx| {
            tx.execute_batch(
                "INSERT INTO assignment_accounts(account, seeded_at_ms) \
                 VALUES ('invented-account-a', 1000); \
                 INSERT INTO seen_assignments(account, kind, item_id, first_seen_ms, last_seen_ms) \
                 VALUES ('invented-account-a', 'issue', 1, 1000, 1000); \
                 INSERT INTO todo_items(source, gitlab_kind, gitlab_ref, gitlab_title, \
                                        gitlab_url, gitlab_account, picked_on, picked_at) \
                 VALUES ('gitlab', 'issue', 'acme/web#1', 'Wymyślony tytuł', \
                         'https://example.invalid/1', 'invented-account-a', \
                         '2026-11-23', '2026-11-23T08:00:00');",
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
    assert_eq!(scalar(&store, "SELECT count(*) FROM seen_assignments"), 1);
    assert_eq!(scalar(&store, "SELECT count(*) FROM todo_items"), 1);
    assert_eq!(count(&store), 0);
}

#[test]
fn record_then_list_returns_newest_first_with_fields() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    assert_eq!(
        record(&mut store, day(23), at(23, 9, 0), &[assigned()]).unwrap(),
        1
    );
    record(
        &mut store,
        day(23),
        at(23, 9, 5),
        &[ticked(), NewEvent::WorkStarted],
    )
    .unwrap();
    let log = list_today(&store, day(23)).unwrap();
    assert!(!log.truncated);
    assert_eq!(log.events.len(), 3);
    assert_eq!(log.events[0].event, EventView::WorkStarted);
    assert_eq!(log.events[0].at, "2026-11-23T09:05:00");
    assert_eq!(
        log.events[1].event,
        EventView::Ticked(Subject {
            source: ItemSource::Github,
            reference: Some("octo/demo#12".to_string()),
            title: "Wymyślony tytuł".to_string(),
        })
    );
    assert_eq!(
        log.events[2].event,
        EventView::Assigned {
            item_kind: ItemKind::MergeRequest,
            reference: "acme/web!421".to_string(),
            title: "Wymyślony tytuł".to_string(),
        }
    );
    assert!(log.events[0].id > log.events[1].id && log.events[1].id > log.events[2].id);
}

#[test]
fn empty_list_writes_nothing() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    assert_eq!(record(&mut store, day(23), at(23, 9, 0), &[]).unwrap(), 0);
    assert_eq!(count(&store), 0);
}

#[test]
fn a_new_day_removes_yesterdays_events() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    record(&mut store, day(23), at(23, 23, 59), &[assigned()]).unwrap();
    record(&mut store, day(24), at(24, 0, 1), &[ticked()]).unwrap();
    assert_eq!(count(&store), 1);
    let log = list_today(&store, day(24)).unwrap();
    assert_eq!(log.events.len(), 1);
    assert!(matches!(log.events[0].event, EventView::Ticked(_)));
}

#[test]
fn a_clock_set_back_removes_tomorrows_events() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    record(&mut store, day(24), at(24, 8, 0), &[assigned()]).unwrap();
    record(&mut store, day(23), at(23, 8, 0), &[ticked()]).unwrap();
    assert_eq!(count(&store), 1);
    assert!(list_today(&store, day(24)).unwrap().events.is_empty());
}

#[test]
fn list_ignores_other_days_without_a_write() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    record(&mut store, day(23), at(23, 8, 0), &[assigned()]).unwrap();
    assert!(list_today(&store, day(24)).unwrap().events.is_empty());
    assert_eq!(list_today(&store, day(23)).unwrap().events.len(), 1);
}

#[test]
fn list_is_capped_at_200_and_flags_truncation() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    let batch: Vec<NewEvent> = (0..200).map(|_| NewEvent::WorkStopped).collect();
    record(&mut store, day(23), at(23, 8, 0), &batch).unwrap();
    let log = list_today(&store, day(23)).unwrap();
    assert_eq!(log.events.len(), 200);
    assert!(!log.truncated);
    record(&mut store, day(23), at(23, 8, 1), &[NewEvent::WorkStarted]).unwrap();
    let log = list_today(&store, day(23)).unwrap();
    assert_eq!(log.events.len(), 200);
    assert!(log.truncated);
    assert_eq!(log.events[0].event, EventView::WorkStarted);
}

#[test]
fn long_title_is_stored_as_300_characters() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    let title: String = "żółw".repeat(100);
    let event = NewEvent::Removed(Subject {
        source: ItemSource::Own,
        reference: None,
        title: title.clone(),
    });
    record(&mut store, day(23), at(23, 8, 0), &[event]).unwrap();
    let log = list_today(&store, day(23)).unwrap();
    let stored = title_of(&log.events[0]);
    assert_eq!(stored.chars().count(), 300);
    assert!(title.starts_with(&stored));
}

#[test]
fn source_error_reports_a_fixed_reason_not_the_kind() {
    let (_tmp, dir) = test_store_dir();
    let mut store = open(&dir);
    let events = [
        NewEvent::SourceError {
            source: SourceId::AppData,
            label: "Dane aplikacji".to_string(),
            error_kind: "Busy".to_string(),
        },
        NewEvent::SourceRecovered {
            source: SourceId::Gitlab,
            label: "GitLab".to_string(),
        },
    ];
    record(&mut store, day(23), at(23, 8, 0), &events).unwrap();
    let log = list_today(&store, day(23)).unwrap();
    assert_eq!(
        log.events[1].event,
        EventView::SourceError {
            source_id: SourceId::AppData,
            label: "Dane aplikacji".to_string(),
            reason: "plik danych zajęty".to_string(),
        }
    );
    assert_eq!(
        log.events[0].event,
        EventView::SourceRecovered {
            source_id: SourceId::Gitlab,
            label: "GitLab".to_string(),
        }
    );
}
