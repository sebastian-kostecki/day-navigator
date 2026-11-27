//! The month report as tab-separated text for the clipboard (spreadsheets).
//!
//! No `tauri` import, no clock and no logging: `now` and the time zone are
//! parameters. The text carries the user's entry notes, so it goes only to the
//! clipboard on the user's click; never to the app log or the event log.
//!
//! Every figure comes from the fields of `MonthReport` (one `compute` in the
//! same `store.read` as the entries), never from a second calculation of the
//! norm. Number rules mirror `src/worktime/format.ts` (`formatHm` floors to the
//! minute, `formatSigned` truncates toward zero), so a cell shows the number
//! the screen shows. The report structs are destructured without `..` and the
//! enums are matched without `_`: a new field or variant must be handled here.

use chrono::{Datelike, NaiveDate, TimeZone, Weekday};

use super::error::WorkTimeError;
use super::norm::{self, MonthPhase, NormFigures};
use super::report::{self, DayKind, DayReport, MonthReport};
use super::rules::{day_bounds, local_string, portion};
use super::{Entry, in_day};
use crate::settings::WorkTimeSettings;
use crate::store::Store;

const MINUTE_MS: i64 = 60_000;
const OWED_SUFFIX: &str = " · należy się inny dzień wolny";

/// The month containing `day` as three tab-separated blocks (summary, days,
/// entries) joined by one empty line. One `store.read`, one snapshot.
pub fn month_report_text<Tz: TimeZone>(
    store: &Store,
    day: NaiveDate,
    now: i64,
    tz: &Tz,
    settings: &WorkTimeSettings,
) -> Result<String, WorkTimeError> {
    store.read(|c| {
        let month = report::compute(c, day, now, tz, settings)?;
        let (first, next_first) = norm::month_days(day);
        let (month_start, _) = day_bounds(first, tz)?;
        let (month_end, _) = day_bounds(next_first, tz)?;
        let entries = in_day(c, month_start, month_end, now)?;
        build(&month, &entries, now, tz)
    })
}

fn build<Tz: TimeZone>(
    month: &MonthReport,
    entries: &[Entry],
    now: i64,
    tz: &Tz,
) -> Result<String, WorkTimeError> {
    let blocks = [
        summary_block(month, tz)?,
        days_block(month)?,
        entries_block(month, entries, now, tz)?,
    ];
    Ok(blocks.join("\n\n"))
}

/// Replaces tabs and control characters with a space, trims, and prefixes a
/// cell a spreadsheet would read as a formula (or a quoted string) with `'`.
pub(super) fn cell(raw: &str) -> String {
    let flat: String = raw
        .chars()
        .map(|c| {
            if c == '\t' || c.is_control() || c == '\u{2028}' || c == '\u{2029}' {
                ' '
            } else {
                c
            }
        })
        .collect();
    let text = flat.trim();
    if text.starts_with(['=', '+', '-', '@', '"', '\'', '＝', '＋', '－', '＠']) {
        format!("'{text}")
    } else {
        text.to_string()
    }
}

/// `H:MM`, floored to the minute, negative as zero.
pub(super) fn hm(ms: i64) -> String {
    let m = ms.max(0) / MINUTE_MS;
    format!("{}:{:02}", m / 60, m % 60)
}

/// Decimal hours with a comma and two places, from whole minutes (half up).
fn decimal(minutes: i64) -> String {
    let hundredths = (minutes * 100 + 30) / 60;
    format!("{},{:02}", hundredths / 100, hundredths % 100)
}

/// Decimal hours of a non-negative time, floored to the minute first.
pub(super) fn hours(ms: i64) -> String {
    decimal(ms.max(0) / MINUTE_MS)
}

/// Decimal hours of a deviation: truncated toward zero to the minute, rounded
/// on the absolute value, ASCII `-` for a negative value, never `+`.
pub(super) fn signed_hours(ms: i64) -> String {
    let m = ms / MINUTE_MS;
    if m < 0 {
        format!("-{}", decimal(-m))
    } else {
        decimal(m)
    }
}

fn opt_hm(ms: Option<i64>) -> String {
    ms.map(hm).unwrap_or_default()
}

fn opt_hours(ms: Option<i64>) -> String {
    ms.map(hours).unwrap_or_default()
}

fn opt_signed(ms: Option<i64>) -> String {
    ms.map(signed_hours).unwrap_or_default()
}

fn opt_count(n: Option<u32>) -> String {
    n.map(|n| n.to_string()).unwrap_or_default()
}

fn weekday_label(day: NaiveDate) -> &'static str {
    match day.weekday() {
        Weekday::Mon => "pn",
        Weekday::Tue => "wt",
        Weekday::Wed => "śr",
        Weekday::Thu => "cz",
        Weekday::Fri => "pt",
        Weekday::Sat => "sb",
        Weekday::Sun => "nd",
    }
}

fn kind_label(kind: DayKind) -> &'static str {
    match kind {
        DayKind::Workday => "roboczy",
        DayKind::HalfDayOff => "pół dnia wolnego",
        DayKind::DayOff => "wolny",
        DayKind::Holiday => "święto",
        DayKind::Weekend => "weekend",
    }
}

fn phase_label(phase: MonthPhase) -> &'static str {
    match phase {
        MonthPhase::Past => "zamknięty",
        MonthPhase::Current => "bieżący",
        MonthPhase::Future => "przyszły",
    }
}

