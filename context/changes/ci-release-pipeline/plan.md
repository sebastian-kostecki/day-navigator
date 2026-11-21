# CI and Release Pipeline Implementation Plan

## Overview

This plan delivers roadmap foundation F-03:

- A private GitHub repository.
- A GitHub Actions workflow that builds, tests and audits every push on a Windows runner.
- A gated release job. It publishes a per-user NSIS installer of "Nawigator dnia" as a GitHub Release whenever a push to `main` carries a version that has not been released yet.
- Because the repository may go public later, a guard against secrets and work data: gitleaks in CI, plus a local pre-push hook that checks outgoing commits against a denylist of work terms kept outside the repo.

No secrets are added. The only credential the workflow uses is the built-in `GITHUB_TOKEN`.

## Current State Analysis

- **No remote, no CI.** `git remote -v` is empty and there is no `.github/` folder. AGENTS.md ("Commits and Pull Requests", `AGENTS.md:40`) says "No commit convention, remote or CI yet" and points to `context/foundation/tech-stack.md`. That file plans GitHub Releases from GitHub Actions on merge to `main` (`tech-stack.md:8-10,24`).
- **`gh` is not installed** on this machine, in Git Bash or in PowerShell. AGENTS.md Hard Rules allow GitHub access only through a logged-in `gh`, so creating the repository is a manual prerequisite.
- **App identity is still the starter's:**
  - `productName` (`src-tauri/tauri.conf.json:3`) and the window title (`tauri.conf.json:15`) are `day-navigator`.
  - `index.html:7` still has the template title.
  - The product name was settled as "Nawigator dnia" (`context/foundation/prd.md:215-216`).
- **The version is defined in three places**, all `0.1.0`: `tauri.conf.json:4`, `src-tauri/Cargo.toml:3` and `package.json:4`.
- **Bundle targets are `"all"`** (`tauri.conf.json:26`), which builds both MSI and NSIS on Windows.
- **Tests must run on Windows.** `src-tauri/tests/vault_guard.rs:241,267` contains `#[cfg(windows)]` junction tests. The fixture vault stays byte-exact on a Windows checkout because `.gitattributes` sets `src-tauri/tests/fixtures/** -text`.
- **Release builds are slow.** The release profile (`src-tauri/Cargo.toml` `[profile.release]`) uses `lto = true` and `codegen-units = 1`.
- **Audit baseline:** `cargo audit` has 0 vulnerabilities and 6 informational warnings (unmaintained/unsound crates, mostly Linux-only). `npm audit` has 0 (`context/changes/bootstrap-verification/verification.md:81-100`). By default `cargo audit` fails only on vulnerabilities, so the 6 warnings do not turn CI red.
- **History is clean.** The 10 tracked commits contain no company hostnames or URLs; only public docs domains and `localhost` appear. Pushing the existing history therefore does not leak work data.

## Desired End State

- **The private GitHub repo exists** (`day-navigator`) with `main` pushed.
- **`.github/workflows/ci.yml` runs on every push to any branch.** It has these jobs:
  - `checks` (Linux): gitleaks, `cargo audit`, `npm audit --audit-level=high`, and the pre-push hook self-test.
  - `build-test` (Windows): `npm ci`, `npm run build`, `cargo build`, `cargo test`.
  - `version` (Linux, `main` only): decides whether this version is new.
  - `release` (Windows, `main` only): `needs` all of the above and runs only when the version is new.
- **Release behavior.** A push to `main` with a version in `src-tauri/Cargo.toml` that has no GitHub Release yet publishes:
  - release `Nawigator dnia v<version>`,
  - tag `app-v<version>`,
  - one asset labelled `Nawigator dnia_<version>_x64-setup.exe` (GitHub renames the file itself to `Nawigator.dnia_<version>_x64-setup.exe`; tauri-action keeps the original name in the asset `label`).

  A push without a version bump runs CI and skips the release, and the run stays green. A red CI never publishes.
