# Notatka przy wpisie czasu pracy — Plan Brief

> Full plan: `context/changes/work-entry-note/plan.md`
> Research: `context/changes/work-entry-note/research.md`

## What & Why

Użytkownik chce zapisać przy wpisie czasu pracy krótką notatkę (np. czego dotyczył blok pracy), ustawić ją przy dopisywaniu i przy edycji wpisu i widzieć ją na liście wpisów dnia. Slice S-06 (M-3), MS-27, issue #27. Notatka to treść wpisana przez użytkownika, więc nie może wyciec do logu aplikacji, dziennika zdarzeń ani treści błędów.

## Starting Point

Wpis (`work_entries`, migracja v1) ma tylko instanty; ostatnia migracja to v10. `add`/`update` w `work_time` to po jednym `store.write`, a wspólny `Entry` czytają lista dnia i raport miesiąca. Wzór notatki i testów wycieku jest w zadaniach własnych (`own_tasks.rs`).

## Desired End State

Formularze dopisywania i edycji mają jednoliniowe pole „Notatka (opcjonalnie)” z licznikiem `N/200`. Lista pokazuje niepustą notatkę pod godzinami, uciętą wielokropkiem. Za długa albo wieloliniowa notatka jest odrzucana przez Rust z polskim komunikatem. Stara baza migruje się do v11 bez utraty danych. AGENTS.md i CHANGELOG są zaktualizowane.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) | Source |
| --- | --- | --- | --- |
| Gdzie żyje notatka | Kolumna `work_entries.note TEXT NOT NULL DEFAULT ''` (v11) w `Entry` i `EntryRow` | Raport miesiąca czyta ten sam `Entry`, więc S-05 skopiuje notatkę bez drugiego zapytania | Research / user |
| Reguła | Przycięcie, jedna linia (`\r`/`\n` → `NoteMultiline`), ≤ 200 znaków w `chars()`, pusta = `""` | Zgodne z tytułem zadania własnego; Rust jest jedynym limitem, CHECK w SQL drugą linią | User (agent default) |
| Kontrakt | Wymagany `note: String` w `add_work_entry`/`update_work_entry`, edycja zastępuje | Jedna mutacja = jeden `write`; bez osobnej komendy | Research |
| Kolejność walidacji | Notatka przed czasami | Czysta funkcja przed `store.write`, test to utrwala | Plan (agent default) |
| Gdzie reguła | Nowy `work_time/note.rs` | Mniej hunków w plikach współdzielonych z S-01/S-03 | Plan (agent default) |
| Styl notatki | Klasa `.body` z `tokens.css`, kolor `var(--ink-faint)` w `WorkTime.css`, bez literalnych kolorów | Precedens `.wt-duration` i `.own-row-note` | Plan (agent default) |
| AGENTS.md:11 | Dopisać notatkę do „work-time entries”; zdanie „Never user content…” zastąpić „tylko wymienione dane, bez vaulta i treści źródeł; notatki użytkownika nigdy w logu ani dzienniku zdarzeń” | Usuwa sprzeczność z notatkami, które już są w app data | User + plan (agent default) |
| Test awarii | Nowy `tests/work_entry_note_crash.rs` | Brak konfliktu z równoległymi slice'ami | Plan (agent default) |
| PRD / roadmapa | Bez nowych FR, bez edycji roadmapy | Decyzja użytkownika i zakaz w zadaniu | User |

## Scope

**In scope:** migracja v11; reguła i błędy `NoteTooLong`/`NoteMultiline`; parametr `note` w domenie i komendach; lustro TS; pole w formularzu i wyświetlanie na liście; testy jednostkowe, migracji, wycieku, dziennika zdarzeń i awarii; AGENTS.md; CHANGELOG `[Unreleased]`.

**Out of scope:** notatka w nagłówku, przypomnieniach, zasobniku, dzienniku zdarzeń i raporcie miesiąca (S-05); notatki wieloliniowe; `maxLength` w UI; zmiany w `lib.rs`, `WorkTimeView.tsx`, README, roadmapie i PRD.

## Architecture / Approach

Ścieżka odczytu: SQLite `note` → `row_to_entry` (indeks 3 w trzech SELECT-ach) → `Entry.note` → `EntryRow.note` → TS `EntryRow.note` → `EntryItem`. Ścieżka zapisu: formularz → `useWorkEntries` → `api.ts` (`invoke` z `note`) → komenda → `normalize_note` → `INSERT`/`UPDATE` z `?N` w jednym `write`. Logi zostają przy `e.kind()`.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Kolumna i odczyt | v11, `note` w typach Rust i TS, wyświetlanie na liście | `ADD COLUMN … CHECK` na istniejących wierszach (rozstrzyga test upgrade'u) |
| 2. Zapis | Reguła, błędy, kontrakt komend + API + formularz w jednym commicie | ~20 wywołań testowych do dostosowania; rozjazd kontraktu Rust↔TS |
| 3. Awaria i dokumentacja | `work_entry_note_crash.rs`, AGENTS.md, CHANGELOG | Konflikt linii w `CHANGELOG [Unreleased]` przy merge'u |

**Prerequisites:** gałąź z `develop` (research na `455cbed`); `node_modules` zainstalowane; brak innych zależności.
**Estimated effort:** ~3 sesje sub-agenta, po jednym commicie na fazę.

## Open Risks & Assumptions

- Zakładamy, że S-06 jako jedyny równoległy slice dodaje migrację; w razie kolizji bierze kolejny numer (precedens przenumerowania).
- Wersja SQLite z `bundled` nie została odczytana; test upgrade'u v10→v11 z wierszami rozstrzyga zachowanie CHECK.
- Brak runnera testów UI: formularz i lista weryfikowane ręcznie na maszynie użytkownika.
- Wszystkie wartości domyślne agenta (limit, kolejność walidacji, styl, brzmienie AGENTS.md) użytkownik może zmienić.

## Success Criteria (Summary)

- Użytkownik dodaje, poprawia i czyści notatkę wpisu i widzi ją na liście dnia; po restarcie zostaje.
- Za długa lub wieloliniowa notatka jest odrzucana komunikatem, nic się nie zapisuje.
- Testy dowodzą: migracja bez utraty danych, brak notatki w błędach, logu i dzienniku zdarzeń, spójność po zabiciu procesu.
