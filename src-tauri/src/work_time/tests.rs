//! Rule tests in `Europe/Warsaw` with invented instants.

use chrono::{NaiveDate, TimeZone, Utc};
use chrono_tz::Europe::Warsaw;
use tempfile::TempDir;

use super::rules::{day_bounds, resolve_local};
use super::*;
use crate::store::schema;

const MIN: i64 = 60_000;
const HOUR: i64 = 60 * MIN;

fn t(local: &str) -> i64 {
    resolve_local(local, &Warsaw).unwrap()
}

/// "Now" for most tests: Monday 2026-11-23, noon local.
fn now() -> i64 {
    t("2026-11-23T12:00")
}

fn open() -> (TempDir, Store) {
    let tmp = tempfile::tempdir().unwrap();
    let today = NaiveDate::from_ymd_opt(2026, 11, 23).unwrap();
    let store = Store::open(&tmp.path().join("data"), schema::MIGRATIONS, today).unwrap();
    (tmp, store)
}

type Snapshot = Vec<(i64, i64, Option<i64>, Option<i64>, i64, i64)>;

fn snapshot(store: &Store) -> Snapshot {
    store
        .read::<_, WorkTimeError>(|c| {
            let mut stmt = c.prepare(
                "SELECT id, start_ms, end_ms, removed_at_ms, created_at_ms, updated_at_ms \
                 FROM work_entries ORDER BY id",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            })?;
            Ok(rows.collect::<Result<_, _>>()?)
        })
        .unwrap()
}

