---
project: day-navigator
checked_at: 2026-11-22T22:10:00+01:00
health_status: needs-attention
context_type: brownfield
language_family: multi
stack_assessment_available: true
checks_run:
  - lockfile
  - dependency_audit
  - outdated_deps
  - test_runner
  - ci_cd
  - configuration
audit_findings:
  critical: 0
  high: 0
  moderate: 1
  low: 0
test_runner_detected: true
ci_provider: GitHub Actions
recommended_fixes: 8
---

## Dependency Health

### Lockfile

```
Status: present (package-lock.json, src-tauri/Cargo.lock)
Package manager: npm + cargo
```

CI installs with `npm ci` and builds and tests with `cargo build --locked` / `cargo test --locked`, so builds are reproducible.

### Security Audit

```
Tool: npm audit --json; cargo audit --json (advisory DB updated 2026-11-22, 505 crates)
Summary: 0 CRITICAL, 0 HIGH, 1 MODERATE, 0 LOW
Direct vs transitive: the 1 vulnerability is transitive; 1 of the 6 warnings is on a direct dependency
```

**npm:** 0 vulnerabilities across 69 packages.

**MODERATE (1):**
- **url** 2.5.8 — RUSTSEC-2026-0592 (GHSA-hrj5-67jp-wqp9, published 2026-11-20): stack overflow in `Url::origin()` on deeply nested `blob:` URLs (denial of service). The advisory has no CVSS score, so it defaults to MODERATE. Transitive path: `schemars` → `tauri-build` / `tauri-plugin` / `tauri-utils`. The app never parses untrusted URLs through `Url::origin()`, so the real risk is low. **Side effect:** the CI step `cargo audit` exits non-zero on any vulnerability, so the next push fails CI until this is fixed. Fix: `cargo update -p url` (2.5.8 → 2.5.9, confirmed with `--dry-run`).

**Warnings (informational, `cargo audit` does not fail on these):**
- unmaintained: **chrono** 0.4.45 (RUSTSEC-2026-0543) — a **direct** dependency (`src-tauri/Cargo.toml`); proc-macro-error 1.0.4 (RUSTSEC-2024-0370), zbus_names 4.3.5, zvariant 5.15.1, zvariant_derive 5.15.1 (RUSTSEC-2026-0370/0371/0372) — transitive, through Tauri.
- unsound: glib 0.18.5 (RUSTSEC-2024-0429) — transitive, from Tauri's Linux (GTK) stack; it is not built for the Windows target.

### Outdated Dependencies

```
Packages with major version gaps: 1 (npm); cargo: skipped — cargo-outdated not installed
```

- **typescript**: 6.0.3 → 7.0.2 (1 major version behind). Nothing is more than 2 majors behind. `cargo update -p url --dry-run` also reported 16 crates behind latest within their semver ranges.

## Test Suite

```
Test runner: cargo test (Rust); frontend: not detected
Tests found: 181 (99 unit in lib, 8 health_probe, 22 tasks, 18 vault_guard, 34 vault_write)
Test execution: passing (180 passed, 1 ignored, 0 failed; locally 2026-11-22)
```

```
Configuration: src-tauri/Cargo.toml (built-in harness; proptest dev-dependency; `test-support` feature enabled only for tests)
Framework: cargo test, proptest 1; fixtures in src-tauri/tests/fixtures/vault (invented data), shared helpers in src-tauri/tests/common
```

⚠ The frontend (`src/`) has no tests and no test runner. That is acceptable while the UI stays thin, but the new MVP adds four views and a persistent header.

## CI/CD

```
Provider: GitHub Actions
Configuration: .github/workflows/ci.yml
```

| Stage      | Status | Notes                                                                 |
|------------|--------|-----------------------------------------------------------------------|
| Lint       | ✗      | not configured (no clippy, no `cargo fmt --check`, no ESLint)          |
| Test       | ✓      | `cargo test --locked`; pre-push hook self-test (`scripts/test-pre-push.sh`) |
| Build      | ✓      | `npm run build`, `cargo build --locked`; installer via `tauri-apps/tauri-action@v1` on version bump |
| Type check | ✓      | `tsc` inside `npm run build`; Rust type-checked by `cargo build`       |
| Security   | ✓      | gitleaks (full history), `cargo audit`, `npm audit --audit-level=high` |

## Configuration

### High severity

