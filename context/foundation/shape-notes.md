---
project: "Nawigator dnia"  # nazwa robocza — patrz Open Questions
context_type: greenfield
created: 2026-11-20
updated: 2026-11-20
product_type: desktop
target_scale:
  users: small
  qps: low
  data_volume: small
timeline_budget:
  mvp_weeks: 12
  hard_deadline: null
  after_hours_only: true
checkpoint:
  current_phase: 8
  phases_completed: [1, 2, 3, 4, 5, 6, 7]
  gray_areas_resolved:
    - topic: "context type"
      decision: "greenfield — nowe repo day-navigator; Ed i Helper App to otoczenie, nie system zmieniany"
    - topic: "assigned GitLab issues flow"
      decision: "nowe przypisane zgłoszenie jest pobierane do plików w vaulcie, wywołuje powiadomienie Windows i można je wciągnąć na listę rzeczy do zrobienia"
    - topic: "insight"
      decision: "dane już istnieją (vault, Helper App, GitLab), ale Obsidian nie przypomina, Helper App nie ma okna, synchronizacja jest ręczna"
    - topic: "access model"
      decision: "jeden użytkownik Windows, bez logowania w aplikacji; zero własnych sekretów; dostęp do GitLab/GitHub tylko przez sesje narzędzi zalogowanych na maszynie"
    - topic: "data boundaries"
      decision: "rozdział Praca/Prywatne, wykluczone ścieżki, odczyt/zapis tylko w skonfigurowanych folderach; zakaz własnej sieci NIE przyjęty jako wymóg"
    - topic: "MVP scope"
      decision: "dłuższy termin świadomie przyjęty: 12 tygodni po godzinach; przepływ dnia 1–9 jako Primary"
    - topic: "work time start"
      decision: "start/koniec kliknięciem; aplikacja przypomina, gdy pora pracy, a czas nie biegnie (i gdy biegnie po godzinach)"
    - topic: "breaks"
      decision: "oba tryby: interwały w tle (mikro + dłuższe przerwy) domyślnie oraz opcjonalne bloki skupienia (pomodoro) przy zadaniu"
    - topic: "distraction blocker"
      decision: "blokuje strony WWW i programy z list w ustawieniach, według harmonogramu; jawny wyjątek od granicy folderów; wyłączenie po krótkim potwierdzeniu; jawna zgoda na podwyższone uprawnienia"
    - topic: "source of truth for today"
      decision: "notatka dzienna w vaulcie danej przestrzeni; pole Today pochodne albo pomijane"
    - topic: "today screen layout"
      decision: "jeden ekran, dwie sekcje Praca i Prywatne; wspólny limit 1–3 priorytetów"
    - topic: "candidate sources"
      decision: "Praca: własne zadania + przypisane zgłoszenia/MR-y (wyróżnione etykiety, np. awaiting_verification); Prywatne: zgłoszenia GitHub"
    - topic: "product framing"
      decision: "desktop; tylko jeden użytkownik; brak twardego terminu; po godzinach"
  frs_drafted: 25
  quality_check_status: accepted
---

# Shape notes

Seed: `notes/idea.md` (pomysł zalążkowy, 2026-11-20). Decyzje i fakty z seeda obowiązują; „rekomendacje agenta” z seeda nie są decyzjami, dopóki nie zostaną potwierdzone w tej sesji.

## Vision & Problem Statement

**Ból.** Jedyny użytkownik pracuje jednocześnie dla firmy i nad własnymi projektami na jednym komputerze z Windows 11. Plan dnia, zadania, przypisane zgłoszenia, czas pracy i stan projektów prywatnych są rozrzucone po kilku miejscach: dwa vaulty Obsidian (praca i prywatny), GitLab, GitHub, `context/` w repozytoriach w WSL. Najmocniej odczuwa to **rano, przy układaniu planu dnia**, a potem w ciągu dnia przy śledzeniu zadań, zgłoszeń i czasu pracy. Koszt dziś:
- dzień zaczyna się bez priorytetów;
- przypisane zgłoszenia są zauważane za późno, bo synchronizację trzeba uruchomić ręcznie (komenda Helper App wywoływana z Obsidiana), a notatki w `Issues/` bywają nieaktualne;
- nic nie sygnalizuje w Windowsie, że coś się pojawiło albo że pora coś zrobić;
- rzeczy do zrobienia nie trafiają nigdzie jako zadanie i giną;
- ewidencja czasu pracy ma dziury (zapomniany start/koniec, suma dnia i miesiąca się nie zgadza);
- projekty prywatne stoją, bo nie wiadomo, gdzie się skończyło.

