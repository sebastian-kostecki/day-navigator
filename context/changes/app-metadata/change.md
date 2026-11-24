---
change_id: app-metadata
title: Metadane aplikacji — opis i autor zamiast wartości z szablonu
status: implementing
created: 2026-11-24
updated: 2026-11-24
archived_at: null
---

## Notes

S-10 z roadmapy (M-2), GitHub issue #15, MS-15. `src-tauri/Cargo.toml` ma nadal `description = "A Tauri App"` i `authors = ["you"]`. Uzupełnić je i sprawdzić, co pokazują właściwości pliku exe i instalatora NSIS. Nie zmieniać `version` (podbicie na `main` publikuje wydanie).