fn time_row(label: &str, ms: Option<i64>) -> String {
    format!("{label}\t{}\t{}", opt_hm(ms), opt_hours(ms))
}

fn summary_block<Tz: TimeZone>(month: &MonthReport, tz: &Tz) -> Result<String, WorkTimeError> {
    let MonthReport {
        month: month_key,
        phase,
        as_of_ms,
        worked_ms,
        figures,
        daily_norm_ms,
        days_off,
        half_days_off,
        days: _,
    } = month;
    let NormFigures {
        norm_ms,
        norm_to_date_ms,
        deviation_ms,
        workdays,
        workdays_after_today,
        half_days_after_today,
        needed_per_day_ms,
        remaining_ms,
    } = figures;
    let as_of = local_string(*as_of_ms, tz)?.replace('T', " ");
    let rows = [
        "Podsumowanie\tH:MM\th".to_string(),
        format!("Miesiąc\t{month_key}\t"),
        format!("Stan\t{}\t", phase_label(*phase)),
        format!("Stan na\t{as_of}\t"),
        time_row("Przepracowano", Some(*worked_ms)),
        time_row("Norma miesiąca", Some(*norm_ms)),
        time_row("Norma do dziś", *norm_to_date_ms),
        format!("Odchylenie\t\t{}", opt_signed(*deviation_ms)),
        time_row("Pozostało", *remaining_ms),
        time_row("Średnio / dzień", *needed_per_day_ms),
        format!("Dni robocze\t{workdays}\t"),
        format!(
            "Dni robocze po dziś\t{}\t",
            opt_count(*workdays_after_today)
        ),
        format!(
            "Dni wolne\t{}\t",
            days_off_value(days_off.len(), half_days_off.len())
        ),
        // Added after the first 13 rows, so references to their cells stay put.
        format!("W tym pół dnia wolnego\t{}\t", half_days_off.len()),
        format!(
            "Dni robocze po dziś, w tym pół dnia\t{}\t",
            opt_count(*half_days_after_today)
        ),
        time_row("Norma dzienna", Some(*daily_norm_ms)),
    ];
    Ok(rows.join("\n"))
}

/// Whole days plus half a day per half day, with a decimal comma, as the
/// "Dni wolne" tile shows it (`formatDaysOff` in `format.ts`).
pub(super) fn days_off_value(whole: usize, half: usize) -> String {
    let halves = whole * 2 + half;
    if halves.is_multiple_of(2) {
        (halves / 2).to_string()
    } else {
        format!("{},5", halves / 2)
    }
}

fn days_block(month: &MonthReport) -> Result<String, WorkTimeError> {
    let mut rows = vec![
        "Data\tDzień\tRodzaj\tŚwięto\tWpisy\tSuma\tSuma h\tNorma\tNorma h\tOdchylenie h"
            .to_string(),
    ];
    for d in &month.days {
        let DayReport {
            day,
            kind,
            worked_ms,
            norm_ms,
            deviation_ms,
            entry_count,
            holiday,
            day_off_owed,
        } = d;
        let date = report::parse_day(day)?;
        let holiday_cell = match holiday {
            Some(name) if *day_off_owed => format!("{}{OWED_SUFFIX}", cell(name)),
            Some(name) => cell(name),
            None => String::new(),
        };
        // The screen shows "—" for the sum exactly here.
        let (sum, sum_h) = if *worked_ms == 0 && deviation_ms.is_none() {
            (String::new(), String::new())
        } else {
            (hm(*worked_ms), hours(*worked_ms))
        };
        rows.push(format!(
            "{day}\t{}\t{}\t{holiday_cell}\t{entry_count}\t{sum}\t{sum_h}\t{}\t{}\t{}",
            weekday_label(date),
            kind_label(*kind),
            hm(*norm_ms),
            hours(*norm_ms),
            opt_signed(*deviation_ms),
        ));
    }
    Ok(rows.join("\n"))
}

/// Local time of an entry end: `HH:MM` on the row's own day, otherwise with
/// the date (as `localLabel` in `WorkTimeEntries.tsx`).
fn local_label<Tz: TimeZone>(ms: i64, row_day: &str, tz: &Tz) -> Result<String, WorkTimeError> {
    let local = local_string(ms, tz)?;
    Ok(match local.strip_prefix(row_day) {
        Some(rest) => rest.trim_start_matches('T').to_string(),
        None => local.replace('T', " "),
    })
}

fn entries_block<Tz: TimeZone>(
    month: &MonthReport,
    entries: &[Entry],
    now: i64,
    tz: &Tz,
) -> Result<String, WorkTimeError> {
    let mut rows = vec!["Data\tOd\tDo\tCzas\tCzas h\tNotatka".to_string()];
    for d in &month.days {
        let date = report::parse_day(&d.day)?;
        let (day_start, day_end) = day_bounds(date, tz)?;
        for e in entries {
            // The same rule as `entry_count`: a non-zero part in the day.
            let part = portion(e.start, e.end, now, day_start, day_end);
            if part <= 0 {
                continue;
            }
            let to = match e.end {
                Some(end) => local_label(end, &d.day, tz)?,
                None => "trwa".to_string(),
            };
            rows.push(format!(
                "{}\t{}\t{to}\t{}\t{}\t{}",
                d.day,
                local_label(e.start, &d.day, tz)?,
                hm(part),
                hours(part),
                cell(&e.note),
            ));
        }
    }
    Ok(rows.join("\n"))
}
