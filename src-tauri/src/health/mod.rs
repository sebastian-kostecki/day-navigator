//! Source health model and the pure rules around it: status diff, toast text,
//! overall state, tooltip and heartbeat staleness. Nothing here imports `tauri`.

pub mod probe;

use std::time::Duration;

use serde::{Serialize, Serializer};

use crate::vault::Space;

pub use probe::{gitlab_status, probe, store_status, unresolved_config_dir};

/// Longest tooltip the Windows tray accepts, in UTF-16 units (`szTip` holds 128
/// including the terminator).
const TOOLTIP_MAX_UTF16: usize = 127;
const MAX_NOTICE_LINES: usize = 3;
pub(crate) const APP_NAME: &str = "Nawigator dnia";

/// A checked source. Serialised as its stable key (`settings`, `vault:work`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceId {
    Settings,
    Vault(Space),
    Exclusions(Space),
    /// The app-owned data store ("Dane aplikacji").
    AppData,
    /// Assigned GitLab issues and merge requests ("GitLab").
    Gitlab,
}

impl SourceId {
    pub fn key(&self) -> String {
        match self {
            SourceId::Gitlab => "gitlab".to_string(),
            SourceId::Settings => "settings".to_string(),
            SourceId::Vault(s) => format!("vault:{s}"),
            SourceId::Exclusions(s) => format!("exclusions:{s}"),
            SourceId::AppData => "app-data".to_string(),
        }
    }
}

impl std::fmt::Display for SourceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.key())
    }
}

impl Serialize for SourceId {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.key())
    }
}

/// `kind` is the English error variant name; `message` is Polish text that may
/// hold the user's own configured path, for the local panel only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum SourceState {
    Ok,
    NotConfigured,
    /// Calm "cannot reach it right now" (for example no VPN): not an error, so
    /// it never turns the tray red and never toasts.
    Unavailable {
        message: String,
    },
    Error {
        kind: String,
        message: String,
    },
}

impl SourceState {
    fn error_kind(&self) -> Option<&str> {
        match self {
            SourceState::Error { kind, .. } => Some(kind),
            _ => None,
        }
    }

