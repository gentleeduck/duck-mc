#!/usr/bin/env bash
# Apply branch protection across gentleeduck pinned repos.
# Requires gh auth with repo admin perms.
#
# Baseline (every pinned repo):
#   - require pull request (0 approvals, dismiss stale reviews)
#   - require linear history
#   - require conversation resolution
#   - block force push
#   - block deletion
#   - admin not enforced (solo merges still possible)
#
# duck-mc adds required CI status checks on top.
#
# Usage:
#   bash .github/protect-branches.sh
#   # one-off:
#   REPO=gentleeduck/duck-ui BRANCH=master bash .github/protect-branches.sh

set -euo pipefail

apply_protection() {
  local repo="$1"
  local branch="$2"
  local extra_contexts_json="${3:-[]}"

  echo ">> protecting $repo : $branch"

  gh api -X PUT "/repos/$repo/branches/$branch/protection" \
    -H "Accept: application/vnd.github+json" \
    --input - <<JSON >/dev/null
{
  "required_status_checks": {
    "strict": true,
    "contexts": $extra_contexts_json
  },
  "enforce_admins": false,
  "required_pull_request_reviews": {
    "required_approving_review_count": 0,
    "dismiss_stale_reviews": true,
    "require_code_owner_reviews": false
  },
  "restrictions": null,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "required_linear_history": true,
  "required_conversation_resolution": true,
  "lock_branch": false,
  "block_creations": false
}
JSON

  echo "   ok"
}

# duck-mc CI job names from .github/workflows/ci.yml
duckmc_contexts='[
  "cargo fmt",
  "cargo clippy",
  "cargo test (ubuntu-latest)",
  "cargo test (macos-latest)",
  "cargo test (windows-latest)",
  "cargo check (feature combos)"
]'

if [ -n "${REPO:-}" ]; then
  apply_protection "$REPO" "${BRANCH:-master}" "[]"
  exit 0
fi

# All five pinned repos in the gentleeduck org.
apply_protection "gentleeduck/duck-mc"          "master" "$duckmc_contexts"
apply_protection "gentleeduck/duck-ui"          "master" "[]"
apply_protection "gentleeduck/duck-linux-utils" "main"   "[]"
apply_protection "gentleeduck/duck-ttlog"       "master" "[]"
apply_protection "gentleeduck/duck-template"    "master" "[]"

echo "done"