All high-severity items are present: `.gitignore` exists, `tsconfig.json` has `strict: true`, and `.gitattributes` keeps the fixture vault byte-exact.

### Medium severity

- **Rust formatter not enforced** — `cargo fmt --check` reports 41 diff hunks, so agents' edits will mix styles and inflate diffs. Fix: run `cargo fmt` once, then gate it.
- **Rust linter not enforced** — `cargo clippy --all-targets` reports 9 warnings (6 collapsible `if`, 2 very complex types, 1 needless lifetime), with no gate. Fix: clean them up and add `-D warnings`.
- **No ESLint / Prettier (or Biome) for `src/`** — the TS/React side has type checking only; nothing catches broken hooks rules or unused imports beyond `tsc`. Fix: add a linter and formatter (see fix 3).

### Low severity

- **`.editorconfig`** missing — newline and indent settings across editors. The repo relies on CRLF/BOM fixtures, which `.gitattributes` already pins. Fix: add a minimal `.editorconfig`.
- **`.env.example`** — not applicable: the app uses no environment variables, and secrets are forbidden by design.

## Stack Assessment Cross-Reference

```
Stack assessment: context/foundation/stack-assessment.md
Agent readiness (from stack-assess): ready-with-compensation
```

| Quality Gate Gap                                | Health-Check Finding                                                                   | Status     |
|-------------------------------------------------|----------------------------------------------------------------------------------------|------------|
| React 19 not convention-based (fail)             | No ESLint/Prettier, no frontend tests; `AGENTS.md` still lacks the "Frontend Conventions" block | Reinforced |
| Tauri 2 training data mixes v1/v2 (partial)      | Builds and tests pass and the lockfile pins 2.12.7, but `AGENTS.md` lacks the "Tauri 2 Only" block | Open       |
| Hand-mirrored Rust↔TS contracts (typed, caveat)  | `tsc` runs in CI (`npm run build`), so TS-side breaks surface; Rust-side renames still surface only at runtime | Partially mitigated |
| No lint/format gate                              | Confirmed: 41 fmt hunks, 9 clippy warnings, no lint stage in CI                         | Reinforced |
| No frontend tests                                | Confirmed: no runner in `src/`; 181 Rust tests passing                                  | Reinforced |
| `AGENTS.md` storage rule conflicts with new PRD   | `AGENTS.md` unchanged: still allows only `settings.json` and the log dir                 | Open       |

The stack assessment's compensation entries are not in `AGENTS.md` yet. Its readiness verdict assumes they will be, so until they land the effective readiness is lower than "ready with compensation".

## Recommended Fixes

### Fix before agent work (Category A)

### 1. Patch the `url` advisory (it also turns CI red)

**Impact**: `cargo audit` in CI fails on any vulnerability, so every push stays red until this is patched, and an agent can no longer use CI to confirm its own work.
**Severity**: medium
**Effort**: quick (< 5 min)
**Fix**:

```bash
cd src-tauri
cargo update -p url        # 2.5.8 -> 2.5.9
cargo audit                # expect: 0 vulnerabilities
cargo test
```

Commit `Cargo.lock` on its own (no version bump, so nothing gets published).

### 2. Enforce rustfmt and clippy

**Impact**: agents get immediate mechanical feedback, and diffs stay limited to real changes.
**Severity**: medium
**Effort**: moderate (15–30 min)
**Fix**:

```bash
cd src-tauri
cargo fmt                                  # one-off: 41 hunks, commit separately ("style: cargo fmt")
cargo clippy --all-targets --fix --allow-dirty
cargo clippy --all-targets -- -D warnings  # fix the remaining 2 complex-type + any leftovers by hand
cargo test
```

Then add to the build/test job in `.github/workflows/ci.yml`, before `cargo build`:

```yaml
      - name: cargo fmt
        working-directory: src-tauri
        run: cargo fmt --check
      - name: cargo clippy
        working-directory: src-tauri
        run: cargo clippy --all-targets --locked -- -D warnings
```

The `dtolnay/rust-toolchain@stable` step needs `with: components: rustfmt, clippy`.

### 3. Add a linter and formatter for `src/`

**Impact**: covers the React side, which the stack assessment marked as convention-less; catches hooks-rule and import mistakes that `tsc` misses.
**Severity**: medium
**Effort**: moderate (15–30 min)
**Fix**:

```bash
npm install -D eslint @eslint/js typescript-eslint eslint-plugin-react-hooks prettier
```

