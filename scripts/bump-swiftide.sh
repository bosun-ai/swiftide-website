#!/usr/bin/env bash
# Moves the vendor/swiftide pin to a newer swiftide commit and re-verifies the site.
#
# Usage: scripts/bump-swiftide.sh [ref]        (default: origin/master)
#   SKIP_CI_CHECK=1  bump even if swiftide's CI has not passed for the commit
#   SKIP_VERIFY=1    only move the pin, do not run `pnpm verify`
#
# Only commits whose swiftide CI is green are accepted (checked with `gh`), so the site never
# documents code that does not build. The new pin is staged (`git add vendor/swiftide`), not
# committed.
set -euo pipefail
cd "$(dirname "$0")/.."

ref="${1:-origin/master}"
repo="bosun-ai/swiftide"

git -C vendor/swiftide fetch --tags --quiet origin
sha="$(git -C vendor/swiftide rev-parse --verify "${ref}^{commit}")"
current="$(git ls-files --stage -- vendor/swiftide | awk '{print $2}')"

if [[ "$sha" == "$current" ]]; then
  echo "vendor/swiftide is already pinned at ${sha:0:9}"
  exit 0
fi

if [[ "${SKIP_CI_CHECK:-0}" != "1" ]]; then
  if ! command -v gh >/dev/null 2>&1; then
    echo "gh is required to check swiftide's CI status (or set SKIP_CI_CHECK=1)" >&2
    exit 1
  fi
  # Every completed check run must have succeeded or been skipped, and at least one must exist.
  conclusions="$(gh api "repos/${repo}/commits/${sha}/check-runs" --paginate --jq '.check_runs[] | "\(.status) \(.conclusion)"')"
  if [[ -z "$conclusions" ]]; then
    echo "No CI runs found for ${sha:0:9} in ${repo}; refusing to bump" >&2
    exit 1
  fi
  if grep -qvE '^completed (success|skipped|neutral)$' <<<"$conclusions"; then
    echo "swiftide CI is not green for ${sha:0:9}:" >&2
    sort <<<"$conclusions" | uniq -c >&2
    exit 1
  fi
fi

git -C vendor/swiftide checkout --quiet --detach "$sha"
git add vendor/swiftide
echo "Pinned vendor/swiftide: ${current:0:9} -> ${sha:0:9} ($(git -C vendor/swiftide describe --tags --long --match 'v[0-9]*'))"

if [[ "${SKIP_VERIFY:-0}" != "1" ]]; then
  # Let cargo adjust snippets/Cargo.lock minimally to the new swiftide; verify then runs --locked.
  (cd snippets && cargo metadata --format-version 1 >/dev/null)
  pnpm verify
fi
