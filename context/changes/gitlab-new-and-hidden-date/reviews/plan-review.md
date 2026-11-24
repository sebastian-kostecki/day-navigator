<!-- PLAN-REVIEW-REPORT -->
# Plan Review: GitLab — flaga NOWE i data ukrycia

- **Plan**: context/changes/gitlab-new-and-hidden-date/plan.md
- **Mode**: Deep
- **Date**: 2026-11-24
- **Verdict**: REVISE → SOUND po poprawkach
- **Findings**: 0 critical, 2 warnings, 0 observations

## Verdicts

| Dimension | Verdict |
|-----------|---------|
| End-State Alignment | PASS |
| Lean Execution | PASS |
| Architectural Fitness | PASS |
| Blind Spots | WARNING |
| Plan Completeness | WARNING |

## Grounding

Ścieżki 10/10 ✓. Symbole 5/5 ✓: `record_round`, `partition`, `shape_items`, `CloseRequested`, `core:event:allow-listen`. Brief i plan są spójne ✓. Progress zgadza się z fazami: 4/4, 4/4, 10/10 ✓. Brak `context/foundation/lessons.md` i `docs/reference/contract-surfaces.md`.

## Findings

### F1 — Znacznik „teraz” gubi NOWE pozycji jeszcze nieopublikowanych

- **Severity**: ⚠️ WARNING
- **Impact**: 🔎 MEDIUM — real tradeoff; pause to reason through it
- **Dimension**: Blind Spots
- **Location**: Phase 2 §2–3 (`record_seen`, `mark_seen`, `last_mark`)
- **Detail**: `mark_seen` zapisywał znacznik równy zegarowi „teraz” (L). Runda wpisuje `first_seen_ms = t0` w `record_round` przed bramką, a snapshot publikuje po niej (`gitlab_refresh.rs:423-547`). Wyjście z widoku między t0 a wzięciem bramki zapisuje L > t0. Wtedy filtr `last_mark` odcina pozycję, której użytkownik nie widział, i przy następnej wizycie też nie jest ona NOWE. To zaprzecza obietnicy planu, że błąd ma iść w stronę „pokaż jeszcze raz”.
- **Fix**: Znacznik ma być czasem rundy, której snapshot był pokazany (`shown_until`), a nie zegarem. `mark_seen` zapisuje `max(stary, shown_until)`. Bez `shown_until` niczego nie zapisuje. `last_mark` dalej odcina spóźnioną rundę.
  - Strength: Gwarancja wynika z danych, bo to ta sama wartość, którą `record_round` wpisuje w `first_seen_ms`.
  - Tradeoff: Jedno pole więcej w stanie.
  - Confidence: HIGH, bo kolejność rundy jest w `gitlab_refresh.rs:423-547`.
  - Blind spot: Czas od publikacji do renderu w webview (ms) dalej liczy się jako „pokazane”.
- **Decision**: FIXED (Fix in plan)

### F2 — Zmiana typu `record_round` psuje kompilację już w fazie 1

- **Severity**: ⚠️ WARNING
- **Impact**: 🏃 LOW — quick decision; fix is obvious and narrowly scoped
- **Dimension**: Plan Completeness
- **Location**: Phase 1 §3 / kryterium 2.2
- **Detail**: Faza 1 zmienia typ zwracany na `RoundSeen`. Plan wspominał o wywołujących (`gitlab_refresh.rs:604`, `tests/new_assignments.rs:77-228`, `tests/new_assignments_crash.rs:104`) dopiero w fazie 2. Przez to kryterium 1.1 nie mogłoby przejść.
- **Fix**: W fazie 1 §3 wymienić wywołujących, z tymczasowym `.map(|r| r.new)` w rundzie. Kryterium 2.2 zostaje jako test regresji.
- **Decision**: FIXED (Fix in plan)
