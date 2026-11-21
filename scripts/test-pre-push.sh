#!/bin/bash
# Self-test for .githooks/pre-push. Uses only invented terms in a temp dir.
set -u
repo_root="$(cd "$(dirname "$0")/.." && pwd)"
hook="$repo_root/.githooks/pre-push"
tmp="$(mktemp -d)"
trap 'cd /; rm -rf "$tmp"' EXIT

G=(-c user.name=selftest -c user.email=selftest@example.test)
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null
TERM1="ACME-WIDGETS"
TERM2="intranet.example.test"
TERM3="ŁADNYKLIENT"
passed=0
out="$tmp/out.txt"

pass() { passed=$((passed + 1)); echo "PASS $1"; }
fail() { echo "FAIL $1"; [ -f "$out" ] && sed 's/^/  | /' "$out" | head -20; exit 1; }

git init -q --bare -b main "$tmp/remote.git" || fail "init bare"
git init -q -b main "$tmp/work" || fail "init work"
cd "$tmp/work" || exit 1
git remote add origin "$tmp/remote.git"
mkdir -p "$tmp/hooks"
cp "$hook" "$tmp/hooks/pre-push"
chmod +x "$tmp/hooks/pre-push"
git config core.hooksPath "$tmp/hooks"
export DAYNAV_DENYLIST="$tmp/denylist.txt"

commit() { # <msg>
  git add -A && git "${G[@]}" commit -q -m "$1" || fail "commit"
}
push() { git push origin HEAD:main > "$out" 2>&1; }
reset_local() { git reset -q --hard "$1"; git clean -fdq; }

printf '# comment one\n# comment two\n\n%s\n%s\n' "$TERM1" "$TERM2" > "$DAYNAV_DENYLIST"

echo "clean file" > a.txt
commit "clean commit"
push || fail "clean commit should push"
pass "clean commit pushes"
base="$(git rev-parse HEAD)"

echo "see $TERM1 here" > b.txt
commit "add b"
push && fail "added-line match should block"
grep -q "added line denylist line 4" "$out" || fail "line number should be 4 (original file line)"
pass "added line blocks, reports original line number 4"
grep -qF "$TERM1" "$out" && fail "output leaks term"
pass "output does not contain term"
reset_local "$base"

echo x > "dir-$TERM2.txt"
commit "add path"
push && fail "path match should block"
grep -q "path denylist line 5" "$out" || fail "path report / line 5"
grep -qF "$TERM2" "$out" && fail "output leaks term (path)"
pass "file path blocks"
reset_local "$base"

echo y > c.txt
commit "mentions $TERM1 in message"
push && fail "message match should block"
grep -q "commit message denylist line 4" "$out" || fail "message report"
pass "commit message blocks"
reset_local "$base"

echo "case test acme-widgets" > d.txt
commit "case"
push && fail "case-insensitive match should block"
pass "case-insensitive match blocks"
reset_local "$base"

echo z > e.txt
commit "clean again"
push || fail "clean after reset should push"
pass "clean commit after reset pushes"
base="$(git rev-parse HEAD)"

rm -f "$DAYNAV_DENYLIST"
echo more > f.txt; commit "f"
push && fail "missing denylist should block"
grep -qF "denylist.txt" "$out" || fail "message should name path"
pass "missing denylist blocks (fail closed)"

printf '# only comments\n\n   \n# more\n' > "$DAYNAV_DENYLIST"
push && fail "comments-only denylist should block"
pass "comments-only denylist blocks"

printf '# c\n\n  \n%s\n' "$TERM1" > "$DAYNAV_DENYLIST"
push || fail "blank line in denylist must not match everything"
pass "blank line does not match everything"
base="$(git rev-parse HEAD)"

printf '\xef\xbb\xbf%s\r\n%s\r\n' "$TERM1" "$TERM2" > "$DAYNAV_DENYLIST"
echo "has $TERM1" > g.txt; commit "g"
push && fail "BOM+CRLF denylist first term should match"
grep -q "denylist line 1" "$out" || fail "first term should be line 1"
pass "CRLF + BOM denylist matches first term"
reset_local "$base"

printf '%s\n' "$TERM3" > "$DAYNAV_DENYLIST"
echo "klient: ładnyklient" > h.txt; commit "h"
push && fail "Polish case folding should block"
pass "case-insensitive incl. Polish letters"
reset_local "$base"

echo "Summary: $passed assertions passed"
