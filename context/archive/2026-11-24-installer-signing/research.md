# Research: free code signing from SignPath Foundation

Date: 2026-11-24. Question: does Nawigator dnia (`sebastian-kostecki/day-navigator`, public, MIT) qualify for SignPath Foundation's free OSS code signing, and if so, how would CI sign the NSIS installer?

Nothing was applied for, no account was created and CI was not changed.

## Verdict

**Not yet. It meets the hard rules but will probably be refused on reputation.**

- Every technical and licence rule is met, or can be met with a small README change (see the table below).
- The application form has a required **Reputation** field: *"links or information showing that your project is widely used or trusted"* (media coverage, blog posts, download statistics, GitHub insights, community discussions). The terms say that executables signed with their certificate need "a certain verifiable reputation". They also say SignPath is "under no obligation to accept your project" and that there is no arbitration.
- Today the repository has 0 stars, 0 forks and 0 watchers. It was created on 2026-11-21. Its 5 releases have 1–2 downloads each, which is probably the author. The README calls it "a small personal desktop app", and its main use (a work GitLab plus `glab`/`gh`) is built for one person. There is nothing to put in the Reputation field.
- Even if accepted, the certificate is issued to SignPath Foundation, so SmartScreen and the file properties show **"SignPath Foundation"** as the publisher, not the author. SignPath's pages do not say whether this certificate carries SmartScreen reputation; this was not verified.

**Recommendation:** keep S-11 parked. Revisit when the app has outside users who can be shown: stars, downloads by others, a blog post or forum thread. Applying costs nothing and a refusal has no penalty. If you want to try anyway, do the "before applying" steps below first, because the download page must already mention SignPath when you apply.

## Sources read

| Source | What it gave |
| --- | --- |
| https://signpath.org/ | Free for OSS; list of signed projects |
| https://signpath.org/terms | Conditions (the page is marked "Draft") |
| https://signpath.org/apply | The application form. A HubSpot form inside an iframe, read on screen; nothing was typed or submitted. |
| https://signpath.org/about | Run by SignPath GmbH |
| https://docs.signpath.io/trusted-build-systems/github | GitHub Actions connector, origin verification |
| https://github.com/SignPath/github-action-submit-signing-request | Action inputs and outputs |
| https://docs.signpath.io/artifact-configuration (+ `/syntax`, `/examples`, `/reference`) | File types, metadata restrictions, deep signing |
| https://docs.signpath.io/projects, https://docs.signpath.io/signing-code | Signing policies, approval |
| https://v2.tauri.app/distribute/sign/windows/ | `signCommand`, certificate thumbprint |
| tauri-bundler `bundle/windows/nsis/mod.rs`, `bundle/windows/sign.rs` (tauri `dev` branch) | Which files `signCommand` signs |
| https://github.com/tauri-apps/tauri-action | Building without publishing |

`signpath.org/faq` returns 404.

## Eligibility, rule by rule

