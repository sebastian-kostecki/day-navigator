# SourceCard

Wzorzec karty „Stan źródeł” — duża karta jednego źródła (GitLab, GitHub, Dane aplikacji) z głową, komunikatem i faktami; to złożenie komponentów, nie eksport paczki.

- W aplikacji: `src/health/SourceCard.tsx`, teksty i tony w `src/health/sourceView.ts`, przycisk w `src/health/OpenBackupsButton.tsx`; widok „Źródła”.
- Zbudowana z `StatusPill` (`glyph={false}`, tekst pigułki: OK, NIEDOSTĘPNY, NIESKONFIGUROWANE, BŁĄD) i `Button`; reszta to klasy `health-card*` z sekcji „Wzorce widoków” w `bundle.css`.
- Układ: `section.health-card.is-<ok|calm|error>` → `div.health-card-head` (glif `● ○ ▲` w `span.health-glyph`, `h2.health-card-name` z nazwą, `div.health-card-kind` z opisem, pigułka) → `p.health-card-message` → opcjonalnie `p.health-card-note` → `dl.health-card-facts` → `div.health-card-actions`.
- Fakty pokazuj tylko wtedy, gdy ma je dane; bez danych nie ma `dl`. Dla „Dane aplikacji”: Sprawdzono, Ostatnia kopia, Kopie („5 z 7”), Schemat („v11”). Przeterminowaną kopię oznacz `dd.is-warn` (kolor `warn`) — to ostrzeżenie, nie błąd.
- Akcja pod faktami: cichy przycisk `sm` z ikoną `folder` „Otwórz folder kopii”; w czasie otwierania `aria-disabled` + `aria-busy`.
- Tony karty: `ok` glif `info`, `calm` glif `ink-faint` (brak VPN lub źródło wyłączone, bez alarmu), `error` na `danger-bg` z komunikatem w `danger-soft`.
