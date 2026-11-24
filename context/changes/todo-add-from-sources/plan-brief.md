# „+” do „Do zrobienia” z widoków źródeł — Plan Brief

> Full plan: `context/changes/todo-add-from-sources/plan.md`

## What & Why

W makietach Konsoli każdy wiersz „Praca (GitLab)” i „Projekty (GitHub)” ma przycisk „+”, który dodaje pozycję do „Do zrobienia”, a pozycja już dodana ma flagę W DO ZROBIENIA. Dziś dodać można tylko z pickera w widoku „Do zrobienia” (FR-011). FR-026 / issue #8 przenosi tę akcję do miejsca, w którym użytkownik i tak przegląda zgłoszenia.

## Starting Point

Komendy dodawania i ich funkcje w `src/todo/api.ts` istnieją. Rust ma zdarzenie `todo-changed`, ale emitują je tylko rundy odświeżania GitLab/GitHub. `useTodo` go nasłuchuje i przeładowuje listę także przy fokusie oraz o północy. Widoki źródeł nie znają dziś stanu listy.

## Desired End State

W obu widokach źródeł każdy wiersz ma „+”. Pozycja na dzisiejszej liście, także odhaczona, ma flagę W DO ZROBIENIA i wypełniony bursztynowy przycisk, którego klik ją zdejmuje. Przy limicie „+” jest wyłączony z podpowiedzią `Limit N otwartych pozycji`. Błąd wczytania listy widać jako `ErrorState`, nigdy jako cichy brak przycisków.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) |
| --- | --- | --- |
| Klik w wypełniony przycisk | Zdejmuje pozycję z listy | Tak mówi README przycisku w design systemie Konsola („nadal klikalny”). |
| Jak widoki poznają stan | `useTodo()` + `todo-changed` emitowane także przez komendy mutujące | Bez pollera; każda zmiana z dowolnego widoku dociera do każdego słuchacza. |
| Odhaczona pozycja | Ta sama flaga i wypełniony przycisk | Rust traktuje ją jako pozycję na liście (`AlreadyOnList`) do roll-overu. |
| Ukryte zgłoszenia / „Zamknięte” | Bez „+”; ukryte na liście mają flagę i przycisk do zdjęcia | Backend odrzuca ukryte (`NotInGitlabList`), a zamknięte odhaczają się same. |
| Reguły limitu | Tylko w Rust, UI tylko wyświetla i pokazuje odmowę | Zasada z `AGENTS.md`: reguły domeny w Rust. |

## Scope

**In scope:**
- Emit `todo-changed` z komend add/tick/remove i jedna stała nazwy zdarzenia
- `src/todo/keys.ts`, `TodoAddButton`, `useTodoPicks`, picker przepięty na wspólny przycisk
- „+”, flaga, komunikaty i błąd listy w `GitlabView` i `GithubView`

**Out of scope:**
- Zmiany reguł limitu i przenoszenia, schematu store i rejestracji komend w `lib.rs`
- Osobna flaga „ZROBIONE”, potwierdzenie przed zdjęciem, „+” w bloku „Zamknięte”
- Test runner UI

## Architecture / Approach

Rust: `todo/commands.rs::run` po udanej mutacji emituje `todo::CHANGED_EVENT` (także gdy odczyt listy po niej zawiódł; `list_todo` nie emituje nigdy). UI: `useTodoPicks` (nad `useTodo`) daje widokom źródeł lookup pozycji po kluczu, limit i akcje. `TodoAddButton` renderuje „+” (quiet) albo wypełniony (`primary` + `pressed`). Odmowy z Rust (limit, nieaktualna lista) pokazujemy przez `todoErrorMessage` w `Notice`.

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Zdarzenie z komend (Rust) | Każda udana mutacja emituje `todo-changed` | Pętla przeładowań, gdyby `list_todo` też emitował |
| 2. Wspólne elementy (UI) | `keys.ts`, `TodoAddButton`, `useTodoPicks`; picker bez zmian zachowania | Regresja pickera przy przenoszeniu kluczy |
| 3. „+” w widokach źródeł | Przycisk, flaga i błędy w GitLab i GitHub | Kolumna akcji GitLaba za wąska na 3 przyciski |

**Prerequisites:** brak (S-01 nie ma wymagań wstępnych).
**Estimated effort:** ~1 sesja, 3 fazy.

## Open Risks & Assumptions

- Widok, który wykonał akcję, przeładuje listę raz więcej po zdarzeniu. To tanie; `mutate` w `useTodo` już obsługuje wyścig odpowiedzi.
- Konflikt przy scalaniu z F-03 jest mało prawdopodobny: nie zmieniamy `lib.rs`, a w plikach refresh tylko stałą.

## Success Criteria (Summary)

- „+” w każdym wierszu GitLab/GitHub dodaje pozycję, a wiersz od razu ma flagę W DO ZROBIENIA i wypełniony przycisk.
- Przy limicie „+” jest wyłączony z podpowiedzią jak w pickerze, a wypełniony przycisk zdejmuje pozycję.
- Wszystkie kontrole sprzed commita z `AGENTS.md` przechodzą.
