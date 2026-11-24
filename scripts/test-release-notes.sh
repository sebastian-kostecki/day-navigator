#!/bin/bash
# Self-test for scripts/release-notes.sh. Uses only invented data in a temp dir.
set -u
repo_root="$(cd "$(dirname "$0")/.." && pwd)"
script="$repo_root/scripts/release-notes.sh"
tmp="$(mktemp -d)"
trap 'cd /; rm -rf "$tmp"' EXIT

G=(-c user.name=selftest -c user.email=selftest@example.test)
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null
unset CHANGELOG REPO_URL
URL="https://example.test/acme/widget"
NOTE='Unsigned installer: if Windows SmartScreen warns, choose "More info" and then "Run anyway". It installs per user, no admin rights needed.'
passed=0
out="$tmp/out.txt"
err="$tmp/err.txt"

pass() { passed=$((passed + 1)); echo "PASS $1"; }
fail() {
  echo "FAIL $1"
  for f in "$out" "$err"; do [ -f "$f" ] && sed 's/^/  | /' "$f" | head -20; done
  exit 1
}
notes() { bash "$script" "$@" > "$out" 2> "$err"; }
commit() { git "${G[@]}" commit -q --allow-empty -m "$1" || fail "commit"; }

git init -q -b main "$tmp/repo" || fail "init"
cd "$tmp/repo" || exit 1

# CRLF line endings, as a Windows checkout with autocrlf would have them.
printf '%s\r\n' \
  '# Changelog' \
  '' \
  '## [Unreleased]' \
  '' \
  '- Pending gadget.' \
  '' \
  '## [0.11.0] - 2030-04-01' \
  '' \
  '- Eleventh widget.' \
  '' \
  '## [0.10.0] - 2030-03-01' \
  '' \
  '### Added' \
  '' \
  '- Tenth widget.' \
  '' \
  '## [0.2.10] - 2030-02-10' \
  '' \
  '- Patch ten.' \
  '' \
  '## [0.2.0] - 2030-02-01' \
  '' \
  '### Added' \
  '' \
  '- Blue widget.' \
  '' \
  '### Fixed' \
  '' \
  '- Red widget no longer squeaks.' \
  '' \
  '## [0.1.1] - 2030-01-15' \
  '' \
  '## [0.1.0] - 2030-01-01' \
  '' \
  '- First widget.' \
  '' > CHANGELOG.md

commit "first"
git tag app-v0.1.0
commit "second"
git tag app-v0.9.0
commit "third"

# --- section extraction -------------------------------------------------------
notes 0.2.0 || fail "0.2.0 should succeed"
expected="$(printf '%s\n' '### Added' '' '- Blue widget.' '' '### Fixed' '' '- Red widget no longer squeaks.' '' "$NOTE")"
[ "$(cat "$out")" = "$expected" ] || fail "0.2.0 body: section, trimmed, then note (no REPO_URL, no link)"
grep -q $'\r' "$out" && fail "output must not carry CR"
pass "middle section extracted, blank edges trimmed, CRLF stripped, note last"

grep -q "Patch ten" "$out" && fail "0.2.0 must not match 0.2.10"
pass "exact version match (0.2.0 is not 0.2.10)"

notes 0.1.0 || fail "0.1.0 should succeed"
[ "$(head -n 1 "$out")" = "- First widget." ] || fail "last section in the file"
pass "last section in the file extracted"

notes 0.3.0 && fail "missing section should fail"
grep -q "no entries under '## \[0.3.0\]'" "$err" || fail "missing section should say why"
pass "missing section fails with a reason"

notes 0.1.1 && fail "empty section should fail"
pass "empty section fails"

CHANGELOG="$tmp/nope.md" notes 0.2.0 && fail "missing changelog should fail"
grep -q "not found" "$err" || fail "missing changelog should say why"
pass "missing changelog file fails"

notes && fail "no argument should fail"
[ $? -ne 2 ] && fail "no argument exit code"
notes 'x.y; rm -rf /' && fail "bad version should fail"
pass "bad arguments fail with exit 2"

# --- compare link ---------------------------------------------------------------
REPO_URL="$URL" notes 0.2.0 || fail "0.2.0 with URL"
grep -qxF "**Full changelog:** $URL/compare/app-v0.9.0...app-v0.2.0" "$out" \
  || fail "previous tag is the highest reachable app-v* tag"
[ "$(tail -n 1 "$out")" = "$NOTE" ] || fail "note stays last"
pass "compare link from the previous tag, note last"

git tag app-v0.10.0
REPO_URL="$URL" notes 0.10.0 || fail "0.10.0 with own tag"
grep -qF "compare/app-v0.9.0...app-v0.10.0" "$out" || fail "own (highest) tag must be skipped"
pass "the version's own tag is never the previous one"

REPO_URL="$URL" notes 0.11.0 || fail "0.11.0"
grep -qF "compare/app-v0.10.0...app-v0.11.0" "$out" || fail "version sort, not string sort"
pass "tags sorted as versions (0.10.0 > 0.9.0)"
git tag -d app-v0.10.0 > /dev/null

git checkout -q -b side HEAD~2
commit "side"
git tag app-v0.99.0
git checkout -q main
REPO_URL="$URL" notes 0.2.0 || fail "0.2.0 with side tag"
grep -qF "compare/app-v0.9.0...app-v0.2.0" "$out" || fail "tags not reachable from HEAD are ignored"
pass "tags not reachable from HEAD are ignored"

git tag -d app-v0.1.0 app-v0.9.0 app-v0.99.0 > /dev/null
REPO_URL="$URL" notes 0.2.0 || fail "0.2.0 without tags"
grep -q "Full changelog" "$out" && fail "no previous tag means no link"
pass "no previous tag, no link"

echo "All $passed release-notes checks passed."