**Wgląd.** Dane już istnieją: vaulty mają notatki i zgłoszenia, Helper App umie synchronizować zgłoszenia i mierzyć czas pracy, GitLab ma przypisania. Brakuje warstwy, która sama pobiera dane, powiadamia w Windowsie i nadaje dniu rytm. Obsidian nie przypomina, Helper App nie ma okna, a synchronizacja jest ręczna. Z własnych danych użytkownika wynika też, że używa on tego, co zasila się samo, a ręczne listy porzuca. Ręczne ma więc zostać tylko to, co jest jego myślą: wpis, decyzja o planie, zamknięcie dnia.

Przepływ zgłoszeń w zamierzeniu użytkownika: nowe przypisane zgłoszenie z GitLaba jest pobierane do plików w vaulcie, wywołuje powiadomienie w Windowsie i można je jednym ruchem wciągnąć na listę rzeczy do zrobienia.

## User & Persona

**Persona główna: autor (jedyny użytkownik).** Programista, który w ciągu jednego dnia przełącza się między pracą dla firmy (GitLab, vault pracy, Helper App, czas pracy rozliczany miesięcznie) a własnymi projektami (GitHub, vault prywatny, `context/` w repozytoriach w WSL). Jeden komputer z Windows 11, telefon nie jest celem. Sięga po produkt rano, żeby ułożyć plan dnia, w ciągu dnia, gdy pojawia się nowe zgłoszenie albo myśl do zapisania, i na koniec dnia przy zamknięciu dnia i czasu pracy.

## Access Control

Jeden użytkownik, jedno konto Windows, jeden komputer. Aplikacja nie ma ekranu logowania ani ról. Nie przechowuje żadnych własnych haseł, tokenów ani kluczy: w aplikacji, w ustawieniach ani w repozytorium. Do GitLaba (praca) i GitHuba (prywatne) sięga wyłącznie przez sesje narzędzi, w których użytkownik jest już zalogowany na tej maszynie. Gdy tam się wyloguje, aplikacja nie ma tych danych i mówi to wprost.

Granice danych:
- **Rozdział Praca/Prywatne.** Dwie przestrzenie, każda z własnymi źródłami. Dane z pracy nigdy nie są kopiowane do przestrzeni prywatnej i odwrotnie.
- **Wykluczone ścieżki.** Lista w ustawieniach (np. notatki z kluczami i dostępami). Aplikacja ich nie czyta, nie indeksuje i nie pokazuje.
- **Tylko wskazane miejsca.** Aplikacja czyta i zapisuje wyłącznie w folderach skonfigurowanych w ustawieniach. Gdy skonfigurowana ścieżka nie istnieje, mówi to wprost i prosi o nową; niczego nie tworzy w ciemno.

- **Jawny wyjątek: bloker rozpraszaczy.** To jedyna funkcja, która działa poza skonfigurowanymi folderami. Blokuje wyłącznie strony i programy z list w ustawieniach, wyłącznie w harmonogramie z ustawień, i zawsze da się ją wyłączyć (po krótkim potwierdzeniu, FR-021). Reszta aplikacji trzyma granicę.

Uwaga: zakaz łączenia się aplikacji z siecią (rekomendacja z seeda) **nie** został przyjęty jako wymóg na tym etapie.

## MVP flow (zablokowany w fazie 3)

Pierwszy dzień z aplikacją, potwierdzony przez użytkownika:

1. Użytkownik włącza komputer, aplikacja startuje sama (ikona w zasobniku).
2. Synchronizacja zgłoszeń rusza sama; nowe przypisane zgłoszenia trafiają do vaultu.
3. Powiadomienie Windows: „Ułóż plan dnia”.
4. Użytkownik otwiera „Dziś” i widzi przypisane zgłoszenia, zadania oraz to, co zostało z wczoraj.
5. Wybiera 1–3 priorytety; plan zapisuje się w vaulcie.
6. Klika start pracy i czas pracy zaczyna się liczyć. Aplikacja przypomina, gdy jest pora pracy, a czas nie biegnie, i gdy biegnie po godzinach.
7. W ciągu dnia nowe przypisane zgłoszenie wywołuje powiadomienie z akcją „dodaj do listy”.
8. W ciągu dnia przypomnienia o przerwach: interwały w tle (mikroprzerwy i dłuższe przerwy) oraz opcjonalny blok skupienia (pomodoro) przy zadaniu. Według harmonogramu z ustawień działa bloker rozpraszaczy (strony WWW i programy z list).
9. Użytkownik klika koniec pracy i widzi sumę dnia i miesiąca.

