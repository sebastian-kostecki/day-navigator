//! Toast texts for a tray STOP: fixed Polish wording, today's time and the
//! month's deviation only. No `tauri` import; no item title, reference or
//! count can reach these strings.

use crate::health::{APP_NAME, Notice};
use crate::tray_view;

/// Signed `H:MM` for whole minutes rounded toward zero; `0:00` under a minute.
/// Mirrors TS `formatHm(ms, true)` (`+`/U+2212 prefix).
pub fn format_signed_hm(ms: i64) -> String {
    let minutes = ms / 60_000;
    if minutes == 0 {
        return "0:00".to_string();
    }
    let sign = if minutes < 0 { '\u{2212}' } else { '+' };
    let abs = minutes.unsigned_abs();
    format!("{sign}{}:{:02}", abs / 60, abs % 60)
}

fn title() -> String {
    format!("{APP_NAME} — koniec pracy")
}

pub fn stop_notice(today_ms: i64, month_deviation_ms: i64) -> Notice {
    Notice {
        title: title(),
        body: format!(
            "Dziś: {}. Odchylenie miesiąca: {}.",
            tray_view::format_hm(today_ms),
            format_signed_hm(month_deviation_ms)
        ),
    }
}

pub fn stop_notice_unavailable() -> Notice {
    Notice {
        title: title(),
        body: "Licznik zatrzymany. Czas pracy chwilowo niedostępny — otwórz „Czas pracy”."
            .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const MIN: i64 = 60_000;

    #[test]
    fn signed_formatter_cases() {
        assert_eq!(format_signed_hm(0), "0:00");
        assert_eq!(format_signed_hm(59_999), "0:00");
        assert_eq!(format_signed_hm(-59_999), "0:00");
        assert_eq!(format_signed_hm(MIN), "+0:01");
        assert_eq!(format_signed_hm(-MIN), "\u{2212}0:01");
        assert_eq!(format_signed_hm(-90 * MIN), "\u{2212}1:30");
        assert_eq!(format_signed_hm(125 * 60 * MIN), "+125:00");
        assert_eq!(format_signed_hm(i64::MIN).chars().next(), Some('\u{2212}'));
    }

    #[test]
    fn notice_texts_are_fixed() {
        let n = stop_notice(465 * MIN, -15 * MIN);
        assert_eq!(n.title, format!("{APP_NAME} — koniec pracy"));
        assert_eq!(n.body, "Dziś: 7:45. Odchylenie miesiąca: \u{2212}0:15.");
        let u = stop_notice_unavailable();
        assert_eq!(u.title, n.title);
        assert_eq!(
            u.body,
            "Licznik zatrzymany. Czas pracy chwilowo niedostępny — otwórz „Czas pracy”."
        );
    }

    fn is_hm(s: &str) -> bool {
        match s.split_once(':') {
            Some((h, m)) => {
                !h.is_empty()
                    && h.chars().all(|c| c.is_ascii_digit())
                    && m.len() == 2
                    && m.chars().all(|c| c.is_ascii_digit())
            }
            None => false,
        }
    }

    proptest! {
        #[test]
        fn body_has_only_two_figures(today in any::<i64>(), dev in any::<i64>()) {
            let body = stop_notice(today, dev).body;
            let rest = body.strip_prefix("Dziś: ").unwrap();
            let rest = rest.strip_suffix('.').unwrap();
            let (a, b) = rest.split_once(". Odchylenie miesiąca: ").unwrap();
            prop_assert!(is_hm(a));
            let b_ok = b == "0:00"
                || b.strip_prefix('+').or_else(|| b.strip_prefix('\u{2212}')).is_some_and(is_hm);
            prop_assert!(b_ok);
        }
    }
}
