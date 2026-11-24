# SourceChip

Krótki chip w `mono` mówiący, skąd pochodzi pozycja: `GL` (`src-gitlab`), `GH` (`src-github`), `MY` (`src-own`), albo jaki to typ w GitLabie: `MR` (`kind-mr`), `ISS` (`src-gitlab`).

- Zawsze przed numerem i tytułem pozycji, w tej samej kolejności w każdym wierszu.
- Kolor tekstu różni chipy, obrys jest wspólny (`line-control`); litery niosą znaczenie, więc chip działa bez rozróżniania kolorów.
- Wspólna klasa `dn-chip` z `Chip` (priorytet): oba są w `mono`, pogrubione i wyśrodkowane. `SourceChip` bierze kolor z `source`, `Chip` z `tone`.
- Nie dodawaj własnych skrótów; nowe źródło = nowy wariant tutaj. `children` podmienia tekst chipa, `title` daje podpowiedź.