fn day(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

fn add_entry(store: &mut Store, from: &str, to: &str) -> i64 {
    add(store, from, to, now(), &Warsaw).unwrap()
}

/// Asserts the refusal and that the table is unchanged.
fn refused<T: std::fmt::Debug>(
    store: &Store,
    before: &Snapshot,
    result: Result<T, WorkTimeError>,
    check: impl FnOnce(&WorkTimeError) -> bool,
) {
    let err = result.unwrap_err();
    assert!(check(&err), "unexpected refusal: {err:?}");
    assert_eq!(&snapshot(store), before, "refusal changed the table");
}

#[test]
fn ambiguous_local_time_resolves_to_the_earlier_instant() {
    // 2026-10-25 02:30 happens twice; the earlier one is still CEST (UTC+2).
    let earlier = Utc
        .with_ymd_and_hms(2026, 10, 25, 0, 30, 0)
        .unwrap()
        .timestamp_millis();
    assert_eq!(t("2026-10-25T02:30"), earlier);
}

#[test]
fn gap_local_time_is_refused_and_nothing_is_saved() {
    let (_tmp, mut store) = open();
    let before = snapshot(&store);
    // 2027-03-28 02:30 does not exist in Poland.
    let r = add(
        &mut store,
        "2027-03-28T02:30",
        "2027-03-28T03:30",
        t("2027-04-01T12:00"),
        &Warsaw,
    );
    refused(&store, &before, r, |e| {
        matches!(e, WorkTimeError::InvalidLocalTime)
    });
    assert!(matches!(
        resolve_local("not a time", &Warsaw),
        Err(WorkTimeError::InvalidLocalTime)
    ));
}

#[test]
fn entry_across_midnight_counts_two_hours_on_each_day() {
    let (_tmp, mut store) = open();
    let id = add_entry(&mut store, "2026-11-20T22:00", "2026-11-21T02:00");
    for d in ["2026-11-20", "2026-11-21"] {
        let got = entries_for_day(&store, day(d), now(), &Warsaw).unwrap();
        assert_eq!(got.entries.len(), 1, "{d}");
        assert_eq!(got.entries[0].id, id);
        assert_eq!(got.entries[0].day_portion_ms, 2 * HOUR, "{d}");
        assert_eq!(got.closed_ms, 2 * HOUR, "{d}");
        // The row keeps the full start and end.
        assert_eq!(got.entries[0].start_local, "2026-11-20T22:00");
        assert_eq!(
            got.entries[0].end_local.as_deref(),
            Some("2026-11-21T02:00")
        );
    }
    let other = entries_for_day(&store, day("2026-11-22"), now(), &Warsaw).unwrap();
    assert!(other.entries.is_empty());
    assert_eq!(other.closed_ms, 0);
}

#[test]
fn dst_days_are_25_and_23_hours_long() {
    let (_tmp, mut store) = open();
    let late = t("2027-04-01T12:00");
    let (s, e) = day_bounds(day("2026-10-25"), &Warsaw).unwrap();
    assert_eq!(e - s, 25 * HOUR);
    let (s, e) = day_bounds(day("2027-03-28"), &Warsaw).unwrap();
    assert_eq!(e - s, 23 * HOUR);

    add(
        &mut store,
        "2026-10-25T00:00",
        "2026-10-26T00:00",
        late,
        &Warsaw,
    )
    .unwrap();
    add(
        &mut store,
        "2027-03-28T00:00",
        "2027-03-29T00:00",
        late,
        &Warsaw,
    )
    .unwrap();
    let autumn = entries_for_day(&store, day("2026-10-25"), late, &Warsaw).unwrap();
    assert_eq!(autumn.closed_ms, 25 * HOUR);
    let spring = entries_for_day(&store, day("2027-03-28"), late, &Warsaw).unwrap();
    assert_eq!(spring.closed_ms, 23 * HOUR);
}

#[test]
fn adjacent_entries_are_allowed() {
    let (_tmp, mut store) = open();
    add_entry(&mut store, "2026-11-20T08:00", "2026-11-20T12:00");
    add_entry(&mut store, "2026-11-20T12:00", "2026-11-20T13:00");
    add_entry(&mut store, "2026-11-20T07:00", "2026-11-20T08:00");
    let got = entries_for_day(&store, day("2026-11-20"), now(), &Warsaw).unwrap();
    assert_eq!(got.entries.len(), 3);
    assert_eq!(got.closed_ms, 6 * HOUR);
    // Sorted by start.
    assert!(
        got.entries
            .windows(2)
            .all(|w| w[0].start_ms < w[1].start_ms)
    );
}

#[test]
fn overlap_with_a_closed_entry_is_refused_by_id() {
    let (_tmp, mut store) = open();
    let id = add_entry(&mut store, "2026-11-20T08:00", "2026-11-20T12:00");
    let before = snapshot(&store);
    for (from, to) in [
        ("2026-11-20T11:30", "2026-11-20T13:00"),
        ("2026-11-20T07:00", "2026-11-20T08:30"),
        ("2026-11-20T09:00", "2026-11-20T10:00"),
        ("2026-11-20T07:00", "2026-11-20T13:00"),
    ] {
        let r = add(&mut store, from, to, now(), &Warsaw);
        refused(
            &store,
            &before,
            r,
            |e| matches!(e, WorkTimeError::Overlap { entry_id } if *entry_id == id),
        );
    }
}

#[test]
fn overlap_with_the_running_entry_is_refused() {
    let (_tmp, mut store) = open();
    let running = start(&mut store, t("2026-11-23T09:00")).unwrap();
    let before = snapshot(&store);
    // The running entry counts as [start, +inf): even a past-but-after-start
    // closed entry collides with it.
    let r = add(
        &mut store,
        "2026-11-23T10:00",
        "2026-11-23T11:00",
        now(),
        &Warsaw,
    );
    refused(
        &store,
        &before,
        r,
        |e| matches!(e, WorkTimeError::Overlap { entry_id } if *entry_id == running),
    );
    // Ending exactly at the running start is adjacent, so allowed.
    add_entry(&mut store, "2026-11-23T08:00", "2026-11-23T09:00");
}

#[test]
fn future_times_are_refused() {
    let (_tmp, mut store) = open();
    let before = snapshot(&store);
    let r = add(
        &mut store,
        "2026-11-23T11:00",
        "2026-11-23T12:01",
        now(),
        &Warsaw,
    );
    refused(&store, &before, r, |e| matches!(e, WorkTimeError::InFuture));
    let r = add(
        &mut store,
        "2026-11-23T12:01",
        "2026-11-23T12:30",
        now(),
        &Warsaw,
    );
    refused(&store, &before, r, |e| matches!(e, WorkTimeError::InFuture));
    // End exactly at now is fine.
    add_entry(&mut store, "2026-11-23T11:00", "2026-11-23T12:00");

    let id = add_entry(&mut store, "2026-11-23T08:00", "2026-11-23T09:00");
    let before = snapshot(&store);
    let r = update(
        &mut store,
        id,
        "2026-11-23T08:00",
        Some("2026-11-23T12:01"),
        now(),
        &Warsaw,
    );
    refused(&store, &before, r, |e| matches!(e, WorkTimeError::InFuture));
}

#[test]
fn end_not_after_start_is_refused() {
    let (_tmp, mut store) = open();
    let before = snapshot(&store);
    for (from, to) in [
        ("2026-11-20T10:00", "2026-11-20T09:00"),
        ("2026-11-20T10:00", "2026-11-20T10:00"),
    ] {
        let r = add(&mut store, from, to, now(), &Warsaw);
        refused(&store, &before, r, |e| {
            matches!(e, WorkTimeError::EndNotAfterStart)
        });
    }
    let id = add_entry(&mut store, "2026-11-20T08:00", "2026-11-20T09:00");
    let before = snapshot(&store);
    let r = update(
        &mut store,
        id,
        "2026-11-20T08:00",
        Some("2026-11-20T07:00"),
        now(),
        &Warsaw,
    );
    refused(&store, &before, r, |e| {
        matches!(e, WorkTimeError::EndNotAfterStart)
    });
}

#[test]
fn start_and_stop_rules() {
    let (_tmp, mut store) = open();
    let before = snapshot(&store);
    let r = stop(&mut store, now());
    refused(&store, &before, r, |e| {
        matches!(e, WorkTimeError::NotRunning)
    });

    let id = start(&mut store, now()).unwrap();
    assert!(is_running(&store).unwrap());
    let before = snapshot(&store);
    // A double click: the second START must not create a second row.
    let r = start(&mut store, now() + 1);
    refused(&store, &before, r, |e| {
        matches!(e, WorkTimeError::AlreadyRunning)
    });
    // Stop within the same millisecond as start.
    let r = stop(&mut store, now());
    refused(&store, &before, r, |e| {
        matches!(e, WorkTimeError::EndNotAfterStart)
    });

    stop(&mut store, now() + HOUR).unwrap();
    assert!(!is_running(&store).unwrap());
    let rows = snapshot(&store);
    assert_eq!(rows.len(), 1);
    assert_eq!(
        (rows[0].0, rows[0].1, rows[0].2),
        (id, now(), Some(now() + HOUR))
    );
}

#[test]
fn start_inside_a_closed_entry_is_refused() {
    let (_tmp, mut store) = open();
    let id = add_entry(&mut store, "2026-11-23T08:00", "2026-11-23T11:00");
    let before = snapshot(&store);
    let r = start(&mut store, t("2026-11-23T10:00"));
    refused(
        &store,
        &before,
        r,
        |e| matches!(e, WorkTimeError::Overlap { entry_id } if *entry_id == id),
    );
    // Starting exactly when the last entry ended is allowed.
    start(&mut store, t("2026-11-23T11:00")).unwrap();
}

#[test]
fn running_entry_rejects_an_end_edit_and_allows_a_start_edit() {
    let (_tmp, mut store) = open();
    let id = start(&mut store, t("2026-11-23T10:00")).unwrap();
    let before = snapshot(&store);
    let r = update(
        &mut store,
        id,
        "2026-11-23T10:00",
        Some("2026-11-23T11:00"),
        now(),
        &Warsaw,
    );
    refused(&store, &before, r, |e| {
        matches!(e, WorkTimeError::RunningEndEdit)
    });

    update(&mut store, id, "2026-11-23T09:00", None, now(), &Warsaw).unwrap();
    let s = status(&store, now(), &Warsaw).unwrap();
    assert_eq!(s.running.unwrap().start_ms, t("2026-11-23T09:00"));
    assert_eq!(s.running_from_ms, Some(t("2026-11-23T09:00")));

    // A closed entry cannot be saved without an end.
    let closed = add_entry(&mut store, "2026-11-20T08:00", "2026-11-20T09:00");
    let before = snapshot(&store);
    let r = update(&mut store, closed, "2026-11-20T08:00", None, now(), &Warsaw);
    refused(&store, &before, r, |e| {
        matches!(e, WorkTimeError::InvalidLocalTime)
    });
}

#[test]
fn removed_entries_are_ignored_by_totals_and_overlap_checks() {
    let (_tmp, mut store) = open();
    let id = add_entry(&mut store, "2026-11-20T08:00", "2026-11-20T12:00");
    remove(&mut store, id, now()).unwrap();

    // The row stays in the table, soft removed.
    let rows = snapshot(&store);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].3, Some(now()));

    let got = entries_for_day(&store, day("2026-11-20"), now(), &Warsaw).unwrap();
    assert!(got.entries.is_empty());
    assert_eq!(got.closed_ms, 0);
    // The same interval can be added again.
    add_entry(&mut store, "2026-11-20T09:00", "2026-11-20T10:00");

    // Removed ids are unknown for edit and remove.
    let before = snapshot(&store);
    let r = remove(&mut store, id, now());
    refused(&store, &before, r, |e| matches!(e, WorkTimeError::NotFound));
    let r = update(
        &mut store,
        id,
        "2026-11-20T08:00",
        Some("2026-11-20T09:00"),
        now(),
        &Warsaw,
    );
    refused(&store, &before, r, |e| matches!(e, WorkTimeError::NotFound));
    let r = remove(&mut store, 9999, now());
    refused(&store, &before, r, |e| matches!(e, WorkTimeError::NotFound));
}

