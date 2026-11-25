# Konfiguracja Tauri: CSP, wygląd okna i metadane instalatora — Implementation Plan

## Overview

Domykamy konfigurację Tauri z issue #21 (https://github.com/sebastian-kostecki/day-navigator/issues/21):
- włączamy wąską politykę CSP i `freezePrototype`, zamykając odłożony follow-up z `source-health-status`;
- nadajemy oknu ciemny motyw, tło w kolorze aplikacji i wyśrodkowanie przy pierwszym starcie;
- dodajemy link do projektu we wpisie instalatora.

Zmiana dotyczy konfiguracji i dokumentacji, bez kodu Rust ani TS.

## Current State Analysis

- `src-tauri/tauri.conf.json:22-24`: `"security": { "csp": null }`, brak `freezePrototype`.
- `src-tauri/tauri.conf.json:11-20`: okno bez `theme`, `backgroundColor` i `center`. Okno startuje `visible:false`, a `show_main` (`src-tauri/src/lib.rs:171-172`, `src-tauri/src/runtime.rs:123-130`) pokazuje je po przywróceniu stanu przez window-state.
- `src-tauri/tauri.conf.json:26-37`: bundle bez `homepage` i `publisher`. Domyślny publisher to `"sebas"`, czyli drugi segment identifiera.
- Frontend nie potrzebuje niczego poza same-origin i IPC (`research.md` §2):
  - 4 atrybuty `style={…}` w `HoursBar` i `LabelChip` zapisują przez CSSOM;
  - fonty są lokalne, obrazków i zewnętrznych żądań brak;
  - 3 formularze wywołują `preventDefault`.
- W `tauri dev` (desktop, `devUrl`) Tauri nie stosuje CSP. Nagłówek trafia tylko na protokół `tauri` (`research.md` §1).

## Desired End State

- Zbudowana aplikacja serwuje stronę z nagłówkiem CSP. Wszystkie widoki są ostylowane, stan źródeł aktualizuje się na żywo, linki do GitLab/GitHub otwierają się w przeglądarce, a konsola nie pokazuje naruszeń CSP ani błędu „IPC custom protocol failed”.
- `Object.prototype` jest zamrożony, a aplikacja działa bez błędów w konsoli, w dev i w buildzie.
- Pasek tytułu jest ciemny niezależnie od motywu systemu. Przed pierwszym renderem okno ma kolor `--bg`, nie biały. Przy braku zapisanego stanu okno pojawia się na środku ekranu.
- Wpis „Nawigator dnia” w Ustawienia > Aplikacje ma link do repozytorium. Publisher pozostaje `"sebas"`, a aktualizacja instalatorem przebiega jak dotąd.
- `AGENTS.md` i `tokens.css` mówią, że `tauri.conf.json` ma kopię `--bg` i że nowe zasoby lub połączenia webview wymagają zmiany CSP.

### Key Discoveries:

- CSP jest wybierane jako `devCsp ?? csp` w dev i `csp` w release (`tauri-codegen-2.7.6/src/context.rs:162-174`). Przy `devUrl` nie ma osadzonych zasobów ani nagłówka (`context.rs:176-180`, `tauri-2.12.7/src/protocol/tauri.rs:175-185`).
- Tauri dopisuje do `script-src` wartość `'self'` i hashe plików JS. `style-src` zostawia bez zmian, bo build Vite nie ma inline `<style>` (`research.md` §1).
- IPC zablokowane przez CSP przechodzi po cichu na `postMessage` i loguje błąd (`scripts/ipc-protocol.js:58-67`). Dlatego `connect-src ipc: http://ipc.localhost` jest obowiązkowe.
- `center` przegrywa z zapisaną pozycją window-state, chyba że zapisany monitor zniknął (`tauri-plugin-window-state-2.5.0/src/lib.rs:206-225`).
- `homepage` trafia do `URLInfoAbout`, `URLUpdateInfo` i `HelpLink` wpisu w Aplikacjach (`installer.nsi:760-764`).

## What We're NOT Doing

