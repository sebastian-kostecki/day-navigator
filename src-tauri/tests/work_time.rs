//! S-08 durability: work-time entries survive closing and reopening the real
//! store, and removed entries stay as rows. S-10: `reminder_facts` against a
//! real temp store.

mod common;

use chrono::NaiveDate;
use chrono_tz::Europe::Warsaw;
use common::test_store_dir;
use day_navigator_lib::store::{Store, schema};
use day_navigator_lib::work_time::reminders::reminder_facts;
use day_navigator_lib::work_time::report;
use day_navigator_lib::work_time::rules::resolve_local;
use day_navigator_lib::work_time::{self, DayKind, WorkTimeError};

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 23).unwrap()
}

fn t(local: &str) -> i64 {
    resolve_local(local, &Warsaw).unwrap()
}

type Row = (i64, i64, Option<i64>, Option<i64>);

fn rows(store: &Store) -> Vec<Row> {
    store
        .read::<_, WorkTimeError>(|c| {
            let mut stmt = c.prepare(
                "SELECT id, start_ms, end_ms, removed_at_ms FROM work_entries ORDER BY id",
            )?;
            let out = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?;
            Ok(out.collect::<Result<_, _>>()?)
        })
        .unwrap()
}

#[test]
fn running_entry_survives_dropping_and_reopening_the_store() {
    let (_tmp, dir) = test_store_dir();
    let started = t("2026-11-23T08:00") + 12_345;
    let id = {
        let mut store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
        work_time::start(&mut store, started, &Warsaw).unwrap()
    };

    let mut store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
    assert_eq!(rows(&store), vec![(id, started, None, None)]);
    assert!(work_time::is_running(&store).unwrap());
    let later = t("2026-11-23T11:00");
    let s = work_time::status(&store, later, &Warsaw).unwrap();
    let running = s.running.unwrap();
    assert_eq!((running.id, running.start_ms), (id, started));
    // Downtime counts: the live total is measured from the stored start.
    assert_eq!(later - s.running_from_ms.unwrap(), later - started);
    assert!(matches!(
        work_time::start(&mut store, later, &Warsaw),
        Err(WorkTimeError::AlreadyRunning)
    ));
}

#[test]
fn days_off_survive_dropping_and_reopening_the_store() {
    let (_tmp, dir) = test_store_dir();
    let now = t("2026-11-23T12:00");
    {
        let mut store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
        for day in ["2026-11-11", "2026-11-24", "2026-12-24"] {
            let day = NaiveDate::parse_from_str(day, "%Y-%m-%d").unwrap();
            report::set_day_off(&mut store, day, true, now).unwrap();
        }
        let unmarked = NaiveDate::from_ymd_opt(2026, 11, 24).unwrap();
        report::set_day_off(&mut store, unmarked, false, now).unwrap();
    }

    let store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
    let nov = report::month_report(&store, today(), now, &Warsaw).unwrap();
    assert_eq!(nov.days_off, vec!["2026-11-11".to_string()]);
    assert_eq!(nov.figures.workdays, 20);
    let dec = report::month_report(
        &store,
        NaiveDate::from_ymd_opt(2026, 12, 1).unwrap(),
        now,
        &Warsaw,
    )
    .unwrap();
    assert_eq!(dec.days_off, vec!["2026-12-24".to_string()]);
    assert!(!report::is_workday(&store, NaiveDate::from_ymd_opt(2026, 11, 11).unwrap()).unwrap());
    assert!(report::is_workday(&store, NaiveDate::from_ymd_opt(2026, 11, 24).unwrap()).unwrap());
}

#[test]
fn history_and_removed_rows_survive_reopening() {
    let (_tmp, dir) = test_store_dir();
    let now = t("2026-11-23T12:00");
    let (kept, removed) = {
        let mut store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
        let removed = work_time::add(
            &mut store,
            "2026-11-20T08:00",
            "2026-11-20T12:00",
            "",
            now,
            &Warsaw,
        )
        .unwrap();
        let kept = work_time::start(&mut store, t("2026-11-23T09:00"), &Warsaw).unwrap();
        work_time::stop(&mut store, t("2026-11-23T10:00"), &Warsaw).unwrap();
        work_time::remove(&mut store, removed, now).unwrap();
        (kept, removed)
    };

    let store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
    assert_eq!(
        rows(&store),
        vec![
            (
                removed,
                t("2026-11-20T08:00"),
                Some(t("2026-11-20T12:00")),
                Some(now)
            ),
            (
                kept,
                t("2026-11-23T09:00"),
                Some(t("2026-11-23T10:00")),
                None
            ),
        ]
    );
    let removed_day =
        work_time::entries_for_day(&store, today().pred_opt().unwrap(), now, &Warsaw).unwrap();
    assert!(removed_day.entries.is_empty());
    let s = work_time::status(&store, now, &Warsaw).unwrap();
    assert_eq!(s.today_closed_ms, 3_600_000);
    assert!(s.running.is_none());
}

