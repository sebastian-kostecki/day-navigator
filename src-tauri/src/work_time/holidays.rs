//! Polish public holidays, computed locally.
//!
//! Pure like `norm.rs`: no store, clock or settings. Current law with two
//! year gates (6 January counts from 2011, Christmas Eve from 2025).

use chrono::{Datelike, Days, NaiveDate};

/// Which holiday calendar applies to the norm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolidayCalendar {
    Off,
    Pl,
}

impl HolidayCalendar {
    /// The holiday's name when `day` is a public holiday under this calendar.
    pub fn holiday(self, day: NaiveDate) -> Option<&'static str> {
        match self {
            Self::Off => None,
            Self::Pl => polish_holidays(day.year())
                .into_iter()
                .find(|(date, _)| *date == day)
                .map(|(_, name)| name),
        }
    }
}

/// Easter Sunday by the anonymous Gregorian algorithm (Meeus/Jones/Butcher).
/// Valid from 1583; `None` before that or when the date cannot be built.
pub fn easter_sunday(year: i32) -> Option<NaiveDate> {
    if year < 1583 {
        return None;
    }
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31;
    let day = (h + l - 7 * m + 114) % 31 + 1;
    NaiveDate::from_ymd_opt(year, u32::try_from(month).ok()?, u32::try_from(day).ok()?)
}

/// All Polish public holidays of `year`, sorted by date.
pub fn polish_holidays(year: i32) -> Vec<(NaiveDate, &'static str)> {
    let mut out: Vec<(NaiveDate, &'static str)> = Vec::with_capacity(14);
    let mut fixed = |month: u32, day: u32, name: &'static str| {
        if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
            out.push((date, name));
        }
    };
    fixed(1, 1, "Nowy Rok");
    if year >= 2011 {
        fixed(1, 6, "Święto Trzech Króli");
    }
    fixed(5, 1, "Święto Pracy");
    fixed(5, 3, "Święto Konstytucji 3 Maja");
    fixed(8, 15, "Wniebowzięcie NMP");
    fixed(11, 1, "Wszystkich Świętych");
    fixed(11, 11, "Święto Niepodległości");
    if year >= 2025 {
        fixed(12, 24, "Wigilia");
    }
    fixed(12, 25, "Boże Narodzenie");
    fixed(12, 26, "Drugi dzień Bożego Narodzenia");
    if let Some(easter) = easter_sunday(year) {
        for (offset, name) in [
            (0, "Wielkanoc"),
            (1, "Poniedziałek Wielkanocny"),
            (49, "Zielone Świątki"),
            (60, "Boże Ciało"),
        ] {
            if let Some(date) = easter.checked_add_days(Days::new(offset)) {
                out.push((date, name));
            }
        }
    }
    out.sort_by_key(|(date, _)| *date);
    out
}
