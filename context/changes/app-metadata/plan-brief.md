# Metadane aplikacji — Plan Brief

> Full plan: `context/changes/app-metadata/plan.md`

## What & Why

`src-tauri/Cargo.toml` ma nadal wartości z szablonu („A Tauri App”, „you”). Cel S-10: właściwości exe i instalatora mają pokazywać prawdziwego autora zamiast wartości z szablonu.

## Starting Point

Na Windows `description` i `authors` z Cargo nie trafiają nigdzie. Exe i instalator NSIS biorą nazwę i opis z `productName`, firmę/wydawcę z `bundle.publisher` (fallback „sebas” z identyfikatora), a prawa autorskie z `bundle.copyright` (dziś puste).

## Desired End State

`Cargo.toml` ma prawdziwy opis i autora. Exe i instalator pokazują „Prawa autorskie: © 2026 Sebastian Kostecki”. Wersja, publisher i zachowanie aktualizacji są bez zmian.

## Key Decisions Made

| Decision        | Choice                                         | Why (1 sentence)                                                        |
| --------------- | ---------------------------------------------- | ----------------------------------------------------------------------- |
| `authors`       | `["Sebastian Kostecki"]`, bez e-maila          | Prawdziwy autor bez danych kontaktowych w repo.                         |
| `description`   | Opis po polsku (jedno miejsce na dzień pracy…) | Spójny z PRD i polskim UI.                                              |
| Autor w plikach | `bundle.copyright = "© 2026 Sebastian Kostecki"` | Jedyne pole, które pokazuje autora w exe i instalatorze bez ryzyka.   |
| Publisher       | Bez zmian („sebas”)                            | Wyznacza klucz rejestru, z którego aktualizacja czyta katalog instalacji. |

## Scope

**In scope:** `description`, `authors` w `Cargo.toml`; `bundle.copyright` w `tauri.conf.json`; odczyt `VersionInfo` z exe i lokalnego instalatora.

**Out of scope:** `version`; `bundle.publisher`; własny zasób VERSIONINFO; `shortDescription`/`longDescription`; e-mail.

## Phases at a Glance

| Phase                     | What it delivers                              | Key risk                                  |
| ------------------------- | --------------------------------------------- | ----------------------------------------- |
| 1. Metadane i weryfikacja | Nowe pola + potwierdzony `LegalCopyright`     | Przypadkowa zmiana `version` → wydanie    |

**Prerequisites:** brak.
**Estimated effort:** jedna krótka sesja (lokalny build release NSIS trwa kilka minut).

## Open Risks & Assumptions

- `FileDescription` dalej pokazuje „Nawigator dnia”, nie opis — tak działa `tauri-build`; zmiana wymagałaby własnego zasobu.
- Rok w copyright trzeba aktualizować ręcznie.

## Success Criteria (Summary)

- Właściwości exe i instalatora pokazują autora.
- `version` nadal `0.2.0`; kontrole sprzed commita przechodzą.
