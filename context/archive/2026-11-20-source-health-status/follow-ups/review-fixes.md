# Review follow-ups: source-health-status

Items from `reviews/impl-review.md` that need a person or a running GUI.

## Content Security Policy (from F6)

- **Now:** `tauri.conf.json` has `"security": { "csp": null }` (from the starter).
- **Why it was not done in the review:** `csp` is also injected under `tauri dev` unless `devCsp` is set. Vite's dev server injects styles inline and uses an HMR websocket on `localhost:1420`, and Tauri adds nonces/hashes that make `'unsafe-inline'` ignored. A wrong policy gives a blank window, and that can only be seen by running the app.
- **Starting point:** for the build, use `"csp": "default-src 'self' ipc: http://ipc.localhost; img-src 'self' asset: http://asset.localhost"`. Then either set a looser `devCsp` (with `connect-src ws://localhost:1420` and inline styles) or confirm the dev window still renders.
- **Done when:** `npm run tauri dev` and an installed build both show the health panel, styled, with live updates, and the CSP is set.

## Manual checks (from F7)

All Manual rows in plan.md Progress (1.5, 2.5–2.12, 3.5–3.7) still need a human run before `/10x-archive`. Check 2.9 (stall switch) now runs through `health::watchdog_step`.
