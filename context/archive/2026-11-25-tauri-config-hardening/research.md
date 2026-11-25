---
date: 2026-11-25T08:57:45+01:00
researcher: Claude (Opus 5.5) dla Sebastian Kostecki
git_commit: a67196cba206cd0296ca295c57a30a6d269ad35b
branch: develop
repository: day-navigator
topic: "Konfiguracja Tauri: CSP, freezePrototype, wygląd okna i metadane instalatora (issue #21)"
tags: [research, tauri, csp, security, window, nsis, bundle]
status: complete
last_updated: 2026-11-25
last_updated_by: Claude (Opus 5.5)
---

# Research: Konfiguracja Tauri: CSP, freezePrototype, wygląd okna i metadane instalatora (#21)

**Date**: 2026-11-25T08:57:45+01:00
**Researcher**: Claude (Opus 5.5) dla Sebastian Kostecki
**Git Commit**: a67196cba206cd0296ca295c57a30a6d269ad35b
**Branch**: develop
**Repository**: day-navigator

## Research Question

Co dokładnie zmienią klucze z issue #21 (https://github.com/sebastian-kostecki/day-navigator/issues/21) w `src-tauri/tauri.conf.json`: `app.security.csp` (+ ewentualnie `devCsp`), `app.security.freezePrototype`, okno `main` (`theme`, `backgroundColor`, `center`) oraz `bundle` (`publisher`, `homepage`, `license`, `shortDescription`)? Co może się zepsuć i jak to sprawdzić?

Wersje: tauri 2.12.7, tauri-codegen 2.7.6, tauri-utils 2.10.4, tauri-runtime-wry 2.12.5, wry 0.57.3, tauri-plugin-window-state 2.5.0, CLI 2.12.6 (tauri-bundler 2.10.6, `installer.nsi` pobrany z tagu `@tauri-apps/cli-v2.12.6` i porównany bajtowo z szablonem w binarce CLI), react-dom 19.3.1, vite 8.3.8.

## Summary

- **CSP jest bezpieczne do włączenia w buildzie produkcyjnym.** Produkcyjny frontend ładuje tylko jeden zewnętrzny moduł JS i jeden arkusz CSS, lokalne fonty woff2 i rozmawia z backendem wyłącznie przez IPC. Nie ma obrazków, iframe'ów, `eval`, inline `<style>` ani zewnętrznych żądań. Dwa miejsca z `style={…}` (`HoursBar`, `LabelChip`) piszą przez CSSOM, którego CSP nie blokuje.
- **W `tauri dev` na desktopie CSP w ogóle nie działa.** Gdy jest `devUrl`, okno ładuje `http://localhost:1420` bezpośrednio z Vite, a nagłówek CSP Tauri dokłada tylko na własnym protokole. `devCsp` niczego tu nie zmieni, więc nie trzeba go ustawiać. Pełny test CSP wymaga zainstalowanego buildu (lub `tauri build` + uruchomienia exe).
- **`freezePrototype`** to jedno `Object.freeze(Object.prototype)`, wstrzykiwane jako skrypt inicjalizujący w dev i w release. Obecne zależności nie modyfikują prototypów wbudowanych, więc ryzyko jest niskie. Wystarczy uruchomić dev i sprawdzić konsolę.
- **Okno:**
  - `theme: "Dark"` wymusza ciemny motyw okna i `prefers-color-scheme: dark` w WebView2.
  - `backgroundColor` przyjmuje hex `#0f1115`. Powiela on jednak wartość `--bg` z `tokens.css`, co kłóci się z regułą „tokens.css to jedyne miejsce na wartości”; trzeba to rozstrzygnąć w planie.
  - `center: true` działa tylko wtedy, gdy window-state nie przywraca pozycji: przy pierwszym uruchomieniu lub gdy zapisany monitor zniknął.
- **Bundle:**
  - `shortDescription` i `license` nie mają żadnego efektu w NSIS. Przydaje się tylko `homepage`: dopisuje „Więcej informacji”/link pomocy do wpisu w Aplikacjach.
  - **Zmiana `publisher` odwraca świadomą decyzję z `app-metadata`.** Według szablonu NSIS pierwsza aktualizacja po zmianie najpewniej pokaże błąd deinstalacji przy domyślnej opcji.
  - Rekomendacja: zostawić `publisher` bez zmian, chyba że użytkownik wybierze inaczej.

## Detailed Findings

### 1. CSP: jak Tauri 2.12 go stosuje