Dodatki zgłoszone w fazie 3 (wcześniej nieobecne w seedzie): przypomnienia o przerwach w stylu pomodoro/stretchly, bloker rozpraszaczy z interfejsem graficznym.

## Success Criteria

### Primary
- Przez 2 kolejne tygodnie pracy plan dnia (1–3 priorytety) jest układany w aplikacji każdego dnia roboczego.
- Każde nowe przypisane zgłoszenie użytkownik widzi (powiadomienie) tego samego dnia, bez ręcznego uruchamiania synchronizacji.
- Cały przepływ dnia 1–9 przechodzi w aplikacji, od autostartu do sumy czasu pracy.

### Secondary
- Większość przypomnianych przerw jest wykonywana, a nie pomijana.
- Do przeglądu przypisanych zgłoszeń użytkownik nie musi otwierać GitLaba w przeglądarce.

### Guardrails
- Dane w vaultach: żadna notatka nie traci treści ani nagłówka przez zapis aplikacji; Obsidian i obsidian-git działają jak dotąd.
- Dane firmowe nie wyciekają: nic z pracy nie ląduje w przestrzeni prywatnej, w repozytorium, w testach ani poza maszyną.
- Bloker nie zamyka pracy: nigdy nie blokuje narzędzi do pracy i da się go wyłączyć w każdej chwili (po krótkim potwierdzeniu, FR-021).

## Timeline acknowledgment

Acknowledged on 2026-11-20: 12-week MVP requires sustained dedication; user accepted. Użytkownik zobaczył ujawniony koszt zakresu (9-krokowy przepływ, działania w tle, dwie przestrzenie, kilka integracji, dodatkowo przerwy i bloker) i świadomie wybrał dłuższy termin zamiast zmniejszania zakresu („czas nie ma znaczenia”).

## Functional Requirements

Aktor: Użytkownik (jedyny). Priorytety i treść po rundzie Sokratesa (2026-11-20).

### Rytm dnia
- FR-001: Użytkownik ma aplikację, która startuje razem z Windows i działa w zasobniku. Priority: must-have
  > Socrates: Kontrargument: „ciche awarie” — jeśli aplikacja nie wystartuje albo padnie w tle, rano nie będzie powiadomienia i nikt tego nie zauważy. Rozstrzygnięcie: zostaje; ikona zasobnika pokazuje stan (OK / błąd źródła / nie działa), a każda nieudana synchronizacja lub niedostępne źródło daje widoczny komunikat (→ NFR).
- FR-002: Użytkownik dostaje powiadomienie Windows „Ułóż plan dnia” przy pierwszym włączeniu lub odblokowaniu komputera w dniu roboczym, jeśli plan na ten dzień jeszcze nie istnieje; godzina z ustawień działa jako zapas. Priority: must-have
  > Socrates: Kontrargument: „stała godzina mija się z dniem” — przy wcześniejszym/późniejszym starcie powiadomienie przychodzi w złym momencie. Rozstrzygnięcie: wyzwalaczem jest pierwsze włączenie/odblokowanie w dniu roboczym; godzina tylko jako zapas.
- FR-003: Użytkownik widzi na jednym ekranie „Dziś” dwie sekcje — Praca (własne zadania oraz zgłoszenia i MR-y przypisane do niego, z wyróżnieniem etykiet z ustawień, np. `awaiting_verification`) i Prywatne (otwarte zgłoszenia GitHub z jego repozytoriów) — wraz z niezrobionymi pozycjami z poprzedniego dnia, w kolejności rekomendowanej przez aplikację; przy porannym planie każda zaległość wymaga decyzji: dziś / skrzynka / porzuć. Priority: must-have
  > Socrates: Kontrargument: „zaległości rosną w nieskończoność” — niezrobione przechodzą dzień w dzień i zapychają ekran. Rozstrzygnięcie: rano każda zaległość wymaga decyzji (dziś / skrzynka / porzuć).
- FR-004: Użytkownik może wybrać 1–3 priorytety dnia i listę „jeśli starczy czasu”; plan zapisuje się w notatce dziennej w vaulcie, z linkami do zgłoszeń i zadań. Priority: must-have
  > Socrates: Kontrargument: „dwa źródła prawdy” — plan w notatce dziennej i pole `Today` w `Issues/`/`Tasks/` mogą się rozjechać. Rozstrzygnięcie: jedynym źródłem „co dziś” jest notatka dzienna; pole `Today` jest pochodną (aktualizowane razem) albo pomijane.
