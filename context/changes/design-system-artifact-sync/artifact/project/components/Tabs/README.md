# Tabs

Zakładki z licznikami nad listą w panelu: podkreślenie 2px `accent` pod wybraną, licznik w `mono` `ink-faint`, wysokość `size-touch`.

- Do podziału jednej listy na części: Praca / Prywatne / Zrobione w „Moich zadaniach”.
- Props: `tabs` (`{ value, label, count? }`), `value`, `onChange(value)`, `label` (nazwa grupy dla czytnika), `panelId` i `idPrefix`. Typ `value` jest generyczny (unia napisów).
- `panelId` to `id` panelu zakładek (`role="tabpanel"`), który zakładki kontrolują (`aria-controls`); konsument ustawia na nim `aria-labelledby` na `<idPrefix>-<value>` aktywnej zakładki. `idPrefix` buduje `id` każdej zakładki.
- Klawiatura: strzałki w lewo i prawo przechodzą cyklicznie i od razu wybierają zakładkę, `Home` wybiera pierwszą, `End` ostatnią. Fokus jest „roving”: tylko wybrana zakładka ma `tabIndex=0`, pozostałe `-1`.
- `role="tablist"`, każda zakładka `role="tab"` z `aria-selected`. Licznik to liczba pozycji w zakładce, bez nawiasów.
- Nawigacja między widokami to `AppHeader`, nie `Tabs`.
