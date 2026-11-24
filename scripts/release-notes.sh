#!/bin/bash
# Prints the GitHub Release body for one version: its section from
# CHANGELOG.md, a compare link from the previous app-v* tag, and the
# unsigned-installer note. Run from the repository root.
#
# Usage: release-notes.sh <version>
# Env:   CHANGELOG  path to the changelog (default: CHANGELOG.md)
#        REPO_URL   e.g. https://github.com/owner/repo; without it, no link
# Exit:  0 ok, 1 no usable section or git failure, 2 bad arguments.
set -u -o pipefail

NOTE='Unsigned installer: if Windows SmartScreen warns, choose "More info" and then "Run anyway". It installs per user, no admin rights needed.'

if [ $# -ne 1 ]; then
  echo "usage: release-notes.sh <version>" >&2
  exit 2
fi
version="$1"
if ! [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-+][0-9A-Za-z.+-]+)?$ ]]; then
  echo "release-notes: '$version' is not a version" >&2
  exit 2
fi

changelog="${CHANGELOG:-CHANGELOG.md}"
if [ ! -f "$changelog" ]; then
  echo "release-notes: $changelog not found" >&2
  exit 1
fi

# The section runs from "## [<version>]" (any suffix, e.g. a date) to the
# next "## " heading or the end of the file; blank edge lines are trimmed.
section="$(awk -v head="## [$version]" '
  { sub(/\r$/, "") }
  /^## / {
    if (found) exit
    if (index($0, head) == 1) { found = 1; next }
  }
  found {
    lines[++n] = $0
    if ($0 ~ /[^[:space:]]/) { if (!first) first = n; last = n }
  }
  END { for (i = first; first && i <= last; i++) print lines[i] }
' "$changelog")" || {
  echo "release-notes: could not read $changelog" >&2
  exit 1
}

if [ -z "$section" ]; then
  echo "release-notes: $changelog has no entries under '## [$version]'." >&2
  echo "Add the section (move the [Unreleased] entries) before bumping the version." >&2
  exit 1
fi

# Previous release: the highest app-v* tag reachable from HEAD, other than
# this version's own tag (it already exists when CI completes a release).
tags="$(git tag --list 'app-v*' --merged HEAD --sort=-v:refname)" || {
  echo "release-notes: git tag failed" >&2
  exit 1
}
prev="$(printf '%s\n' "$tags" | grep -vxF "app-v$version" | head -n 1)"

printf '%s\n' "$section"
if [ -n "${REPO_URL:-}" ] && [ -n "$prev" ]; then
  printf '\n**Full changelog:** %s/compare/%s...app-v%s\n' "$REPO_URL" "$prev" "$version"
fi
printf '\n%s\n' "$NOTE"
