---
project: "Nawigator dnia"  # nazwa robocza
context_type: brownfield
created: 2026-11-22
updated: 2026-11-22
product_type: desktop      # No change — existing desktop
target_scale:              # No change — existing single user
  users: small
  qps: low
  data_volume: small
timeline_budget:
  delivery_weeks: 6
  hard_deadline: null
  after_hours_only: true
checkpoint:
  current_phase: 8
  phases_completed: [1, 2, 3, 4, 5, 6, 7]
  gray_areas_resolved:
    - topic: "session restart"
      decision: "nowa sesja shape 2026-11-22; poprzednia (greenfield, 25 FR, 12-tyg. MVP) zarchiwizowana w context/foundation/archive/shape-notes-2026-11-22-2031.md"
    - topic: "context type"
      decision: "brownfield — w repo istnieje kod (F-01, F-02, F-03, S-05), CI i PRD v2; nowe MVP zawęża zakres"
    - topic: "why re-scope"
      decision: "plan był za duży; potrzebne coś używalnego szybko; mniej funkcji na teraz"
    - topic: "S-05 work tasks from template"
      decision: "usunąć — własne zadania żyją w aplikacji"
    - topic: "must preserve"
      decision: "ikona stanu w zasobniku (F-02) oraz CI i wydania instalatora (F-03); F-01 nie wskazane jako konieczne"
    - topic: "vault boundary"
      decision: "MVP w ogóle nie dotyka vaultów — ani odczytu, ani zapisu"
    - topic: "top priority"
      decision: "przypisane zgłoszenia z GitLaba"
    - topic: "access model"
      decision: "bez zmian — jeden użytkownik, bez logowania, zero własnych sekretów, GitLab/GitHub tylko przez zalogowane glab/gh"
    - topic: "work/private separation"
      decision: "własne zadania mają przestrzeń Praca albo Prywatne; licznik czasu dotyczy tylko pracy"
    - topic: "layout"
      decision: "3 osobne widoki źródeł + widok „Do zrobienia” do wyboru z nich; licznik zawsze u góry"
    - topic: "MVP scope"
      decision: "pełny przepływ 1–8; dłuższy termin świadomie przyjęty: 6 tygodni po godzinach"
    - topic: "Socratic round"
      decision: "16 FR zakwestionowanych; FR-009 (usuń zadanie) usunięty; FR-017 i FR-018 wydzielone z FR-013; MR-y przypisane dodane do FR-001/FR-004; FR-015 zmieniony na modified (brak VPN = spokojny stan)"
    - topic: "business rule"
      decision: "widoczność (co wymaga uwagi) + norma 8 h × dni robocze; zastępuje regułę z PRD v2 (rekomendacja kandydatów i wymuszanie decyzji)"
    - topic: "norm deviation"
      decision: "odchylenie do dziś + średnio ile dziennie do końca miesiąca; dni robocze pn–pt minus dni wolne oznaczane ręcznie"
    - topic: "time history"
      decision: "bez migracji; nowa ewidencja od 2026-12-01; Helper App działa dalej obok, niezależnie"
    - topic: "freshness"
      decision: "powiadomienie o nowym przypisaniu ≤ 15 min przy włączonym VPN"
    - topic: "product framing"
      decision: "desktop bez zmian; tylko jeden użytkownik; bez twardego terminu; po godzinach"
    - topic: "non-goals"
      decision: "bez zapisu do GitLaba/GitHuba, bez vaultów, bez automatyki czasu pracy; reszta PRD v2 rozważona, nie wykluczona, bez FR"
  frs_drafted: 17
  quality_check_status: accepted
---

# Shape notes

Seed (2026-11-22, argument `/10x-shape`, dosłownie):

> ### MVP
> - chce 3 źródła informacji
>     - gitlab z pracy (przez cli) -> chcę widzieć co mam do zrobienia (przypisane do mnie issues z gitlaba; mozna zrobic ustawienia aby pokazywać te ktore chce -> chodzi tu tylko o same wyświetlanie informacji + powiadomienie o nowych) (uwaga: gitlab z pracy dziala tylko przez vpn)
>     - github z moich projektow (przez cli); issue z moich projektow
>     - własne taski dodawane przez aplikacje - moje zadania ktore sam sobie określam
> - licznik czasu pracy (przez buttony - całkowicie inne miejsce liczenia - zaczniemy od nowego miesiaca)
> - ograniczenia:
>     - nic nie dodajemy ani nie modyfikujemy do moich vaultow z obsidiana (to później); tylko sam odczyt

