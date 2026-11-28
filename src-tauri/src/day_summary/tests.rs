//! Day summary rules on invented data in `Europe/Warsaw`. No test asserts a
//! norm number: the deviation is compared with `work_time::status`.

use chrono::{NaiveDate, NaiveDateTime};
use chrono_tz::Europe::Warsaw;
use tempfile::TempDir;

use super::*;
use crate::gitlab::{GitlabItem, ItemKind};
use crate::own_tasks::{self, OwnTaskInput};
use crate::space::Space;
use crate::store::{StoreError, schema};
use crate::todo::{TodoEntry, TodoError};
use crate::work_time::WorkTimeError;
use crate::work_time::rules::resolve_local;

const MIN: i64 = 60_000;
const D1: &str = "2026-11-23";
const D2: &str = "2026-11-24";
const ACC: &str = "account-a";

fn day(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

fn at(d: &str, h: u32, m: u32) -> NaiveDateTime {
    day(d).and_hms_opt(h, m, 0).unwrap()
}

fn t(local: &str) -> i64 {
    resolve_local(local, &Warsaw).unwrap()
}

fn open() -> (TempDir, Store) {
    let tmp = tempfile::tempdir().unwrap();
    let store = Store::open(&tmp.path().join("data"), schema::MIGRATIONS, day(D1)).unwrap();
    (tmp, store)
}

fn own(store: &mut Store, title: &str) -> i64 {
    let input = OwnTaskInput {
        space: Space::Work,
        title: title.into(),
        note: String::new(),
        due: None,
        priority: None,
    };
    own_tasks::add(store, &input, at(D1, 7, 0)).unwrap().id
}

fn gl(n: u32) -> GitlabItem {
    GitlabItem {
        kind: ItemKind::Issue,
        id: u64::from(n),
        project_id: 1,
        iid: u64::from(n),
        changed: None,
        milestone: None,
        is_new: false,
        hidden_at_ms: None,
        reference: format!("invented/project#{n}"),
        title: format!("Pozycja testowa {n}"),
        web_url: None,
        updated_at: "2026-11-20T10:00:00+01:00".into(),
        labels: vec![],
        draft: false,
        review_requested: false,
    }
}

fn pick_own(store: &mut Store, task: i64, d: &str, h: u32) -> i64 {
    todo::add_own(store, task, day(d), at(d, h, 0), Ok(10)).unwrap()
}

fn pick_gl(store: &mut Store, n: u32, d: &str, h: u32) -> i64 {
    todo::add_gitlab(store, ACC, &gl(n), day(d), at(d, h, 0), Ok(10)).unwrap()
}

fn tick(store: &mut Store, id: i64, d: &str, h: u32) {
    todo::set_ticked(store, id, true, day(d), at(d, h, 30)).unwrap();
}

fn title(i: &TodoItem) -> &str {
    match &i.entry {
        TodoEntry::Own { title, .. }
        | TodoEntry::Gitlab { title, .. }
        | TodoEntry::Github { title, .. } => title,
    }
}

fn titles(items: &[TodoItem]) -> Vec<&str> {
    items.iter().map(title).collect()
}

fn settings() -> WorkTimeSettings {
    WorkTimeSettings::default()
}

fn read_at(store: &mut Store, now: i64) -> DaySummary {
    read(store, now, &Warsaw, &settings()).unwrap()
}

fn add_entry(store: &mut Store, from: &str, to: &str, now: i64) {
    work_time::add(store, from, to, "", now, &Warsaw).unwrap();
}

#[test]
fn done_and_carry_over_split_keeps_list_order() {
    let (_t, mut s) = open();
    let a = own(&mut s, "Zadanie A");
    let b = own(&mut s, "Zadanie B");
    let c = own(&mut s, "Zadanie C");
    let ia = pick_own(&mut s, a, D1, 8);
    pick_own(&mut s, b, D1, 9);
    pick_own(&mut s, c, D1, 10);
    let g1 = pick_gl(&mut s, 1, D1, 11);
    pick_gl(&mut s, 2, D1, 12);
    tick(&mut s, ia, D1, 13);
    tick(&mut s, g1, D1, 14);

    let sum = read_at(&mut s, t("2026-11-23T15:00"));
    assert_eq!(sum.day, D1);
    assert_eq!(titles(&sum.done), ["Zadanie A", "Pozycja testowa 1"]);
    assert_eq!(
        titles(&sum.carry_over),
        ["Zadanie B", "Zadanie C", "Pozycja testowa 2"]
    );
    // Same order as todo::list: open entries first, then ticked ones.
    let list = todo::list(&mut s, day(D1), at(D1, 15, 0), Ok(5)).unwrap();
    let expected: Vec<&str> = list.items.iter().map(title).collect();
    let got: Vec<&str> = sum.carry_over.iter().chain(&sum.done).map(title).collect();
    assert_eq!(got, expected);
}

#[test]
fn item_picked_yesterday_and_still_open_carries_over_flagged() {
    let (_t, mut s) = open();
    let a = own(&mut s, "Zadanie A");
    pick_own(&mut s, a, D1, 8);
    let sum = read_at(&mut s, t("2026-11-24T10:00"));
    assert_eq!(sum.day, D2);
    assert!(sum.done.is_empty());
    assert_eq!(sum.carry_over.len(), 1);
    assert!(sum.carry_over[0].carried_over);
}

#[test]
fn item_ticked_yesterday_is_in_neither_list() {
    let (_t, mut s) = open();
    let a = own(&mut s, "Zadanie A");
    let ia = pick_own(&mut s, a, D1, 8);
    tick(&mut s, ia, D1, 10);
    let sum = read_at(&mut s, t("2026-11-24T10:00"));
    assert!(sum.done.is_empty());
    assert!(sum.carry_over.is_empty());
}

#[test]
fn closed_work_day_total_equals_status() {
    let (_t, mut s) = open();
    let now = t("2026-11-23T16:00");
    add_entry(&mut s, "2026-11-23T08:00", "2026-11-23T15:30", now);
    let sum = read_at(&mut s, now);
    let status = work_time::status(&s, now, &Warsaw, &settings()).unwrap();
    assert_eq!(sum.today_ms, 450 * MIN);
    assert_eq!(sum.today_ms, status.today_closed_ms);
}

#[test]
fn running_entry_adds_elapsed_time() {
    let (_t, mut s) = open();
    work_time::start(&mut s, t("2026-11-23T08:00"), &Warsaw).unwrap();
    let now = t("2026-11-23T10:30");
    let sum = read_at(&mut s, now);
    let status = work_time::status(&s, now, &Warsaw, &settings()).unwrap();
    let from = status.running_from_ms.unwrap();
    assert_eq!(sum.today_ms, status.today_closed_ms + (now - from));
    assert_eq!(sum.today_ms, 150 * MIN);
}

#[test]
fn month_deviation_is_passed_through_from_status() {
    let (_t, mut s) = open();
    let now = t("2026-11-23T16:00");
    add_entry(&mut s, "2026-11-23T08:00", "2026-11-23T14:00", now);
    let sum = read_at(&mut s, now);
    let status = work_time::status(&s, now, &Warsaw, &settings()).unwrap();
    assert_eq!(sum.month_deviation_ms, status.month_deviation_ms);
}

#[test]
fn empty_day_is_a_summary_not_an_error() {
    let (_t, mut s) = open();
    let sum = read_at(&mut s, t("2026-11-23T16:00"));
    assert_eq!(sum.today_ms, 0);
    assert!(sum.done.is_empty());
    assert!(sum.carry_over.is_empty());
}

#[test]
fn stop_after_midnight_summarizes_the_new_day() {
    let (_t, mut s) = open();
    let now = t("2026-11-24T00:31");
    add_entry(&mut s, "2026-11-23T22:00", "2026-11-24T00:30", now);
    let sum = read_at(&mut s, now);
    assert_eq!(sum.day, D2);
    assert_eq!(sum.today_ms, 30 * MIN);
}

fn status_with(closed: i64, running_from: Option<i64>, as_of: i64, dev: i64) -> WorkTimerStatus {
    WorkTimerStatus {
        running: None,
        today_closed_ms: closed,
        running_from_ms: running_from,
        day_start_ms: 0,
        day_end_ms: 1,
        month_deviation_ms: dev,
        as_of_ms: as_of,
    }
}

fn item(id: i64, ticked: bool) -> TodoItem {
    TodoItem {
        id,
        picked_on: D1.into(),
        carried_over: false,
        ticked,
        picked_at: "2026-11-23T08:00:00".into(),
        entry: TodoEntry::Own {
            task_id: id,
            space: Space::Work,
            title: format!("Pozycja testowa {id}"),
        },
    }
}

#[test]
fn compose_splits_and_passes_figures_through() {
    let list = TodoList {
        day: D1.into(),
        limit: Some(5),
        limit_error: None,
        open_count: 2,
        items: vec![item(1, false), item(2, true), item(3, false)],
    };
    let sum = compose(&status_with(1_000, Some(5_000), 9_000, -42), list);
    assert_eq!(sum.day, D1);
    assert_eq!(sum.today_ms, 5_000);
    assert_eq!(sum.month_deviation_ms, -42);
    assert_eq!(sum.done.iter().map(|i| i.id).collect::<Vec<_>>(), [2]);
    assert_eq!(
        sum.carry_over.iter().map(|i| i.id).collect::<Vec<_>>(),
        [1, 3]
    );
}

#[test]
fn today_ms_never_goes_negative_on_a_future_start() {
    assert_eq!(today_ms(&status_with(10, Some(100), 50, 0)), 10);
}

#[test]
fn error_serialization_and_mappings() {
    assert_eq!(
        serde_json::to_string(&DaySummaryError::Background).unwrap(),
        r#"{"kind":"Background"}"#
    );
    assert_eq!(
        serde_json::to_string(&DaySummaryError::Store(StoreError::Busy)).unwrap(),
        r#"{"kind":"Store","detail":{"kind":"Busy"}}"#
    );
    assert!(matches!(
        DaySummaryError::from(WorkTimeError::InvalidLocalTime),
        DaySummaryError::InvalidLocalTime
    ));
    assert!(matches!(
        DaySummaryError::from(WorkTimeError::Background),
        DaySummaryError::Background
    ));
    assert!(matches!(
        DaySummaryError::from(WorkTimeError::Store(StoreError::Busy)),
        DaySummaryError::Store(StoreError::Busy)
    ));
    assert!(matches!(
        DaySummaryError::from(WorkTimeError::NotRunning),
        DaySummaryError::Store(StoreError::Sql { .. })
    ));
    assert!(matches!(
        DaySummaryError::from(TodoError::Store(StoreError::Busy)),
        DaySummaryError::Store(StoreError::Busy)
    ));
    assert!(matches!(
        DaySummaryError::from(TodoError::Background),
        DaySummaryError::Background
    ));
    assert!(matches!(
        DaySummaryError::from(TodoError::NotFound),
        DaySummaryError::Store(StoreError::Sql { .. })
    ));
    assert_eq!(DaySummaryError::Store(StoreError::Busy).kind(), "Busy");
}
