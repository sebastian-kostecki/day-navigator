---
project: "Nawigator dnia"
version: 2
status: draft
created: 2026-11-22
updated: 2026-11-26
context_type: brownfield
product_type: desktop
target_scale:
  users: small
  qps: low
  data_volume: small
timeline_budget:
  delivery_weeks: 6
  hard_deadline: null
  after_hours_only: true
---

# Nawigator dnia — PRD (zawężone MVP)

Źródło: `context/foundation/shape-notes.md` (sesja 2026-11-22, brownfield). Zastępuje PRD v2 z 2026-11-20 (greenfield, 12 tygodni, 25 FR), które pozostaje w historii git.

Wersja 2 tego PRD (2026-11-24): dopisane FR-019–FR-026 z GitHub issues #1–#8 (uzupełnienia z makiet Konsoli, milestone M-2), patrz `### Uzupełnienia z makiet Konsoli (M-2)`. Nie mylić z dawnym greenfield PRD v2.

## Current System Overview

- **Cel systemu:** osobista aplikacja desktopowa na Windows 11, która ma dawać jednemu użytkownikowi jedno miejsce na dzień pracy.
- **Architektura:** jedna lokalna aplikacja desktopowa w zasobniku (jedna instancja), bez serwera.
- **Stack:** Tauri 2; Rust (edition 2024) w `src-tauri/`; React 19 + TypeScript (Vite) w `src/`; ustawienia w `settings.json` w katalogu konfiguracji aplikacji; log aplikacji.
- **Użytkownicy:** jeden — autor, na jednym komputerze.
- **Co działa dziś:**
  - **F-01: strzeżony dostęp do vaultu.** Odczyt i zapis tylko w skonfigurowanych folderach, wykluczone ścieżki, zapis zmienia tylko wskazaną linię lub sekcję, usuwanie do kosza vaultu.
  - **F-02: stan źródeł.** Ikona w zasobniku (OK / błąd źródła / nie działa) i powiadomienie Windows przy awarii źródła; ekrany pokazują błąd zamiast pustej listy.
  - **F-03: CI i wydania.** Prywatne repozytorium na GitHubie; każdy push uruchamia skan sekretów, audyty, build i testy; pre-push hook blokuje push z terminami z denylisty pracy; podbicie wersji w `src-tauri/Cargo.toml` (jedyne źródło wersji) na `main` publikuje instalator NSIS w GitHub Releases.
  - **S-05: zadania pracy z szablonu.** Lista zadań z `Tasks/` w vaulcie pracy, tworzenie z szablonu i zmiana statusu (zapisuje do vaultu).
