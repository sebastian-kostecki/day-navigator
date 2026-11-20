# Pomysł zalążkowy: aplikacja dnia na Windows

Robocza nazwa: Nawigator dnia (nazwa nierozstrzygnięta). Plik jest wejściem do `/10x-shape`. Zapisy są podzielone na **decyzje użytkownika**, **fakty** sprawdzone 2026-11-20 i **rekomendacje agenta**; rekomendacja nie jest decyzją i wymaga potwierdzenia w rozmowie.

## Pomysł

Aplikacja desktopowa na Windows 11, która daje mi jedno miejsce na dzień pracy: plan na dziś, dziennik tego, co zrobione, zamknięcie dnia, czas pracy, zadania i zgłoszenia oraz notatki — z pracy i z moich prywatnych projektów. Aplikacja niczego nie przechowuje poza moimi notatkami: pokazuje dane z narzędzi, które mam zalogowane na maszynie, i czyta oraz dopisuje pliki Markdown w moich vaultach.

## Dla kogo i po co

- Jeden użytkownik (ja), na jednym komputerze. Telefon nie jest celem.
- Problem: plan dnia, notatki, zgłoszenia, czas pracy i projekty prywatne żyją dziś w kilku miejscach (dwa vaulty Obsidian, GitLab, GitHub, `context/` w repozytoriach). Chcę mieć do nich dostęp z jednego okna i dostawać przypomnienia w Windowsie.
- Obserwacja z moich danych: używam tego, co zasila się samo (czas pracy, synchronizacja zgłoszeń), a ręczne listy porzucam. Aplikacja ma więc brać dane automatycznie, a ręczne ma być tylko to, co jest moją myślą (wpis, skrzynka, decyzja przy zamknięciu dnia).

## Decyzje użytkownika

1. Wariant D: aplikacja desktopowa (Tauri) nad CLI i plikami, lokalnie. Odrzucone: aplikacja na VPS-ie, API na VPS-ie z klientem, samo rozszerzenie Helper App bez własnego okna.
2. Aplikacja obsługuje pracę i projekty prywatne, w jednym miejscu.
3. Bezpieczeństwo: do firmowego GitLaba i prywatnego GitHuba aplikacja sięga wyłącznie przez CLI zalogowane na maszynie (`glab`, `gh`). Aplikacja tylko wizualizuje dane z tych narzędzi i trzyma własne notatki lokalnie. Bez serwera, bez własnych tokenów.
4. Pliki zostają w dwóch osobnych vaultach. Bez przenoszenia do wspólnego miejsca i bez kopii roboczych.
5. Aplikacja służy do codziennych operacji; pełna edycja treści, zmiana nazw i przenoszenie notatek zostają w Obsidianie (na start przycisk „Otwórz w Obsidianie”).
6. Ścieżki muszą być ustawiane w aplikacji, na wypadek zmian. Nic na stałe w kodzie.
7. Potrzebne są powiadomienia w Windowsie.
8. Makieta w stylu Atramentu (system designu Domowego Skarbnika).

## Fakty o stanie wyjściowym

- Vault pracy: `D:\panelalpha\panelalpha-docs`. Zawiera `Issues/` (notatka na zgłoszenie, pola: `Status`, `Milestone`, `Priority`, `Labels`, `Today`), `Tasks/` (szablon: `Status`, `Priority`, `Due`, `Issue`, `Today`, `Done`), `Management/Timeline.md` (start, koniec i suma czasu pracy), `DASHBOARD.md`, `ToDo List.md`, `Notatki bieżące.md`. Pole `Status` jest listą w zgłoszeniach i pojedynczą wartością w zadaniach.
- Vault prywatny: `D:\my-docs`. Hub `moje_projekty` jest nieaktualny od 2026-08-19; źródłem prawdy o projektach jest `context/` w repozytoriach w WSL (`~/my-projects`).
- Helper App: `D:\panelalpha\panelalpha-helper`, PHP 8.2, Symfony Console. Komendy: start i koniec pracy, synchronizacja zgłoszeń, raport mailowy, sprzątanie skrzynek. Od 2026-10-26 pobiera zgłoszenia przez `glab`.
- Oba vaulty mają gita bez zdalnego repozytorium; vault pracy commituje wtyczka obsidian-git co trzy godziny.
- Narzędzia w Windowsie: `cargo`, `node`, `git`, `glab`, `php` są; `gh` nie ma (jest w WSL).
- Rozpoczęty projekt Tauri 2 + Vue 3 + Pinia + Tailwind: `D:\my-projects\ai-assistant` („Ed”), zbudowany wokół czatu.
- Vault pracy zawiera dane służbowe (zgłoszenia klientów, adresy serwerów, notatki z kluczami).

## Zakres pierwszej wersji (rekomendacja agenta)

| Element | Co robi | Skąd dane |
|---|---|---|
| Dziś | 1–3 priorytety, lista „jeśli starczy czasu”, dziennik dnia z polem wpisu | pola `Today` w `Issues/` i `Tasks/`, notatka dzienna |
| Szybki wpis | okienko pod skrótem globalnym, zapis do dziennika albo skrzynki | ręcznie |
| Skrzynka | myśli „na potem”, rozdzielane na dziś, jutro albo do usunięcia | ręcznie |
| Zgłoszenia | lista zgłoszeń i MR-ów z pracy oraz zgłoszeń i PR-ów prywatnych, tylko odczyt | `glab`, `gh` |
| Czas pracy | start, koniec, suma dnia i miesiąca | Helper App, `Management/Timeline.md` |
| Zamknięcie dnia | co zrobione, los niezrobionych (jutro, skrzynka, porzuć), jedno zdanie o dniu, szkic jutra | notatka dzienna, pola `Today` |
| Tydzień | wykonanie planu dzień po dniu i szkic podsumowania | notatki dzienne, czas pracy, zamknięte zgłoszenia |
| Powiadomienia | plan rano, koniec bloku skupienia, zamknięcie dnia | harmonogram w aplikacji |
| Zasobnik | priorytety do odhaczenia, skróty, wstrzymanie powiadomień | jak wyżej |
| Ustawienia | ścieżki i narzędzia dla każdej przestrzeni | plik ustawień |

