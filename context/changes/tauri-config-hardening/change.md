---
change_id: tauri-config-hardening
title: Konfiguracja Tauri - CSP, wygląd okna i metadane instalatora (#21)
status: impl_reviewed
created: 2026-11-25
updated: 2026-11-25
archived_at: null
---

## Notes

GitHub issue #21 (https://github.com/sebastian-kostecki/day-navigator/issues/21): Konfiguracja Tauri: CSP, wygląd okna i metadane instalatora. Zakres: app.security.csp (wąska polityka + devCsp lub weryfikacja tauri dev), app.security.freezePrototype, okno main (theme Dark, backgroundColor = --bg #0f1115, center), bundle publisher/homepage/license MIT/shortDescription. Do sprawdzenia: tauri dev i zainstalowany build bez błędów CSP, style inline React (HoursBar, LabelChip), aktualizacja instalatorem po zmianie publisher (HKCU\Software\<publisher>\<produkt>). Poza zakresem: polski instalator NSIS, removeUnusedCommands, podpisywanie (#13). Otwarty follow-up CSP: context/archive/2026-11-20-source-health-status/follow-ups/review-fixes.md.