| SignPath condition (terms page) | This repo | Status |
| --- | --- | --- |
| OSI-approved licence, no commercial dual licensing, for all components | MIT (`LICENSE`); npm/crate dependencies are OSS | Met |
| No proprietary components | None | Met |
| No malware or PUP; no hacking tools | Read-only `glab`/`gh` wrapper | Met |
| Actively maintained | Daily commits since 2026-11-20 | Met |
| Already released in the form to be signed | NSIS installer on GitHub Releases since 0.1.0 | Met |
| Functionality described on the download page | README "Features"; the release page links to it | Met |
| Built from source in a verifiable, automated way; origin verified for each release | `release` job builds with `tauri-action` on `windows-latest` from the pushed commit | Met. The OSS rule also requires every job leading up to signing to run on GitHub-hosted runners, which CI already does. |
| Code signing policy on the project home page, and the download page mentions SignPath | Not present; the README says "not code-signed" | **To add** |
| Team roles: authors/committers, reviewers, approvers | One maintainer. The rules do not forbid one person holding every role, but they are written for a team. | **To add**; acceptable for one person is unclear |
| MFA for every team member, on SignPath and GitHub | Not checked here | **User to confirm** |
| Privacy: a policy, or the stated sentence that nothing is sent unless the user asks | App sends nothing except through the user's own `glab`/`gh` | **To add** (one sentence) |
| Announce system changes; provide uninstallation | The NSIS installer has an uninstaller; settings are a hand-written file | Met |
| Signed binaries have enforced metadata (product name = project name, product version) | `productName` "Nawigator dnia"; the version comes from `Cargo.toml` | Met; enforced in the artifact configuration |
| "Verifiable reputation" for downloadable executables (terms) / required "Reputation" field (form) | 0 stars, about 6 downloads in total, 3 days old, built for one user | **Not met** |
| Project name: "a Google search for this name should clearly identify your project" (form) | "Nawigator dnia" is a generic Polish phrase. Not checked by search here. | Probably not met |
| Every release approved by a person | One approval per release by the maintainer | Fits |

There are no published limits on cost (free), signing volume, or project age.

## What the user would have to do (not the agent)

These need the user's own accounts and decisions. AGENTS.md also forbids agents from adding credentials.

**Before applying**
1. Turn on MFA on GitHub, if it is not already on.
2. Add a `## Code signing policy` section to the README, and a line about it on the release page (release notes), containing:
   - "Free code signing provided by [SignPath.io](https://about.signpath.io), certificate by [SignPath Foundation](https://signpath.org)".
   - Roles: committers and reviewers, and approvers: the maintainer (GitHub profile link).
   - The privacy sentence: "This program will not transfer any information to other networked systems unless specifically requested by the user or the person installing or operating it."
   An agent can draft this section, but it should go in only once signing actually exists. Otherwise the README claims a signature the installer does not have.
3. Gather the Reputation evidence (see the verdict above).