#[test]
fn removing_the_running_entry_stops_counting_and_allows_a_new_start() {
    let (_tmp, mut store) = open();
    let id = start(&mut store, t("2026-11-23T09:00")).unwrap();
    remove(&mut store, id, now()).unwrap();
    assert!(!is_running(&store).unwrap());
    let s = status(&store, now(), &Warsaw).unwrap();
    assert!(s.running.is_none());
    assert_eq!(s.today_closed_ms, 0);
    start(&mut store, now()).unwrap();
}

#[test]
fn unique_index_rejects_a_second_running_row() {
    let (_tmp, mut store) = open();
    start(&mut store, now()).unwrap();
    let r = store.write::<_, WorkTimeError>(|tx| {
        tx.execute(
            "INSERT INTO work_entries(start_ms, end_ms, created_at_ms, updated_at_ms) \
             VALUES (1, NULL, 1, 1)",
            [],
        )?;
        Ok(())
    });
    assert!(matches!(
        r,
        Err(WorkTimeError::Store(crate::store::StoreError::Sql { .. }))
    ));
    assert_eq!(snapshot(&store).len(), 1);
}

#[test]
fn check_constraint_rejects_end_not_after_start() {
    let (_tmp, mut store) = open();
    let r = store.write::<_, WorkTimeError>(|tx| {
        tx.execute(
            "INSERT INTO work_entries(start_ms, end_ms, created_at_ms, updated_at_ms) \
             VALUES (10, 10, 1, 1)",
            [],
        )?;
        Ok(())
    });
    assert!(r.is_err());
    assert!(snapshot(&store).is_empty());
}