- **Wybór polityki.** Codegen bierze `devCsp ?? csp` w dev i `csp` w release (`tauri-codegen-2.7.6/src/context.rs:162-174`). Runtime robi to samo (`tauri-2.12.7/src/manager/mod.rs:361-373`).
- **Dev na desktopie omija CSP.**
  - `get_app_url` zwraca w dev `devUrl` (`manager/mod.rs:344-358`).
  - Przy `dev && devUrl` codegen nie osadza zasobów (`context.rs:176-180`).
  - Nagłówek `Content-Security-Policy` jest dokładany tylko w ścieżce protokołu `tauri` (`tauri-2.12.7/src/protocol/tauri.rs:175-185`); proxy dev istnieje wyłącznie dla `cfg(all(dev, mobile))`.
  - Wniosek: przy `npm run tauri dev` strona idzie prosto z Vite bez CSP, a `devCsp` jest bez znaczenia. Starsza notatka w `review-fixes.md` zakładała, że CSP obejmie dev; to twierdzenie jest **nieaktualne dla desktopu**.
- **Dostarczanie.** Na Windows CSP jest nagłówkiem HTTP odpowiedzi protokołu `tauri` (`protocol/tauri.rs:182-183`). Meta-tag Tauri dodaje tylko dla URL-i `data:` (funkcja `webview-data-url`, nieużywana).
- **Nonce i hashe** (według raportu agenta, kotwice w crate'ach):
  - Przy buildzie hashowane są wszystkie pliki `.js`/`.mjs` z `frontendDist` (`tauri-utils` `embedded_assets.rs:170-197`) oraz inline `<script>`.
  - Nonce dostają skrypty z absolutnym `src^='http'` i inline `<style>` (`tauri-utils/src/html2.rs:67-77`).
  - Przy serwowaniu `'self'` + nonce/hashe trafiają do `script-src` i `style-src`, ale tylko do tej dyrektywy, która ma co najmniej jeden nonce lub hash (`manager/mod.rs:53-107`, `126-153`).
  - Dla buildu Vite (`dist/index.html` ma jeden `<script type="module" src="/assets/…js">` i jeden `<link rel="stylesheet">`): `script-src` powstaje jawnie jako `'self' 'sha256-…'`. `style-src` nie jest ruszane, więc spada do `default-src` albo zostaje takie, jak je ustawimy.
  - `dangerousDisableAssetCspModification` wyłącza to wstrzykiwanie (całe lub per dyrektywa). Nie jest potrzebne.
- **Origins na Windows** (`useHttpsScheme` domyślnie `false`): strona `http://tauri.localhost` (= `'self'`), IPC `http://ipc.localhost`, asset `http://asset.localhost` (`tauri-2.12.7/src/protocol/mod.rs:32-39`).
  - IPC to `fetch` do `http://ipc.localhost/<cmd>`. Gdy CSP go zablokuje, Tauri loguje błąd i do końca sesji przechodzi na `postMessage` (`scripts/ipc-protocol.js:37`, `58-67`). To nie jest awaria, ale `connect-src ipc: http://ipc.localhost` jest potrzebne, żeby uniknąć fallbacku (tak samo w przykładzie z https://v2.tauri.app/security/csp/).
  - Protokół asset nie jest włączony: `src-tauri/Cargo.toml` ma tylko feature `tray-icon`, a frontend nie używa `convertFileSrc`. `asset:` i `http://asset.localhost` z polityki w issue są więc zbędne.
- **Tauri nic nie dopisuje samo** do `default-src` ani `connect-src`; wyjątkiem jest wzorzec isolation, nieużywany (`manager/mod.rs:430-437`).
- **Skrypty spoza strony nie podlegają CSP strony:**
  - skrypty inicjalizujące Tauri i pluginów rejestrowane przez WebView2 (`manager/webview.rs:159-204`);
  - wywołania callbacków IPC wykonywane z Rusta (`node_modules/@tauri-apps/api/core.js:63`, komentarz).

### 2. Frontend: co polityka może zepsuć

Zakres sprawdzenia: wszystkie pliki `src/`, `index.html`, `vite.config.ts`, `package.json`, zbudowane `dist/index.html`, `dist/assets/index-DebY4MBF.js` i `dist/assets/index-CzUFiB4z.css`. Katalogu `public/` nie ma.

- **Style inline:**
  - dokładnie 4 atrybuty `style={…}` w 2 komponentach: `src/components/HoursBar/HoursBar.tsx:23` (width), `:25` (left); `src/components/LabelChip/LabelChip.tsx:56`, `:67` (zmienne `--dn-label-bg`, `--dn-label-ink`);
  - react-dom 19.3.1 zapisuje je przez CSSOM, `style[name]=` lub `style.setProperty` (`node_modules/react-dom/cjs/react-dom-client.production.js:1439-1456`); potwierdzone też w bundlu;
  - zapisy CSSOM nie podlegają `style-src`; blokowany byłby `style="…"` w markupie lub `setAttribute('style', …)`, a ich w `src/` nie ma;
  - brak `<style>`, `dangerouslySetInnerHTML`, `innerHTML`, `cssText` i CSS-in-JS.
- **Skrypty:**
  - w `src/` i bundlu nie ma `eval`, `new Function`, stringowego `setTimeout`, dynamicznego `import()`, WebAssembly, workerów ani `blob:`;
  - jedyny skrypt to moduł z `src`.
- **Sieć:**
  - brak `fetch`, `XMLHttpRequest`, `WebSocket` i `EventSource` w `src/`;
  - `invoke` jest w 7 plikach `src/*/api.ts`, `listen` w 9 hookach, `openUrl` w `src/github/api.ts:16` i `src/gitlab/api.ts:39`;
  - całość idzie przez IPC.
- **Obrazy i fonty:**
  - brak `<img>` i `url()` w CSS poza fontami; ikony to inline `<svg>` (`src/components/Icon/Icon.tsx:81`);
  - 6 `@font-face` z lokalnymi plikami woff2 (`src/styles/tokens.css:3-49`), emitowanymi jako `/assets/*.woff2`, czyli same-origin;
  - wnioski: `img-src … data:` jest zbędne, `font-src` spada do `default-src 'self'`.
- **Nawigacja:**
  - brak `<a href>`, `window.open` i iframe'ów;
  - 3 formularze z samym `onSubmit` (`src/own-tasks/OwnTaskEditPanel.tsx:58`, `src/own-tasks/OwnTaskForm.tsx:51`, `src/worktime/WorkTimeEntries.tsx:56`);
  - linki zewnętrzne otwiera `openUrl`, poza webview.
- **Tryb dev** (gdyby ktoś kiedyś wymusił CSP w dev):
  - Vite wstrzykuje `<style>` (`node_modules/vite/dist/client/client.mjs:1280`);
  - `@vitejs/plugin-react` dodaje inline preamble (`node_modules/@vitejs/plugin-react/dist/index.js:143`, `:183`);
  - HMR używa `ws://localhost:1420`.
  - Według punktu 1 Tauri na desktopie tego nie dotyka.

**Polityka wynikająca z ustaleń** (węższa niż w issue, bo bez asset, `data:` i `customprotocol:`):

```
default-src 'self'; connect-src ipc: http://ipc.localhost; object-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'
```

Tauri dopisze `script-src 'self' 'sha256-…'`. `style-src` i `font-src` spadają do `default-src 'self'`. `form-action 'none'` jest bezpieczne: formularze nie mają `action`, a wszystkie 3 handlery wywołują `e.preventDefault()` (`src/own-tasks/OwnTaskEditPanel.tsx:40`, `src/own-tasks/OwnTaskForm.tsx:26`, `src/worktime/WorkTimeEntries.tsx:47`). `frame-ancestors` w nagłówku jest poprawne, choć praktycznie bez znaczenia.

### 3. freezePrototype

- Skrypt to `Object.freeze(Object.prototype)` (`tauri-2.12.7/scripts/freeze_prototype.js`). Wstawiany jest na początku `init.js` jako skrypt inicjalizujący ramki głównej, przed skryptami pluginów i przed stroną, w dev i w release (`manager/webview.rs:159-204`, `428-449`; flaga w `tauri-utils/src/config.rs:3075`).
- Ryzyko to „override mistake”: przypisanie `obj.toString = …` (lub `constructor` itp.) na obiekcie bez własnej właściwości rzuca `TypeError` w strict mode.
  - W Tauri v1 domyślne włączenie wycofano przez lodash-es, emotion i Pixi (tauri-apps/tauri#3416, #3406, #3438).
  - Bezpośrednie zależności runtime (`react`, `react-dom`, `scheduler`, `@tauri-apps/api`, `@tauri-apps/plugin-opener`) nie patchują prototypów wbudowanych. Jedyne `S.constructor=x` w PureComponent trafia na zapisywalną właściwość własnego prototypu i nie rzuca.
- Nie zweryfikowano w runtime. Ponieważ skrypt działa także w dev, `npm run tauri dev` z czystą konsolą to wystarczający pierwszy test.

### 4. Okno `main`

- **`theme`** (`"Light"`, `"Dark"` lub null; `tauri-utils/src/config.rs:2111`):
  - motyw okna trafia do WebView2 jako `SetPreferredColorScheme` (`tauri-runtime-wry-2.12.5/src/lib.rs:4999-5003`, `wry-0.57.3/src/webview2/mod.rs:2033-2045`);
  - CSS aplikacji nie używa `prefers-color-scheme` (`AGENTS.md`: „The theme is always dark”, `src/styles/tokens.css:125` ma `color-scheme: dark`), więc wpływ na treść jest żaden;
  - zysk to ciemny pasek tytułu niezależnie od motywu systemu. Nie potwierdzono tego w źródle tao, tylko ręcznie.
- **`backgroundColor`** (`config.rs:2243`):
  - hex `#RGB`, `#RRGGBB` lub `#RRGGBBAA` albo tablica lub obiekt RGBA;
  - stosowany do okna i do webview (`tauri-runtime-wry` `lib.rs:885-886`, `tauri-runtime/src/webview.rs:498-499`); na Windows alfa okna jest ignorowana;
  - wartość docelowa to `--bg: #0f1115` (`src/styles/tokens.css:53`, `body { background: var(--bg) }` w `:128-130`);
  - nie ma udokumentowanego białego mignięcia. `show_main` wywoływane jest na końcu `setup` (`src-tauri/src/lib.rs:171-172`, `src-tauri/src/runtime.rs:123-130`), zanim webview się narysuje, więc białe tło przed pierwszym renderem jest prawdopodobne. To wniosek, nie obserwacja.
- **`center`** (`config.rs:1997`):
  - liczone przy tworzeniu okna (`tauri-runtime-wry` `lib.rs:4451-4513`);
  - window-state w `on_window_ready` przywraca pozycję tylko wtedy, gdy zapisany prostokąt przecina bieżący monitor (`tauri-plugin-window-state-2.5.0/src/lib.rs:206-225`, `433-456`);
  - efekt: wyśrodkowanie przy braku pliku stanu lub po zniknięciu monitora, a zapisana pozycja wygrywa w pozostałych przypadkach;
  - bez mignięcia, bo okno jest `visible:false` do `show_main`.

### 5. Bundle i NSIS

- **Domyślny publisher** to `identifier.split('.').nth(1)`, czyli dziś `"sebas"` (bundler `nsis/mod.rs:281-284`). Tak samo jest dla `CompanyName` w exe (`tauri-build-2.7.6/src/lib.rs:849-859`).
- **Klucze rejestru** (`installer.nsi:67-69`):
  - wpis w Aplikacjach: `UNINSTKEY = Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCTNAME}`, zależny od `productName`, nie od publishera;
  - `MANUPRODUCTKEY = Software\${MANUFACTURER}\${PRODUCTNAME}` przechowuje katalog instalacji (domyślna wartość klucza).
- **Co zapisuje wpis w Aplikacjach:** `Publisher = MANUFACTURER` (`installer.nsi:749`). `URLInfoAbout`, `URLUpdateInfo` i `HelpLink` są zapisywane tylko przy niepustym `HOMEPAGE` (`installer.nsi:760-764`). `homepage` spada do `homepage` z `Cargo.toml` (`settings.rs:1271-1277`); `Cargo.toml` ma tylko `repository`, więc dziś nic się nie zapisuje.
- **`shortDescription`, `longDescription` i `license`** są przekazywane do szablonu (`nsis/mod.rs:309-317`), ale `installer.nsi` ich nie używa, a `license` (SPDX) nie jest czytane w ogóle. W NSIS nie mają więc efektu.
- **`licenseFile`** dodaje stronę licencji `MUI_PAGE_LICENSE` (`installer.nsi:173-177`). Nie ustawiać.
- **Zmiana `publisher`** z `"sebas"` na inną wartość, według szablonu (nie przetestowano):
  - Wykrycie istniejącej instalacji działa, bo czyta `UNINSTKEY` (`installer.nsi:228-241`). Wpis w Aplikacjach jest nadpisywany, więc nie powstanie drugi.
  - `RestorePreviousInstallLocation` czyta **nowy** `MANUPRODUCTKEY`, który jest pusty, i zostaje przy domyślnym `$LOCALAPPDATA\Nawigator dnia` (`installer.nsi:942-946`). To ten sam katalog, chyba że ktoś wcześniej wybrał inny.
  - Domyślna opcja „odinstaluj przed instalacją” czyta pusty `$4` z nowego `MANUPRODUCTKEY` i uruchamia `uninstall.exe _?=` z pustą ścieżką (`installer.nsi:371-376`). Według źródła NSIS (`exehead/Main.c`) kończy się to komunikatem o błędzie inicjalizacji deinstalatora, po czym instalator wraca do strony wyboru (`installer.nsi:388-399`). Opcja „nie odinstalowuj” nadpisuje instalację w miejscu.
  - Tryby `/UPDATE`, silent i passive pomijają deinstalację.
  - Stary klucz `HKCU\Software\sebas\Nawigator dnia` zostaje osierocony; jest nieszkodliwy.
- **Ścieżki danych aplikacji** (`app_config_dir`, `app_data_dir`, `app_local_data_dir`, `app_log_dir`, katalog danych WebView2) to `<base>/<identifier>` (`tauri-2.12.7/src/path/desktop.rs:239-306`). Żaden z kluczy z #21 ich nie zmienia; zmieniłby je tylko `identifier`, a `useHttpsScheme` przeniósłby storage WebView.

## Code References

- `src-tauri/tauri.conf.json:22-24` – `"security": { "csp": null }`
- `src-tauri/tauri.conf.json:11-20` – jedyne okno, `visible: false`, bez `theme`, `backgroundColor`, `center`
- `src-tauri/tauri.conf.json:26-37` – bundle bez `publisher`, `homepage`, `license`, `shortDescription`
- `src-tauri/src/lib.rs:135-142` – window-state z `StateFlags::all() & !VISIBLE`
- `src-tauri/src/lib.rs:171-172`, `src-tauri/src/runtime.rs:123-130` – `show_main` na końcu `setup`
- `src-tauri/capabilities/default.json` – `core:event:allow-listen`, `core:event:allow-unlisten`, `opener:allow-open-url` dla `https://*`
- `src/components/HoursBar/HoursBar.tsx:23,25`, `src/components/LabelChip/LabelChip.tsx:56,67` – style przez CSSOM
- `src/styles/tokens.css:53,125,128-130` – `--bg`, `color-scheme: dark`, tło `body`
- `tauri-codegen-2.7.6/src/context.rs:162-180` – wybór CSP i brak osadzonych zasobów przy `devUrl`
- `tauri-2.12.7/src/manager/mod.rs:344-373` – URL aplikacji w dev, wybór CSP w runtime
- `tauri-2.12.7/src/protocol/tauri.rs:175-185` – nagłówek CSP tylko na protokole `tauri`
- `installer.nsi:67-69, 228-241, 371-376, 749, 760-764, 942-946` (tauri-bundler 2.10.6) – klucze rejestru, wykrycie upgrade'u, deinstalacja, publisher, homepage, przywracanie katalogu

## Architecture Insights

- Jedna konfiguracja, jedno okno. Każda z tych zmian to edycja `tauri.conf.json` bez kodu Rust ani TS. Jedynym kodem mógłby być ewentualny hook NSIS, gdyby zmieniać publishera.
- Warstwa webview ma trzy kanały, z których tylko pierwszy podlega CSP strony:
  - zasoby same-origin (`http://tauri.localhost`);
  - IPC (`http://ipc.localhost`);
  - skrypty inicjalizujące i callbacki z Rusta (poza CSP).
- `AGENTS.md` mówi, że `tokens.css` jest jedynym miejscem na wartości i że w innym CSS nie ma literalnych kolorów. `backgroundColor` w JSON nie jest CSS, ale jest drugą kopią `--bg`. JSON nie ma komentarzy, więc powiązanie trzeba zapisać gdzie indziej (np. w `AGENTS.md` lub komentarzu w `tokens.css`).

## Historical Context (from prior changes)

- `context/archive/2026-11-20-source-health-status/reviews/impl-review.md:118-126` (F6): CSP wyłączone, odłożone do follow-upu. Uzasadnienie: „csp also applies under tauri dev … blank window”.
  - Werdykt: **częściowo nieaktualne**. Na desktopie z `devUrl` CSP nie jest stosowane w dev (punkt 1). Ryzyko pustego okna dotyczy tylko buildu.
- `context/archive/2026-11-20-source-health-status/follow-ups/review-fixes.md`, sekcja „Content Security Policy”:
  - polityka startowa `default-src 'self' ipc: http://ipc.localhost; img-src 'self' asset: http://asset.localhost` jest poprawna, ale szersza niż potrzeba (asset niewłączony);
  - teza, że `'unsafe-inline'` jest ignorowane przy nonce/hash, jest **poprawna** tylko dla dyrektyw, do których Tauri dopisał nonce lub hash (u nas `script-src`). Nie dotyczy to stylów CSSOM;
  - kryterium „Done when” (dev i zainstalowany build ostylowane, z live update) nadal ma sens.
- `context/archive/2026-11-20-work-tasks-from-template/reviews/impl-review.md:48,53-54`: przy `csp: null` każda wstrzyknięta treść ma dostęp do komend; CSP warte osobnej zmiany. **Aktualne.**
- `context/archive/2026-11-20-source-health-status/reviews/impl-review-round1.md:106-113`: brak sinka XSS (React escapuje). **Aktualne:** nadal brak `dangerouslySetInnerHTML` i `innerHTML` (potwierdzone też w `2026-11-24-shared-ui-tools/reviews/impl-review.md:26`).
- `context/archive/2026-11-24-app-metadata/plan.md`:
  - `:24`, `:29`: świadoma decyzja „Nie ustawiamy `bundle.publisher`”, bo `Software\<publisher>\Nawigator dnia` przechowuje katalog poprzedniej instalacji. **Potwierdzone** w `installer.nsi:371-376`, `942-946`. Skutkiem nie jest jednak całkowite zepsucie: najpewniej pojawi się jednorazowy błąd deinstalacji przy domyślnej opcji, a wpis w Aplikacjach się nie dubluje.
  - `:31`: brak `shortDescription`/`longDescription`, bo NSIS ich nie używa. **Potwierdzone.**
- `context/archive/2026-11-23-konsola-design-system/plan.md:54`, `:162-168` i `reviews/plan-review.md:64-72` (F4): `visible:false` i `show_main` wprowadzono przeciw mignięciu **rozmiaru**, nie koloru. `backgroundColor` uzupełnia ten mechanizm.
- `context/archive/2026-11-20-ci-release-pipeline/plan.md:43,53`: NSIS, `currentUser`, `identifier` niezmienny dla stabilnych ścieżek danych. Zmiana `productName` dałaby nowy wpis w Aplikacjach i nowy katalog. Poza zakresem.
- `context/foundation/roadmap.md`: brak pozycji dla #21. M-2 jest zamknięty. Najbliżej jest S-10 `app-metadata` (MS-15), który publishera i opisy świadomie pominął.

## Related Research

- `context/archive/2026-11-24-installer-signing/research.md` – podpisywanie nie wymaga zmian w `tauri.conf.json` (`:126`). Przy SignPath wydawcą w podpisie byłby „SignPath Foundation” (`:14`), niezależnie od `bundle.publisher`.

## Open Questions

1. **`publisher`:** zostawić `"sebas"` (rekomendacja, zgodnie z `app-metadata`), zmienić i zaakceptować jednorazowy błąd przy pierwszej aktualizacji, czy zmienić z hookiem NSIS (`bundle.windows.nsis.installerHooks`), który przeniesie katalog ze starego klucza? Hooka nie badano.
2. **`backgroundColor`:** akceptujemy drugą kopię `#0f1115` w `tauri.conf.json` z odnotowaniem powiązania (gdzie?), czy rezygnujemy?
3. **`license` / `shortDescription`:** w NSIS bez efektu. Ustawić dla porządku w metadanych (przydatne tylko przy innych formatach bundla), czy pominąć jak w `app-metadata`?
4. **`homepage`:** ustawić `bundle.homepage` na URL repozytorium (link w Aplikacjach), czy dodać `homepage` do `Cargo.toml`?
5. **Weryfikacja ręczna:** CSP w pełni widać tylko w buildzie. Potrzebny jest test zainstalowanego instalatora: czy widoki są ostylowane, czy działa live update stanu źródeł, czy otwieranie linków działa, czy konsola nie ma naruszeń CSP i błędu „IPC custom protocol failed”. `freezePrototype` i `theme` sprawdza się w dev. Nie istnieje stała procedura testu instalacji i aktualizacji (wiersz 1.6 z `ci-release-pipeline` nadal nieodhaczony).
