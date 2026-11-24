# Button

Przycisk w czterech wariantach: `primary` (wypełnienie `accent`, najwyżej jeden na widok), `secondary` (domyślny, obrys `line-strong`), `quiet` (narzędzia w wierszu, obrys `line-control`) i `dashed` („Dopisz brakujący wpis”).

- Rozmiary: `sm` 36px (pasek narzędzi, Odśwież, stopka panelu), `md` 40px (formularze), `lg` 44px (START/STOP, „Sprawdź teraz”).
- `iconOnly` z `icon` daje kwadrat 40×36 — zawsze podaj `aria-label` („Dodaj do listy Do zrobienia”, „Ukryj do zmiany”).
- Przycisk „+” pozycji, która już jest w „Do zrobienia”: `variant="primary"`, `iconOnly`, `pressed` — wypełniony bursztynem, nadal klikalny (zdejmuje z listy).
- `pressed` dla przełączników („Pokaż ukryte”). Etykieta zaczyna się od czasownika: „Odśwież”, „Zapisz”, „Sprawdź teraz”.
- `disabled`: ikona `ink-disabled`, bez zmiany kursora; przycisk wypada z kolejności tabulatora.
- Zajęty (trwa akcja, kliknięcie nic by nie zmieniło): `aria-disabled` + `aria-busy` zamiast `disabled` — wygląda jak wyłączony, ale zostaje fokusowalny, więc fokus nie ginie. Etykieta zostaje tekstem akcji; handler sam ignoruje kliknięcie.
- `ref` przekazujesz jak zwykły prop (np. by przenieść fokus na „Anuluj”). Domyślnie `type="button"`.
- Na przycisku z `className="dn-btn-mono"` (START/STOP, TAK) tekst jest w `mono`, pogrubiony.