Materiał tła: `notes/idea.md`, zarchiwizowane shape-notes (link wyżej), `context/foundation/prd.md` (v2), `context/foundation/roadmap.md`.

## Current System

Nawigator dnia: osobista aplikacja desktopowa na Windows 11 (Tauri 2), jeden użytkownik, jeden komputer. Stan na 2026-11-22 (z repo, potwierdzony przez użytkownika):

- **Stack:** Rust (edition 2024) w `src-tauri/`, React 19 + TypeScript (Vite) w `src/`; ustawienia w `settings.json` w katalogu konfiguracji aplikacji.
- **Działa w zasobniku:** jedna instancja, powiadomienia Windows, log aplikacji.
- **F-01 (gotowe): strzeżony dostęp do vaultu.** Odczyt i zapis tylko w skonfigurowanych folderach, wykluczone ścieżki, zapis zmienia tylko wskazaną linię lub sekcję, usuwanie do kosza vaultu.
- **F-02 (gotowe): stan źródeł.** Ikona w zasobniku (OK / błąd źródła / nie działa) i powiadomienie Windows przy awarii źródła; ekrany pokazują błąd zamiast pustej listy.
- **F-03 (gotowe): CI i wydania.** Prywatne repo GitHub; każdy push uruchamia skan sekretów, audyty, build i testy; podbicie wersji na `main` publikuje instalator NSIS w GitHub Releases.
- **S-05 (gotowe): zadania pracy z szablonu.** Lista zadań z `Tasks/`, tworzenie z szablonu i zmiana statusu — **zapisuje do vaultu pracy**.
- **Nie ma:** ekranu „Dziś”, integracji z GitLabem i GitHubem, licznika czasu pracy.
- **Plan, który był w toku:** PRD v2 i roadmapa M-1 (17 slice'ów, 12-tygodniowe MVP z zapisem do vaultów, przerwami, blokerem, zamknięciem dnia).

## Vision & Problem Statement

**Delta.** Dotychczasowy plan (PRD v2, 12 tygodni, 25 FR) okazał się za duży. Użytkownik potrzebuje czegoś używalnego szybko i nie potrzebuje teraz tylu funkcji. Zmiana zawęża MVP do czterech rzeczy:

1. przypisane do użytkownika zgłoszenia z firmowego GitLaba (przez CLI) — co ma do zrobienia, z ustawieniem, które pokazywać, i powiadomieniem o nowych; tylko wyświetlanie. GitLab z pracy działa wyłącznie przez VPN;
2. zgłoszenia z prywatnych projektów na GitHubie (przez CLI);
3. własne zadania, które użytkownik sam sobie określa, dodawane w aplikacji;
4. licznik czasu pracy z przyciskami start/stop, liczony w całkowicie nowym miejscu, od nowego miesiąca.

**Najpilniejsze** z czterech: przypisane zgłoszenia z GitLaba.

**Granica wobec vaultów.** MVP w ogóle nie dotyka vaultów Obsidian — ani odczytu, ani zapisu. Zapis do vaultów to „później”; jeśli kiedyś pojawi się odczyt, to bez zapisu.

## User & Persona

**Persona główna: autor (jedyny użytkownik).** Programista na jednym komputerze z Windows 11, który pracuje dla firmy (zgłoszenia w firmowym GitLabie, dostępnym tylko przez VPN, czas pracy rozliczany miesięcznie) i rozwija własne projekty (zgłoszenia na GitHubie). Sięga po aplikację, żeby zobaczyć, co ma do zrobienia, dowiedzieć się o nowym przypisanym zgłoszeniu i włączyć lub wyłączyć licznik czasu pracy.

## Access Control

No changes planned — current model preserved.

- Jeden użytkownik, jedno konto Windows, jeden komputer. Bez ekranu logowania i bez ról.
- Zero własnych haseł, tokenów i kluczy w aplikacji, ustawieniach i repozytorium. Do firmowego GitLaba i prywatnego GitHuba aplikacja sięga wyłącznie przez sesje narzędzi zalogowanych na tej maszynie; gdy użytkownik jest wylogowany, aplikacja mówi to wprost.
- **Rozdział Praca/Prywatne.** Zgłoszenia z GitLaba należą do przestrzeni Praca, zgłoszenia z GitHuba do Prywatne. Każde własne zadanie należy do jednej przestrzeni. Licznik czasu dotyczy tylko pracy. Dane z pracy nie trafiają do przestrzeni prywatnej, repozytorium, testów ani logów.

## MVP flow (zablokowany w fazie 3)

Układ: trzy osobne widoki źródeł („Praca (GitLab)”, „Projekty (GitHub)”, „Moje zadania”) i jeden widok „Do zrobienia”, w którym użytkownik wybiera pozycje z tych trzech list. Licznik czasu pracy jest zawsze widoczny u góry okna.

Pierwszy dzień z nową wersją (delta), potwierdzony przez użytkownika:

1. Rano użytkownik otwiera aplikację i klika START — czas pracy zaczyna się liczyć.
2. Widok „Praca (GitLab)”: przypisane do niego zgłoszenia. Może ukryć zgłoszenie; ukryte wraca samo, gdy zmieni się jego status albo pojawi się nowy komentarz.
3. Widok „Projekty (GitHub)”: zgłoszenia z jego prywatnych repozytoriów.
4. Widok „Moje zadania”: dodaje zadania, które sam sobie określa.
5. Widok „Do zrobienia”: wybiera spośród trzech list, czym się dziś zajmie.
6. W ciągu dnia nowe przypisane zgłoszenie z GitLaba daje powiadomienie systemowe Windows.
7. Na koniec dnia klika STOP.
8. Raport: ile przepracował dziś i w miesiącu oraz ogólne odchylenie miesiąca od normy 8 h na dzień — żeby wiedzieć, czy pracować dłużej, czy krócej, by wyrobić normę.

**Promień rażenia.** Zmiana usuwa S-05 (zadania z szablonu w `Tasks/`) i nie korzysta z vaultów. Musi zostać zachowana ikona stanu w zasobniku (F-02) oraz CI i wydania instalatora (F-03).

## Success Criteria

### Primary
- Cały przepływ 1–8 przechodzi w aplikacji.
- Przez 2 kolejne tygodnie użytkownik każdego dnia roboczego używa aplikacji, żeby wybrać, czym się zajmie, i liczyć czas pracy.
- Na koniec każdego dnia użytkownik wie, ile brakuje mu do normy (odchylenie miesiąca od 8 h na dzień).
- Każde nowe przypisane zgłoszenie z GitLaba użytkownik widzi tego samego dnia, dzięki powiadomieniu systemowemu Windows, bez ręcznego sprawdzania.

### Secondary
- Ukryte zgłoszenia nie zaśmiecają listy pracy.

### Guardrails
- **Brak VPN ≠ brak zgłoszeń.** Gdy GitLab jest niedostępny, aplikacja mówi to wprost i nie pokazuje pustej listy.
- **Czas pracy nie ginie.** Zamknięcie aplikacji, restart czy awaria nie gubią biegnącego licznika ani historii.
- **Dane pracy nie wyciekają.** Nic z GitLaba nie trafia do repozytorium, testów, logów ani przestrzeni prywatnej.
- **Vaulty nietknięte.** Aplikacja nie zmienia żadnego pliku w vaultach Obsidian.
- **Zachowane:** ikona stanu w zasobniku (F-02) oraz CI i wydania instalatora (F-03) działają jak dotąd.

## Timeline acknowledgment

Acknowledged on 2026-11-22: 6-week MVP requires sustained dedication; user accepted. Użytkownik zobaczył ujawniony koszt zakresu (8 kroków na dwóch CLI, ukrywanie z powrotem przy zmianie statusu/komentarza, powiadomienia w tle przy GitLabie dostępnym tylko przez VPN, reguła dni roboczych dla normy) i świadomie wybrał dłuższy termin zamiast zmniejszania zakresu.

## Functional Requirements

Aktor: Użytkownik (jedyny). Wszystkie FR są must-have — użytkownik świadomie nie obniżył żadnego do nice-to-have (2026-11-22).

### Zgłoszenia GitLab (Praca)
- FR-001: Użytkownik może zobaczyć w widoku „Praca (GitLab)” zgłoszenia i MR-y z firmowego GitLaba przypisane do niego. Priority: must-have. Change: new
  > Socrates: Kontrargument: „same zgłoszenia to za mało” — pracą są też MR-y, bez nich lista „co mam do zrobienia” kłamie. Rozstrzygnięcie: dodane MR-y przypisane do użytkownika (assignee); MR-y, w których jest reviewerem, nie wchodzą do v1.
- FR-002: Użytkownik może w ustawieniach określić, które przypisane zgłoszenia są widoczne — według projektu, etykiet, milestone'u i stanu (otwarte/zamknięte). Priority: must-have. Change: new
  > Socrates: Kontrargument: „filtr ukryje coś pilnego” — nowe przypisanie spoza filtra nigdy nie dotrze. Rozstrzygnięcie: filtr dotyczy listy; powiadomienie o nowym przypisaniu (FR-004) omija filtr.
- FR-003: Użytkownik może ukryć zgłoszenie; ukryte zgłoszenie wraca samo na listę, gdy zmieni się jego status albo pojawi się nowy komentarz od kogoś innego niż użytkownik. Priority: must-have. Change: new
  > Socrates: Kontrargument: „szum z własnych i automatycznych komentarzy” przywraca zgłoszenie i ukrywanie traci sens. Rozstrzygnięcie: własne komentarze użytkownika nie przywracają zgłoszenia; komentarze innych, w tym botów i wpisy automatyczne, przywracają.
- FR-004: Użytkownik dostaje powiadomienie systemowe Windows o nowym zgłoszeniu lub MR przypisanym do niego, niezależnie od filtra z FR-002; gdy nowych jest więcej niż 3 naraz, dostaje jedno zbiorcze powiadomienie. Priority: must-have. Change: new
  > Socrates: Kontrargument: „zalew po podłączeniu VPN” — po nocy bez VPN albo po urlopie seria powiadomień naraz. Rozstrzygnięcie: 1–3 nowe — osobne powiadomienia; więcej niż 3 — jedno zbiorcze. Po dodaniu MR-ów do FR-001: nowe przypisane MR-y też powiadamiają, tym samym mechanizmem i progiem.

### Zgłoszenia GitHub (Prywatne)
- FR-005: Użytkownik może zobaczyć w widoku „Projekty (GitHub)” otwarte zgłoszenia przypisane do niego z prywatnych repozytoriów wskazanych na liście w ustawieniach. Priority: must-have. Change: new
  > Socrates: Kontrargument: „lista urośnie od starych zgłoszeń” — wszystkie otwarte zgłoszenia, także stare pomysły, zalewają widok. Rozstrzygnięcie: tylko otwarte i przypisane do użytkownika; reszta zostaje backlogiem na GitHubie.

### Moje zadania
- FR-006: Użytkownik może dodać własne zadanie w przestrzeni Praca albo Prywatne; domyślnie Praca, gdy biegnie licznik czasu pracy, w przeciwnym razie Prywatne. Priority: must-have. Change: new
  > Socrates: Kontrargument: „dodatkowy wybór przy każdym dodaniu” — tarcie, a bez wartości domyślnej zadania lądują w złej przestrzeni. Rozstrzygnięcie: domyślna przestrzeń zależy od tego, czy biegnie licznik; zawsze można ją zmienić.
- FR-007: Użytkownik może edytować tytuł i krótką notatkę (bez formatowania) własnego zadania. Priority: must-have. Change: new
  > Socrates: Kontrargument: „edycja rośnie w edytor notatek” — drugi Obsidian. Rozstrzygnięcie: zadanie ma tylko tytuł i krótką notatkę bez formatowania.
- FR-008: Użytkownik może oznaczyć własne zadanie jako zrobione; odhaczenie zadania w „Do zrobienia” i oznaczenie go jako zrobione w „Moich zadaniach” to jeden i ten sam stan. Priority: must-have. Change: new
  > Socrates: Kontrargument: „dwa miejsca «zrobione»” mogą się rozjechać. Rozstrzygnięcie: jeden stan — odhaczenie w „Do zrobienia” oznacza zadanie jako zrobione i odwrotnie.
- (FR-009 usunięty w rundzie Sokratesa — „usuń zadanie”. Kontrargument: „usuń i zrobione to prawie to samo”. Rozstrzygnięcie: zostaje tylko „zrobione”; zadanie dodane przez pomyłkę użytkownik oznacza jako zrobione albo edytuje. Numer FR-009 nie jest używany ponownie.)
- FR-010: Użytkownik może ustawić termin i priorytet własnego zadania; oba pola są opcjonalne i nigdy wymagane. Priority: must-have. Change: new
  > Socrates: Kontrargument: „pola, których nie będziesz wypełniać” — użytkownik porzuca ręczne listy. Rozstrzygnięcie: zadanie da się dodać samym tytułem; termin i priorytet są opcjonalne.

### Do zrobienia
- FR-011: Użytkownik może w widoku „Do zrobienia” wybrać pozycje spośród trzech list (zgłoszenia i MR-y GitLab, zgłoszenia GitHub, własne zadania), do limitu pozycji ustawianego w ustawieniach (domyślnie 5). Priority: must-have. Change: new
  > Socrates: Kontrargument: „bez limitu to kolejna pełna lista”. Rozstrzygnięcie: limit pozycji w ustawieniach, domyślnie 5; żeby dodać ponad limit, trzeba coś zdjąć.
- FR-012: Użytkownik może odhaczać pozycje w „Do zrobienia”; nieodhaczone zostają na liście następnego dnia, dopóki ich nie zdejmie; zgłoszenie lub MR zamknięte w GitLabie/GitHubie odhacza się samo i nie przechodzi na jutro. Priority: must-have. Change: new
  > Socrates: Kontrargument: „zamknięte w źródle dalej wisi” jako niezrobione. Rozstrzygnięcie: zamknięcie w źródle = automatyczne odhaczenie w „Do zrobienia”.

### Czas pracy
- FR-013: Użytkownik może przyciskami START i STOP rozpocząć i zakończyć liczenie czasu pracy; licznik jest zawsze widoczny u góry okna. Priority: must-have. Change: new
  > Socrates: Kontrargument: „zapomniany START albo STOP” — dokładnie ten problem miała stara ewidencja (dziury, licznik bijący przez noc), a bez korekty raport utrwala błąd. Rozstrzygnięcie: przyjęte trzy zabezpieczenia, wydzielone do FR-017 (ręczna korekta) i FR-018 (przypomnienia o STOP i START).
- FR-014: Użytkownik może zobaczyć raport: sumę czasu pracy dnia i miesiąca oraz ogólne odchylenie miesiąca od normy 8 h na dzień roboczy, gdzie dni robocze to poniedziałek–piątek minus dni oznaczone przez użytkownika jako wolne. Priority: must-have. Change: new
  > Socrates: Kontrargument: „norma bez dni wolnych kłamie” — święta i urlop dają fałszywe odchylenie i raportowi przestaje się ufać. Rozstrzygnięcie: pn–pt minus dni wolne oznaczane ręcznie (święta, urlop); bez automatycznego kalendarza świąt.
- FR-017: Użytkownik może ręcznie poprawić godzinę startu lub końca wpisu czasu pracy albo dopisać brakujący wpis. Priority: must-have. Change: new
  > Socrates: Wydzielone z FR-013 w rundzie Sokratesa (zapomniany START/STOP).
- FR-018: Użytkownik dostaje przypomnienie, gdy licznik biegnie długo (próg w ustawieniach, np. ponad 10 h: „czy nadal pracujesz?”), oraz gdy w dzień roboczy licznik nie biegnie po ustalonej godzinie. Priority: must-have. Change: new
  > Socrates: Wydzielone z FR-013 w rundzie Sokratesa (zapomniany START/STOP).

### Zachowane i zmienione
- FR-015: Użytkownik widzi awarię źródła (np. wylogowane `glab`/`gh`) w zasobniku i na ekranie zamiast pustej listy; niedostępny GitLab (brak VPN) jest pokazywany wprost jako spokojny stan „GitLab niedostępny”, bez alarmu i bez powiadomienia o błędzie. Priority: must-have. Change: modified
  > Socrates: Kontrargument: „brak VPN to stan normalny” — wieczorem przy projektach prywatnych ciągły alarm to szum, który użytkownik nauczy się ignorować. Rozstrzygnięcie: brak VPN to spokojny stan widoczny wprost; alarm (czerwona ikona, powiadomienie) tylko przy innych błędach. Zmienia zachowanie F-02, stąd `modified`.
- FR-016: Użytkownik dostaje nową wersję aplikacji jako instalator z GitHub Releases po podbiciu wersji. Priority: must-have. Change: preserved
  > Socrates: Kontrargument: „narzut dla jednego użytkownika” — lokalny build mógłby wystarczyć. Rozstrzygnięcie: zostaje; pipeline już działa, a bez podbicia wersji nic się nie publikuje.

### Usunięte
- [removed] S-05: tworzenie zadań z szablonu w `Tasks/` vaultu pracy i zmiana ich statusu — własne zadania żyją w aplikacji, a MVP nie dotyka vaultów.

## User Stories

### US-01: Poranek z nową wersją

- **Given** jest rano, użytkownik ma VPN, zalogowane `glab` i `gh` oraz niezrobione pozycje z wczoraj
- **When** otwiera aplikację i klika START
- **Then** licznik czasu biegnie u góry okna; widoki pokazują przypisane zgłoszenia GitLab (bez ukrytych), zgłoszenia GitHub z wybranych repozytoriów i jego własne zadania; w „Do zrobienia” są wczorajsze niezrobione pozycje, a on dokłada nowe

#### Acceptance Criteria
- Przed zmianą: zgłoszenia z GitLaba wymagały ręcznej synchronizacji lub przeglądarki, a czasu pracy aplikacja nie liczyła.
- Ukryte zgłoszenia, które się nie zmieniły, nie pojawiają się w widoku „Praca (GitLab)”.
- Gdy GitLab jest niedostępny (brak VPN), widok mówi to wprost zamiast pokazywać pustą listę.

### US-02: Ukryte zgłoszenie wraca po zmianie

- **Given** użytkownik ukrył zgłoszenie w widoku „Praca (GitLab)”
- **When** ktoś zmienia status tego zgłoszenia albo dodaje do niego komentarz
- **Then** zgłoszenie wraca na listę „Praca (GitLab)”, oznaczone jako zmienione

#### Acceptance Criteria
- Zgłoszenie bez zmiany statusu i bez nowego komentarza pozostaje ukryte.
- Własny komentarz użytkownika nie przywraca zgłoszenia; komentarz kogokolwiek innego (także bota) przywraca.

## Business Logic

Aplikacja pokazuje tylko to, co wymaga uwagi użytkownika (ukryte wraca przy zmianie statusu lub cudzym komentarzu, zamknięte w źródle odhacza się samo), i na bieżąco liczy, ile brakuje mu do normy 8 h na dzień roboczy w miesiącu.

**Obecna reguła (PRD v2) i zmiana.** The system currently (w planie PRD v2) proponuje uporządkowaną listę kandydatów na dziś, wymusza decyzję przy każdej pozycji (priorytet / jeśli starczy czasu / skrzynka / porzuć) i zaczepia użytkownika według stanu dnia. This change modifies it to: aplikacja nie rekomenduje kolejności i nie wymusza decyzji; decyduje o **widoczności** pozycji i **liczy normę czasu pracy**.

**Widoczność.** Wejściami są przypisane do użytkownika zgłoszenia i MR-y z GitLaba (zawężone filtrem z ustawień: projekt, etykiety, milestone, stan), otwarte przypisane zgłoszenia GitHub z repozytoriów z listy oraz własne zadania. Ukryte zgłoszenie pozostaje ukryte, dopóki nie zmieni się jego status albo nie pojawi się komentarz kogoś innego niż użytkownik (boty się liczą); wtedy wraca, oznaczone jako zmienione. Nowe przypisanie zawsze daje powiadomienie, także spoza filtra; ponad 3 nowe naraz — jedno zbiorcze. W „Do zrobienia” (limit pozycji z ustawień, domyślnie 5) nieodhaczone przechodzą na następny dzień, a pozycja zamknięta w GitLabie/GitHubie odhacza się sama; odhaczenie własnego zadania to to samo co oznaczenie go jako zrobione.

**Norma.** Dni robocze to poniedziałek–piątek minus dni oznaczone przez użytkownika jako wolne. Norma = 8 h × dni robocze. Raport pokazuje sumę dnia i miesiąca, odchylenie „do dziś” (przepracowane vs norma za dni robocze, które już minęły, w jedną lub drugą stronę) oraz ile średnio na dzień trzeba pracować w pozostałych dniach roboczych, żeby wyrobić normę miesiąca. Przypomnienia pilnują ewidencji: gdy licznik biegnie dłużej niż próg z ustawień, i gdy w dzień roboczy nie biegnie po ustalonej godzinie.

## Constraints & Preserved Behavior

- **Integracje:** firmowy GitLab tylko przez zalogowane `glab` i tylko przy włączonym VPN; GitHub tylko przez zalogowane `gh`. Aplikacja tylko czyta z GitLaba i GitHuba — niczego tam nie zmienia (odhaczenie w „Do zrobienia” nie zamyka zgłoszenia w źródle).
- **Fakt środowiska (2026-11-20, `notes/idea.md`):** w Windowsie nie ma `gh` — jest tylko w WSL. Wymaga rozstrzygnięcia przed FR-005 (patrz Open Questions).
- **Vaulty Obsidian:** MVP ich nie czyta i nie zapisuje.
- **Migracja danych:** brak. Nowa ewidencja czasu pracy zaczyna się 2026-12-01; dotychczasowa historia (`Management/Timeline.md`, Helper App) zostaje tam, gdzie jest, i aplikacja jej nie czyta.
- **Helper App:** działa dalej obok, niezależnie; aplikacja jej nie wywołuje i nie czyta.
- **Usuwane:** S-05 (zadania z szablonu w `Tasks/`).
- **Zachowane:** ikona stanu w zasobniku i powiadomienia o awarii źródła (F-02 — ze zmianą dla braku VPN, FR-015); CI ze skanem sekretów, audytami i testami oraz publikacja instalatora po podbiciu wersji (F-03).
- **Zachowane reguły repozytorium:** zero sekretów; dane pracy nie trafiają do repo, testów, fixture'ów ani logów; testy na zmyślonych danych.
- **Ograniczenia istniejącego systemu (faza 6):** CI musi być zielone przy każdym pushu (skan sekretów, audyty, build, testy), a pre-push hook z denylistą terminów pracy zostaje. Wydanie powstaje tylko przez podbicie wersji w `src-tauri/Cargo.toml` (jedyne źródło wersji); bez podbicia nic się nie publikuje.

## Non-Functional Requirements

- **Trwałość czasu pracy.** Biegnący licznik i historia czasu pracy przetrwają zamknięcie aplikacji, restart Windows i awarię aplikacji; strata najwyżej kilku sekund.
- **Nigdy pusta lista zamiast błędu.** Każdy widok źródła odróżnia „brak pozycji” od „źródło niedostępne” i pokazuje czas ostatniego udanego odświeżenia.
- **Dane pracy i vaulty.** Treść z firmowego GitLaba nie opuszcza maszyny i nie trafia do repozytorium, testów, logów ani przestrzeni prywatnej; żaden plik w vaultach Obsidian nie jest przez aplikację zmieniany; aplikacja, jej ustawienia i repozytorium nie zawierają haseł, tokenów ani kluczy.
- **Lekkość w tle.** W bezczynności w zasobniku aplikacja zużywa orientacyjnie < 1% CPU i < 150 MB RAM.
- **Świeżość.** Przy włączonym VPN powiadomienie o nowym przypisanym zgłoszeniu lub MR przychodzi ≤ 15 minut od przypisania w GitLabie.

## Non-Goals

### Funkcjonalne
- **Brak zapisu do GitLaba i GitHuba.** Aplikacja nie zamyka zgłoszeń, nie zmienia statusów, nie komentuje i nie przypisuje; tylko odczyt.
- **Brak dostępu do vaultów Obsidian.** Ani odczytu, ani zapisu (plan dnia, dziennik, `Tasks/`, `Issues/`) — zapis do vaultów to „później”.
- **Brak automatyki czasu pracy.** Bez automatycznego kalendarza świąt, wykrywania bezczynności i mierzenia aktywności; czas liczy się wyłącznie z przycisków START/STOP i ręcznej korekty (FR-017).
- **Poza v1 (z rundy Sokratesa):** MR-y, w których użytkownik jest reviewerem; usuwanie zadań (FR-009 usunięty).

### Niezmieniane w istniejącym systemie
- Typ produktu (desktop, Windows 11) i skala (jeden użytkownik) bez zmian.
- Model dostępu bez zmian (Access Control).

### Rozważone i świadomie NIE wykluczone
Reszta planu z PRD v2 — przerwy i pomodoro, bloker rozpraszaczy, szybki wpis i skrzynka, zamknięcie dnia, widok tygodnia, poranne „Ułóż plan dnia”, autostart. Nie są non-goals, ale żadne FR ich nie obejmuje, więc w tym MVP nie są budowane z braku wymagania, a nie z zakazu.

## Forward: tech-stack

Informacyjnie, nie jest częścią PRD. Do przejęcia przy ocenie stacku:
- **Stack bez zmian:** Tauri 2, Rust (edition 2024), React 19 + TypeScript (Vite) — `context/foundation/tech-stack.md`.
- **Miejsce danych aplikacji.** Własne zadania, lista „Do zrobienia”, stan ukrytych zgłoszeń i ewidencja czasu pracy („całkowicie inne miejsce liczenia”) są danymi aplikacji, nie vaultu. Obecna twarda reguła w `AGENTS.md` pozwala aplikacji tworzyć poza skonfigurowanymi folderami tylko `settings.json` i log — trzeba ją rozszerzyć o własne dane aplikacji.
- **`gh` tylko w WSL** (fakt z 2026-11-20) — sposób uruchamiania `gh` z aplikacji Windows do rozstrzygnięcia (Open Questions #1).
- **F-01 (strzeżony dostęp do vaultu)** nie jest potrzebny w MVP (brak dostępu do vaultów) i nie został wskazany jako konieczny do zachowania; decyzja, czy kod zostaje uśpiony, czy jest usuwany, przy ocenie stacku/planie. S-05 jest do usunięcia.

## Open Questions

1. **`gh` w Windows** — `gh` jest tylko w WSL; jak aplikacja na Windows ma sięgać do GitHuba (instalacja `gh` w Windows czy wywołanie przez WSL)? Owner: użytkownik. Przed planem FR-005. Blokuje: FR-005.
2. **Start ewidencji 2026-12-01 a gotowość licznika** — nowa ewidencja zaczyna się 2026-12-01, a licznik powstanie później (MVP 6 tygodni, bez twardego terminu). Czy wpisy z grudnia do czasu wdrożenia dopisać ręcznie (FR-017), czy przesunąć start ewidencji na pierwszy pełny miesiąc po wdrożeniu? Owner: użytkownik. Do: przed planem licznika.
3. **Wartości domyślne** — godzina, po której przypomina o niebiegnącym liczniku (FR-018); próg „licznik biegnie długo” (propozycja z rozmowy: 10 h); interwał odświeżania (musi spełnić NFR ≤ 15 min). Owner: użytkownik.
4. **Los PRD v2 i roadmapy M-1** — nowe PRD zastępuje PRD v2; co z gotowymi do pracy slice'ami S-01, S-06, S-17 i z niewykorzystywanym F-01? Owner: użytkownik, przy `/10x-prd` i aktualizacji roadmapy.
5. **Nazwa produktu** — robocza „Nawigator dnia” (przeniesione z poprzedniej sesji). Owner: użytkownik. Blokuje: nie.

## Quality cross-check

Uruchomiona 2026-11-22 (brownfield). Wszystkie elementy obecne: Access Control (bez zmian), Business Logic (reguła w jednym zdaniu), artefakty projektu, potwierdzenie kosztu terminu (6 tygodni), Non-Goals, Constraints & Preserved Behavior. Status: accepted. Brak luk do odzwierciedlenia; 5 otwartych pytań z `## Open Questions` przechodzi do PRD bez zmian, najważniejsze #1 (`gh` tylko w WSL — blokuje FR-005).
