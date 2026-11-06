#!/usr/bin/env bash
# One-time setup for a fresh clone: submodule with tags, node dependencies, Rust toolchain.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> vendor/swiftide submodule"
git submodule sync --recursive
git -c protocol.file.allow=always submodule update --init --recursive
# The site derives its version from `git describe --tags`; shallow or tagless clones fail the build.
if git -C vendor/swiftide rev-parse --is-shallow-repository | grep -q true; then
  git -C vendor/swiftide fetch --unshallow --tags
else
  git -C vendor/swiftide fetch --tags --quiet
fi
git -C vendor/swiftide describe --tags --long --match 'v[0-9]*'

echo "==> node dependencies"
corepack enable >/dev/null 2>&1 || true
pnpm install --frozen-lockfile

echo "==> rust toolchain for snippets/"
if command -v rustup >/dev/null 2>&1; then
  (cd snippets && rustup show active-toolchain >/dev/null 2>&1 || rustup toolchain install "$(sed -n 's/^channel *= *"\(.*\)"/\1/p' rust-toolchain.toml)")
else
  echo "rustup not found; install it to run \`pnpm check:snippets\`" >&2
fi

echo "Done. Run \`pnpm dev\` to start the site, \`pnpm verify\` before pushing."