- **The installer** installs per user (no admin) and launches a window titled "Nawigator dnia". The identifier stays `com.sebas.day-navigator`, so the app-data path is unchanged.
- **`.githooks/pre-push`** blocks a push when any outgoing commit message, path or added line matches a term in `%USERPROFILE%\.day-navigator\push-denylist.txt`. It also blocks when the denylist file is missing. It reports where the match is, never the matched term.
- **AGENTS.md** documents three things: the release flow (bump the version in `Cargo.toml`), the one-time hook setup, and that the hook is never bypassed.

Verify by watching the first `main` run publish `app-v0.1.0`, installing it, then pushing a commit without a version bump and seeing the release job skipped.

### Key Discoveries:

- **tauri-action overwrites existing releases silently.** `tauri-apps/tauri-action@v1` with `tagName: app-v__VERSION__` reuses an existing release with the same tag: it deletes each same-named asset and uploads it again, with no error. That makes a separate "is this version new?" check mandatory. Sources: [tauri-action](https://github.com/tauri-apps/tauri-action), `src/upload-release-assets.ts`.
- **Tauri falls back to Cargo's version.** If `version` is omitted from `tauri.conf.json`, Tauri uses the version in `Cargo.toml` ([config reference](https://v2.tauri.app/reference/config/)). That gives one version source.
- **NSIS fits this app; MSI does not.** NSIS defaults to `installMode: currentUser` (`%LOCALAPPDATA%`, no admin) and has no version-number limits. MSI requires admin, numeric versions with major/minor ≤255, and the VBScript feature ([Windows installer guide](https://v2.tauri.app/distribute/windows-installer/)).
- **GitHub Free cannot enforce checks on a private repo.** Neither rulesets nor branch protection are available there ([plans](https://docs.github.com/en/get-started/learning-about-github/githubs-plans)), so the only gate is `release` needing the check jobs inside the workflow.
- **Actions budget on Free:** about $12/month of compute. Windows costs $0.010/min and Linux $0.006/min, so roughly 1,200 Windows minutes. Cheap checks therefore run on Linux, and only build/test and release run on Windows.
- **gitleaks-action cannot scan a first push.** On push events it scans `--first-parent --no-merges <commits[0]>^..<head>` (`src/gitleaks.js`). When the push contains the root commit, `<root>^` does not exist and the job fails (open issue gitleaks-action#199). Merge commits are skipped too, and v2 declares `node20`, which GitHub removed on 2026-09-23. The plan therefore runs the gitleaks CLI over the full history instead of the action.
- **tauri-action does not roll back.** It builds, creates the release (non-draft), then uploads with `retryAttempts` 0 (`src/index.ts`). A failed upload leaves a published release with no installer. On an existing release it also rewrites name and body and deletes each same-named asset before re-uploading it.
- **`gh release view` exits 1 for every failure.** "release not found", 5xx, network and 401 all exit 1; only "not logged in" exits 4 (cli `pkg/cmd/release/shared/fetch.go`). Only the stderr text `release not found` identifies a missing release.
- **Artifact attestations need a public repo** (or Enterprise on private), so they are out of scope while the repo is private.

## What We're NOT Doing

- No code signing and no Tauri updater. Both need keys stored as secrets, which the NFR "Zero sekretów" rules out. The unsigned installer shows the SmartScreen "More info → Run anyway" prompt, and the release notes say so.
- No MSI installer.
- **Not "every merge publishes".** The roadmap outcome says each merge publishes. This plan publishes only merges that bump the version; others run CI and skip the release. This was decided in planning to avoid silent overwrites and to save Windows minutes.
- No automatic version bumping: no run-number versions, release-please or changesets.
- No `cargo clippy -D warnings` or `cargo fmt --check` gate.
- No `pull_request` trigger. Pushes already run CI, and a solo repo would otherwise run everything twice.
- No path filters that skip CI for documentation-only commits.
- No branch protection or rulesets (not available on Free for private repos).
- No change to the app identifier, crate name or npm package name.
- No checksum or attestation in the release.
- No work-term check in CI. The denylist must never be in the repo, so only the local hook can use it.
- No rewriting of existing history (it is already clean).

## Implementation Approach

The work runs in four steps:

1. **Local first.** Make the app produce the right installer locally: name, NSIS only, single version source.
2. **Then the guard.** The hook must be active before anything leaves the machine.
3. **Then the workflow file.**
4. **Finally the irreversible outward step.** Create the private repo and push, guarded by the hook, then watch the first runs. The order matters: the first push sends the whole history, and the hook scans all of it.

The workflow is one file with four jobs so that `release` can `needs:` the checks. That is the in-workflow substitute for the branch protection Free lacks. The decision to publish lives in a cheap Linux job (`version`), so a push without a version bump spends no Windows release minutes.

## Critical Implementation Details

- **`gh` needs the `workflow` scope.** Pushing `.github/workflows/*` over HTTPS with `gh` as the credential helper requires the `workflow` OAuth scope, which the default `gh auth login` scopes do not include. Use `gh auth login --scopes workflow`, or `gh auth refresh -s workflow`. Otherwise the first push is rejected.
- **Bumping the version touches two files.** The bump changes `src-tauri/Cargo.toml` and, after a local `cargo build`, the root-package entry in `Cargo.lock`. CI builds with `--locked`, so a bump committed without the refreshed `Cargo.lock` fails `build-test` with a clear error. That is intended; the fix is to run `cargo build` and commit the lock.
- **The hook must never print a matched term.** Its output goes to terminals and agent transcripts, and work data must never reach logs. It prints the commit short SHA, the file path (or "commit message") and the denylist line number only.
- **Blank and comment lines in the denylist must be stripped.** An empty pattern in `grep -F -f` matches every line, so the hook removes blank lines and `#` comments before matching.
- **The denylist is a hand-edited Windows file, so it must be normalized.** An editor may save it with CRLF line endings or a UTF-8 BOM. A pattern ending in `\r` never matches LF content, and a BOM hides the first term, so without normalization the hook passes silently. The hook strips the BOM, `\r` and surrounding whitespace from every term. It blocks the push when no term remains (an empty or comments-only file), just as it does when the file is missing.
- **A relative `core.hooksPath` runs no hook when the file is absent.** `.githooks` resolves against the checked-out tree, so on a branch or worktree that predates the hook, git silently pushes unguarded. Every push therefore happens from a checkout that contains `.githooks/pre-push`; Phase 4 checks this before the first push.
- **A missing denylist fails closed.** In line with the Hard Rule "if a configured path is missing, show an error", the hook blocks the push and prints the expected path.
- **The release job builds again on its own.** `release` builds and bundles in its own job and does not reuse the `build-test` debug artifacts. `Swatinem/rust-cache` keys on the job, so `build-test` and `release` keep separate caches. Expect the first release run to be the slowest (about 10–20 minutes, an estimate).
- **`productName` may rename the binary.** After the change from `day-navigator` to "Nawigator dnia", the Phase 1 manual check confirms that the installed app launches and the Start-menu entry reads "Nawigator dnia". If the main executable name changes, that is acceptable; nothing references it yet.

## Phase 1: App identity and installer config

### Overview

The app builds a correctly named, per-user NSIS installer locally, and its version comes from one place.

### Changes Required:

#### 1. Tauri config

**File**: `src-tauri/tauri.conf.json`

**Intent**: Show the settled product name and ship only the per-user NSIS installer, with `Cargo.toml` as the only version source.

**Contract**:
- `productName` → `"Nawigator dnia"`.
- Remove the `version` key, so Tauri falls back to `src-tauri/Cargo.toml`.
- `app.windows[0].title` → `"Nawigator dnia"`.
- `bundle.targets` → `["nsis"]`.
- `identifier` stays `com.sebas.day-navigator`.

#### 2. Web page title

**File**: `index.html`

**Intent**: Replace the template `<title>` with "Nawigator dnia", so the webview title matches.

**Contract**: `<title>` element only.

### Success Criteria:

#### Automated Verification:

- Frontend type check and build pass: `npm run build`
- Rust build passes: `cargo build` (in `src-tauri/`)
- Tests pass: `cargo test` (in `src-tauri/`)
- Local release bundle produces `src-tauri/target/release/bundle/nsis/Nawigator dnia_0.1.0_x64-setup.exe` and no `msi/` folder: `npm run tauri build`

#### Manual Verification:

- Running the local installer needs no admin prompt, installs under `%LOCALAPPDATA%`, and the Start-menu entry and window title read "Nawigator dnia"
- The installed app launches and can be uninstalled from Windows Settings

**Implementation Note**: After automated verification passes, pause for manual confirmation before Phase 2.

---

## Phase 2: Local pre-push guard

### Overview

A versioned pre-push hook stops any push whose outgoing commits contain a work term from a denylist that lives only on this machine. A self-test proves it with invented terms.

### Changes Required:

#### 1. Pre-push hook

**File**: `.githooks/pre-push` (new, POSIX `sh`/`bash`, LF line endings, executable bit set in git: `git update-index --chmod=+x`)

**Intent**: Before anything leaves the machine, check every outgoing commit's message, changed file paths and added diff lines against the denylist. Block the push on any case-insensitive fixed-string match, or when the denylist is missing.

**Contract**:
- Reads the standard pre-push stdin lines (`<local ref> <local sha> <remote ref> <remote sha>`) and skips deletions (local sha of all zeros).
- Outgoing commits: `<remote sha>..<local sha>` when the remote sha is known. For a new branch, `git rev-list <local sha> --not --remotes=<remote name>`; on the very first push that is the whole history.
- Denylist path: `$DAYNAV_DENYLIST` if set, otherwise `$HOME/.day-navigator/push-denylist.txt`. Under Git for Windows, `$HOME` is `%USERPROFILE%`. Blank lines and `#` comments are ignored.
- Denylist normalization: strip a leading UTF-8 BOM, `\r` and leading/trailing whitespace from every line before the blank/comment filter.
- Missing denylist, or no term left after normalization: exit non-zero with a message naming the expected path.
- Matching: loop over the original numbered lines of the denylist, one `grep -F -i` per term, so the report can name the original line number (`grep -F -f` cannot say which pattern matched). Run with `LC_ALL=C.UTF-8`, so `-i` also folds Polish letters (Ł/ł).
- Match: exit non-zero and list `<short sha> <path | commit message> denylist line <n>`, never the term itself.
- No match: exit 0 silently.

#### 2. Hook self-test

**File**: `scripts/test-pre-push.sh` (new)

**Intent**: Prove the hook's behavior in a throwaway repo using invented terms only (for example `ACME-WIDGETS`, `intranet.example.test`). It runs locally and in CI.

**Contract**: Creates a temp bare remote and a clone (`git init -b main`, git identity passed as `-c user.name=… -c user.email=…`, because CI runners have none), points `DAYNAV_DENYLIST` at a temp file, and runs the hook through a real `git push`. It asserts:
- a clean commit pushes,
- a match in an added line, in a file path, and in a commit message each blocks,
- the hook output does not contain the matched term,
- the reported line number is the term's line in the original file (with comments above it),
- a missing denylist blocks,
- a comments-only denylist blocks,
- a blank line in the denylist does not match everything,
- a denylist saved with CRLF line endings and a UTF-8 BOM still matches, including its first term,
- matching is case-insensitive, including a Polish letter (an invented term with `Ł` matches `ł`).

Exits non-zero on the first failed assertion and leaves no files outside the temp dir.

#### 3. Line endings for scripts

**File**: `.gitattributes`

**Intent**: Keep shell scripts LF on Windows checkouts, so `sh` can run them on both runners.

**Contract**: Add `.githooks/* text eol=lf` and `*.sh text eol=lf`. The existing fixture rule stays unchanged.

### Success Criteria:

#### Automated Verification:

- Hook self-test passes: `bash scripts/test-pre-push.sh`
- Hook is executable in the index: `git ls-files -s .githooks/pre-push` shows mode `100755`
- No real work term appears in tracked files (file names only, terms never printed): `git grep -l -i -F -f <(sed -e '1s/^\xEF\xBB\xBF//' -e 's/\r$//' "$HOME/.day-navigator/push-denylist.txt" | grep -v -E '^[[:space:]]*(#|$)')` lists nothing (run after the denylist exists)

#### Manual Verification:

- Denylist created at `%USERPROFILE%\.day-navigator\push-denylist.txt` with real work terms (company names, hostnames, project keys), and `git config core.hooksPath .githooks` set in this clone
- A push to a throwaway local bare remote with a commit containing one denylist term is blocked, and the message shows no term

**Implementation Note**: After automated verification passes, pause for manual confirmation before Phase 3.

---

## Phase 3: CI and release workflow

### Overview

A single workflow builds, tests and audits every push. On `main` it publishes the release only when the version is new and every check is green.

### Changes Required:

#### 1. Workflow

**File**: `.github/workflows/ci.yml` (new)

**Intent**: Run CI on every push and publish the NSIS installer from `main` when the version is new and all checks passed. Use only `GITHUB_TOKEN`.

**Contract**:
- **Triggers:** `push` on all branches, plus `workflow_dispatch`.
- **Top-level permissions:** `contents: read`.
- **Concurrency:** group per ref. `cancel-in-progress` is true except for `refs/heads/main`.
- **`checks` (`ubuntu-latest`):**
  - Checkout with `fetch-depth: 0`.
  - gitleaks CLI (a pinned release binary, not `gitleaks-action`), scanning the full history on every push: `gitleaks git --redact --exit-code 1 .`. It covers the root commit and merge commits; the action covers neither (see Key Discoveries). The repo is small, so a full scan takes seconds.
  - `cargo audit` in `src-tauri/`, installed via `taiki-e/install-action` (prebuilt binary). Default deny level, which fails on vulnerabilities only.
  - `npm audit --audit-level=high`.
  - `bash scripts/test-pre-push.sh`.
- **`build-test` (`windows-latest`):**
  - `actions/setup-node` with npm cache, Node 24.
  - `dtolnay/rust-toolchain@stable`.
  - `Swatinem/rust-cache@v2` (`workspaces: './src-tauri -> target'`).
  - `npm ci`, `npm run build`, `cargo build --locked`, `cargo test --locked`.
- **`version` (`ubuntu-latest`):**
  - Runs only when `github.ref == 'refs/heads/main'`.
  - Reads `version` from the `[package]` table of `src-tauri/Cargo.toml`.
  - Outputs `version` and `publish`, using `gh release view app-v<version> --json assets` (with `GH_TOKEN`):
    - stderr contains `release not found` → `publish=true`;
    - the release exists but has no asset whose `label` ends in `-setup.exe` (an earlier upload failed) → `publish=true`, and tauri-action completes the existing release;
    - the release exists with that asset → `publish=false`;
    - any other failure (5xx, network, auth) → the job fails, which blocks `release`. It never defaults to publishing.
- **`release` (`windows-latest`):**
  - `needs: [checks, build-test, version]`, with `if: needs.version.outputs.publish == 'true'`.
  - `permissions: contents: write`.
  - Same toolchain setup and `npm ci` as `build-test`.
  - `tauri-apps/tauri-action@v1` with:
    - `tagName: app-v__VERSION__`
    - `releaseName: 'Nawigator dnia v__VERSION__'`
    - `releaseBody`: one short note that the installer is unsigned (SmartScreen: More info → Run anyway) and installs per user
    - `releaseDraft: false`, `prerelease: false`
    - `args: --bundles nsis`
    - `uploadUpdaterJson: false`
  - Concurrency group `release` with no cancellation.

Action versions: `actions/checkout`, `actions/setup-node` and the others are pinned to their current major tags at implementation time.

#### 2. Contributor guide

**File**: `AGENTS.md`

**Intent**: Replace "No commit convention, remote or CI yet" with the real flow, so future agents bump versions and keep the guard on.

**Contract**:
- **"Commits and Pull Requests":** the private GitHub remote, reached via `gh`. CI is `.github/workflows/ci.yml`. To release, bump `version` in `src-tauri/Cargo.toml`, run `cargo build` to refresh `Cargo.lock`, commit both, and push to `main`. Without a bump, nothing is published. One-time clone setup: `git config core.hooksPath .githooks` and the denylist at `%USERPROFILE%\.day-navigator\push-denylist.txt`.
- **Hard Rules:** add "Never bypass the pre-push hook (`--no-verify`) and never commit the denylist or its terms." Also add that `core.hooksPath` is relative, so pushes happen only from a checkout that contains `.githooks/pre-push`.

#### 3. Foundation docs

**Files**: `context/foundation/tech-stack.md`, `context/foundation/roadmap.md`

**Intent**: Record the narrowing decided in this plan, so later reviews do not flag it as drift.

**Contract**: `tech-stack.md:24` and the F-03 outcome in the roadmap say that a merge to `main` publishes an installer only when it bumps the version in `src-tauri/Cargo.toml`. Every other merge only runs CI.

### Success Criteria:

#### Automated Verification:

- Frontend build still passes: `npm run build`
- Rust build still passes: `cargo build` (in `src-tauri/`)
- Workflow file names the four jobs: `node -e "const y=require('fs').readFileSync('.github/workflows/ci.yml','utf8'); for (const j of ['checks:','build-test:','version:','release:']) if(!y.includes(j)) process.exit(1)"`
- No secret references besides GITHUB_TOKEN: `git grep -n "secrets\." .github/` lists only `secrets.GITHUB_TOKEN`

#### Manual Verification:

- Workflow reviewed against the Contract: release `needs` all three jobs and is skipped when `publish` is not `true`; only `release` has `contents: write`
- AGENTS.md release and hook sections read correctly to someone new to the repo

**Implementation Note**: After automated verification passes, pause for manual confirmation before Phase 4.

---

## Phase 4: Private repo and first release

### Overview

The irreversible outward step: create the private repo, push through the hook, and prove the pipeline end to end. This phase is mostly manual, because the agent has no `gh` and the push is outward-facing.

### Changes Required:

#### 1. One-time machine setup (user)

**Intent**: Give this machine a logged-in `gh` that can push workflow files, with the hook active.

**Contract**:
- Install GitHub CLI (`winget install GitHub.cli`).
- `gh auth login --scopes workflow` (HTTPS, browser).
- In the main checkout: `git config core.hooksPath .githooks`, with the denylist present from Phase 2.

#### 2. Create the repo and push (user, or the agent after explicit approval)

**Intent**: Create the private repo from the existing local history. The hook scans the whole history on this first push.

**Contract**:
- Pre-flight in the main checkout: `test -x .githooks/pre-push && git config core.hooksPath` prints `.githooks`. Otherwise the push would run unguarded.
- From the main checkout, on `main`, after this change is merged there: `gh repo create day-navigator --private --source . --remote origin --push`.
- Visibility must be `PRIVATE`. Check with `gh repo view --json visibility`.

#### 3. Confirm the release-skip path

**Intent**: Prove that a push without a version bump does not republish.

**Contract**: Push a commit to `main` that changes no version (for example, the Progress update of this plan). Its run shows `release` as skipped and the run is green.

### Success Criteria:

#### Automated Verification:

- Repo is private: `gh repo view --json visibility -q .visibility` prints `PRIVATE`
- First `main` run succeeded: `gh run list --branch main --limit 1 --json conclusion -q '.[0].conclusion'` prints `success`
- Release exists with exactly one NSIS asset: `gh release view app-v0.1.0 --json name,assets -q '.name, (.assets|length), .assets[0].label'` prints `Nawigator dnia v0.1.0`, `1`, `Nawigator dnia_0.1.0_x64-setup.exe`

#### Manual Verification:

- Installer downloaded from the release installs per user after SmartScreen "Run anyway" and launches "Nawigator dnia"
- A second push to `main` without a version bump shows the `release` job skipped, the run green, and `app-v0.1.0` assets unchanged
- A push on a non-`main` branch runs `checks` and `build-test` only

**Implementation Note**: This phase closes F-03. `/10x-archive` later marks the roadmap item `done`.

---

## Testing Strategy

### Unit Tests:

- No Rust or TypeScript code changes, so no new unit tests. Existing `cargo test` runs in CI on Windows and must stay green.

### Integration Tests:

- `scripts/test-pre-push.sh` is the integration test for the guard. It runs a real `git push` against a temp bare remote with invented terms, covering block/allow, path/message/line matches, original line numbers, missing and comments-only denylists, blank lines, a CRLF+BOM denylist, case-insensitivity (including Polish letters) and no term echoed. It runs locally and in the `checks` job.

### Manual Testing Steps:

1. Build the installer locally (Phase 1), install it, and confirm the name, per-user install and launch.
2. Try a push containing a denylist term to a local bare remote and confirm it is blocked without printing the term.
3. Create the repo, then watch the first `main` run: `checks`, `build-test`, `version` (publish=true), `release`.
4. Install from the GitHub Release.
5. Push again without a bump and confirm `release` is skipped.

## Performance Considerations

Actions cost is the budget that matters: about 1,200 Windows minutes a month on Free.

- Linux runs everything that does not need Windows: gitleaks, audits, the hook self-test and the version check.
- Branch pushes cancel superseded runs.
- Release builds (LTO, `codegen-units = 1`) run only on a version bump.

If minutes become tight, the first lever is a `paths-ignore` for `context/**`. That is deferred, not planned.

## Migration Notes

- After Phase 1, the local version source is `src-tauri/Cargo.toml`. `package.json` keeps `0.1.0` but is no longer used for the app version. AGENTS.md says so.
- An earlier local install named "day-navigator" (if any) is a different product name. Uninstall it manually; app data under the unchanged identifier stays where it is.
- The main checkout currently has uncommitted work (`context/foundation/roadmap.md`, `context/changes/source-health-status/`). Commit it, or confirm it is clean of work data, before the first push, because the hook only scans committed content.

## References

- Roadmap item: `context/foundation/roadmap.md` (F-03, Change ID `ci-release-pipeline`)
- PRD: `context/foundation/prd.md:164-165` (NFR "Dane pracy nie opuszczają maszyny", "Zero sekretów"), `prd.md:215-216` (name)
- Stack hand-off: `context/foundation/tech-stack.md:8-10,24`
- Audit baseline: `context/changes/bootstrap-verification/verification.md:81-104`
- Windows-only tests: `src-tauri/tests/vault_guard.rs:241,267`
- tauri-action: https://github.com/tauri-apps/tauri-action, Tauri GitHub pipeline guide: https://v2.tauri.app/distribute/pipelines/github/
- Tauri config (version fallback, bundle targets): https://v2.tauri.app/reference/config/
- Windows installers (NSIS installMode): https://v2.tauri.app/distribute/windows-installer/
- GitHub plans and Actions billing: https://docs.github.com/en/get-started/learning-about-github/githubs-plans, https://docs.github.com/en/billing/concepts/product-billing/github-actions

## Progress

> Convention: `- [ ]` pending, `- [x]` done. Append ` — <commit sha>` when a step lands. Do not rename step titles. See `references/progress-format.md`.

### Phase 1: App identity and installer config

#### Automated

- [x] 1.1 Frontend type check and build pass: `npm run build` — 8a1c12d
- [x] 1.2 Rust build passes: `cargo build` (in `src-tauri/`) — 8a1c12d
- [x] 1.3 Tests pass: `cargo test` (in `src-tauri/`) — 8a1c12d
- [x] 1.4 Local release bundle produces `src-tauri/target/release/bundle/nsis/Nawigator dnia_0.1.0_x64-setup.exe` and no `msi/` folder: `npm run tauri build` — 8a1c12d

#### Manual

- [ ] 1.5 Running the local installer needs no admin prompt, installs under `%LOCALAPPDATA%`, and the Start-menu entry and window title read "Nawigator dnia"
- [ ] 1.6 The installed app launches and can be uninstalled from Windows Settings

### Phase 2: Local pre-push guard

#### Automated

- [x] 2.1 Hook self-test passes: `bash scripts/test-pre-push.sh` — a8413ca
- [x] 2.2 Hook is executable in the index: `git ls-files -s .githooks/pre-push` shows mode `100755` — a8413ca
- [x] 2.3 No real work term appears in tracked files (file names only, terms never printed): `git grep -l -i -F -f <(sed -e '1s/^\xEF\xBB\xBF//' -e 's/\r$//' "$HOME/.day-navigator/push-denylist.txt" | grep -v -E '^[[:space:]]*(#|$)')` lists nothing (run after the denylist exists) (zaakceptowane przez użytkownika: 2 dopasowania terminu #2 to zwykłe słowo, nie dane z pracy) — a8413ca

#### Manual

- [ ] 2.4 Denylist created at `%USERPROFILE%\.day-navigator\push-denylist.txt` with real work terms (company names, hostnames, project keys), and `git config core.hooksPath .githooks` set in this clone
- [ ] 2.5 A push to a throwaway local bare remote with a commit containing one denylist term is blocked, and the message shows no term

### Phase 3: CI and release workflow

#### Automated

- [x] 3.1 Frontend build still passes: `npm run build` — 85143b7
- [x] 3.2 Rust build still passes: `cargo build` (in `src-tauri/`) — 85143b7
- [x] 3.3 Workflow file names the four jobs: `node -e "const y=require('fs').readFileSync('.github/workflows/ci.yml','utf8'); for (const j of ['checks:','build-test:','version:','release:']) if(!y.includes(j)) process.exit(1)"` — 85143b7
- [x] 3.4 No secret references besides GITHUB_TOKEN: `git grep -n "secrets\." .github/` lists only `secrets.GITHUB_TOKEN` — 85143b7

#### Manual

- [ ] 3.5 Workflow reviewed against the Contract: release `needs` all three jobs and is skipped when `publish` is not `true`; only `release` has `contents: write`
- [ ] 3.6 AGENTS.md release and hook sections read correctly to someone new to the repo

### Phase 4: Private repo and first release

#### Automated

- [ ] 4.1 Repo is private: `gh repo view --json visibility -q .visibility` prints `PRIVATE`
- [ ] 4.2 First `main` run succeeded: `gh run list --branch main --limit 1 --json conclusion -q '.[0].conclusion'` prints `success`
- [ ] 4.3 Release exists with exactly one NSIS asset: `gh release view app-v0.1.0 --json name,assets -q '.name, (.assets|length), .assets[0].label'` prints `Nawigator dnia v0.1.0`, `1`, `Nawigator dnia_0.1.0_x64-setup.exe`

#### Manual

- [ ] 4.4 Installer downloaded from the release installs per user after SmartScreen "Run anyway" and launches "Nawigator dnia"
- [ ] 4.5 A second push to `main` without a version bump shows the `release` job skipped, the run green, and `app-v0.1.0` assets unchanged
- [ ] 4.6 A push on a non-`main` branch runs `checks` and `build-test` only