- **Czego nie ma:** ekranu „Dziś”, integracji z GitLabem i GitHubem, licznika czasu pracy.
- **Otoczenie:** firmowy GitLab dostępny wyłącznie przez VPN, osiągany przez zalogowane CLI `glab`; GitHub przez zalogowane CLI `gh` (według stanu z 2026-11-20 dostępne tylko w WSL). Helper App (PHP, Symfony Console) prowadzi dotychczasową ewidencję czasu pracy (`Management/Timeline.md`) i synchronizację zgłoszeń do `Issues/`.
- **Plan w toku przed tą zmianą:** PRD v2 i roadmapa M-1 (17 slice'ów, 12-tygodniowe MVP z zapisem do vaultów, przerwami, blokerem, zamknięciem dnia).

## Problem Statement & Motivation

**Luka.** Dotychczasowy plan (PRD v2, 12 tygodni, 25 FR) okazał się za duży. Użytkownik potrzebuje czegoś używalnego szybko i nie potrzebuje teraz tylu funkcji.

**Zmiana.** MVP zawęża się do czterech rzeczy:

1. przypisane do użytkownika zgłoszenia z firmowego GitLaba — co ma do zrobienia, z ustawieniem, które pokazywać, i powiadomieniem o nowych; tylko wyświetlanie. GitLab z pracy działa wyłącznie przez VPN;
2. zgłoszenia z prywatnych projektów na GitHubie;
3. własne zadania, które użytkownik sam sobie określa, dodawane w aplikacji;
4. licznik czasu pracy z przyciskami start/stop, liczony w całkowicie nowym miejscu, od nowego miesiąca.

**Najpilniejsze** z czterech: przypisane zgłoszenia z GitLaba.

**Granica wobec vaultów.** MVP w ogóle nie dotyka vaultów Obsidian — ani odczytu, ani zapisu. Zapis do vaultów to „później”; jeśli kiedyś pojawi się odczyt, to bez zapisu.

**Budżet czasu.** Acknowledged on 2026-11-22: 6-week MVP requires sustained dedication; user accepted. Użytkownik zobaczył ujawniony koszt zakresu (8 kroków na dwóch źródłach zgłoszeń, ukrywanie z powrotem przy zmianie statusu/komentarza, powiadomienia w tle przy GitLabie dostępnym tylko przez VPN, reguła dni roboczych dla normy) i świadomie wybrał dłuższy termin zamiast zmniejszania zakresu.

## User & Persona

**Persona główna: autor (jedyny użytkownik).** Programista na jednym komputerze z Windows 11, który pracuje dla firmy (zgłoszenia w firmowym GitLabie, dostępnym tylko przez VPN, czas pracy rozliczany miesięcznie) i rozwija własne projekty (zgłoszenia na GitHubie). Sięga po aplikację, żeby zobaczyć, co ma do zrobienia, dowiedzieć się o nowym przypisanym zgłoszeniu i włączyć lub wyłączyć licznik czasu pracy.

**Co się zmienia dla niego:** zamiast planu dnia w vaulcie, przerw i blokera dostaje wcześniej jeden ekran z trzema źródłami pozycji, listą „Do zrobienia” i licznikiem czasu z raportem normy.

## Success Criteria

### Primary
- Cały przepływ 1–8 (patrz `## Scope of Change`) przechodzi w aplikacji.
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
- **Zachowane (brak regresji):** stan źródeł widoczny w zasobniku (F-02) oraz automatyczna kontrola każdej zmiany i wydania (F-03) działają jak dotąd.

## User Stories

### US-01: Poranek z nową wersją

- **Given** jest rano, użytkownik ma VPN, jest zalogowany do GitLaba i GitHuba na tej maszynie i ma niezrobione pozycje z wczoraj
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

## Scope of Change

**Układ.** Trzy osobne widoki źródeł („Praca (GitLab)”, „Projekty (GitHub)”, „Moje zadania”) i jeden widok „Do zrobienia”, w którym użytkownik wybiera pozycje z tych trzech list. Licznik czasu pracy jest zawsze widoczny u góry okna.

**Przepływ docelowy** (pierwszy dzień z nową wersją):

1. Rano użytkownik otwiera aplikację i klika START — czas pracy zaczyna się liczyć.
2. Widok „Praca (GitLab)”: przypisane do niego zgłoszenia. Może ukryć zgłoszenie; ukryte wraca samo, gdy zmieni się jego status albo pojawi się nowy komentarz.
3. Widok „Projekty (GitHub)”: zgłoszenia z jego prywatnych repozytoriów.
4. Widok „Moje zadania”: dodaje zadania, które sam sobie określa.
5. Widok „Do zrobienia”: wybiera spośród trzech list, czym się dziś zajmie.
6. W ciągu dnia nowe przypisane zgłoszenie z GitLaba daje powiadomienie systemowe Windows.
7. Na koniec dnia klika STOP.
8. Raport: ile przepracował dziś i w miesiącu oraz ogólne odchylenie miesiąca od normy 8 h na dzień — żeby wiedzieć, czy pracować dłużej, czy krócej, by wyrobić normę.

Aktor: Użytkownik (jedyny). Wszystkie pozycje są must-have — użytkownik świadomie nie obniżył żadnej do nice-to-have (2026-11-22).

### Zgłoszenia GitLab (Praca)
- [new] FR-001: Użytkownik może zobaczyć w widoku „Praca (GitLab)” zgłoszenia i MR-y z firmowego GitLaba przypisane do niego. Priority: must-have
  > Socrates: Kontrargument: „same zgłoszenia to za mało” — pracą są też MR-y, bez nich lista „co mam do zrobienia” kłamie. Rozstrzygnięcie: dodane MR-y przypisane do użytkownika (assignee); MR-y, w których jest reviewerem, nie wchodzą do v1.
- [new] FR-002: Użytkownik może w ustawieniach określić, które przypisane zgłoszenia są widoczne — według projektu, etykiet, milestone'u i stanu (otwarte/zamknięte). Priority: must-have
  > Socrates: Kontrargument: „filtr ukryje coś pilnego” — nowe przypisanie spoza filtra nigdy nie dotrze. Rozstrzygnięcie: filtr dotyczy listy; powiadomienie o nowym przypisaniu (FR-004) omija filtr.
- [new] FR-003: Użytkownik może ukryć zgłoszenie; ukryte zgłoszenie wraca samo na listę, gdy zmieni się jego status albo pojawi się nowy komentarz od kogoś innego niż użytkownik. Priority: must-have
  > Socrates: Kontrargument: „szum z własnych i automatycznych komentarzy” przywraca zgłoszenie i ukrywanie traci sens. Rozstrzygnięcie: własne komentarze użytkownika nie przywracają zgłoszenia; komentarze innych, w tym botów i wpisy automatyczne, przywracają.
- [new] FR-004: Użytkownik dostaje powiadomienie systemowe Windows o nowym zgłoszeniu lub MR przypisanym do niego, niezależnie od filtra z FR-002; gdy nowych jest więcej niż 3 naraz, dostaje jedno zbiorcze powiadomienie. Priority: must-have
  > Socrates: Kontrargument: „zalew po podłączeniu VPN” — po nocy bez VPN albo po urlopie seria powiadomień naraz. Rozstrzygnięcie: 1–3 nowe — osobne powiadomienia; więcej niż 3 — jedno zbiorcze. Po dodaniu MR-ów do FR-001: nowe przypisane MR-y też powiadamiają, tym samym mechanizmem i progiem.

### Zgłoszenia GitHub (Prywatne)
- [new] FR-005: Użytkownik może zobaczyć w widoku „Projekty (GitHub)” otwarte zgłoszenia przypisane do niego z prywatnych repozytoriów wskazanych na liście w ustawieniach. Priority: must-have
  > Socrates: Kontrargument: „lista urośnie od starych zgłoszeń” — wszystkie otwarte zgłoszenia, także stare pomysły, zalewają widok. Rozstrzygnięcie: tylko otwarte i przypisane do użytkownika; reszta zostaje backlogiem na GitHubie.

### Moje zadania
- [new] FR-006: Użytkownik może dodać własne zadanie w przestrzeni Praca albo Prywatne; domyślnie Praca, gdy biegnie licznik czasu pracy, w przeciwnym razie Prywatne. Priority: must-have
  > Socrates: Kontrargument: „dodatkowy wybór przy każdym dodaniu” — tarcie, a bez wartości domyślnej zadania lądują w złej przestrzeni. Rozstrzygnięcie: domyślna przestrzeń zależy od tego, czy biegnie licznik; zawsze można ją zmienić.
- [new] FR-007: Użytkownik może edytować tytuł i krótką notatkę (bez formatowania) własnego zadania. Priority: must-have
  > Socrates: Kontrargument: „edycja rośnie w edytor notatek” — drugi Obsidian. Rozstrzygnięcie: zadanie ma tylko tytuł i krótką notatkę bez formatowania.
- [new] FR-008: Użytkownik może oznaczyć własne zadanie jako zrobione; odhaczenie zadania w „Do zrobienia” i oznaczenie go jako zrobione w „Moich zadaniach” to jeden i ten sam stan. Priority: must-have
  > Socrates: Kontrargument: „dwa miejsca «zrobione»” mogą się rozjechać. Rozstrzygnięcie: jeden stan — odhaczenie w „Do zrobienia” oznacza zadanie jako zrobione i odwrotnie.
- (FR-009 usunięty w rundzie Sokratesa — „usuń zadanie”. Kontrargument: „usuń i zrobione to prawie to samo”. Rozstrzygnięcie: zostaje tylko „zrobione”; zadanie dodane przez pomyłkę użytkownik oznacza jako zrobione albo edytuje. Numer FR-009 nie jest używany ponownie.)
- [new] FR-010: Użytkownik może ustawić termin i priorytet własnego zadania; oba pola są opcjonalne i nigdy wymagane. Priority: must-have
  > Socrates: Kontrargument: „pola, których nie będziesz wypełniać” — użytkownik porzuca ręczne listy. Rozstrzygnięcie: zadanie da się dodać samym tytułem; termin i priorytet są opcjonalne.

### Do zrobienia
- [new] FR-011: Użytkownik może w widoku „Do zrobienia” wybrać pozycje spośród trzech list (zgłoszenia i MR-y GitLab, zgłoszenia GitHub, własne zadania), do limitu pozycji ustawianego w ustawieniach (domyślnie 5). Priority: must-have
  > Socrates: Kontrargument: „bez limitu to kolejna pełna lista”. Rozstrzygnięcie: limit pozycji w ustawieniach, domyślnie 5; żeby dodać ponad limit, trzeba coś zdjąć.
- [new] FR-012: Użytkownik może odhaczać pozycje w „Do zrobienia”; nieodhaczone zostają na liście następnego dnia, dopóki ich nie zdejmie; zgłoszenie lub MR zamknięte w GitLabie/GitHubie odhacza się samo i nie przechodzi na jutro. Priority: must-have
  > Socrates: Kontrargument: „zamknięte w źródle dalej wisi” jako niezrobione. Rozstrzygnięcie: zamknięcie w źródle = automatyczne odhaczenie w „Do zrobienia”.

### Czas pracy
- [new] FR-013: Użytkownik może przyciskami START i STOP rozpocząć i zakończyć liczenie czasu pracy; licznik jest zawsze widoczny u góry okna. Priority: must-have
  > Socrates: Kontrargument: „zapomniany START albo STOP” — dokładnie ten problem miała stara ewidencja (dziury, licznik bijący przez noc), a bez korekty raport utrwala błąd. Rozstrzygnięcie: przyjęte trzy zabezpieczenia, wydzielone do FR-017 (ręczna korekta) i FR-018 (przypomnienia o STOP i START).
- [new] FR-014: Użytkownik może zobaczyć raport: sumę czasu pracy dnia i miesiąca oraz ogólne odchylenie miesiąca od normy 8 h na dzień roboczy, gdzie dni robocze to poniedziałek–piątek minus polskie święta państwowe (kalendarz liczony lokalnie, wyłączany w ustawieniach) i dni oznaczone przez użytkownika jako wolne. Priority: must-have
  > Socrates: Kontrargument: „norma bez dni wolnych kłamie” — święta i urlop dają fałszywe odchylenie i raportowi przestaje się ufać. Rozstrzygnięcie: pn–pt minus polskie święta państwowe z kalendarza liczonego lokalnie (MS-25) i dni wolne oznaczane ręcznie; dzień wolny należny za święto w sobotę oraz urlop zostają ręczne.
- [new] FR-017: Użytkownik może ręcznie poprawić godzinę startu lub końca wpisu czasu pracy albo dopisać brakujący wpis. Priority: must-have
  > Socrates: Wydzielone z FR-013 w rundzie Sokratesa (zapomniany START/STOP).
- [new] FR-018: Użytkownik dostaje przypomnienie, gdy licznik biegnie długo (próg w ustawieniach, np. ponad 10 h: „czy nadal pracujesz?”), oraz gdy w dzień roboczy licznik nie biegnie po ustalonej godzinie. Priority: must-have
  > Socrates: Wydzielone z FR-013 w rundzie Sokratesa (zapomniany START/STOP).

### Zmienione i zachowane
- [modified] FR-015: Użytkownik widzi awarię źródła (np. wylogowanie z GitLaba lub GitHuba na tej maszynie) w zasobniku i na ekranie zamiast pustej listy; niedostępny GitLab (brak VPN) jest pokazywany wprost jako spokojny stan „GitLab niedostępny”, bez alarmu i bez powiadomienia o błędzie. Priority: must-have. Było: każda niedostępność źródła, także brak VPN, była alarmem (F-02).
  > Socrates: Kontrargument: „brak VPN to stan normalny” — wieczorem przy projektach prywatnych ciągły alarm to szum, który użytkownik nauczy się ignorować. Rozstrzygnięcie: brak VPN to spokojny stan widoczny wprost; alarm (czerwona ikona, powiadomienie) tylko przy innych błędach. Zmienia zachowanie F-02, stąd `modified`.
- [preserved] FR-016: Użytkownik dostaje nową wersję aplikacji wyłącznie wtedy, gdy wersja została świadomie podniesiona; każda zmiana przed wydaniem przechodzi automatyczną kontrolę (sekrety, zależności, testy). Priority: must-have
  > Socrates: Kontrargument: „narzut dla jednego użytkownika” — lokalny build mógłby wystarczyć. Rozstrzygnięcie: zostaje; pipeline już działa, a bez podbicia wersji nic się nie publikuje.

### Usunięte
- [removed] S-05: tworzenie zadań z szablonu w `Tasks/` vaultu pracy i zmiana ich statusu — własne zadania żyją w aplikacji, a MVP nie dotyka vaultów.

### Uzupełnienia z makiet Konsoli (M-2)

Źródło: GitHub issues #1–#8 (2026-11-24), opisujące elementy makiet „Konsola”, których nie ma w wydaniu 0.2.0. Priorytet must-have oznacza „w zakresie M-2” (użytkownik wziął wszystkie do zrobienia). Bez rundy Sokratesa; treść za issue.

- [new] FR-019: Użytkownik widzi na dashboardzie („Do zrobienia”) panel „Dziennik zdarzeń” z dzisiejszymi zdarzeniami: nowe przypisanie, odhaczenie lub zdjęcie pozycji, START/STOP, przejście źródła w błąd i powrót. Zdarzenia trzymają tylko to, co UI i tak pokazuje. Priority: must-have (issue #1)
- [new] FR-020: Użytkownik widzi w „Czas pracy” tabelę dni miesiąca: wpisy, pasek na tle 8 h, sumę, odchylenie od normy dnia i przełącznik dnia wolnego; dziś jest wyróżniony, a dni wolne zakreskowane; święta zakreskowane z nazwą, bez przełącznika. Priority: must-have (issue #2)
- [new] FR-021: Użytkownik widzi w „Czas pracy” panel „Przypomnienia” z progami przypomnień i informacją, czy są włączone; bez edycji, ustawienia zostają w pliku. Priority: must-have (issue #3)
- [modified] FR-022: Użytkownik widzi w „Stanie źródeł” przy „Dane aplikacji” czas ostatniej kopii, liczbę zachowanych kopii i wersję schematu oraz może otworzyć folder kopii. Priority: must-have. Było: karta pokazywała tylko stan bazy. (issue #4)
- [modified] FR-023: Użytkownik widzi w „Praca (GitLab)” flagę NOWE przy pozycjach, które pojawiły się od ostatniego sprawdzenia, oraz datę ukrycia przy ukrytych wierszach. Priority: must-have. Było: lista bez tych informacji (FR-001, FR-003). (issue #5)
- [modified] FR-024: Użytkownik widzi nad listą „Praca (GitLab)” wartości aktywnego filtra z linkiem do ustawień, a w karcie „GitLab niedostępny” interwał odświeżania i czas następnej próby. Priority: must-have. Było: tylko informacja, że filtr jest aktywny (FR-002, FR-015). (issue #6)
- [modified] FR-025: Użytkownik widzi etykiety przy zgłoszeniach w „Projekty (GitHub)”, tak jak w GitLabie. Priority: must-have. Było: zgłoszenia bez etykiet (FR-005). (issue #7)
- [modified] FR-026: Użytkownik może dodać pozycję do „Do zrobienia” przyciskiem „+” prosto z wiersza w „Praca (GitLab)” i „Projekty (GitHub)”, z blokadą limitu i podpowiedzią jak w pickerze; pozycja już dodana ma flagę W DO ZROBIENIA. Priority: must-have. Było: wybór tylko w widoku „Do zrobienia” (FR-011). (issue #8)

## Constraints & Compatibility

- **Integracje:** firmowy GitLab tylko przy włączonym VPN; do GitLaba i GitHuba aplikacja sięga wyłącznie przez sesje, w których użytkownik jest już zalogowany na tej maszynie. Aplikacja tylko czyta z GitLaba i GitHuba — niczego tam nie zmienia (odhaczenie w „Do zrobienia” nie zamyka zgłoszenia w źródle).
- **Fakt środowiska (2026-11-20):** narzędzie dostępu do GitHuba jest dziś zainstalowane tylko w WSL, nie w Windows. Wymaga rozstrzygnięcia przed FR-005 (patrz Open Questions #1).
- **Vaulty Obsidian:** MVP ich nie czyta i nie zapisuje.
- **Migracja danych:** brak. Nowa ewidencja czasu pracy zaczyna się 2026-12-01; dotychczasowa historia (`Management/Timeline.md`, Helper App) zostaje tam, gdzie jest, i aplikacja jej nie czyta.
- **Helper App:** działa dalej obok, niezależnie; aplikacja jej nie wywołuje i nie czyta.
- **Usuwane:** S-05 (zadania z szablonu w `Tasks/`).
- **Zachowane:** stan źródeł w zasobniku i powiadomienia o awarii źródła (F-02 — ze zmianą dla braku VPN, FR-015); automatyczna kontrola każdej zmiany oraz publikacja nowej wersji wyłącznie po świadomym podniesieniu wersji (F-03).
- **Zachowane reguły repozytorium:** zero sekretów; dane pracy nie trafiają do repozytorium, testów, danych testowych ani logów; testy na zmyślonych danych.
- **Ograniczenia istniejącego systemu:** automatyczna kontrola musi przechodzić przy każdej zmianie wysyłanej do repozytorium (sekrety, zależności, build, testy), a blokada wysyłki terminów pracy zostaje. Nowa wersja powstaje tylko przez świadome podniesienie wersji; bez niego nic się nie publikuje. Szczegóły w `## Current System Overview`.

### Właściwości jakościowe (NFR)
- **Trwałość czasu pracy.** Biegnący licznik i historia czasu pracy przetrwają zamknięcie aplikacji, restart Windows i awarię aplikacji; strata najwyżej kilku sekund.
- **Nigdy pusta lista zamiast błędu.** Każdy widok źródła odróżnia „brak pozycji” od „źródło niedostępne” i pokazuje czas ostatniego udanego odświeżenia.
- **Dane pracy i vaulty.** Treść z firmowego GitLaba nie opuszcza maszyny i nie trafia do repozytorium, testów, logów ani przestrzeni prywatnej; żaden plik w vaultach Obsidian nie jest przez aplikację zmieniany; aplikacja, jej ustawienia i repozytorium nie zawierają haseł, tokenów ani kluczy.
- **Lekkość w tle.** W bezczynności w tle aplikacja zużywa orientacyjnie < 1% CPU i < 150 MB RAM.
- **Świeżość.** Przy włączonym VPN powiadomienie o nowym przypisanym zgłoszeniu lub MR przychodzi ≤ 15 minut od przypisania w GitLabie.

## Business Logic Changes

Aplikacja pokazuje tylko to, co wymaga uwagi użytkownika (ukryte wraca przy zmianie statusu lub cudzym komentarzu, zamknięte w źródle odhacza się samo), i na bieżąco liczy, ile brakuje mu do normy 8 h na dzień roboczy w miesiącu.

**Obecna reguła i zmiana.** The system currently (w planie PRD v2) proponuje uporządkowaną listę kandydatów na dziś, wymusza decyzję przy każdej pozycji (priorytet / jeśli starczy czasu / skrzynka / porzuć) i zaczepia użytkownika według stanu dnia. This change modifies it to: aplikacja nie rekomenduje kolejności i nie wymusza decyzji; decyduje o **widoczności** pozycji i **liczy normę czasu pracy**.

**Widoczność.** Wejściami są przypisane do użytkownika zgłoszenia i MR-y z GitLaba (zawężone filtrem z ustawień: projekt, etykiety, milestone, stan), otwarte przypisane zgłoszenia GitHub z repozytoriów z listy oraz własne zadania. Ukryte zgłoszenie pozostaje ukryte, dopóki nie zmieni się jego status albo nie pojawi się komentarz kogoś innego niż użytkownik (boty się liczą); wtedy wraca, oznaczone jako zmienione. Nowe przypisanie zawsze daje powiadomienie, także spoza filtra; ponad 3 nowe naraz — jedno zbiorcze. W „Do zrobienia” (limit pozycji z ustawień, domyślnie 5) nieodhaczone przechodzą na następny dzień, a pozycja zamknięta w GitLabie/GitHubie odhacza się sama; odhaczenie własnego zadania to to samo co oznaczenie go jako zrobione.

**Norma.** Dni robocze to poniedziałek–piątek minus polskie święta państwowe (kalendarz liczony lokalnie, wyłączany w ustawieniach) i dni oznaczone przez użytkownika jako wolne. Norma = 8 h × dni robocze. Raport pokazuje sumę dnia i miesiąca, odchylenie „do dziś” (przepracowane vs norma za dni robocze, które już minęły, w jedną lub drugą stronę) oraz ile średnio na dzień trzeba pracować w pozostałych dniach roboczych, żeby wyrobić normę miesiąca. Przypomnienia pilnują ewidencji: gdy licznik biegnie dłużej niż próg z ustawień, i gdy w dzień roboczy nie biegnie po ustalonej godzinie.

## Access Control Changes

No access control changes — current model preserved.

- Jeden użytkownik, jedno konto Windows, jeden komputer. Bez ekranu logowania i bez ról.
- Zero własnych haseł, tokenów i kluczy w aplikacji, ustawieniach i repozytorium. Do firmowego GitLaba i prywatnego GitHuba aplikacja sięga wyłącznie przez sesje, w których użytkownik jest już zalogowany na tej maszynie; gdy jest wylogowany, aplikacja mówi to wprost.
- **Rozdział Praca/Prywatne.** Zgłoszenia z GitLaba należą do przestrzeni Praca, zgłoszenia z GitHuba do Prywatne. Każde własne zadanie należy do jednej przestrzeni. Licznik czasu dotyczy tylko pracy. Dane z pracy nie trafiają do przestrzeni prywatnej, repozytorium, testów ani logów.

## Non-Goals

### Funkcjonalne
- **Brak zapisu do GitLaba i GitHuba.** Aplikacja nie zamyka zgłoszeń, nie zmienia statusów, nie komentuje i nie przypisuje; tylko odczyt.
- **Brak dostępu do vaultów Obsidian.** Ani odczytu, ani zapisu (plan dnia, dziennik, `Tasks/`, `Issues/`) — zapis do vaultów to „później”.
- **Brak automatyki czasu pracy.** Bez wykrywania bezczynności i mierzenia aktywności; czas liczy się wyłącznie z przycisków START/STOP i ręcznej korekty (FR-017).
- **Poza v1 (z rundy Sokratesa):** MR-y, w których użytkownik jest reviewerem; usuwanie zadań (FR-009 usunięty).

### Niezmieniane w istniejącym systemie
- Typ produktu (desktop, Windows 11) i skala (jeden użytkownik) bez zmian.
- Model dostępu bez zmian (`## Access Control Changes`).

### Rozważone i świadomie NIE wykluczone
Reszta planu z PRD v2 — przerwy i pomodoro, bloker rozpraszaczy, szybki wpis i skrzynka, zamknięcie dnia, widok tygodnia, poranne „Ułóż plan dnia”. Nie są non-goals, ale żadne FR ich nie obejmuje, więc w tym MVP nie są budowane z braku wymagania, a nie z zakazu.
Autostart z Windows jest dostarczany w M-3 jako MS-17 (roadmapa): domyślnie wyłączony, przełącznik `autostart.enabled` w `settings.json`, start do zasobnika bez okna; bez nowego FR.

## Open Questions

1. **`gh` w Windows** — narzędzie dostępu do GitHuba jest tylko w WSL; jak aplikacja na Windows ma sięgać do GitHuba (instalacja w Windows czy wywołanie przez WSL)? Owner: użytkownik. Przed planem FR-005. Blokuje: FR-005.
2. **Start ewidencji 2026-12-01 a gotowość licznika** — nowa ewidencja zaczyna się 2026-12-01, a licznik powstanie później (MVP 6 tygodni, bez twardego terminu). Czy wpisy z grudnia do czasu wdrożenia dopisać ręcznie (FR-017), czy przesunąć start ewidencji na pierwszy pełny miesiąc po wdrożeniu? Owner: użytkownik. Do: przed planem licznika.
3. **Wartości domyślne** — godzina, po której przypomina o niebiegnącym liczniku (FR-018); próg „licznik biegnie długo” (propozycja z rozmowy: 10 h); częstotliwość odświeżania (musi spełnić NFR ≤ 15 min). Owner: użytkownik.
4. **Los PRD v2 i roadmapy M-1** — to PRD zastępuje PRD v2; co z gotowymi do pracy slice'ami S-01, S-06, S-17 i z niewykorzystywanym F-01? Owner: użytkownik, przy aktualizacji roadmapy.
5. **Nazwa produktu** — robocza „Nawigator dnia”. Owner: użytkownik. Blokuje: nie.