    fn is_unavailable(&self) -> bool {
        matches!(self, SourceState::Unavailable { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceStatus {
    pub id: SourceId,
    pub label: String,
    #[serde(flatten)]
    pub state: SourceState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Overall {
    Starting,
    Ok,
    Error,
    NotRunning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthSnapshot {
    pub overall: Overall,
    pub sources: Vec<SourceStatus>,
    /// Wall-clock time of the last finished check, for display only.
    pub checked_at_ms: Option<u64>,
    /// Bumped on every change to the stored snapshot. Consumers order
    /// snapshots by this, never by `checked_at_ms`, which jumps when the
    /// clock is changed.
    pub seq: u64,
}

/// A change worth telling the user about. Carries no message text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transition {
    pub id: SourceId,
    pub label: String,
    pub from_kind: Option<String>,
    pub to_kind: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub title: String,
    pub body: String,
}

/// Transitions from `prev` to `next`: entering an error, changing error kind,
/// or recovering (`Error` to `Ok`). A source missing from `prev` is Unknown.
/// A source absent from `next`, and `Error` to `NotConfigured` or `Unavailable`,
/// are silent; so is entering `Unavailable` from anywhere.
pub fn transitions(prev: &[SourceStatus], next: &[SourceStatus]) -> Vec<Transition> {
    let mut out = Vec::new();
    for n in next {
        let before = prev.iter().find(|p| p.id == n.id).map(|p| &p.state);
        let prev_kind = before.and_then(SourceState::error_kind);
        let (to_kind, changed) = match &n.state {
            SourceState::Error { kind, .. } => (Some(kind.clone()), prev_kind != Some(kind)),
            SourceState::Ok => (None, prev_kind.is_some()),
            SourceState::NotConfigured | SourceState::Unavailable { .. } => (None, false),
        };
        if changed {
            out.push(Transition {
                id: n.id,
                label: n.label.clone(),
                from_kind: prev_kind.map(str::to_string),
                to_kind,
            });
        }
    }
    out
}

/// The diff baseline for the next round: `next` plus every earlier entry whose
/// id is absent from `next`, so a source hidden by a settings outage is
/// compared against its last known status when it comes back.
pub fn baseline(prev_baseline: &[SourceStatus], next: &[SourceStatus]) -> Vec<SourceStatus> {
    let mut out = next.to_vec();
    out.extend(
        prev_baseline
            .iter()
            .filter(|p| !next.iter().any(|n| n.id == p.id))
            .cloned(),
    );
    out
}

/// Fixed Polish phrase for an error kind. Never the status message, which can
/// hold paths or exclusion entries.
fn short_reason(kind: &str) -> &'static str {
    match kind {
        "RootMissing" => "folder niedostępny",
        "RootsOverlap" => "foldery vaultów nakładają się",
        "SettingsInvalid" => "nieprawidłowe ustawienia",
        "NotConfigured" => "brak pliku ustawień",
        "NoSpaceConfigured" => "nie skonfigurowano vaultu",
        "UnmatchedExclusions" => "wykluczenia nic nie chronią",
        "Io" => "błąd odczytu",
        "Damaged" => "plik danych uszkodzony",
        "Missing" => "brak pliku danych",
        "NewerSchema" => "dane z nowszej wersji aplikacji",
        "MigrationFailed" => "aktualizacja danych nie powiodła się",
        "DirUnavailable" => "folder danych niedostępny",
        "Busy" => "plik danych zajęty",
        "LoggedOut" => "wylogowano",
        "ToolMissing" => "brak programu",
        "Failed" => "błąd wywołania",
        "ResponseInvalid" => "nieczytelna odpowiedź",
        "ConfigDirUnresolved" => "brak folderu konfiguracji",
        "ThreadDown" => "wątek odświeżania nie działa",
        _ => "błąd źródła",
    }
}

/// One toast for a batch of transitions, or `None` when there are none.
pub fn notice_for(transitions: &[Transition]) -> Option<Notice> {
    if transitions.is_empty() {
        return None;
    }
    let has_error = transitions.iter().any(|t| t.to_kind.is_some());
    let title = if has_error {
        format!("{APP_NAME} — błąd źródła")
    } else {
        format!("{APP_NAME} — źródła działają")
    };
    let mut lines: Vec<String> = transitions
        .iter()
        .take(MAX_NOTICE_LINES)
        .map(|t| match &t.to_kind {
            Some(kind) => format!("{}: {}", t.label, short_reason(kind)),
            None => format!("{}: działa ponownie", t.label),
        })
        .collect();
    if transitions.len() > MAX_NOTICE_LINES {
        lines.push(format!("i {} więcej", transitions.len() - MAX_NOTICE_LINES));
    }
    Some(Notice {
        title,
        body: lines.join("\n"),
    })
}

pub fn overall(sources: &[SourceStatus], heartbeat_stale: bool, started: bool) -> Overall {
    if heartbeat_stale {
        Overall::NotRunning
    } else if !started {
        Overall::Starting
    } else if sources.iter().any(|s| s.state.error_kind().is_some()) {
        Overall::Error
    } else {
        Overall::Ok
    }
}

/// Tray tooltip, at most 127 UTF-16 units, never cutting a character in half.
pub fn tooltip(overall: Overall, sources: &[SourceStatus]) -> String {
    let text = match overall {
        Overall::Starting => format!("{APP_NAME} — uruchamianie…"),
        Overall::Ok => {
            let down: Vec<&str> = sources
                .iter()
                .filter(|s| s.state.is_unavailable())
                .map(|s| s.label.as_str())
                .collect();
            if down.is_empty() {
                format!("{APP_NAME} — OK")
            } else {
                format!("{APP_NAME} — OK ({} niedostępny)", down.join(", "))
            }
        }
        Overall::NotRunning => format!("{APP_NAME} — sprawdzanie nie działa"),
        Overall::Error => {
            let failing: Vec<&SourceStatus> = sources
                .iter()
                .filter(|s| s.state.error_kind().is_some())
                .collect();
            match failing.split_first() {
                None => format!("{APP_NAME} — błąd"),
                Some((first, [])) => format!("{APP_NAME} — błąd: {}", first.label),
                Some((first, rest)) => {
                    format!("{APP_NAME} — błąd: {} i {} więcej", first.label, rest.len())
                }
            }
        }
    };
    cap_utf16(text, TOOLTIP_MAX_UTF16)
}

fn cap_utf16(text: String, max: usize) -> String {
    if text.encode_utf16().count() <= max {
        return text;
    }
    let mut out = String::new();
    let mut units = 0;
    for c in text.chars() {
        // Reserve one unit for the ellipsis.
        if units + c.len_utf16() > max - 1 {
            break;
        }
        units += c.len_utf16();
        out.push(c);
    }
    out.push('…');
    out
}

/// True when the watchdog's own gap since its previous wake-up exceeds
/// `resume_gap`: the machine slept (or the watchdog was starved), so an old
/// heartbeat proves nothing. The runtime uses two watchdog intervals (30 s).
/// Without a suspend the heartbeat is then at most about 60 s old plus the probe
/// time, so even a short suspend cannot push it past the stall threshold unseen.
pub fn is_resume(watchdog_gap: Duration, resume_gap: Duration) -> bool {
    watchdog_gap > resume_gap
}

/// True when the check loop has stalled: the last heartbeat is older than
/// `threshold` while the watchdog itself kept running (see [`is_resume`]).
pub fn is_stale(
    heartbeat_age: Duration,
    watchdog_gap: Duration,
    threshold: Duration,
    resume_gap: Duration,
) -> bool {
    heartbeat_age > threshold && !is_resume(watchdog_gap, resume_gap)
}

/// What one watchdog tick does with the stall flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchdogAction {
    /// The loop just stalled: go grey, log and toast once.
    Stall,
    /// A stalled loop stamped a fresh heartbeat again.
    Clear,
    Nothing,
}

/// One watchdog tick. `since_resume` is the time since the last detected
/// resume, `Some(0)` on the resume tick itself. Setting a stall uses the
/// resume-adjusted [`stall_age`]; clearing one needs a really fresh heartbeat.
pub fn watchdog_step(
    was_stale: bool,
    heartbeat_age: Duration,
    watchdog_gap: Duration,
    since_resume: Option<Duration>,
    threshold: Duration,
    resume_gap: Duration,
) -> WatchdogAction {
    if was_stale {
        if heartbeat_age <= threshold {
            WatchdogAction::Clear
        } else {
            WatchdogAction::Nothing
        }
    } else if is_stale(
        stall_age(heartbeat_age, since_resume),
        watchdog_gap,
        threshold,
        resume_gap,
    ) {
        WatchdogAction::Stall
    } else {
        WatchdogAction::Nothing
    }
}

/// Heartbeat age as the stall rule sees it: time since the later of the last
/// heartbeat and the last resume from sleep. After a resume the check thread
/// gets a full threshold to finish a round, whether or not the OS counted the
/// sleep in its timers.
pub fn stall_age(heartbeat_age: Duration, since_resume: Option<Duration>) -> Duration {
    match since_resume {
        Some(r) => heartbeat_age.min(r),
        None => heartbeat_age,
    }
}

/// Gate for toasts. Always `true` until the pause and day-off modes (FR-023,
/// FR-024) plug in here. The tray is never gated, and suppressed toasts are not
/// replayed afterwards.
pub fn notices_allowed() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn err(id: SourceId, label: &str, kind: &str, message: &str) -> SourceStatus {
        SourceStatus {
            id,
            label: label.to_string(),
            state: SourceState::Error {
                kind: kind.to_string(),
                message: message.to_string(),
            },
        }
    }

    fn st(id: SourceId, label: &str, state: SourceState) -> SourceStatus {
        SourceStatus {
            id,
            label: label.to_string(),
            state,
        }
    }

    const W: SourceId = SourceId::Vault(Space::Work);
    const P: SourceId = SourceId::Vault(Space::Private);

    #[test]
    fn source_keys_are_stable() {
        assert_eq!(SourceId::Settings.key(), "settings");
        assert_eq!(W.key(), "vault:work");
        assert_eq!(
            SourceId::Exclusions(Space::Private).key(),
            "exclusions:private"
        );
    }

    #[test]
    fn app_data_source_key_and_serialisation() {
        assert_eq!(SourceId::AppData.key(), "app-data");
        assert_eq!(
            serde_json::to_string(&SourceId::AppData).unwrap(),
            "\"app-data\""
        );
    }

    #[test]
    fn short_reason_covers_store_kinds() {
        assert_eq!(short_reason("Damaged"), "plik danych uszkodzony");
        assert_eq!(short_reason("Missing"), "brak pliku danych");
        assert_eq!(
            short_reason("NewerSchema"),
            "dane z nowszej wersji aplikacji"
        );
        assert_eq!(
            short_reason("MigrationFailed"),
            "aktualizacja danych nie powiodła się"
        );
        assert_eq!(short_reason("DirUnavailable"), "folder danych niedostępny");
        assert_eq!(short_reason("Busy"), "plik danych zajęty");
        assert_eq!(short_reason("Io"), "błąd odczytu");
    }

    #[test]
    fn snapshot_serialises_camel_case() {
        let snap = HealthSnapshot {
            overall: Overall::NotRunning,
            sources: vec![err(W, "Vault Praca", "RootMissing", "x")],
            checked_at_ms: Some(5),
            seq: 7,
        };
        let v = serde_json::to_value(&snap).unwrap();
        assert_eq!(v["overall"], "notRunning");
        assert_eq!(v["checkedAtMs"], 5);
        assert_eq!(v["seq"], 7);
        assert_eq!(v["sources"][0]["id"], "vault:work");
        assert_eq!(v["sources"][0]["state"], "error");
        assert_eq!(v["sources"][0]["kind"], "RootMissing");
    }

    #[test]
    fn first_check_notifies_only_errors() {
        let next = vec![
            st(SourceId::Settings, "Ustawienia", SourceState::Ok),
            st(P, "Vault Prywatne", SourceState::NotConfigured),
            err(W, "Vault Praca", "RootMissing", "m"),
        ];
        let t = transitions(&[], &next);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].id, W);
        assert_eq!(t[0].from_kind, None);
        assert_eq!(t[0].to_kind.as_deref(), Some("RootMissing"));
    }

