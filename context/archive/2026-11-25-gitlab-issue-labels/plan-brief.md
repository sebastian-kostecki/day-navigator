# Etykiety GitLaba jako kolorowe chipy i filtr widoku — Plan Brief

> Full plan: `context/changes/gitlab-issue-labels/plan.md`

## What & Why

W „Praca (GitLab)” etykiety mają być widoczne jako chipy w tych samych kolorach co w GitLabie, a całą listę da się zawęzić w samym widoku po projektach, etykietach i kamieniach milowych. Dziś etykiety to szary tekst po przecinku, a zawężać można tylko na stałe w `settings.json`.

## Starting Point

Wywołania `glab` zwracają etykiety jako same nazwy, bez kolorów. Kamień milowy jest parsowany w Rust, ale nie trafia do UI. Widok ma tylko przełącznik typu i „Pokaż ukryte” w lokalnym stanie, który ginie po zmianie zakładki. Filtr z `settings.json` (`gitlabFilter`) działa w Rust, a nad listą pokazuje go `GitlabFilterBar`.

## Desired End State

Każdy wiersz (przypisane, ukryte, zamknięte) ma chipy etykiet: tło w kolorze z GitLaba, tekst w jego `text_color`, a scoped `klucz::wartość` jest dzielony jak w GitLabie. W pasku narzędzi trzy listy wyboru („Projekty”, „Etykiety”, „Kamienie milowe”) z licznikami zawężają listę. Klik w chip albo w projekt w wierszu przełącza tę wartość. Wybory widać jako usuwalne chipy z „Wyczyść” i trwają do zamknięcia aplikacji.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) |
| --- | --- | --- |
| Wygląd chipa | Wypełniony jak w GitLabie, scoped dzielony | Te same kolory co w GitLabie, statusy scoped czytelne od razu. |
| Źródło kolorów | `with_labels_details=true` w czterech wywołaniach listy | Jedno pole w istniejących GET-ach zamiast osobnych wywołań na projekt. |
| Postać etykiety w danych | `GitlabLabel { name, color, textColor }`; parser przyjmuje też samą nazwę | Serwer bez obsługi parametru daje neutralne chipy zamiast błędu. |
| Walidacja koloru | W Rust: `#` + 3/6 hex, oba kolory albo żaden | Kolor z sieci trafia do CSS tylko sprawdzony; tło bez koloru tekstu bywa nieczytelne. |
| Relacja do filtra z ustawień | Dodatkowa warstwa nad `gitlabFilter` | Bez zmian w Rust, powiadomieniach i „Do zrobienia”. |
| Wymiary | Projekty, etykiety, kamienie milowe (+ istniejący typ) | Kamień milowy jest już parsowany, wystarczy go wysłać. |
| Semantyka | LUB w wymiarze, I między wymiarami | Ta sama reguła co filtr z ustawień. |
| Trwałość | Kontekst React do zamknięcia aplikacji | Przeżywa zmianę zakładki, bez zapisu danych pracy na dysk. |
| Kontrolka | Wspólny `MultiSelect` (lista pól wyboru z licznikami) | Skaluje się do wielu etykiet i projektów. |
| Klik w wierszu | Chip etykiety i projekt przełączają filtr | Najszybszy sposób zawężania. |
| Zasięg | Wszystkie sekcje tabel, liczniki osobno od filtra z ustawień | Spójnie z przełącznikiem typu; pusta sekcja zawsze wyjaśnia dlaczego. |

## Scope

**In scope:**
- `with_labels_details=true` w wywołaniach przypisanych i zamkniętych, strażnik i testy argumentów
- `GitlabLabel` z walidacją kolorów, `milestone` w migawce, lustro TS, `fake-cli`
- Reguły czytające nazwy (filtr z ustawień, scoped ukrytych) bez zmiany zachowania
- Wspólne `LabelChip` i `MultiSelect`, tokeny zapasowe
- `GitlabListViewProvider`, `viewFilter.ts`, pasek wyboru, aktywne chipy, liczniki, pusty stan, klik z wiersza
- Zdanie w AGENTS.md o nowym kontekście

**Out of scope:**
- Zmiany `gitlabFilter`, zapis wyborów na dysk, migracje
- Filtrowanie powiadomień, „Do zrobienia”, pickera i licznika w nagłówku
- Wyszukiwanie po tytule, filtry po flagach, opcje „bez kamienia” / „bez etykiet”, przełącznik LUB/I
- Kolory etykiet w GitHubie i w „Do zrobienia”
- Aktualizacja artefaktu design systemu

## Architecture / Approach

`glab api …&with_labels_details=true` → `RawItem.labels` (untagged: nazwa | obiekt) → `checked_color` → `GitlabItem.labels: Vec<GitlabLabel>` + `milestone` → migawka → `types.ts` → `LabelChip` w kolumnie etykiet. Filtr z ustawień i reguła scoped czytają `label_names()`. W UI `GitlabListViewProvider` (w `App.tsx`) trzyma typ, ukryte i `ViewSelection`. `viewFilter.ts` liczy opcje i dopasowanie, a `GitlabView` filtruje każdą sekcję `matchesKind && matchesView`.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Etykiety z kolorami i kamień milowy w danych | Nowe wywołania ze strażnikiem, `GitlabLabel`, `milestone`, lustro TS, `fake-cli`, testy | Pominięta kopia kształtu w strażniku albo zmiana wyniku `scoped_labels` (ukryte wracałyby jako zmienione) |
| 2. Chipy etykiet w wierszach | `LabelChip`, tokeny, kolumna z zawijanymi chipami | Jasne etykiety rażą na ciemnym tle; długie listy rozpychają wiersz |
| 3. Filtr widoku | Kontekst stanu, `MultiSelect`, pasek, liczniki, pusty stan, klik z wiersza | Pusta sekcja wyglądająca jak brak przypisań; dostępność popovera |

**Prerequisites:** `glab` zalogowany i VPN do ręcznych przeglądów; poza tym brak (S-02, S-03, S-04 gotowe).
**Estimated effort:** ~2–3 sesje, 3 fazy.

## Open Risks & Assumptions

- Firmowy GitLab obsługuje `with_labels_details` dla zgłoszeń i MR-ów. Jeśli nie, parametr jest ignorowany i chipy są neutralne (parser przyjmuje nazwy), co sprawdza krok 1.7.
- Filtr widoku żyje w UI bez testów jednostkowych (UI nie ma test runnera). Logika jest wydzielona do czystego `viewFilter.ts` i sprawdzana ręcznie w fazie 3.
- Artefakt design systemu nie zna `LabelChip` i `MultiSelect` do czasu osobnej synchronizacji.

## Success Criteria (Summary)

- Etykiety w „Praca (GitLab)” wyglądają jak w GitLabie, a niepoprawny kolor nigdy nie psuje czytelności.
- Kilkoma kliknięciami da się zawęzić listę do projektów, etykiet i kamieni milowych, z jasną informacją, co filtr ukrył i jak go wyczyścić.
- Wywołania `glab` dalej tylko czytają, a powiadomienia, ukryte i „Do zrobienia” działają jak przed zmianą.
