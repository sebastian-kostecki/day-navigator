# Wspólne narzędzia UI — Plan Brief

> Full plan: `context/changes/shared-ui-tools/plan.md`

## What & Why

Przy dopasowaniu widoków do makiet Konsoli osiem narzędzi UI powstało lokalnie i część z nich jest skopiowana (zegar „N min temu” cztery razy, polska odmiana trzy razy). Przenosimy je do `src/components/`, żeby kolejne wycinki M-2 — pasek filtrów GitLaba (S-03) i tabela dni w „Czasie pracy” (S-05) — budowały na wspólnych elementach zamiast dokładać następne kopie. To F-01 z roadmapy (issue #10).

## Starting Point

Zegary w GitLabie, GitHubie, „Stanie źródeł”, panelu źródeł i „Czasie pracy”; `plural` w GitLabie, GitHubie i `todo/summary.ts`; zakładki i chip priorytetu tylko w „Moich zadaniach”; struktura i typografia tabeli w `GitlabView.css`; `Segmented` nadpisywany z zewnątrz do 40 px; `ErrorState.message` jako `string`; „Sprawdź teraz” klikalne w trakcie odświeżania.

## Desired End State

`src/components/` daje `useNow`, `plural`, `Tabs`, `Chip`, szkielet `DataTable` i `Segmented size`, a `ErrorState`/`SourceState` przyjmują węzeł w treści i blokadę akcji. Widoki z nich korzystają i wyglądają jak dotąd — poza dwiema uzgodnionymi zmianami: `gh auth login` jako `<code>` w błędzie GitHuba i zablokowane „Sprawdź teraz”, gdy GitLab/GitHub się odświeża.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) |
| --- | --- | --- |
| Kształt tabeli | Szkielet (`DataTable`, `DataTableRow`, `DataTableCell`) + szablon kolumn i boczny odstęp w CSS widoku (`--dn-table-cols`, `--dn-table-pad-x`) | Responsywne ukrywanie kolumn i typografia kolumn GitLaba (11 px etykiet, tytuł 14/20) zostają u niego, a S-05 użyje tego samego szkieletu bez zgadywania API kolumn. |
| Chip priorytetu | Nowy komponent `Chip` (tony `warn`/`soft`/`faint`) | `Flag` zostaje sygnałem zmiany (mono, wielkie litery), a priorytet zachowuje dzisiejszy wygląd. |
| Zakres `useNow` | `useNow(intervalMs \| null)` dla czterech zegarów 30 s i warunkowego tyku 1 s w `WorkTimeView` | Żaden widok nie trzyma własnego `setInterval` dla zegara; `useWorkTimer` zostaje dla F-02. |
| Widoczne zmiany | Obie z issue: `<code>gh auth login</code>` i `actionDisabled` na „Sprawdź teraz” | Nowe API od razu ma użytkownika, zgodnie z zamiarem zgłoszenia. |
| Rozmiar `Segmented` | `size?: "sm" \| "md"` jak w `Button` (36 / 40 px) | Ta sama konwencja rozmiarów w całym zestawie komponentów. |

## Scope

**In scope:**
- `useNow` + `RELATIVE_TICK_MS`, `plural` w `src/components/`; przepięcie GitLaba, GitHuba, „Stanu źródeł”, `SourcesPanel`, `TodoView`, `WorkTimeView`
- `Tabs` (z `OwnTaskTabs`), `Chip`, `Segmented size`, `DataTable`; przepięcie „Moich zadań” i tabeli GitLaba
- `ErrorState.message: ReactNode`, `SourceState.actionDisabled`; dwie uzgodnione zmiany w GitHubie i GitLabie

**Out of scope:**
- `useWorkTimer` i subskrypcje licznika (F-02)
- Tabela sterowana danymi, nowe tony `Flag`, pasek filtrów (S-03), tabela dni (S-05)
- Artefakt design systemu (F-05), Rust i kontrakty TS↔Rust, runner testów UI

## Architecture / Approach

Każdy element trafia do `src/components/` w konwencji folderu komponentu (`Name/Name.tsx` + `Name.css` z prefiksem `dn-`, wartości tylko z tokenów) i jest eksportowany z `index.ts`. Style przenoszone z CSS widoków zachowują identyczne wartości, więc wygląd się nie zmienia; CSS widoków zostaje wyłącznie z układem (w GitLabie: szablon kolumn i odstęp per próg 900/600 px oraz typografia jego kolumn). CSS komponentów ładuje się przed CSS widoków, więc nadpisania w widoku wygrywają kolejnością.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Zegar i odmiana | `useNow`, `plural`; widoki bez własnych zegarów i kopii | Tyk „Czasu pracy” po STOP lub przy dniu przeszłym zachowuje się inaczej niż dziś |
| 2. Komponenty prezentacyjne | `Tabs`, `Chip`, `Segmented size`, `DataTable`; „Moje zadania” i GitLab przepięte | Drobna różnica wyglądu tabeli GitLaba przy 900/600 px |
| 3. Stany źródeł | `message: ReactNode`, `actionDisabled`; `<code>` i blokada „Sprawdź teraz” | Przycisk nie odblokowuje się, jeśli `refreshing` utknie |

**Prerequisites:** brak — F-01 nie ma warunków wstępnych w roadmapie.
**Estimated effort:** ~1 sesja, 3 fazy, każda z ręcznym przeglądem ekranów.

## Open Risks & Assumptions

- Nakładanie z F-02 w `WorkTimeView.tsx` i `TodoView.tsx`: gdy obie zmiany idą równolegle, konflikt jest mały (inne linie), ale łatwiej robić je po kolei.
- Brak runnera testów UI — regresje wyglądu łapie wyłącznie ręczny przegląd przy 1280 i 800 px.

## Success Criteria (Summary)

- Pełna bramka z AGENTS.md jest zielona po każdej fazie, a wyszukiwania nie znajdują lokalnych kopii zegarów, `plural`, zakładek, chipu ani nadpisań `.dn-seg-btn`.
- Każdy widok wygląda i zachowuje się jak przed zmianą, poza `<code>gh auth login</code>` i blokadą „Sprawdź teraz” w trakcie odświeżania.
