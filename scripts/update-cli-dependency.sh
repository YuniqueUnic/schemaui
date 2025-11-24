#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CLI_CARGO="$ROOT_DIR/schemaui-cli/Cargo.toml"

PYTHON_BIN="${PYTHON_BIN:-python3}"
if ! command -v "$PYTHON_BIN" >/dev/null 2>&1; then
  if command -v python >/dev/null 2>&1; then
    PYTHON_BIN="$(command -v python)"
  else
    echo "python3 (or override via PYTHON_BIN) is required to run this script." >&2
    exit 1
  fi
fi

"$PYTHON_BIN" - "$ROOT_DIR" "$CLI_CARGO" <<'PY'
import pathlib
import re
import sys

root = pathlib.Path(sys.argv[1])
cli_cargo = pathlib.Path(sys.argv[2])

def extract_version(cargo_path):
    text = cargo_path.read_text(encoding="utf-8")
    in_package = False
    version = None
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("[") and line.endswith("]"):
            in_package = (line == "[package]")
            continue
        if in_package and line.startswith("version"):
            match = re.match(r'version\s*=\s*"([^"]+)"', line)
            if match:
                version = match.group(1)
                break
    if not version:
        raise SystemExit(f"Unable to find version in {cargo_path}")
    return version

root_version = extract_version(root / "Cargo.toml")

contents = cli_cargo.read_text(encoding="utf-8")
pattern = re.compile(r'(schemaui\s*=\s*\{[^}]*version\s*=\s*")([^"]+)(")', re.MULTILINE)

replacement = rf'\g<1>{root_version}\g<3>'
new_contents, count = pattern.subn(replacement, contents, count=1)
if count == 0:
    print("No schemaui dependency entry found in schemaui-cli/Cargo.toml; no changes made.")
    raise SystemExit(0)

if new_contents != contents:
    cli_cargo.write_text(new_contents, encoding="utf-8")
    print(f"Updated schemaui dependency version in schemaui-cli/Cargo.toml to {root_version}.")
else:
    print("schemaui dependency version already up to date.")
PY