- Nie ustawiamy ani nie zmieniamy `bundle.publisher`. Zmiana przeniosłaby klucz `HKCU\Software\<publisher>\Nawigator dnia` i zepsuła domyślną ścieżkę aktualizacji (decyzja z `app-metadata` potwierdzona w researchu).
- Nie ustawiamy `bundle.license`, `licenseFile`, `shortDescription` ani `longDescription`. W NSIS nie mają efektu albo dodają stronę licencji.
- Nie ustawiamy `devCsp`, bo na desktopie z `devUrl` nie jest stosowany.
- Nie ustawiamy `dangerousDisableAssetCspModification`. Polityka nie zawiera `'unsafe-inline'`, więc nie ma czego chronić przed nonce/hashami.
- Nie włączamy protokołu asset ani `data:` w CSP. Nic ich nie używa.
- Polski instalator NSIS, `build.removeUnusedCommands` i podpisywanie (#13) są poza zakresem.
- Nie zmieniamy `identifier`, `productName`, `useHttpsScheme` ani `installMode`.
- Nie piszemy stałej procedury testu instalatora w README.

## Implementation Approach

Dwie fazy rosnącego ryzyka:
- Faza 1 dodaje klucze, które da się w pełni sprawdzić w `tauri dev`.
- Faza 2 włącza CSP i `freezePrototype`. CSP działa tylko w zbudowanej aplikacji, więc jej weryfikacja wymaga buildu.

Każda faza to jeden commit z bramkami z `AGENTS.md`.

## Critical Implementation Details

- **Weryfikacja CSP wymaga buildu, nie dev.** Najprościej zbudować wersję debug z produkcyjnymi zasobami: `npm run tauri build -- --debug --no-bundle`. Daje to `src-tauri/target/debug/day-navigator.exe`. Oczekujemy, że ma DevTools (debug), serwuje zasoby protokołem `tauri` z nagłówkiem CSP i nie startuje Vite. Jeśli DevTools się nie otworzą, sprawdzenie konsoli zastępuje obserwacja zachowania (style, live update, linki) w zainstalowanym buildzie release.
- **Dane.** Build debug, dev i zainstalowana aplikacja współdzielą `%APPDATA%\com.sebas.day-navigator`. Przed uruchomieniem buildu skopiuj `day-navigator.db` (tak jak w `own-tasks/plan.md`), a zainstalowaną aplikację zamknij z traya. Single-instance zablokuje drugą kopię.

## Phase 1: Okno i metadane instalatora

### Overview

Ciemny motyw okna, tło `--bg` przed pierwszym renderem, wyśrodkowanie przy braku zapisanego stanu, link do repozytorium we wpisie instalatora. Do tego dokumentacja powiązania kolorów.

### Changes Required:

#### 1. Okno `main`

**File**: `src-tauri/tauri.conf.json`

**Intent**: Usunąć białe tło i jasny pasek tytułu przed i w trakcie startu oraz wyśrodkować okno przy pierwszym uruchomieniu, zanim window-state zapisze pozycję.

**Contract**: W jedynym obiekcie `app.windows[0]` dochodzą trzy klucze: `"theme": "Dark"`, `"backgroundColor": "#0f1115"` (wartość `--bg` z `src/styles/tokens.css:53`) i `"center": true`. Istniejące klucze, w tym `visible: false`, zostają bez zmian.

#### 2. Link do projektu w instalatorze

**File**: `src-tauri/tauri.conf.json`

**Intent**: Wpis w Ustawienia > Aplikacje dostaje link do repozytorium.

**Contract**: `bundle.homepage` = `"https://github.com/sebastian-kostecki/day-navigator"`, ten sam URL co `repository` w `src-tauri/Cargo.toml:7`. `publisher` pozostaje nieustawiony.

#### 3. Powiązanie koloru tła

**File**: `src/styles/tokens.css`, `AGENTS.md`

**Intent**: Duplikacja `#0f1115` w `tauri.conf.json` ma być widoczna dla kogoś, kto zmienia `--bg`.

**Contract**:
- W `tokens.css` dochodzi komentarz przy `--bg` (linia 53): wartość jest skopiowana do `app.windows[0].backgroundColor` w `src-tauri/tauri.conf.json`, zmieniaj obie.
- W `AGENTS.md`, w punkcie „Styles” sekcji Frontend Conventions, dochodzi zdanie o tym samym wyjątku: `tokens.css` jest jedynym miejscem na wartości, a jedyną kopią jest `backgroundColor` okna w `tauri.conf.json`.

### Success Criteria:

#### Automated Verification:

- Config przechodzi walidację Tauri i build Rust: `cargo build` w `src-tauri/`
- Bramki przed commitem przechodzą: `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`

#### Manual Verification:

- `npm run tauri dev`: pasek tytułu jest ciemny także przy jasnym motywie Windows, a przy starcie nie widać białego tła
- Po usunięciu pliku stanu window-state w katalogu konfiguracji aplikacji okno startuje na środku ekranu w rozmiarze 1280×800, a po przesunięciu i ponownym uruchomieniu wraca na zapisaną pozycję

**Implementation Note**: Po fazie i zielonych bramkach zatrzymaj się na ręczne potwierdzenie przed fazą 2.

---

## Phase 2: CSP i freezePrototype

### Overview

Włączenie wąskiej polityki CSP i zamrożenia `Object.prototype` oraz reguła w `AGENTS.md`, która pilnuje polityki przy przyszłych zmianach.

### Changes Required:

#### 1. Polityka CSP i freezePrototype

**File**: `src-tauri/tauri.conf.json`

**Intent**: Zablokować ładowanie i wykonywanie czegokolwiek spoza aplikacji. Ogranicza to skutki ewentualnego wstrzyknięcia treści, które dziś ma dostęp do wszystkich komend (`work-tasks-from-template/reviews/impl-review.md:48`). Do tego ochrona przed prototype pollution.

**Contract**: `app.security.csp` zastępuje `null` dokładnie tą polityką (string):

```
default-src 'self'; connect-src ipc: http://ipc.localhost; object-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'
```

oraz `app.security.freezePrototype: true`. Bez `devCsp`, bez `'unsafe-inline'`, bez `asset:` i `data:`. `script-src`, `style-src` i `font-src` dziedziczą z `default-src`; Tauri sam dopisze hashe skryptów do `script-src`.

#### 2. Reguła dla przyszłych zmian

**File**: `AGENTS.md`

**Intent**: Następny agent, który doda obrazek, zewnętrzne żądanie z webview, inline `<style>`, plugin z własnym protokołem lub protokół asset, ma wiedzieć, że musi zaktualizować CSP i sprawdzić je w buildzie, nie w dev.

**Contract**: Jeden punkt w sekcji „Tauri 2 Only” mówiący, że:
- `app.security.csp` w `tauri.conf.json` dopuszcza tylko same-origin i IPC;
- `freezePrototype` jest włączony;
- nowy rodzaj zasobu lub połączenia webview wymaga zmiany polityki;
- CSP działa tylko w zbudowanej aplikacji (`npm run tauri build -- --debug --no-bundle`), nie w `tauri dev`.

### Success Criteria:

#### Automated Verification:

- Config przechodzi walidację Tauri i build Rust: `cargo build` w `src-tauri/`
- Build debug z produkcyjnymi zasobami powstaje: `npm run tauri build -- --debug --no-bundle`
- Bramki przed commitem przechodzą: `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`

#### Manual Verification:

- `npm run tauri dev` działa jak przed zmianą, a konsola DevTools nie ma `TypeError` od `freezePrototype`
- Build debug (`src-tauri/target/debug/day-navigator.exe`): w DevTools odpowiedź `index.html` ma nagłówek `Content-Security-Policy` z polityką i hashami w `script-src`. Konsola nie ma naruszeń CSP ani „IPC custom protocol failed”
- Build debug: wszystkie widoki są ostylowane z fontami, łącznie z paskiem godzin i etykietami GitLab. Stan źródeł aktualizuje się na żywo, licznik czasu pracy działa, linki GitLab/GitHub otwierają się w przeglądarce, a formularze zadań własnych i wpisów czasu pracy zapisują
- Zainstalowany instalator release (`npm run tauri build -- --bundles nsis`) na istniejącą instalację: aktualizacja przebiega bez błędu przy domyślnej opcji, wpis w Aplikacjach ma link do repozytorium i publisher „sebas”, a aplikacja działa jak w buildzie debug

**Implementation Note**: Po fazie i zielonych bramkach zatrzymaj się na ręczne potwierdzenie.

---

## Testing Strategy

### Unit Tests:

- Brak nowych. Zmiana nie dotyka logiki; istniejące `cargo test` jest bramką regresji.

### Integration Tests:

- Brak. UI nie ma test runnera, a CSP działa tylko w zbudowanej aplikacji.

### Manual Testing Steps:

1. Zrób kopię `%APPDATA%\com.sebas.day-navigator\day-navigator.db` i zamknij zainstalowaną aplikację z traya.
2. `npm run tauri dev`: pasek tytułu, brak białego tła, wyśrodkowanie po usunięciu pliku stanu, konsola czysta.
3. `npm run tauri build -- --debug --no-bundle`, uruchom `src-tauri/target/debug/day-navigator.exe`, otwórz DevTools: nagłówek CSP, konsola, przejście przez wszystkie widoki, link, formularz.
4. `npm run tauri build -- --bundles nsis`, uruchom instalator na istniejącą instalację z domyślną opcją, sprawdź wpis w Aplikacjach i działanie.

## Performance Considerations

Brak. Hashe skryptów są liczone przy buildzie.

## Migration Notes

Brak migracji danych. Publisher i `productName` się nie zmieniają, więc klucze rejestru instalatora i katalog instalacji zostają te same. Wycofanie to revert commitu.

## References

- Research: `context/changes/tauri-config-hardening/research.md`
- Issue: https://github.com/sebastian-kostecki/day-navigator/issues/21
- Odłożony follow-up CSP: `context/archive/2026-11-20-source-health-status/follow-ups/review-fixes.md`
- Decyzja o publisherze: `context/archive/2026-11-24-app-metadata/plan.md:24,29`
- Tauri CSP: https://v2.tauri.app/security/csp/

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: Okno i metadane instalatora

#### Automated

- [x] 1.1 Config przechodzi walidację Tauri i build Rust: `cargo build` w `src-tauri/` — 46d593d
- [x] 1.2 Bramki przed commitem przechodzą: `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` — 46d593d

#### Manual

- [ ] 1.3 `npm run tauri dev`: pasek tytułu jest ciemny także przy jasnym motywie Windows, a przy starcie nie widać białego tła
- [ ] 1.4 Po usunięciu pliku stanu window-state okno startuje na środku ekranu w 1280×800, a po przesunięciu i restarcie wraca na zapisaną pozycję

### Phase 2: CSP i freezePrototype

#### Automated

- [x] 2.1 Config przechodzi walidację Tauri i build Rust: `cargo build` w `src-tauri/`
- [x] 2.2 Build debug z produkcyjnymi zasobami powstaje: `npm run tauri build -- --debug --no-bundle`
- [x] 2.3 Bramki przed commitem przechodzą: `npm run lint`, `npm run format:check`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`

#### Manual

- [ ] 2.4 `npm run tauri dev` działa jak przed zmianą, konsola bez `TypeError` od `freezePrototype`
- [ ] 2.5 Build debug: nagłówek CSP z hashami w `script-src`, konsola bez naruszeń CSP i bez „IPC custom protocol failed”
- [ ] 2.6 Build debug: widoki ostylowane, live update stanu źródeł, licznik, linki GitLab/GitHub i formularze działają
- [ ] 2.7 Instalator release na istniejącą instalację: aktualizacja bez błędu, link i publisher „sebas” w Aplikacjach, aplikacja działa