- FR-005: Użytkownik ma dziennik dnia w notatce dziennej, który zasila się sam (zamknięte zgłoszenia i zadania, czas pracy, zakończone bloki skupienia); ręcznie dopisuje tylko własne wpisy. Priority: must-have
  > Socrates: Kontrargument: „kolejna ręczna lista, którą porzucisz”. Rozstrzygnięcie: dziennik zasila się automatycznie; ręcznie tylko myśli.
- FR-006: Użytkownik może skrótem globalnym z dowolnego miejsca w Windows otworzyć okienko szybkiego wpisu, wybrać przestrzeń (Praca/Prywatne; domyślnie Praca, gdy biegnie czas pracy) i zapisać myśl do dziennika dnia albo skrzynki. Priority: must-have
  > Socrates: Kontrargument: „wpis bez przestrzeni” — myśl prywatna ląduje w vaulcie pracy. Rozstrzygnięcie: okienko ma przełącznik przestrzeni z domyślną wartością zależną od czasu pracy.
- FR-007: Użytkownik może przejrzeć skrzynkę i rozdzielić każdą pozycję na dziś, jutro albo do usunięcia; niepusta skrzynka pojawia się w porannym planie. Priority: must-have
  > Socrates: Kontrargument: „cmentarz myśli” — skrzynka się zapełnia i nikt jej nie czyści. Rozstrzygnięcie: niepusta skrzynka jest częścią porannego planu.
- FR-008: Użytkownik może zobaczyć tydzień: wykonanie planu dzień po dniu i szkic podsumowania tygodnia. Priority: nice-to-have
  > Socrates: Kontrargument: „dużo pracy, mało wartości” — widok tygodnia nie wspiera żadnego kryterium Primary. Rozstrzygnięcie: obniżony do nice-to-have.
- FR-009: Użytkownik może przeprowadzić zamknięcie dnia: co zrobione, los niezrobionych (jutro / skrzynka / porzuć), jedno zdanie o dniu, szkic jutra. Priority: must-have
  > Socrates: Kontrargument: „bez domknięcia dziennik i plan nigdy się nie domykają”. Rozstrzygnięcie: podniesiony z nice-to-have do must-have.

### Zgłoszenia i zadania
- FR-010: Użytkownik ma przypisane zgłoszenia i MR-y z GitLaba synchronizowane do vaultu pracy automatycznie w interwale z ustawień oraz na żądanie („synchronizuj teraz”), bez ręcznego uruchamiania komendy. Priority: must-have
  > Socrates: Kontrargument: „częstotliwość” — za rzadko = spóźnione zgłoszenia, za często = obciążenie. Rozstrzygnięcie: interwał w ustawieniach + synchronizacja na żądanie.
- FR-011: Użytkownik dostaje powiadomienie Windows o nowym przypisanym zgłoszeniu lub MR z akcją „dodaj do listy”; gdy nowych jest więcej niż kilka naraz, dostaje jedno zbiorcze powiadomienie. Priority: must-have
  > Socrates: Kontrargument: „burza powiadomień” — po urlopie 20 powiadomień naraz. Rozstrzygnięcie: powyżej kilku nowych jedno zbiorcze powiadomienie.
- FR-012: Użytkownik może z listy zgłoszeń wciągnąć zgłoszenie na dzisiejszą listę rzeczy do zrobienia. Priority: must-have
  > Socrates: Kontrargument: „dubluje akcję z FR-011”. Rozstrzygnięcie: zostaje, bo nie każde zgłoszenie przychodzi przez powiadomienie (np. dawniej przypisane).
- FR-013: Użytkownik może utworzyć zadanie z szablonu w vaulcie pracy i zmieniać jego status. Priority: must-have
  > Socrates: Kontrargument: „tylko praca?” — zadania prywatne nie mają miejsca. Rozstrzygnięcie: początkowo zadania w obu przestrzeniach; w fazie 5 użytkownik ustalił, że projekty prywatne opierają się na zgłoszeniach GitHub, więc zadania prywatne z szablonu wydzielono do FR-025 (nice-to-have).
- FR-014: Użytkownik może otworzyć notatkę zgłoszenia lub zadania w Obsidianie jednym kliknięciem, we właściwym vaulcie; gdy to niemożliwe, aplikacja mówi to wprost. Priority: must-have
  > Socrates: Kontrargument: „zły vault” — Obsidian otwiera notatkę w złym vaulcie albo nic się nie dzieje. Rozstrzygnięcie: otwiera właściwy vault albo jawny komunikat.