#[test]
fn reminder_facts_report_the_running_entry_and_counted_work() {
    let (_tmp, dir) = test_store_dir();
    let now = t("2026-11-23T12:00");
    let mut store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();

    // Empty store: a workday, nothing running, no work yet.
    let f = reminder_facts(&store, now, &Warsaw).unwrap();
    assert_eq!(f.today, today());
    assert_eq!(f.day_kind, DayKind::Workday);
    assert!(f.running.is_none() && !f.worked_today);

    // A removed entry today and one that ends exactly at midnight do not count.
    let add = |store: &mut Store, from: &str, to: &str| {
        work_time::add(store, from, to, "", now, &Warsaw).unwrap()
    };
    let removed = add(&mut store, "2026-11-23T08:00", "2026-11-23T09:00");
    work_time::remove(&mut store, removed, now).unwrap();
    add(&mut store, "2026-11-22T22:00", "2026-11-23T00:00");
    assert!(!reminder_facts(&store, now, &Warsaw).unwrap().worked_today);

    // A closed entry today counts.
    add(&mut store, "2026-11-23T10:00", "2026-11-23T10:30");
    assert!(reminder_facts(&store, now, &Warsaw).unwrap().worked_today);

    // The running entry keeps its true start.
    let started = t("2026-11-23T11:00") + 12_345;
    let id = work_time::start(&mut store, started, &Warsaw).unwrap();
    let running = reminder_facts(&store, now, &Warsaw)
        .unwrap()
        .running
        .unwrap();
    assert_eq!((running.id, running.start_ms), (id, started));
}

#[test]
fn reminder_facts_count_a_midnight_spanning_portion() {
    let (_tmp, dir) = test_store_dir();
    let now = t("2026-11-23T12:00");
    let mut store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
    work_time::add(
        &mut store,
        "2026-11-22T23:00",
        "2026-11-23T00:30",
        "",
        now,
        &Warsaw,
    )
    .unwrap();
    assert!(reminder_facts(&store, now, &Warsaw).unwrap().worked_today);
    // The next day only the tail of the previous one is left: nothing counts.
    let next_day = t("2026-11-24T09:00");
    assert!(
        !reminder_facts(&store, next_day, &Warsaw)
            .unwrap()
            .worked_today
    );
}

#[test]
fn reminder_facts_report_a_day_off_and_a_weekend() {
    let (_tmp, dir) = test_store_dir();
    let now = t("2026-11-23T12:00");
    let mut store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
    report::set_day_off(&mut store, today(), true, now).unwrap();
    let f = reminder_facts(&store, now, &Warsaw).unwrap();
    assert_eq!(f.day_kind, DayKind::DayOff);

    let saturday = t("2026-11-28T10:00");
    let f = reminder_facts(&store, saturday, &Warsaw).unwrap();
    assert_eq!(f.day_kind, DayKind::Weekend);
    assert_eq!(f.today, NaiveDate::from_ymd_opt(2026, 11, 28).unwrap());
}

#[test]
fn start_and_stop_store_events_in_local_time_and_refusals_store_none() {
    use day_navigator_lib::events::{self, EventView};
    let (_tmp, dir) = test_store_dir();
    let mut store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
    let list = |s: &Store| events::list_today(s, today()).unwrap().events;

    assert!(matches!(
        work_time::stop(&mut store, t("2026-11-23T08:00"), &Warsaw),
        Err(WorkTimeError::NotRunning)
    ));
    assert!(list(&store).is_empty());

    let started = t("2026-11-23T08:15") + 7_000;
    work_time::start(&mut store, started, &Warsaw).unwrap();
    assert!(matches!(
        work_time::start(&mut store, started + 1_000, &Warsaw),
        Err(WorkTimeError::AlreadyRunning)
    ));
    assert_eq!(list(&store).len(), 1);

    let stopped = t("2026-11-23T11:40") + 3_000;
    work_time::stop(&mut store, stopped, &Warsaw).unwrap();
    let events = list(&store);
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event, EventView::WorkStopped);
    assert_eq!(events[0].at, "2026-11-23T11:40:03");
    assert_eq!(events[1].event, EventView::WorkStarted);
    assert_eq!(events[1].at, "2026-11-23T08:15:07");
}