#[test]
fn serialized_overlap_holds_only_the_entry_id() {
    let v = serde_json::to_value(WorkTimeError::Overlap { entry_id: 7 }).unwrap();
    assert_eq!(
        v,
        serde_json::json!({ "kind": "Overlap", "detail": { "entryId": 7 } })
    );
    let v = serde_json::to_value(WorkTimeError::AlreadyRunning).unwrap();
    assert_eq!(v, serde_json::json!({ "kind": "AlreadyRunning" }));
}

#[test]
fn status_counts_only_todays_closed_entries() {
    let (_tmp, mut store) = open();
    add_entry(&mut store, "2026-11-23T08:00", "2026-11-23T09:00");
    // Yesterday's entry does not count today.
    add_entry(&mut store, "2026-11-22T08:00", "2026-11-22T10:00");
    let s = status(&store, now(), &Warsaw).unwrap();
    let (day_start, day_end) = day_bounds(day("2026-11-23"), &Warsaw).unwrap();
    assert_eq!(s.today_closed_ms, HOUR);
    assert_eq!((s.day_start_ms, s.day_end_ms), (day_start, day_end));
    assert!(s.running.is_none() && s.running_from_ms.is_none());
}

#[test]
fn running_entry_from_before_midnight_counts_only_from_midnight() {
    let (_tmp, mut store) = open();
    let started = t("2026-11-22T23:00");
    start(&mut store, started).unwrap();
    let late = t("2026-11-23T01:00");
    let s = status(&store, late, &Warsaw).unwrap();
    let (day_start, _) = day_bounds(day("2026-11-23"), &Warsaw).unwrap();
    assert_eq!(s.running.as_ref().unwrap().start_ms, started);
    assert_eq!(s.running_from_ms, Some(day_start));
    assert_eq!(s.today_closed_ms, 0);

    let yesterday = entries_for_day(&store, day("2026-11-22"), late, &Warsaw).unwrap();
    assert_eq!(yesterday.entries.len(), 1);
    assert_eq!(yesterday.entries[0].day_portion_ms, HOUR);
    assert_eq!(yesterday.running_from_ms, None);
    let today = entries_for_day(&store, day("2026-11-23"), late, &Warsaw).unwrap();
    assert_eq!(today.entries[0].day_portion_ms, HOUR);
    assert_eq!(today.running_from_ms, Some(day_start));
    // A day that has not begun does not list the running entry.
    let tomorrow = entries_for_day(&store, day("2026-11-24"), late, &Warsaw).unwrap();
    assert!(tomorrow.entries.is_empty());
}