- FR-015: Użytkownik może zobaczyć w przestrzeni Prywatne zgłoszenia i PR-y z GitHuba, notatki z vaultu prywatnego i stan projektów z `context/` w repozytoriach oraz wciągnąć otwarte zgłoszenie GitHub do prywatnej sekcji planu dnia (zapis w notatce dziennej vaultu prywatnego) — bez synchronizacji zgłoszeń do vaultu i bez powiadomień o nowych. Priority: must-have
  > Socrates: Kontrargument: „podwaja zakres” — drugie źródło zgłoszeń i WSL podwajają pracę. Rozstrzygnięcie: w v1 przestrzeń Prywatne nie synchronizuje i nie powiadamia; w fazie 5 doprecyzowano, że zgłoszenia GitHub są źródłem kandydatów do planu prywatnego.
- FR-025: Użytkownik może utworzyć zadanie z szablonu w vaulcie prywatnym i zmieniać jego status. Priority: nice-to-have
  > Socrates: Wydzielone z FR-013 w fazie 5: projekty prywatne opierają się na zgłoszeniach GitHub, nie na zadaniach w Obsidianie.

### Czas pracy, przerwy, bloker
- FR-016: Użytkownik może kliknięciem rozpocząć i zakończyć czas pracy oraz zobaczyć sumę dnia i miesiąca; ewidencja czasu ma jednego właściciela zapisu, a aplikacja go uruchamia i czyta. Priority: must-have
  > Socrates: Kontrargument: „dwa zapisy czasu” — aplikacja i istniejąca ewidencja liczą czas osobno i się rozjeżdżają. Rozstrzygnięcie: jedna ewidencja, jeden właściciel zapisu.
- FR-017: Użytkownik dostaje przypomnienie, gdy jest pora pracy, a czas nie biegnie, oraz gdy czas biegnie po godzinach pracy — tylko w dni robocze z ustawień, jedno przypomnienie na zdarzenie. Priority: must-have
  > Socrates: Kontrargument: „fałszywe alarmy” przy elastycznych godzinach. Rozstrzygnięcie: tylko dni robocze, jedno przypomnienie, nie seria.
- FR-018: Użytkownik dostaje w czasie pracy przypomnienia o mikroprzerwach i dłuższych przerwach w interwałach z ustawień, może je odłożyć albo pominąć, a jednym kliknięciem wstrzymać przerwy na N minut („nie teraz”, np. na spotkanie). Priority: must-have
  > Socrates: Kontrargument: „spotkania” — przerwa wyskakuje w trakcie rozmowy. Rozstrzygnięcie: wstrzymanie przerw na N minut jednym kliknięciem.
- FR-019: Użytkownik może uruchomić blok skupienia (pomodoro) przy zadaniu; po jego końcu dostaje powiadomienie o przerwie, a zakończony blok dopisuje się do dziennika dnia z nazwą zadania. Priority: must-have
  > Socrates: Kontrargument: „bez śladu” — blok nie zostawia informacji, nad czym była praca. Rozstrzygnięcie: blok dopisuje się do dziennika dnia.
- FR-020: Użytkownik może w interfejsie graficznym zdefiniować listy blokowanych stron WWW i programów oraz harmonogram blokady; w tym czasie są one niedostępne. Priority: must-have
  > Socrates: Kontrargument: „ingerencja w system” — blokowanie wymaga podwyższonych uprawnień i zmian w systemie. Rozstrzygnięcie: akceptowane; aplikacja prosi o zgodę jawnie i cofa zmiany przy wyłączeniu blokady.
- FR-021: Użytkownik może wyłączyć blokadę w każdej chwili po krótkim potwierdzeniu. Priority: must-have
  > Socrates: Kontrargument: „za łatwe” — jeden klik niweczy sens blokera. Rozstrzygnięcie: wyłączenie wymaga krótkiego potwierdzenia, ale zawsze jest możliwe.

### Ustawienia i zasobnik
- FR-022: Użytkownik może ustawić w aplikacji ścieżki vaultów, repozytoriów i narzędzi dla każdej przestrzeni, listę wykluczonych ścieżek oraz listę wyróżnianych etykiet zgłoszeń; przy pierwszym uruchomieniu prowadzi go przez to kreator. Priority: must-have
  > Socrates: Kontrargument: „pusty start” — pierwsze uruchomienie bez konfiguracji daje pustą aplikację. Rozstrzygnięcie: kreator pierwszego uruchomienia.
- FR-023: Użytkownik może ustawić godziny powiadomień i dni robocze oraz oznaczyć dzień wolny/urlop, który wyłącza powiadomienia, przerwy i bloker pracy. Priority: must-have
  > Socrates: Kontrargument: „urlop i święta” — aplikacja nie wie o urlopie i nagabuje. Rozstrzygnięcie: tryb dnia wolnego.