// ---- S-06: work-entry note column (migration v11) ----

mod note_column {
    use super::*;
    use day_navigator_lib::store::StoreError;

    const SENTINEL: &str = "SENTINEL-invented-7c1e5";

    fn note_migration() -> usize {
        schema::MIGRATIONS
            .iter()
            .position(|m| m.sql.contains("ADD COLUMN note"))
            .expect("the work-entry note migration")
    }

    fn set_note(store: &mut Store, note: &str) -> Result<(), StoreError> {
        store.write::<_, StoreError>(|tx| {
            tx.execute("UPDATE work_entries SET note = ?1 WHERE id = 1", [note])?;
            Ok(())
        })
    }

    fn store_with_one_entry() -> (tempfile::TempDir, Store) {
        let (tmp, dir) = test_store_dir();
        let mut store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
        store
            .write::<_, StoreError>(|tx| {
                tx.execute_batch(
                    "INSERT INTO work_entries(id, start_ms, end_ms, created_at_ms, updated_at_ms) \
                     VALUES (1, 1000, 2000, 1000, 1000);",
                )?;
                Ok(())
            })
            .unwrap();
        (tmp, store)
    }

    #[test]
    fn upgrade_from_v10_adds_an_empty_note_and_keeps_rows() {
        let (_tmp, dir) = test_store_dir();
        let before = note_migration();
        assert_eq!(before, 10);
        let closed_start = t("2026-11-23T08:00");
        let closed_end = t("2026-11-23T09:00");
        let running_start = t("2026-11-23T10:00");
        let removed_start = t("2026-11-22T08:00");
        let removed_end = t("2026-11-22T09:00");
        {
            let mut old = Store::open(&dir, &schema::MIGRATIONS[..before], today()).unwrap();
            old.write::<_, StoreError>(|tx| {
                tx.execute(
                    "INSERT INTO work_entries(id, start_ms, end_ms, removed_at_ms, \
                     created_at_ms, updated_at_ms) VALUES (1, ?1, ?2, NULL, 1, 1)",
                    [closed_start, closed_end],
                )?;
                tx.execute(
                    "INSERT INTO work_entries(id, start_ms, end_ms, removed_at_ms, \
                     created_at_ms, updated_at_ms) VALUES (2, ?1, NULL, NULL, 1, 1)",
                    [running_start],
                )?;
                tx.execute(
                    "INSERT INTO work_entries(id, start_ms, end_ms, removed_at_ms, \
                     created_at_ms, updated_at_ms) VALUES (3, ?1, ?2, 5, 1, 1)",
                    [removed_start, removed_end],
                )?;
                Ok(())
            })
            .unwrap();
        }
        let store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
        let version: i64 = store
            .read::<_, StoreError>(|c| Ok(c.query_row("PRAGMA user_version", [], |r| r.get(0))?))
            .unwrap();
        assert_eq!(version, schema::MIGRATIONS.len() as i64);
        assert_eq!(
            rows(&store),
            vec![
                (1, closed_start, Some(closed_end), None),
                (2, running_start, None, None),
                (3, removed_start, Some(removed_end), Some(5)),
            ]
        );
        let notes: Vec<String> = store
            .read::<_, StoreError>(|c| {
                let mut stmt = c.prepare("SELECT note FROM work_entries ORDER BY id")?;
                let out = stmt.query_map([], |r| r.get(0))?;
                Ok(out.collect::<Result<_, _>>()?)
            })
            .unwrap();
        assert_eq!(notes, vec!["", "", ""]);

        let day = work_time::entries_for_day(&store, today(), running_start + 1, &Warsaw).unwrap();
        let first = &day.entries[0];
        assert_eq!(first.id, 1);
        assert_eq!(first.note, "");
        assert_eq!(serde_json::to_value(first).unwrap()["note"], "");
    }

    #[test]
    fn note_check_refuses_raw_values_and_counts_characters() {
        let (_tmp, mut store) = store_with_one_entry();
        assert!(set_note(&mut store, &"ż".repeat(200)).is_ok());
        assert!(set_note(&mut store, &"ż".repeat(201)).is_err());
        assert!(set_note(&mut store, "a\nb").is_err());
        assert!(set_note(&mut store, "a\rb").is_err());
    }

