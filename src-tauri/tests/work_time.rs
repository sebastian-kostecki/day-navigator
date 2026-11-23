//! S-08 durability: work-time entries survive closing and reopening the real
//! store, and removed entries stay as rows.

mod common;

use chrono::NaiveDate;
use chrono_tz::Europe::Warsaw;
use common::test_store_dir;
use day_navigator_lib::store::{Store, schema};
use day_navigator_lib::work_time::rules::resolve_local;
use day_navigator_lib::work_time::{self, WorkTimeError};

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
        work_time::start(&mut store, started).unwrap()
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
        work_time::start(&mut store, later),
        Err(WorkTimeError::AlreadyRunning)
    ));
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
            now,
            &Warsaw,
        )
        .unwrap();
        let kept = work_time::start(&mut store, t("2026-11-23T09:00")).unwrap();
        work_time::stop(&mut store, t("2026-11-23T10:00")).unwrap();
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