- FR-024: Użytkownik może z menu zasobnika rozpocząć i zakończyć czas pracy oraz wstrzymać powiadomienia aplikacji na określony czas (np. 1 h / do jutra), po którym wracają same. Priority: must-have
  > Socrates: Kontrargument: „zapomniane wstrzymanie” — nic już nie przypomina. Rozstrzygnięcie: wstrzymanie zawsze na czas określony.

## User Stories

### US-01: Poranny plan dnia

- **Given** komputer został właśnie włączony, a w GitLabie są nowe przypisane zgłoszenia
- **When** użytkownik klika powiadomienie „Ułóż plan dnia”
- **Then** widzi „Dziś” z przypisanymi zgłoszeniami (już pobranymi do vaultu), zadaniami i niezrobionymi pozycjami z wczoraj, wybiera 1–3 priorytety, a plan zapisuje się w vaulcie pracy

#### Acceptance Criteria
- Zgłoszenia są aktualne bez ręcznego uruchamiania synchronizacji.
- Nie da się wybrać więcej niż 3 priorytetów.
- Plan jest widoczny w Obsidianie jako zwykła notatka.
- Gdy ścieżka vaultu nie istnieje, aplikacja mówi to wprost zamiast pokazywać pusty plan.

### US-02: Nowe zgłoszenie w ciągu dnia

- **Given** użytkownik ma ułożony plan dnia, a w GitLabie zostaje mu przypisane nowe zgłoszenie
- **When** pojawia się powiadomienie Windows i użytkownik klika „dodaj do listy”
- **Then** zgłoszenie trafia na listę „jeśli starczy czasu”; priorytety dnia pozostają nietknięte, a o ewentualnym awansie zgłoszenia na priorytet decyduje użytkownik

#### Acceptance Criteria
- Zgłoszenie jest w vaulcie, zanim pojawi się powiadomienie.
- Kliknięcie „dodaj do listy” nie zmienia 1–3 priorytetów.

## Business Logic

Każdego dnia roboczego aplikacja proponuje uporządkowaną listę kandydatów na dziś, wymusza na każdej pozycji decyzję (priorytet, jeśli starczy czasu, skrzynka, porzuć) i zaczepia albo blokuje użytkownika wyłącznie wtedy, gdy wynika to ze stanu jego dnia.

**Wejścia i rekomendacja.** W przestrzeni Praca kandydatami są własne zadania użytkownika (na nich głównie pracuje) oraz odświeżane zgłoszenia i MR-y przypisane do niego. Zgłoszenia z etykietami wskazanymi w ustawieniach (np. `awaiting_verification`) są wyróżnione i wyżej w rekomendacji. W przestrzeni Prywatne kandydatami są otwarte zgłoszenia GitHub z repozytoriów użytkownika. Do tego dochodzą niezrobione pozycje z poprzedniego dnia i niepusta skrzynka. Kolejność wynika z priorytetu i terminu, milestone'u oraz świeżości przypisania (nowe od ostatniego planu wyżej). Kolejność jest **rekomendacją**: aplikacja podsuwa, użytkownik decyduje.

**Workflow pozycji.** Jeden ekran „Dziś” ma dwie sekcje, Praca i Prywatne. Limit 1–3 priorytetów jest wspólny dla całego dnia. Każda sekcja zapisuje plan w notatce dziennej swojego vaultu. Notatka dzienna jest jedynym źródłem „co dziś”. Każda pozycja dostaje decyzję: priorytet, jeśli starczy czasu, skrzynka albo porzuć. Zaległość nie przechodzi na kolejny dzień bez decyzji: rano decyduje się „dziś / skrzynka / porzuć”, a zamknięcie dnia rozstrzyga „jutro / skrzynka / porzuć”. Zgłoszenie dodane w ciągu dnia z powiadomienia trafia na „jeśli starczy czasu” i nie rusza priorytetów.

**Rytm.** Aplikacja zaczepia użytkownika tylko wtedy, gdy wynika to ze stanu dnia:
- przy pierwszym włączeniu lub odblokowaniu komputera w dniu roboczym, jeśli nie ma planu;
- gdy w godzinach pracy czas pracy nie biegnie albo biegnie po godzinach (jedno przypomnienie na zdarzenie);
- w interwałach przerw, gdy biegnie czas pracy, chyba że użytkownik wstrzymał je („nie teraz”) albo trwa blok skupienia;
- po zakończonym bloku skupienia;
- przy nowych przypisanych zgłoszeniach (zbiorczo, gdy jest ich kilka).

Bloker działa według harmonogramu z ustawień. Dzień wolny wyłącza powiadomienia, przerwy i bloker pracy.

## Non-Functional Requirements