Create `eslint.config.js` (flat config: `@eslint/js` recommended + `typescript-eslint` recommended + `react-hooks` recommended, scoped to `src/**/*.{ts,tsx}`) and `.prettierrc` (`{}` is enough). Add the scripts `"lint": "eslint src"` and `"format:check": "prettier --check src"` to `package.json`, then a `npm run lint` step in CI after `npm ci`.

### 4. Review the unmaintained `chrono` direct dependency

**Impact**: the work-time counter and norm calculation in the new MVP will lean on date/time handling, so building that core on an unmaintained crate is a long-term risk.
**Severity**: low
**Effort**: quick (< 5 min) to decide
**Fix**: read https://rustsec.org/advisories/RUSTSEC-2026-0543 for its recommended alternative, and record the decision (keep or migrate) in the plan for the work-time slice. No code change is needed now; `cargo audit` does not fail on this warning.

### 5. Decide on frontend tests

**Impact**: without UI tests an agent cannot check view logic (the "To do" limit display, hidden/changed markers, error versus empty states) except by hand.
**Severity**: low (provided the domain rules stay in Rust, as stack-assessment recommends)
**Effort**: significant (> 1 hour) to set up and write the first tests
**Fix**: keep the rules in Rust and covered by `cargo test`. If UI logic grows, add Vitest with Testing Library:

```bash
npm install -D vitest @testing-library/react @testing-library/jest-dom jsdom
```

Add `test: { environment: "jsdom" }` to `vite.config.ts`, a `"test": "vitest run"` script, and a CI step.

### 6. Plan the TypeScript 7 upgrade (do not do it now)

**Impact**: TS 7 is one major version ahead. Staying on 6 is safe; upgrading in the middle of the MVP would add avoidable churn.
**Severity**: low
**Effort**: moderate (15–30 min) when done
**Fix**: keep `"typescript": "~6.0.3"` pinned. Upgrade between slices on its own branch: `npm install -D typescript@7`, then `npm run build`, and read the 7.0 release notes for breaking changes.

### 7. Add `.editorconfig`

**Impact**: keeps agent edits consistent across editors; minor.
**Severity**: low
**Effort**: quick (< 5 min)
**Fix**:

```ini
root = true

[*]
charset = utf-8
insert_final_newline = true
indent_style = space
indent_size = 2

[*.rs]
indent_size = 4

[src-tauri/tests/fixtures/**]
# byte-exact fixtures (CRLF/BOM) are governed by .gitattributes; do not normalise
charset = unset
end_of_line = unset
insert_final_newline = unset
```

### 8. Install `cargo-outdated` for future checks (optional)

**Impact**: the outdated-crates check was skipped. Without it, drift in Rust crates beyond semver ranges is invisible.
**Severity**: low
**Effort**: quick (< 5 min)
**Fix**: `cargo install cargo-outdated`, then `cargo outdated --root-deps-only` in `src-tauri/`.

### Addressed in upcoming lessons (Category B)

### Compensation entries in `AGENTS.md`

**Lesson**: [Agent Onboarding: Agents.md, AI Rules i feedback loops (M1L4)](https://platforma.przeprogramowani.pl/external/10xdevs-3/m1-l4)
**What you'll do there**: turn the ready-to-paste blocks from `stack-assessment.md` (frontend conventions, Rust↔TS contracts, Tauri 2 only, verification commands) into the project's agent rules and feedback loops. One exception should not wait for the lesson: the storage-rule replacement. The current hard rule forbids the app-owned data that the new PRD requires, so update it before the first MVP slice is planned.

CI/CD and deployment are already in place (GitHub Actions + installer releases), so there are no infrastructure items to defer.

## Summary

Health status: needs-attention

The project rests on a solid base: lockfiles are pinned and used in CI, 181 Rust tests pass, there are no npm vulnerabilities, and CI already covers secret scanning, audits, the type check, build and tests. Four things need attention: a freshly published advisory on the transitive `url` crate, which also makes the next CI run fail; no formatter or linter gate on either side (41 rustfmt hunks, 9 clippy warnings, nothing for `src/`); no frontend tests; and the stack assessment's compensation entries, including the storage-rule conflict, are not yet in `AGENTS.md`.

Next step: apply fix 1 (`cargo update -p url`) and fix 2 now, update the `AGENTS.md` storage rule, then proceed to agent onboarding and to updating the roadmap for the new PRD.
