#!/usr/bin/env bash
# Sync the duck-mc CI + OSS scaffolding into another Rust repo on disk.
# Idempotent: re-running overwrites the files in the target repo to
# match this one.
#
# Usage:
#   bash .github/sync-rust-scaffolding.sh /path/to/target/repo
#
# Files copied:
#   - .github/workflows/{ci,audit,codeql,typos,scorecard,release}.yml
#   - .github/{labels.sh,protect-branches.sh,PULL_REQUEST_TEMPLATE.md,
#              dependabot.yml,ISSUE_TEMPLATE/}
#   - .husky/pre-push
#   - .editorconfig, .typos.toml
#   - LICENSE, CONTRIBUTING.md, CODE_OF_CONDUCT.md, SECURITY.md
#   - deny.toml, release-plz.toml
#
# Files NOT copied (repo-specific):
#   - README.md, CHANGELOG.md, Cargo.toml, src/

set -euo pipefail

SRC="$(cd "$(dirname "$0")/.." && pwd)"
DST="${1:-}"

if [ -z "$DST" ]; then
  echo "usage: $0 /path/to/target/repo"
  exit 1
fi
if [ ! -d "$DST" ]; then
  echo "error: $DST does not exist"
  exit 1
fi

copy() {
  local rel="$1"
  local src="$SRC/$rel"
  local dst="$DST/$rel"
  if [ ! -e "$src" ]; then
    echo "skip $rel (not in source)"
    return
  fi
  mkdir -p "$(dirname "$dst")"
  cp -r "$src" "$dst"
  echo "  $rel"
}

echo "== syncing scaffolding into $DST"

copy .github/workflows/ci.yml
copy .github/workflows/audit.yml
copy .github/workflows/typos.yml
copy .github/workflows/scorecard.yml
copy .github/workflows/release.yml
# CodeQL is js/ts-only; copy only into repos that have TS sources.
copy .github/labels.sh
copy .github/protect-branches.sh
copy .github/PULL_REQUEST_TEMPLATE.md
copy .github/dependabot.yml
copy .github/ISSUE_TEMPLATE
copy .husky/pre-push
copy .editorconfig
copy .typos.toml
copy LICENSE
copy CONTRIBUTING.md
copy CODE_OF_CONDUCT.md
copy SECURITY.md
copy deny.toml
copy release-plz.toml

echo "done"
echo
echo "Next steps in $DST:"
echo "  1. git status                           # review the diff"
echo "  2. Adjust .github/labels.sh REPO default"
echo "  3. Adjust .github/protect-branches.sh defaults"
echo "  4. Bump Cargo.toml workspace.package metadata"
echo "  5. git add -A && git commit -m 'chore: sync ci scaffolding'"
echo "  6. git push"