- **Widoczny stan zdrowia.** Każda awaria źródła (nieudana synchronizacja, narzędzie wylogowane, brak skonfigurowanej ścieżki) jest widoczna w zasobniku w ≤ 1 min od wystąpienia. Aplikacja nigdy nie pokazuje pustego planu ani pustej listy zamiast błędu.
- **Nienaruszalność notatek.** Zapis aplikacji zmienia wyłącznie wskazaną linię lub sekcję notatki; reszta pliku pozostaje bajt w bajt identyczna. Nic nie jest usuwane trwale: usunięcie trafia do kosza vaultu.
- **Dane pracy nie opuszczają maszyny.** Żadna treść z vaultu pracy ani ze zgłoszeń firmowych nie jest przez aplikację wysyłana poza komputer ani przenoszona do przestrzeni prywatnej, repozytorium, testów czy logów.
- **Zero sekretów.** Aplikacja, jej ustawienia i repozytorium nie zawierają żadnych haseł, tokenów ani kluczy.
- **Lekkość w tle.** W bezczynności w zasobniku aplikacja nie obciąża odczuwalnie komputera (orientacyjnie < 1% CPU, < 150 MB RAM).

Rozważone i **nieprzyjęte** jako NFR w v1: czas otwarcia szybkiego wpisu < 300 ms, maksymalne opóźnienie powiadomienia o nowym zgłoszeniu, gwarantowana odwracalność blokera w ≤ 1 min (zachowanie blokera opisują FR-020/FR-021).

## Non-Goals

### Funkcjonalne
- **Brak własnego edytora notatek.** Pełna edycja Markdownu, zmiana nazw i przenoszenie notatek zostają w Obsidianie; aplikacja daje „Otwórz w Obsidianie” (FR-014).
- **Brak telefonu i wielu maszyn.** Jeden komputer z Windows 11; bez wersji mobilnej i bez synchronizacji między komputerami.
- **Brak wielu użytkowników.** Bez kont, ról i funkcji zespołowych. To narzędzie osobiste i skala nie jest celem (odpowiedź na pytanie o 100x: „nic się nie łamie, bo to narzędzie osobiste”).
- **Poza MVP (nice-to-have):** widok tygodnia (FR-008) i zadania prywatne z szablonu (FR-025).

### Rozważone i świadomie NIE wykluczone
Te pozycje nie są non-goals, ale żadne FR ich nie obejmuje, więc w v1 nie są budowane z braku wymagania, a nie z zakazu: zapis do GitLaba/GitHuba (status, komentarze), automatyczny pomiar aktywności (typu ActivityWatch), integracje z Domowym Skarbnikiem i Menu Makerem, serwer MCP / asystent AI, bloker odporny na zdeterminowane obejście.

## Forward: tech-stack

Informacyjnie, nie jest częścią PRD. Decyzje i fakty techniczne z seeda (`notes/idea.md`) do przejęcia przy wyborze stacku:
- **Decyzja użytkownika (seed, wariant D):** aplikacja desktopowa Tauri nad CLI i plikami, lokalnie. Odrzucone: aplikacja na VPS-ie, API na VPS-ie z klientem, samo rozszerzenie Helper App bez okna.
- **Decyzja użytkownika (seed):** dostęp do firmowego GitLaba i prywatnego GitHuba wyłącznie przez zalogowane CLI (`glab`, `gh`); bez serwera i bez własnych tokenów.
- **Decyzja użytkownika (seed):** makieta w stylu Atramentu (system designu Domowego Skarbnika). Makieta: https://claude.ai/artifact/UiEpZrnscd3716PPtte8nf (prywatna).
- **Fakty środowiska:** w Windowsie są `cargo`, `node`, `git`, `glab`, `php`; `gh` jest tylko w WSL. Repozytoria prywatne i `context/` są w WSL (`~/my-projects`).
- **Fakt:** istnieje rozpoczęty projekt Tauri 2 + Vue 3 + Pinia + Tailwind („Ed”, `D:\my-projects\ai-assistant`), zbudowany wokół czatu. Rekomendacja z seeda (niepotwierdzona): nowe repo, z „Eda” tylko konfiguracja Tauri i Vue.
- **Fakt:** Helper App (`D:\panelalpha\panelalpha-helper`, PHP 8.2, Symfony Console) ma komendy start/koniec pracy, synchronizację zgłoszeń (przez `glab` od 2026-10-26), raport mailowy i sprzątanie skrzynek.
- **Wymaganie bezpieczeństwa (seed):** prawdziwe dane z vaultów nie trafiają do repozytorium, testów, fixture'ów ani CI; testy używają małego zmyślonego vaultu.
- **Bloker (FR-020):** wymaga podwyższonych uprawnień / zmian w systemie; użytkownik to zaakceptował pod warunkiem jawnej zgody i cofania zmian.

