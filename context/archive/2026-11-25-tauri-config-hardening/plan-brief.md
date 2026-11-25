# Konfiguracja Tauri: CSP, wygląd okna i metadane instalatora — Plan Brief

> Full plan: `context/changes/tauri-config-hardening/plan.md`
> Research: `context/changes/tauri-config-hardening/research.md`

## What & Why

Issue #21: `tauri.conf.json` ma tylko podstawowe klucze ze startera. CSP jest wyłączone od początku projektu (`"csp": null`). Dwa review z 2026-11-20 wskazały to jako lukę, a follow-up wciąż czeka. Przy okazji niewielkim kosztem poprawiamy start okna (ciemny pasek, brak białego tła, wyśrodkowanie) i wpis instalatora (link do projektu).

## Starting Point

- Okno startuje ukryte, a `show_main` pokazuje je po przywróceniu stanu przez window-state.
- Okno nie ma ustawionego motywu ani tła.
- Frontend używa wyłącznie zasobów własnych i IPC.
- W `tauri dev` Tauri nie stosuje CSP wcale, bo strona idzie prosto z Vite.

## Desired End State

- Zbudowana aplikacja wysyła wąski nagłówek CSP (same-origin + IPC) i ma zamrożony `Object.prototype`. Działa bez błędów w konsoli i bez zmian w zachowaniu.
- Okno ma ciemny pasek tytułu i tło `#0f1115` od pierwszej klatki, a bez zapisanego stanu startuje na środku.
- Wpis w Aplikacjach linkuje do repozytorium.
- Aktualizacje instalatorem działają jak dotąd.

## Key Decisions Made

| Decision | Choice | Why (1 sentence) | Source |
| --- | --- | --- | --- |
| `bundle.publisher` | Bez zmian (`"sebas"`, nieustawiony) | Zmiana przenosi klucz rejestru z katalogiem instalacji i psuje domyślną ścieżkę aktualizacji. | Research / Plan |
| `backgroundColor` | `#0f1115` + notka w `tokens.css` i `AGENTS.md` | Usuwa białe tło, a duplikacja `--bg` jest udokumentowana. | Plan |
| `license`, `shortDescription` | Pominięte | W NSIS nie mają efektu (zgodnie z `app-metadata`). | Research / Plan |
| `homepage` | `bundle.homepage` = URL repozytorium | Wszystko o bundlu w jednym pliku, który i tak zmieniamy. | Plan |
| `devCsp` | Nie ustawiamy | Na desktopie z `devUrl` Tauri nie stosuje CSP w dev. | Research |
| Polityka CSP | `default-src 'self'; connect-src ipc: http://ipc.localhost; object-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'` | Węższa niż w issue: asset, `data:` i `customprotocol:` nie są używane. | Research |
| Weryfikacja CSP | Build debug z DevTools + instalator release | CSP działa tylko w zbudowanej aplikacji. | Research |

## Scope

**In scope:**
- `tauri.conf.json`: `theme`, `backgroundColor`, `center`, `bundle.homepage`, `security.csp`, `security.freezePrototype`
- Komentarz przy `--bg` w `tokens.css`
- Dwie reguły w `AGENTS.md` (kopia koloru, utrzymanie CSP)

**Out of scope:**
- `publisher`, `license`, `shortDescription`, `licenseFile`, `devCsp`
- Polski instalator, `removeUnusedCommands`, podpisywanie (#13)
- Zmiany `identifier`, `productName`, `useHttpsScheme`, `installMode`

## Architecture / Approach

Tylko konfiguracja i dokumentacja. Faza 1 dodaje klucze sprawdzalne w `tauri dev`. Faza 2 włącza zabezpieczenia, których działanie widać dopiero w zbudowanej aplikacji (nagłówek CSP na protokole `tauri`).

## Phases at a Glance

| Phase | What it delivers | Key risk |
| --- | --- | --- |
| 1. Okno i metadane instalatora | Ciemny pasek, tło `--bg`, wyśrodkowanie, link w Aplikacjach | Niska; `theme` może nie przyciemnić paska na starszym Windows |
| 2. CSP i freezePrototype | Nagłówek CSP, zamrożony prototyp, reguła w `AGENTS.md` | Polityka blokuje coś, czego dev nie pokaże; wychwytuje to build debug |

**Prerequisites:** kopia `day-navigator.db` przed uruchomieniem buildów (wspólne dane z zainstalowaną aplikacją); zainstalowana poprzednia wersja do testu aktualizacji.
**Estimated effort:** ~1 sesja, 2 commity; większość czasu to buildy i ręczne sprawdzenia.

## Open Risks & Assumptions

- Zakładamy, że `tauri build --debug` serwuje zasoby protokołem `tauri` z CSP i ma DevTools. Jeśli DevTools nie będzie, zostaje obserwacja zachowania w buildzie release.
- Działanie `freezePrototype` z obecnymi zależnościami nie było testowane w runtime. Analiza nie znalazła patchowania prototypów.

## Success Criteria (Summary)

- Zainstalowana aplikacja działa jak wcześniej, a konsola nie pokazuje naruszeń CSP.
- Okno startuje ciemne, bez białego tła, wyśrodkowane przy pierwszym uruchomieniu.
- Aktualizacja instalatorem przebiega bez błędu, a wpis w Aplikacjach ma link do projektu.