#[test]
fn editing_only_the_end_keeps_the_start_milliseconds() {
    let (_tmp, mut store) = open();
    let s = t("2026-11-23T09:00") + 15_123;
    let e = t("2026-11-23T09:30") + 45_000;
    let id = start(&mut store, s).unwrap();
    stop(&mut store, e).unwrap();
    // The form re-sends both fields; only the end differs.
    update(
        &mut store,
        id,
        "2026-11-23T09:00",
        Some("2026-11-23T10:00"),
        now(),
        &Warsaw,
    )
    .unwrap();
    let rows = snapshot(&store);
    assert_eq!(rows[0].1, s, "start moved");
    assert_eq!(rows[0].2, Some(t("2026-11-23T10:00")));

    // Editing only the start keeps the end's milliseconds.
    let id2 = add_entry(&mut store, "2026-11-23T10:00", "2026-11-23T11:00");
    let e2 = t("2026-11-23T11:00") + 7_000;
    store
        .write::<_, WorkTimeError>(|tx| {
            tx.execute(
                "UPDATE work_entries SET end_ms = ?1 WHERE id = ?2",
                (e2, id2),
            )?;
            Ok(())
        })
        .unwrap();
    update(
        &mut store,
        id2,
        "2026-11-23T10:15",
        Some("2026-11-23T11:00"),
        now(),
        &Warsaw,
    )
    .unwrap();
    let rows = snapshot(&store);
    assert_eq!(rows[1].1, t("2026-11-23T10:15"));
    assert_eq!(rows[1].2, Some(e2), "end moved");
}

#[test]
fn entry_started_right_after_a_stop_in_the_same_minute_stays_editable() {
    let (_tmp, mut store) = open();
    let first = start(&mut store, t("2026-11-23T11:00")).unwrap();
    // STOP 12:00:30, START 12:00:40.
    stop(&mut store, now() + 30_000).unwrap();
    let second = start(&mut store, now() + 40_000).unwrap();
    let later = now() + 5 * MIN;

    // Re-sending the untouched start must not resolve to 12:00:00 and collide.
    update(&mut store, second, "2026-11-23T12:00", None, later, &Warsaw).unwrap();
    assert_eq!(snapshot(&store)[1].1, now() + 40_000);

    // Same for the first entry's end, with its start edited.
    update(
        &mut store,
        first,
        "2026-11-23T10:30",
        Some("2026-11-23T12:00"),
        later,
        &Warsaw,
    )
    .unwrap();
    let rows = snapshot(&store);
    assert_eq!(rows[0].2, Some(now() + 30_000));
}
