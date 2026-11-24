Ikony aplikacji: znak Day Navigatora (igła kompasu skierowana na północny wschód w pierścieniu dnia) i trzy stany ikony w zasobniku. SVG pokazuje się tylko przez `<img>`, który nie dziedziczy koloru, więc barwy są wpisane w pliki.

- `app-icon.svg` — ikona aplikacji i instalatora. Pierścień i czubek igły w `accent` `#F5A524`, ogon igły w `ink-faint` `#8A93A3`, tło przezroczyste (bez kafla).
- `tray-ok.svg` — zasobnik, ogólny stan OK. Bursztyn `#F5A524` i `#8A93A3`.
- `tray-error.svg` — zasobnik, ogólny stan błąd. `danger` `#FF7B72` z kropką-plakietką w rogu; ogon `#8A93A3`.
- `tray-idle.svg` — zasobnik, start lub sprawdzanie nie działa. Szarości `#8A93A3`, `#B4BBC7` i `#5A6372`.

Ikony w zasobniku mają grubsze kreski, żeby czytały się w 16 px. Użyj ich w pliku bez zmian; nie przekolorowuj. PNG paczki i zasobnika powstają z tych SVG poleceniem `npx tauri icon`.