## Forward: technical-roadmap

Informacyjnie, nie jest częścią PRD. Rekomendacje agenta z seeda, niepotwierdzone w tej sesji, chyba że zaznaczono inaczej:
- Kolejność budowy (seed): okno, zasobnik, skrót i ustawienia przestrzeni → „Dziś” z szybkim wpisem → zgłoszenia z CLI → czas pracy i powiadomienia → zamknięcie dnia i tydzień. Do aktualizacji o przerwy, blok skupienia i bloker.
- Własność plików (seed): `Issues/*` zapisuje mechanizm synchronizacji; `Tasks/*` użytkownik i aplikacja; `Management/Timeline.md` ewidencja czasu (Helper App); nowe `Dziennik/RRRR/RRRR-MM-DD.md` i `Skrzynka.md` aplikacja; `context/` w repo tylko odczyt. Zmiana pola w nagłówku podmienia jedną linię. **Potwierdzone w sesji:** jeden właściciel zapisu dla ewidencji czasu (FR-016). Dla synchronizacji zgłoszeń (FR-010) użytkownik w rundzie Sokratesa wybrał „interwał + na żądanie”, a nie regułę jednego właściciela — kwestia otwarta.
- Fakty o vaulcie pracy (seed): `Issues/` (pola `Status` jako lista, `Milestone`, `Priority`, `Labels`, `Today`), `Tasks/` (`Status` pojedynczy, `Priority`, `Due`, `Issue`, `Today`, `Done`), `Management/Timeline.md`, `DASHBOARD.md`, `ToDo List.md`, `Notatki bieżące.md`. Oba vaulty mają gita bez remote; obsidian-git commituje vault pracy co 3 h.
- Architektura (seed): ustawienia w `%APPDATA%`, pamięć podręczna wyników CLI odbudowywalna; obserwowanie zmian w vaultach; jedna instancja.

## Open Questions

1. **Nazwa produktu** — robocza „Nawigator dnia”; kandydaci: Sternik, Skryba, Kompas, Brulion, Strażnik dnia. Owner: użytkownik. Blokuje: nie.
2. **Rola Helper App** — czy pozostaje jedynym właścicielem zapisu zgłoszeń (`Issues/`) i ewidencji czasu (`Timeline.md`), wywoływanym przez aplikację? FR-016 wymaga jednego właściciela ewidencji czasu, ale nie przesądza którego; dla zgłoszeń (FR-010) reguła jednego właściciela nie została przyjęta — ryzyko równoległego zapisu z ręczną komendą w Obsidianie. Owner: użytkownik, przed planem implementacji.
3. **Ewidencja czasu docelowo** — czy start i koniec pracy zapisywać w notatce dziennej, a `Timeline.md` generować? (seed, pytanie 5). Owner: użytkownik.
4. **Pole `Today` w zgłoszeniach i zadaniach** — aktualizować razem z notatką dzienną jako pochodną, czy pomijać? (FR-004). Owner: użytkownik.
5. **Stare listy** — co z `DASHBOARD.md`, `ToDo List.md` i `Notatki bieżące.md` po wdrożeniu „Dziś” i skrzynki? (seed, pytanie 8). Owner: użytkownik.
6. **Skrót globalny i domyślne godziny** — propozycja Ctrl+Alt+Spacja; domyślne godziny pracy, interwały przerw, długość bloku skupienia, próg „kilku” zgłoszeń dla powiadomienia zbiorczego. Owner: użytkownik.
7. **Repozytorium publiczne (portfolio)?** — jeśli tak, w repo nie mogą się pojawić nazwy hostów ani dane z pracy (seed, pytanie 9). Owner: użytkownik.
8. **Funkcje rozważone, nie wykluczone, bez FR** — zapis do GitLaba/GitHuba, pomiar aktywności, integracje (Domowy Skarbnik, Menu Maker), MCP/AI, odporność blokera. Czy którakolwiek ma trafić do v1? Owner: użytkownik.

## Quality cross-check

Uruchomiona 2026-11-20. Wszystkie elementy obecne: Access Control, Business Logic (reguła w jednym zdaniu), artefakty projektu, potwierdzenie kosztu terminu (12 tygodni), Non-Goals; Preserved behavior n/a (greenfield). Status: accepted. Brak luk do odzwierciedlenia; 8 otwartych pytań z `## Open Questions` przechodzi do PRD bez zmian, najważniejsze #2 (właściciel zapisu `Issues/` i ewidencji czasu).
