//! S-08 durability: work-time entries survive closing and reopening the real
//! store, and removed entries stay as rows.

mod common;

use chrono::NaiveDate;
use chrono_tz::Europe::Warsaw;
use common::test_store_dir;
use day_navigator_lib::store::{Store, schema};
use day_navigator_lib::work_time::report;
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

#[test]
fn reminder_facts_report_the_running_entry_and_counted_work() {
    use day_navigator_lib::work_time::DayKind;
    use day_navigator_lib::work_time::reminders::reminder_facts;

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
        work_time::add(store, from, to, now, &Warsaw).unwrap()
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
    let id = work_time::start(&mut store, started).unwrap();
    let running = reminder_facts(&store, now, &Warsaw)
        .unwrap()
        .running
        .unwrap();
    assert_eq!((running.id, running.start_ms), (id, started));
}

#[test]
fn reminder_facts_count_a_midnight_spanning_portion() {
    use day_navigator_lib::work_time::reminders::reminder_facts;

    let (_tmp, dir) = test_store_dir();
    let now = t("2026-11-23T12:00");
    let mut store = Store::open(&dir, schema::MIGRATIONS, today()).unwrap();
    work_time::add(
        &mut store,
        "2026-11-22T23:00",
        "2026-11-23T00:30",
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
    use day_navigator_lib::work_time::DayKind;
    use day_navigator_lib::work_time::reminders::reminder_facts;

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