Kolejność budowy (rekomendacja): okno, zasobnik, skrót i ustawienia przestrzeni → „Dziś” z szybkim wpisem → zgłoszenia z CLI → czas pracy i powiadomienia → zamknięcie dnia i tydzień.

## Poza zakresem pierwszej wersji (rekomendacja agenta)

Własny edytor Markdownu, zmiana nazw i przenoszenie notatek, telefon, synchronizacja między maszynami, serwer MCP, rytuały, integracje z Domowym Skarbnikiem i Menu Makerem, automatyczny pomiar aktywności (ActivityWatch).

## Architektura (rekomendacja agenta w ramach wariantu D)

- **Interfejs:** Vue 3. Widoki: Dziś, Tydzień, Skrzynka, Zgłoszenia, Czas pracy, Ustawienia.
- **Rdzeń Tauri (Rust):** odczyt i zapis plików tylko w skonfigurowanych folderach; uruchamianie tylko dozwolonych poleceń z ustalonymi argumentami; obserwowanie zmian w vaultach; harmonogram powiadomień; zasobnik, skrót globalny, autostart, jedna instancja.
- **Źródła:** `glab` (praca), `gh` (prywatne), `git` (commity), vaulty (Markdown), Helper App (synchronizacja zgłoszeń, czas pracy).
- **Dwie przestrzenie:** Praca i Prywatne, każda z własnym vaultem, własnym CLI i własną pamięcią podręczną. Dane z pracy nie są kopiowane do części prywatnej.
- **Poza vaultami:** ustawienia w `%APPDATA%`, pamięć podręczna wyników CLI w danych aplikacji; pamięć podręczną można usunąć i odbudować.
- **Sieć:** aplikacja sama nie łączy się z siecią i nie ma klienta HTTP.

## Własność plików

| Pliki | Kto zapisuje | Aplikacja |
|---|---|---|
| `Issues/*` | Helper App | czyta; zmienia tylko pole `Today` |
| `Tasks/*` | użytkownik w Obsidianie, aplikacja | czyta; zmienia `Status`, `Today`, `Done`; tworzy z szablonu |
| `Management/Timeline.md` | Helper App | czyta; start i koniec pracy wywołuje przez Helper App |
| `Dziennik/RRRR/RRRR-MM-DD.md` (nowe) | aplikacja | tworzy i dopisuje |
| `Skrzynka.md` (nowe) | aplikacja | dopisuje i usuwa pozycje |
| reszta vaultu | użytkownik w Obsidianie | czyta albo pomija |
| `context/` w repozytoriach w WSL | użytkownik i agenci | tylko czyta |

Zmiana pola w nagłówku notatki podmienia jedną linię; aplikacja nie zapisuje nagłówka od nowa. Usuwanie przenosi do kosza vaultu.

## Wymagania bezpieczeństwa

- Żadnych tokenów, haseł ani kluczy w aplikacji, w ustawieniach i w repozytorium.
- Lista wykluczonych ścieżek w ustawieniach (np. notatki z kluczami i dostępami); aplikacja ich nie czyta, nie indeksuje i nie pokazuje w wyszukiwaniu.
- Prawdziwe dane z vaultów nie trafiają do repozytorium, testów, fixture'ów ani CI. Testy używają małego zmyślonego vaultu napisanego na potrzeby testu.
- Gdy skonfigurowana ścieżka nie istnieje, aplikacja mówi to wprost i prosi o wskazanie nowej; niczego nie tworzy w ciemno.

## Otwarte pytania

1. Nazwa: Sternik, Skryba, Nawigator, Kompas, Brulion, Strażnik dnia albo inna.
2. Nowe repo czy przebudowa „Eda”? Rekomendacja: nowe repo w `D:\my-projects`, z „Eda” tylko konfiguracja Tauri i Vue.
3. Czy Helper App zostaje silnikiem synchronizacji i czasu pracy, wywoływanym przez aplikację? Rekomendacja: tak, żeby każdy plik miał jednego właściciela zapisu.
4. Czy Helper App ma objąć też przestrzeń prywatną, czy prywatne zgłoszenia aplikacja czyta bezpośrednio z `gh`?
5. Czy start i koniec pracy docelowo zapisywać w notatce dziennej, a `Timeline.md` generować?
6. Skrót globalny (propozycja: Ctrl+Alt+Spacja) i godziny powiadomień.
7. Czy plan dnia łączy obie przestrzenie na jednym ekranie, czy są przełączane?
8. Co z `DASHBOARD.md`, `ToDo List.md` i `Notatki bieżące.md` po wdrożeniu ekranu „Dziś”?
9. Czy repo ma być publiczne (portfolio)? Jeśli tak, w repo nie mogą się pojawić nazwy hostów ani dane z pracy.

## Materiały

- Makieta (ekrany Dziś, Tydzień, Skrzynka, zamknięcie dnia, szybki wpis, zasobnik, powiadomienia): https://claude.ai/artifact/UiEpZrnscd3716PPtte8nf (prywatna).
- Pełny zapis ustaleń: `moj-dzien-podsumowanie.md` w katalogu `claude-assistant` w WSL.
