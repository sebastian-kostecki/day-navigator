# Konfigurowalna norma dzienna i pół dnia wolnego — Plan Brief

> Full plan: `context/changes/configurable-daily-norm/plan.md`
> Research: `context/changes/configurable-daily-norm/research.md`

## What & Why

Norma dzienna przestaje być stałą 8 h: użytkownik ustawia ją w `settings.json` (`workTime.dailyNormHours`, domyślnie 8 h), a w tabeli dni może oznaczyć pół dnia wolnego (połowa normy). Wszystkie liczby zależne od normy — norma miesiąca, odchylenie, „Średnio / dzień”, odchylenie w statusie, przypomnienia, kafelki pulpitu — liczą oba. Slice S-02, MS-26, issue #26.

## Starting Point

Reguła siedzi w `src-tauri/src/work_time/norm.rs` ze stałą `DAILY_NORM_MS` mnożoną przez liczbę dni roboczych; jedynym konsumentem jest `report::compute`. Dni wolne to tabela flag `days_off` (v5) bez rodzaju; przełączniki w UI są boolowskie, a „Jak liczę” ma „8 h” na sztywno. Ostatnia migracja to v11.

## Desired End State

`"dailyNormHours": 7.5` daje 7:30 na dzień roboczy we wszystkich liczbach; błędna wartość pokazuje błąd raportu, a START/STOP działa dalej. Tabela dni i panel „Wybrany dzień” mają trzystanowy przełącznik (nie / ½ / tak); pół dnia ma połowę normy, dopisek i kreskę normy w połowie paska. „Dni wolne” liczy pół dnia jako 0,5, „Średnio / dzień” podaje czas na pełny dzień. Zmiana normy przelicza wszystkie miesiące, także zamknięte.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) | Source |
| --- | --- | --- | --- |
| Historia normy | Brak: jedna wartość działa wstecz | Decyzja użytkownika; konsekwencja w README i „Jak liczę”. | Caller (user) |
| Pół dnia | Trzeci stan przełącznika, połowa normy | Decyzja użytkownika. | Caller (user) |
| Klucz i zakres | `workTime.dailyNormHours`, 1–10 h, pełne minuty, domyślnie 8 | Pasuje do skali `HoursBar` (10 h) i trzyma `Copy + Eq` (ms). | Research D1 (agent) |
| Weekend / święto | Pół dnia odrzucany jak cały dzień | Jedna reguła blokady w `set_day_off`. | Research D3 (agent) |
| Zapis | v12: `days_off.kind 'full'/'half'` | Klucz główny wyklucza oba stany naraz; stare wiersze = całe dni. | Research D4 (agent) |
| Średnio / dzień | Na pełny dzień; pół dnia = 0,5 | Bez pół dni wynik identyczny z dzisiejszym. | Plan (agent, Q1) |
| Kafelek „Dni wolne” | `daysOff` tylko całe + nowe `halfDaysOff`; wartość „2,5” | Zgodne dla S-05, czytelne w kafelku. | Plan (agent, Q2) |
| Przypomnienie startu | Ciche w pół dnia | Pracująca połowa może być popołudniem; kod już porównuje `!= Workday`. | Plan (agent, Q3) |
| „Wybrany dzień” | Ten sam trzystanowy kontroler | Jeden kontrakt `DayMark` w obu miejscach. | Plan (agent, Q4) |
| Lenient | Bez zmian | Żaden widok nie pokazuje `monthDeviationMs`; S-07 decyduje. | Plan (agent, Q6) |
| Kontroler UI | Istniejący `Segmented` + `disabled` | Trzy widoczne stany zamiast cyklicznego przycisku. | Plan (agent, D5) |
| Kontrakt komendy | `set_day_off(day, mark: DayMark)`, nazwa bez zmian | Brak zmian w `lib.rs`; `DayMark` = podzbiór `DayKind`. | Plan (agent) |
| Jedno źródło norm | `day_norm_ms` + `day_norms(zakres)` w `norm.rs` | S-08 sumuje tydzień tą samą regułą. | Plan (agent) |

## Scope

**In scope:**
- Ustawienie normy w `WorkTimeSettings` i jego walidacja.
- `figures` z sumy norm dni; pół dnia w regule, raporcie, klasyfikacji i zapisie.
- Migracja v12 z testem upgrade'u i testem awarii.
- Trzystanowy przełącznik w tabeli i w panelu, kafelki, „Jak liczę”.
- README, CHANGELOG, sformułowania PRD o 8 h.

**Out of scope:**
- Historia normy, norma per miesiąc, edytor ustawień w UI.
- Nowe FR w PRD, zmiany w `roadmap.md` i `lib.rs`.
- Pół dnia w weekend/święto, ćwiartki dnia.
- Zmiana polityki Lenient.
- Odchylenie w nagłówku: nagłówek pokazuje tylko sumę dnia; „odchylenie w nagłówku” z outcome to pole statusu `monthDeviationMs`, które liczy nową regułę (z testem), bez nowego widoku.

## Architecture / Approach

`settings.json` → `WorkTimeSettings { holidays, daily_norm_ms }` (Strict dla raportu i przełącznika, Lenient dla statusu i przypomnień) → czyste `norm.rs` (`day_norm_ms`, `day_norms`, `figures` z sumy norm dni) → `report::compute` (klasyfikacja z `DayKind::HalfDayOff`, `days_off.kind` z v12) → `MonthReport` (`dailyNormMs`, `halfDaysOff`, `halfDaysAfterToday`) → UI tylko wyświetla i wysyła `DayMark`.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Norma dzienna z settings.json | Klucz `dailyNormHours`, `figures` z norm dni, `dailyNormMs`, „Jak liczę” bez 8 h | Zmiana sygnatur `norm.rs` (S-08) |
| 2. Pół dnia w danych i regule | v12, `DayKind::HalfDayOff`, `DayMark`, pola pół dni w raporcie, test awarii | Numer migracji przy merge'u |
| 3. Trzystanowy przełącznik w UI | Komenda z `mark`, `Segmented` w tabeli i panelu, kafelki | Szerokość tabeli w oknie 800 px |
| 4. Dokumentacja | README, CHANGELOG, PRD | — |

**Prerequisites:** S-01 (kalendarz świąt) scalony — jest w `develop`.
**Estimated effort:** ~3 sesje agenta w 4 fazach plus ręczne sprawdzenie przez użytkownika.

## Open Risks & Assumptions

- v12 może kolidować z migracją innej gałęzi — gałąź scalana druga przenumerowuje.
- S-08 i S-05 łamią się na nowych sygnaturach / wariancie / polach (zamierzone, do dopasowania przy merge'u).
- Ręczne przebiegi migrują prawdziwą bazę: kopia katalogu danych przed i przywrócenie po testach.
- Wszystkie domyślne decyzje agenta (tabela wyżej) użytkownik może zmienić.

## Success Criteria (Summary)

- Norma z ustawień i pół dnia zmieniają każdą liczbę normy spójnie (proptest: wiersze dni = liczby miesiąca, plus niezależna wyrocznia normy miesiąca i warunek kontrolny „Średnio / dzień”).
- Dotychczasowe dni wolne i liczby przy 8 h zostają bez zmian po migracji.
- Użytkownik oznacza pół dnia w tabeli lub panelu i widzi to w kafelkach i na pulpicie.
