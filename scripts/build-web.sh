#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
UI_DIR="$ROOT_DIR/web/ui"

if ! command -v pnpm >/dev/null 2>&1; then
  echo "pnpm is required to build the SchemaUI web bundle." >&2
  exit 1
fi

pushd "$UI_DIR" >/dev/null

# Ensure a clean install each time and avoid leaving node_modules in the repo.
rm -rf node_modules
pnpm install --frozen-lockfile --silent
pnpm run build
rm -rf node_modules

popd >/dev/null
rm -rf "$UI_DIR/node_modules"
