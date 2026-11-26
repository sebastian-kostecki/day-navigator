//! What the tray shows: the icon, the two-line tooltip, the START/STOP menu
//! item and the text of a failed tray action's toast. Pure rules over the health
//! state and the work line; nothing here imports `tauri` and nothing reads a
//! clock or the store.

use crate::health::{self, APP_NAME, Notice, Overall, SourceStatus, TOOLTIP_MAX_UTF16, cap_utf16};
use crate::store::StoreError;
use crate::work_time::WorkTimeError;
use crate::work_time::today::TodayWork;

/// The work-time part of the tray state, as last read by the health round.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkLine {
    /// Not read yet.
    Loading,
    /// The last read failed.
    Unavailable,
    Known(TodayWork),
}

/// Which compiled-in tray image to show. Not `TrayIcon`, which is the
/// tray handle type `runtime.rs` works with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayIconKind {
    Idle,
    Ok,
    Running,
    Error,
}

/// A source error keeps priority over the running counter; `Starting` and a
/// stalled check loop show the idle icon; the running icon needs `Ok`.
pub fn tray_icon(overall: Overall, work: &WorkLine) -> TrayIconKind {
    match overall {
        Overall::Error => TrayIconKind::Error,
        Overall::Starting | Overall::NotRunning => TrayIconKind::Idle,
        Overall::Ok => match work {
            WorkLine::Known(TodayWork { running: true, .. }) => TrayIconKind::Running,
            _ => TrayIconKind::Ok,
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerAction {
    Start,
    Stop,
}

/// The tray menu's work item. Enabled exactly when `action` is `Some`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkItem {
    pub text: &'static str,
    pub action: Option<TimerAction>,
}

const LOADING_TEXT: &str = "Czas pracy: wczytywanie…";
const UNAVAILABLE_TEXT: &str = "Czas pracy niedostępny";

pub fn work_item(overall: Overall, work: &WorkLine) -> WorkItem {
    if overall == Overall::NotRunning {
        // The check loop is stalled, so the tray can no longer refresh.
        return WorkItem {
            text: UNAVAILABLE_TEXT,
            action: None,
        };
    }
    match work {
        WorkLine::Known(TodayWork { running: true, .. }) => WorkItem {
            text: "STOP",
            action: Some(TimerAction::Stop),
        },
        WorkLine::Known(_) => WorkItem {
            text: "START",
            action: Some(TimerAction::Start),
        },
        WorkLine::Loading => WorkItem {
            text: LOADING_TEXT,
            action: None,
        },
        WorkLine::Unavailable => WorkItem {
            text: UNAVAILABLE_TEXT,
            action: None,
        },
    }
}

/// `H:MM`, floored to whole minutes; negative is `0:00`.
pub fn format_hm(ms: i64) -> String {
    let minutes = ms.max(0) / 60_000;
    format!("{}:{:02}", minutes / 60, minutes % 60)
}

pub fn work_line_text(overall: Overall, work: &WorkLine) -> String {
    if overall == Overall::NotRunning {
        return UNAVAILABLE_TEXT.to_string();
    }
    match work {
        WorkLine::Known(w) if w.running => {
            format!("Czas pracy dziś: {} (trwa)", format_hm(w.today_ms))
        }
        WorkLine::Known(w) => format!("Czas pracy dziś: {}", format_hm(w.today_ms)),
        WorkLine::Loading => LOADING_TEXT.to_string(),
        WorkLine::Unavailable => UNAVAILABLE_TEXT.to_string(),
    }
}

pub const LINE_SEPARATOR: &str = "\n";

fn units(s: &str) -> usize {
    s.encode_utf16().count()
}

/// Health text, a line break, then the work line; at most 127 UTF-16 units.
/// When it is too long the health part is cut with "…" and the work line stays
/// whole. Only the app name, source labels and the user's own counter appear.
pub fn tooltip(overall: Overall, sources: &[SourceStatus], work: &WorkLine) -> String {
    let line = work_line_text(overall, work);
    let budget = TOOLTIP_MAX_UTF16.saturating_sub(units(LINE_SEPARATOR) + units(&line));
    let head = cap_utf16(health::tooltip(overall, sources), budget.max(1));
    format!("{head}{LINE_SEPARATOR}{line}")
}

/// The toast for a failed tray action, or `None` when the refusal only means
/// "changed elsewhere" (the tray just refreshes). Fixed text by kind; no ids,
/// times or error detail.
pub fn failure_notice(action: TimerAction, err: &WorkTimeError) -> Option<Notice> {
    if matches!(
        err,
        WorkTimeError::AlreadyRunning | WorkTimeError::NotRunning
    ) {
        return None;
    }
    let title = match action {
        TimerAction::Start => format!("{APP_NAME} — START nie zadziałał"),
        TimerAction::Stop => format!("{APP_NAME} — STOP nie zadziałał"),
    };
    let body = match (action, err) {
        (TimerAction::Start, WorkTimeError::Overlap { .. }) => {
            "Zegar systemowy wskazuje czas sprzed końca innego wpisu — sprawdź zegar lub popraw wpis w „Czas pracy”."
        }
        (TimerAction::Stop, WorkTimeError::EndNotAfterStart) => {
            "Zegar systemowy wskazuje czas sprzed początku wpisu — STOP zadziała, gdy zegar go dogoni."
        }
        (_, WorkTimeError::Store(StoreError::Busy)) => "Otwórz aplikację i spróbuj ponownie.",
        (_, WorkTimeError::Store(_)) => {
            "Dane aplikacji są niedostępne — szczegóły w panelu „Źródła”."
        }
        _ => "Otwórz aplikację i spróbuj ponownie.",
    };
    Some(Notice {
        title,
        body: body.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::{SourceId, SourceState};
    use proptest::prelude::*;

    const H: i64 = 3_600_000;
    const M: i64 = 60_000;
    const ALL: [Overall; 4] = [
        Overall::Starting,
        Overall::Ok,
        Overall::Error,
        Overall::NotRunning,
    ];

    fn known(running: bool, today_ms: i64) -> WorkLine {
        WorkLine::Known(TodayWork { running, today_ms })
    }

    fn lines() -> Vec<WorkLine> {
        vec![
            WorkLine::Loading,
            WorkLine::Unavailable,
            known(true, 3 * H),
            known(false, 3 * H),
        ]
    }

    fn failing(label: &str) -> SourceStatus {
        SourceStatus {
            id: SourceId::AppData,
            label: label.to_string(),
            state: SourceState::Error {
                kind: "Damaged".to_string(),
                message: "m".to_string(),
            },
        }
    }

    #[test]
    fn format_hm_floors_and_clamps() {
        assert_eq!(format_hm(0), "0:00");
        assert_eq!(format_hm(59_999), "0:00");
        assert_eq!(format_hm(60_000), "0:01");
        assert_eq!(format_hm(3 * H + 42 * M + 59_000), "3:42");
        assert_eq!(format_hm(25 * H), "25:00");
        assert_eq!(format_hm(-1), "0:00");
    }

    #[test]
    fn exact_tooltips() {
        assert_eq!(
            tooltip(Overall::Ok, &[], &known(true, 3 * H + 42 * M + 30_000)),
            "Nawigator dnia — OK\nCzas pracy dziś: 3:42 (trwa)"
        );
        assert_eq!(
            tooltip(Overall::Ok, &[], &known(false, 0)),
            "Nawigator dnia — OK\nCzas pracy dziś: 0:00"
        );
        assert_eq!(
            tooltip(Overall::Starting, &[], &WorkLine::Loading),
            "Nawigator dnia — uruchamianie…\nCzas pracy: wczytywanie…"
        );
        assert_eq!(
            tooltip(
                Overall::Error,
                &[failing("Dane aplikacji")],
                &WorkLine::Unavailable
            ),
            "Nawigator dnia — błąd: Dane aplikacji\nCzas pracy niedostępny"
        );
        assert_eq!(
            tooltip(Overall::NotRunning, &[], &known(true, H)),
            "Nawigator dnia — sprawdzanie nie działa\nCzas pracy niedostępny"
        );
    }

    #[test]
    fn long_label_cuts_health_part_and_keeps_work_line() {
        let work = known(true, 3 * H);
        let t = tooltip(Overall::Error, &[failing(&"Żółć".repeat(60))], &work);
        assert!(units(&t) <= 127);
        let (head, tail) = t.split_once(LINE_SEPARATOR).unwrap();
        assert!(head.ends_with('…'));
        assert_eq!(tail, work_line_text(Overall::Error, &work));
        assert!(t.ends_with("Czas pracy dziś: 3:00 (trwa)"));
    }

    fn work_strategy() -> impl Strategy<Value = WorkLine> {
        (0u8..4, any::<bool>(), 0i64..=25 * H).prop_map(|(k, running, ms)| match k {
            0 => WorkLine::Loading,
            1 => WorkLine::Unavailable,
            _ => known(running, ms),
        })
    }

    proptest! {
        #[test]
        fn tooltip_is_always_within_limit_and_ends_with_work_line(
            labels in proptest::collection::vec("[a-zA-ZąćęłńóśźżĄĆĘŁŃÓŚŹŻ 😀]{0,200}", 0..6),
            work in work_strategy(),
        ) {
            let sources: Vec<SourceStatus> = labels.iter().map(|l| failing(l)).collect();
            for o in ALL {
                let t = tooltip(o, &sources, &work);
                prop_assert!(units(&t) <= 127);
                prop_assert!(t.starts_with("Nawigator dnia"));
                prop_assert!(t.ends_with(&work_line_text(o, &work)));
            }
        }
    }

    #[test]
    fn icon_table() {
        for o in ALL {
            for w in lines() {
                let icon = tray_icon(o, &w);
                if o == Overall::Error {
                    assert_eq!(icon, TrayIconKind::Error);
                }
                let running = matches!(w, WorkLine::Known(TodayWork { running: true, .. }));
                assert_eq!(icon == TrayIconKind::Running, o == Overall::Ok && running);
                match o {
                    Overall::Starting | Overall::NotRunning => {
                        assert_eq!(icon, TrayIconKind::Idle)
                    }
                    Overall::Ok if !running => assert_eq!(icon, TrayIconKind::Ok),
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn item_table() {
        for o in ALL {
            for w in lines() {
                let item = work_item(o, &w);
                if o == Overall::NotRunning {
                    assert_eq!(item.action, None);
                    assert_eq!(item.text, "Czas pracy niedostępny");
                    continue;
                }
                match w {
                    WorkLine::Known(TodayWork { running: true, .. }) => {
                        assert_eq!((item.text, item.action), ("STOP", Some(TimerAction::Stop)))
                    }
                    WorkLine::Known(_) => {
                        assert_eq!(
                            (item.text, item.action),
                            ("START", Some(TimerAction::Start))
                        )
                    }
                    WorkLine::Loading => {
                        assert_eq!((item.text, item.action), ("Czas pracy: wczytywanie…", None))
                    }
                    WorkLine::Unavailable => {
                        assert_eq!((item.text, item.action), ("Czas pracy niedostępny", None))
                    }
                }
            }
        }
    }

    fn notice(action: TimerAction, e: WorkTimeError) -> Notice {
        failure_notice(action, &e).expect("a notice")
    }

    #[test]
    fn changed_elsewhere_is_silent() {
        for a in [TimerAction::Start, TimerAction::Stop] {
            assert!(failure_notice(a, &WorkTimeError::AlreadyRunning).is_none());
            assert!(failure_notice(a, &WorkTimeError::NotRunning).is_none());
        }
    }

    #[test]
    fn failure_texts_by_kind() {
        let n = notice(
            TimerAction::Start,
            WorkTimeError::Overlap { entry_id: 4242 },
        );
        assert_eq!(n.title, "Nawigator dnia — START nie zadziałał");
        assert_eq!(
            n.body,
            "Zegar systemowy wskazuje czas sprzed końca innego wpisu — sprawdź zegar lub popraw wpis w „Czas pracy”."
        );
        assert!(!format!("{} {}", n.title, n.body).contains("4242"));

        let n = notice(TimerAction::Stop, WorkTimeError::EndNotAfterStart);
        assert_eq!(n.title, "Nawigator dnia — STOP nie zadziałał");
        assert_eq!(
            n.body,
            "Zegar systemowy wskazuje czas sprzed początku wpisu — STOP zadziała, gdy zegar go dogoni."
        );

        let n = notice(
            TimerAction::Start,
            WorkTimeError::Store(StoreError::Io {
                kind: "x".to_string(),
            }),
        );
        assert_eq!(
            n.body,
            "Dane aplikacji są niedostępne — szczegóły w panelu „Źródła”."
        );

        for e in [
            WorkTimeError::Store(StoreError::Busy),
            WorkTimeError::InFuture,
            WorkTimeError::Background,
        ] {
            let n = notice(TimerAction::Stop, e);
            assert_eq!(n.body, "Otwórz aplikację i spróbuj ponownie.");
            assert!(n.title.starts_with("Nawigator dnia"));
        }
    }
}
