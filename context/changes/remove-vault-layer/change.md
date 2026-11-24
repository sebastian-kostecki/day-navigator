---
change_id: remove-vault-layer
title: Usunięcie nieużywanej warstwy vaultów i źródeł vault/exclusions z „Stanu źródeł”
status: implementing
created: 2026-11-24
updated: 2026-11-24
archived_at: null
---

## Notes

Foundation F-03 z milestone'u M-2 (GitHub issue #11, MS-11). Usunąć `src-tauri/src/vault/` i wszystko, co z niego korzysta: źródła `vault:*` / `exclusions:*` w health, ich testy i fixtures, helpery testowe bez użytkowników, regułę `.gitattributes` dla fixtures oraz ustawienia ścieżek vaultów (stary `settings.json` z tymi polami nadal się wczytuje). AGENTS.md traci kontrakt kodu vaultów, ale zachowuje zasadę, że aplikacja nie czyta i nie zapisuje vaultów. Wpis w `## Parked` roadmapy dostosowany.