    #[test]
    fn entering_error_changing_kind_and_recovery_are_transitions() {
        let ok = vec![st(W, "Vault Praca", SourceState::Ok)];
        let e1 = vec![err(W, "Vault Praca", "RootMissing", "a")];
        let e2 = vec![err(W, "Vault Praca", "RootsOverlap", "a")];
        assert_eq!(transitions(&ok, &e1).len(), 1);
        let kind_change = transitions(&e1, &e2);
        assert_eq!(kind_change.len(), 1);
        assert_eq!(kind_change[0].from_kind.as_deref(), Some("RootMissing"));
        assert_eq!(kind_change[0].to_kind.as_deref(), Some("RootsOverlap"));
        let rec = transitions(&e2, &ok);
        assert_eq!(rec.len(), 1);
        assert_eq!(rec[0].to_kind, None);
        assert!(transitions(&ok, &ok).is_empty());
    }

    #[test]
    fn same_kind_with_new_message_is_not_a_transition() {
        let a = vec![err(W, "Vault Praca", "RootMissing", "first")];
        let b = vec![err(W, "Vault Praca", "RootMissing", "second")];
        assert!(transitions(&a, &b).is_empty());
    }

    #[test]
    fn error_to_not_configured_is_silent() {
        let a = vec![err(W, "Vault Praca", "RootMissing", "m")];
        let b = vec![st(W, "Vault Praca", SourceState::NotConfigured)];
        assert!(transitions(&a, &b).is_empty());
    }