**Applying** (https://signpath.org/apply). Fields: project name, repository URL, homepage URL, download URL (this page "must mention that the project uses the SignPath Foundation"), privacy policy URL, Wikipedia URL (optional), tagline, category, description, **Reputation** (required), maintainer type, build system, first and last name and e-mail (a SignPath user account is created from these), company (optional), how you found SignPath. There is a reCAPTCHA, so the user must fill in and submit the form. No waiting time is published.

**After acceptance**
1. In SignPath: turn on MFA, then confirm the organisation, project and `release-signing` policy SignPath sets up. Link the GitHub connector as the trusted build system, set the artifact configuration (below), and make yourself the approver.
2. Create an API token for a CI user with the submitter role, and store it as the GitHub Actions secret `SIGNPATH_API_TOKEN` (Settings → Secrets and variables → Actions). The token lives only in GitHub's encrypted secrets, never in the repo. MS-13 already allows this ("without secrets in the repo"). AGENTS.md's hard rule ("no passwords, tokens or keys in code, settings or the repo") would need one line saying a CI-only signing token in GitHub Actions secrets is the single exception. The SignPath action has no tokenless (OIDC) mode in the docs read.
3. Approve each release's signing request in the SignPath web UI. Approvers get an e-mail.

## CI design (if accepted)

### Why not Tauri's `signCommand`

`bundle.windows.signCommand` runs once per file during `tauri build`. The bundler signs the NSIS plugin DLLs, the uninstaller (through NSIS's `!uninstfinalize` hook) and the final setup `.exe`, each as a separate synchronous call. SignPath Foundation signing is asynchronous. Each request must come from a GitHub workflow artifact (for origin verification) and waits for a person to approve it. One release would mean 5 or more requests and 5 or more approvals inside a running build. That is not workable.

### Recommended: sign only the setup `.exe`, after the build

SmartScreen judges the file the user downloads, and that file is the setup `.exe`. The installed app and the uninstaller stay unsigned. This is one request and one approval per release.

SignPath has no NSIS file type. A setup `.exe` is a plain `<pe-file>`, so SignPath cannot deep-sign the app `.exe` inside it. The alternative, a two-pass build (`tauri build --no-bundle`, sign the app `.exe`, `tauri bundle --bundles nsis`, sign the installer), means two requests and two approvals per release. It also leaves the uninstaller and the plugin DLLs unsigned, which gains little. Do this only if Windows flags the installed exe in practice.

Changes to `.github/workflows/ci.yml`, `release` job:

1. Build without publishing. Drop `tagName`/`releaseName`/`releaseBody` from `tauri-action`, per its README ("simply omit `tagName`, `releaseName` and `releaseId`"). Or replace the action with `npx tauri build --bundles nsis`.
2. Upload the unsigned setup `.exe` with `actions/upload-artifact` (v4+, with `id:`). Either let it zip the file (the artifact configuration root is `<zip-file>`), or use `archive: false` together with `skip-decompress: true` on the submit step.
3. Submit and wait:
   ```yaml
   - uses: signpath/github-action-submit-signing-request@v3   # pin to a commit SHA
     with:
       api-token: '${{ secrets.SIGNPATH_API_TOKEN }}'
       organization-id: '<from SignPath>'
       project-slug: 'day-navigator'
       signing-policy-slug: 'release-signing'
       github-artifact-id: '${{ steps.upload.outputs.artifact-id }}'
       wait-for-completion: true
       wait-for-completion-timeout-in-seconds: 86400   # default 600 is too short for a manual approval
       output-artifact-directory: 'signed'
   ```
   Add `permissions: actions: read` next to `contents: write` (the action reads the workflow artifact). Check the job `timeout-minutes` (the GitHub-hosted maximum is 6 h) against how quickly you approve. Otherwise use a shorter wait and let a rerun pick up the result. Note that SignPath's optional `disallow_reruns` build policy interacts with this.
4. Publish the signed file with `gh release create app-v$VERSION signed/*-setup.exe --title … --notes …`, using the notes from the `version` job. The `version` job's check for an existing installer must still find the `-setup.exe` asset (it filters on `.label`; check that `gh release upload` sets a label, or match on `.name`).
5. Artifact configuration (in SignPath, not in the repo), sketch:
   ```xml
   <artifact-configuration xmlns="http://signpath.io/artifact-configuration/v1">
     <zip-file>
       <pe-file path="Nawigator dnia_*_x64-setup.exe" product-name="Nawigator dnia" product-version="${version}">
         <authenticode-sign/>
       </pe-file>
     </zip-file>
   </artifact-configuration>
   ```
   Check the exact `ProductVersion` string NSIS writes (for example `0.9.0` or `0.9.0.0`) against one real installer before turning on the restriction.
6. Text: the release notes script (`scripts/release-notes.sh`, MS-14) and the README "Install" section drop "not code-signed / Run anyway" and add the code signing policy line. `scripts/test-release-notes.sh` follows.
7. Optional hardening: SignPath `github-build-policies` with `runners.require_github_hosted: true` (available on the Open Source plan).

### Not affected

The `checks` and `build-test` jobs, gitleaks (the token never touches the repo), the pre-push hook, `tauri.conf.json` (no `signCommand` or thumbprint needed), and the Rust code.

## If refused, or until then

- Stay unsigned: the current state, with the README and release notes explaining "More info → Run anyway".
- OV certificate: about $129–319 a year, needs a cloud HSM, and still builds SmartScreen reputation slowly (earlier research, see roadmap `## Parked`).
- Azure Artifact Signing: not open to individuals in Poland (earlier research).

## Open points not verified here

- Whether SmartScreen treats installers signed with the SignPath Foundation certificate as already reputable.
- Whether SignPath accepts a one-person team holding the author, reviewer and approver roles.
- Whether a Google search for "Nawigator dnia" identifies the project.
- SignPath's current response time for applications.