    #[test]
    fn note_check_error_carries_no_note() {
        let (_tmp, mut store) = store_with_one_entry();
        let long = format!("{SENTINEL}{}", "x".repeat(200));
        let err = set_note(&mut store, &long).unwrap_err();
        let json = serde_json::to_string(&err).unwrap();
        assert!(!json.contains(SENTINEL));
        assert!(!err.to_string().contains(SENTINEL));
        assert!(!format!("{err:?}").contains(SENTINEL));
    }
}

// ---- S-06: writing the note ----

mod note_write {
    use super::*;
    use day_navigator_lib::events;
    use day_navigator_lib::store::StoreError;

    const SENTINEL: &str = "SENTINEL-invented-4d2a9";

    fn sentinel_hits(store: &Store) -> i64 {
        store
            .read::<_, StoreError>(|c| {
                Ok(c.query_row(
                    "SELECT count(*) FROM events WHERE coalesce(reference,'') || \
                     coalesce(title,'') || coalesce(source_label,'') || \
                     coalesce(error_kind,'') LIKE '%' || ?1 || '%'",
                    [SENTINEL],
                    |r| r.get::<_, i64>(0),
                )?)
            })
            .unwrap()
    }

    #[test]
    fn note_survives_closing_and_reopening_the_store() {
        let (_tmp, dir) = test_store_dir();
        let now = t("2026-11-23T12:00");
        {
            let mut store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
            work_time::add(
                &mut store,
                "2026-11-23T08:00",
                "2026-11-23T09:00",
                "  Wymyślona notatka ",
                now,
                &Warsaw,
            )
            .unwrap();
        }
        let store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
        let day = work_time::entries_for_day(&store, today(), now, &Warsaw).unwrap();
        assert_eq!(day.entries[0].note, "Wymyślona notatka");
    }

    #[test]
    fn note_never_reaches_the_event_log() {
        let (_tmp, dir) = test_store_dir();
        let now = t("2026-11-23T12:00");
        let mut store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
        let id = work_time::add(
            &mut store,
            "2026-11-23T08:00",
            "2026-11-23T09:00",
            SENTINEL,
            now,
            &Warsaw,
        )
        .unwrap();
        work_time::update(
            &mut store,
            id,
            "2026-11-23T08:00",
            Some("2026-11-23T09:00"),
            SENTINEL,
            now,
            &Warsaw,
        )
        .unwrap();
        assert!(
            events::list_today(&store, today())
                .unwrap()
                .events
                .is_empty()
        );

        let running = work_time::start(&mut store, t("2026-11-23T10:00"), &Warsaw).unwrap();
        work_time::update(
            &mut store,
            running,
            "2026-11-23T10:00",
            None,
            SENTINEL,
            now,
            &Warsaw,
        )
        .unwrap();
        work_time::stop(&mut store, t("2026-11-23T11:00"), &Warsaw).unwrap();
        assert_eq!(events::list_today(&store, today()).unwrap().events.len(), 2);
        assert_eq!(sentinel_hits(&store), 0);
    }

    #[test]
    fn refusals_with_a_note_leak_nothing() {
        let (_tmp, dir) = test_store_dir();
        let now = t("2026-11-23T12:00");
        let mut store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
        let id = work_time::add(
            &mut store,
            "2026-11-23T08:00",
            "2026-11-23T09:00",
            "",
            now,
            &Warsaw,
        )
        .unwrap();
        let overlap = work_time::add(
            &mut store,
            "2026-11-23T08:30",
            "2026-11-23T09:30",
            SENTINEL,
            now,
            &Warsaw,
        )
        .unwrap_err();
        assert!(matches!(overlap, WorkTimeError::Overlap { .. }));
        work_time::remove(&mut store, id, now).unwrap();
        let missing = work_time::update(
            &mut store,
            id,
            "2026-11-23T08:00",
            Some("2026-11-23T09:00"),
            SENTINEL,
            now,
            &Warsaw,
        )
        .unwrap_err();
        assert!(matches!(missing, WorkTimeError::NotFound));
        for e in [overlap, missing] {
            let text = format!("{e} {e:?} {}", serde_json::to_string(&e).unwrap());
            assert!(!text.contains(SENTINEL));
            assert!(!e.kind().contains(SENTINEL));
        }
    }
}