    #[test]
    fn absent_source_is_silent_and_baseline_prevents_repeat_toast() {
        let broken = err(W, "Vault Praca", "RootMissing", "m");
        let settings_ok = st(SourceId::Settings, "Ustawienia", SourceState::Ok);
        let base0 = baseline(&[], &[settings_ok.clone(), broken.clone()]);

        // Settings outage: the vault source vanishes from `next`.
        let outage = vec![err(
            SourceId::Settings,
            "Ustawienia",
            "SettingsInvalid",
            "r",
        )];
        let t = transitions(&base0, &outage);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].id, SourceId::Settings);
        let base1 = baseline(&base0, &outage);
        assert!(base1.iter().any(|s| s.id == W));

        // Settings come back and the vault is still broken with the same kind.
        let back = vec![settings_ok, broken];
        let t = transitions(&base1, &back);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].id, SourceId::Settings);
        assert_eq!(t[0].to_kind, None);
        assert!(t.iter().all(|x| x.id != W));
    }

    fn tr(label: &str, to: Option<&str>) -> Transition {
        Transition {
            id: SourceId::Settings,
            label: label.to_string(),
            from_kind: None,
            to_kind: to.map(str::to_string),
        }
    }

    #[test]
    fn notice_none_without_transitions() {
        assert!(notice_for(&[]).is_none());
    }

    #[test]
    fn notice_error_title_wins_over_recovery() {
        let n = notice_for(&[
            tr("Vault Praca", None),
            tr("Ustawienia", Some("SettingsInvalid")),
        ])
        .unwrap();
        assert_eq!(n.title, "Nawigator dnia — błąd źródła");
        assert_eq!(
            n.body,
            "Vault Praca: działa ponownie\nUstawienia: nieprawidłowe ustawienia"
        );
    }

    #[test]
    fn notice_recovery_only_title() {
        let n = notice_for(&[tr("Vault Praca", None)]).unwrap();
        assert_eq!(n.title, "Nawigator dnia — źródła działają");
    }

    #[test]
    fn notice_caps_lines_at_three() {
        let ts: Vec<Transition> = (0..5)
            .map(|i| tr(&format!("Z{i}"), Some("RootMissing")))
            .collect();
        let n = notice_for(&ts).unwrap();
        let lines: Vec<&str> = n.body.lines().collect();
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[3], "i 2 więcej");
    }

    #[test]
    fn notice_never_carries_message_text() {
        let prev = vec![];
        let reason = "work: invalid excluded entry: Klient-Testowy/hasla.md";
        let next = vec![
            err(
                SourceId::Settings,
                "Ustawienia",
                "SettingsInvalid",
                &format!("Ustawienia są nieprawidłowe: {reason}"),
            ),
            err(
                W,
                "Vault Praca",
                "RootMissing",
                r"Folder vaultu nie istnieje lub jest niedostępny: C:\Klient-Testowy\vault",
            ),
            err(
                SourceId::Exclusions(Space::Work),
                "Wykluczenia Praca",
                "UnmatchedExclusions",
                "Klient-Testowy/hasla.md",
            ),
        ];
        let n = notice_for(&transitions(&prev, &next)).unwrap();
        let all = format!("{}\n{}", n.title, n.body);
        for bad in ["Klient-Testowy", "hasla", r"C:\", "excluded entry"] {
            assert!(!all.contains(bad), "{bad} leaked: {all}");
        }
    }

    #[test]
    fn overall_rules() {
        let ok = vec![
            st(W, "V", SourceState::Ok),
            st(P, "V", SourceState::NotConfigured),
        ];
        let bad = vec![err(W, "V", "RootMissing", "m")];
        assert_eq!(overall(&ok, false, false), Overall::Starting);
        assert_eq!(overall(&ok, false, true), Overall::Ok);
        assert_eq!(overall(&bad, false, true), Overall::Error);
        assert_eq!(overall(&ok, true, true), Overall::NotRunning);
        assert_eq!(overall(&bad, true, true), Overall::NotRunning);
    }

    #[test]
    fn tooltip_examples() {
        let one = vec![err(W, "Vault Praca", "RootMissing", "m")];
        let two = vec![
            err(W, "Vault Praca", "RootMissing", "m"),
            err(P, "Vault Prywatne", "RootMissing", "m"),
        ];
        assert_eq!(tooltip(Overall::Ok, &[]), "Nawigator dnia — OK");
        assert_eq!(
            tooltip(Overall::Error, &one),
            "Nawigator dnia — błąd: Vault Praca"
        );
        assert_eq!(
            tooltip(Overall::Error, &two),
            "Nawigator dnia — błąd: Vault Praca i 1 więcej"
        );
        assert_eq!(
            tooltip(Overall::NotRunning, &[]),
            "Nawigator dnia — sprawdzanie nie działa"
        );
        assert_eq!(
            tooltip(Overall::Starting, &[]),
            "Nawigator dnia — uruchamianie…"
        );
    }

    #[test]
    fn tooltip_truncates_long_labels_with_ellipsis() {
        let long = "Żółć".repeat(60);
        let s = vec![err(W, &long, "RootMissing", "m")];
        let t = tooltip(Overall::Error, &s);
        assert!(t.encode_utf16().count() <= 127);
        assert!(t.ends_with('…'));
    }

    proptest! {
        #[test]
        fn tooltip_is_always_within_limit(
            labels in proptest::collection::vec("[a-zA-ZąćęłńóśźżĄĆĘŁŃÓŚŹŻ 😀]{0,200}", 0..6),
        ) {
            let sources: Vec<SourceStatus> = labels
                .iter()
                .map(|l| err(W, l, "RootMissing", "m"))
                .collect();
            for o in [Overall::Starting, Overall::Ok, Overall::Error, Overall::NotRunning] {
                let t = tooltip(o, &sources);
                prop_assert!(t.encode_utf16().count() <= 127);
                prop_assert!(t.starts_with("Nawigator dnia"));
            }
        }
    }

    const TH: Duration = Duration::from_secs(75);
    const RESUME: Duration = Duration::from_secs(30);
    const NORMAL: Duration = Duration::from_secs(15);

    fn secs(s: u64) -> Duration {
        Duration::from_secs(s)
    }

    #[test]
    fn staleness_rules() {
        assert!(!is_stale(secs(30), NORMAL, TH, RESUME));
        assert!(is_stale(secs(90), NORMAL, TH, RESUME));
        assert!(!is_stale(secs(3600), secs(3600), TH, RESUME));
        // A short suspend: the gap is below the threshold but above two intervals.
        assert!(!is_stale(secs(77), secs(65), TH, RESUME));
        assert!(is_resume(secs(31), RESUME));
        assert!(!is_resume(secs(30), RESUME));
    }

    #[test]
    fn second_tick_after_resume_is_not_stale() {
        // The heartbeat age still includes an hour of sleep; the tick after the
        // resume tick has a normal gap.
        let age = stall_age(secs(3615), Some(secs(15)));
        assert!(!is_stale(age, NORMAL, TH, RESUME));
        // A loop that stays dead after the resume is reported once the grace runs out.
        let age = stall_age(secs(3690), Some(secs(90)));
        assert!(is_stale(age, NORMAL, TH, RESUME));
        // Without a resume the heartbeat age is used as is.
        assert_eq!(stall_age(secs(90), None), secs(90));
    }

    #[test]
    fn watchdog_step_table() {
        use WatchdogAction::*;
        // (was_stale, heartbeat age, watchdog gap, since resume) -> action
        let cases = [
            // Healthy loop.
            (false, secs(20), NORMAL, None, Nothing),
            // Real stall with the watchdog ticking normally.
            (false, secs(90), NORMAL, None, Stall),
            // A 52 s suspend: gap 65 s, heartbeat 77 s; the resume tick itself.
            (false, secs(77), secs(65), Some(secs(0)), Nothing),
            // The tick after that short suspend, check thread still probing.
            (false, secs(92), NORMAL, Some(secs(15)), Nothing),
            // An hour of sleep, resume tick and the tick after it.
            (false, secs(3600), secs(3600), Some(secs(0)), Nothing),
            (false, secs(3615), NORMAL, Some(secs(15)), Nothing),
            // The loop stays dead after a resume: reported once the grace runs out.
            (false, secs(3690), NORMAL, Some(secs(90)), Stall),
            // Already stalled: no second stall while the heartbeat stays old.
            (true, secs(120), NORMAL, None, Nothing),
            // A resume does not clear a stall; only a fresh heartbeat does.
            (true, secs(120), NORMAL, Some(secs(0)), Nothing),
            (true, secs(5), NORMAL, None, Clear),
        ];
        for (was_stale, age, gap, since_resume, want) in cases {
            assert_eq!(
                watchdog_step(was_stale, age, gap, since_resume, TH, RESUME),
                want,
                "was_stale={was_stale} age={age:?} gap={gap:?} since_resume={since_resume:?}"
            );
        }
    }

    const G: SourceId = SourceId::Gitlab;

    fn unavailable(id: SourceId, label: &str) -> SourceStatus {
        st(
            id,
            label,
            SourceState::Unavailable {
                message: "m".to_string(),
            },
        )
    }

    #[test]
    fn gitlab_source_key_and_short_reasons() {
        assert_eq!(G.key(), "gitlab");
        assert_eq!(short_reason("LoggedOut"), "wylogowano");
        assert_eq!(short_reason("ToolMissing"), "brak programu");
        assert_eq!(short_reason("Failed"), "błąd wywołania");
        assert_eq!(short_reason("ResponseInvalid"), "nieczytelna odpowiedź");
        assert_eq!(
            short_reason("ConfigDirUnresolved"),
            "brak folderu konfiguracji"
        );
        assert_eq!(short_reason("ThreadDown"), "wątek odświeżania nie działa");
    }

    #[test]
    fn unavailable_serialises_calmly() {
        let s = unavailable(G, "GitLab");
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["id"], "gitlab");
        assert_eq!(v["state"], "unavailable");
        assert_eq!(v["message"], "m");
    }

    #[test]
    fn unavailable_never_makes_overall_an_error() {
        let src = vec![st(W, "V", SourceState::Ok), unavailable(G, "GitLab")];
        assert_eq!(overall(&src, false, true), Overall::Ok);
        assert_eq!(overall(&src, false, false), Overall::Starting);
        assert_eq!(overall(&src, true, true), Overall::NotRunning);
    }

    #[test]
    fn unavailable_is_silent_in_both_directions() {
        let ok = vec![st(G, "GitLab", SourceState::Ok)];
        let down = vec![unavailable(G, "GitLab")];
        let bad = vec![err(G, "GitLab", "LoggedOut", "m")];
        assert!(transitions(&[], &down).is_empty());
        assert!(transitions(&ok, &down).is_empty());
        assert!(transitions(&down, &ok).is_empty());
        assert!(transitions(&bad, &down).is_empty());
        // Leaving Unavailable into an error is a normal error transition.
        assert_eq!(transitions(&down, &bad).len(), 1);
    }

    #[test]
    fn logged_out_recovery_is_a_transition() {
        let ok = vec![st(G, "GitLab", SourceState::Ok)];
        let bad = vec![err(G, "GitLab", "LoggedOut", "m")];
        let entered = transitions(&ok, &bad);
        assert_eq!(entered.len(), 1);
        let n = notice_for(&entered).unwrap();
        assert_eq!(n.body, "GitLab: wylogowano");
        let rec = transitions(&bad, &ok);
        assert_eq!(rec.len(), 1);
        assert_eq!(rec[0].to_kind, None);
    }

    #[test]
    fn tooltip_notes_unavailable_sources_when_ok() {
        let one = vec![st(W, "V", SourceState::Ok), unavailable(G, "GitLab")];
        assert_eq!(
            tooltip(Overall::Ok, &one),
            "Nawigator dnia — OK (GitLab niedostępny)"
        );
        let two = vec![unavailable(G, "GitLab"), unavailable(P, "Vault Prywatne")];
        assert_eq!(
            tooltip(Overall::Ok, &two),
            "Nawigator dnia — OK (GitLab, Vault Prywatne niedostępny)"
        );
        // Other states are unchanged.
        assert_eq!(
            tooltip(Overall::NotRunning, &one),
            "Nawigator dnia — sprawdzanie nie działa"
        );
        let long = "Żółć".repeat(60);
        let t = tooltip(Overall::Ok, &[unavailable(G, &long)]);
        assert!(t.encode_utf16().count() <= 127);
    }

    #[test]
    fn notices_gate_is_open() {
        assert!(notices_allowed());
    }
}
