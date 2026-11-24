# Button

Przycisk w czterech wariantach: `primary` (wypełnienie `accent`, najwyżej jeden na widok), `secondary` (domyślny, obrys `line-strong`), `quiet` (narzędzia w wierszu, obrys `line-control`) i `dashed` („Dopisz brakujący wpis”).

- Rozmiary: `sm` 36px (pasek narzędzi, Odśwież), `md` 40px (formularze), `lg` 44px (START/STOP, „Sprawdź teraz”).
- `iconOnly` z `icon` daje kwadrat 40×36 — zawsze podaj `aria-label` („Dodaj do listy Do zrobienia”, „Ukryj do zmiany”).
- Przycisk „+” pozycji, która już jest w „Do zrobienia”: `variant="primary"`, `iconOnly`, `pressed` — wypełniony bursztynem, nadal klikalny (zdejmuje z listy).
- `pressed` dla przełączników („Pokaż ukryte”). Etykieta zaczyna się od czasownika: „Odśwież”, „Zapisz”, „Sprawdź teraz”.
- Wyłączony: ikona `ink-disabled`, bez zmiany kursora.
