#!/usr/bin/env bash
set -euo pipefail

START_DIR="${1:-$PWD}"

PYTHON_BIN="${PYTHON_BIN:-python3}"
if ! command -v "$PYTHON_BIN" >/dev/null 2>&1; then
    if command -v python >/dev/null 2>&1; then
        PYTHON_BIN="$(command -v python)"
    else
        echo "python3 (or override via PYTHON_BIN) is required to update README versions." >&2
        exit 1
    fi
fi

"$PYTHON_BIN" - "$START_DIR" <<'PY'
import pathlib
import re
import sys

IGNORE_DIRS = {'.git', 'target', 'node_modules', '.serena', '.idea', '.vscode'}

start = pathlib.Path(sys.argv[1]).resolve()

pkg_dir = None
current = start
while True:
    if (current / "Cargo.toml").is_file():
        pkg_dir = current
        break
    if current.parent == current:
        sys.exit(f"Unable to locate Cargo.toml starting from {start}")
    current = current.parent

cargo_text = (pkg_dir / "Cargo.toml").read_text(encoding="utf-8")
pkg_name = None
pkg_version = None
in_package = False
for raw_line in cargo_text.splitlines():
    line = raw_line.strip()
    if not line or line.startswith('#'):
        continue
    if line.startswith('[') and line.endswith(']'):
        in_package = (line == '[package]')
        continue
    if not in_package:
        continue
    if line.startswith('name'):
        name_match = re.match(r'name\s*=\s*"([^"]+)"', line)
        if name_match:
            pkg_name = name_match.group(1)
    if line.startswith('version'):
        version_match = re.match(r'version\s*=\s*"([^"]+)"', line)
        if version_match:
            pkg_version = version_match.group(1)
    if pkg_name and pkg_version:
        break

if not pkg_name or not pkg_version:
    sys.exit(f"Could not parse name/version in {pkg_dir / 'Cargo.toml'}")

pattern = re.compile(rf'({re.escape(pkg_name)}\s*=\s*")([^"]+)(")')

changed = []
for path in pkg_dir.rglob("README*.md"):
    if any(part in IGNORE_DIRS for part in path.parts):
        continue
    try:
        contents = path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        continue
    replacement = rf'\g<1>{pkg_version}\g<3>'
    new_contents, count = pattern.subn(replacement, contents)
    if count:
        path.write_text(new_contents, encoding="utf-8")
        rel = path.relative_to(pkg_dir)
        changed.append(f"{rel} ({count} match{'es' if count > 1 else ''})")

if changed:
    print(f"Updated {pkg_name} docs to {pkg_version}:")
    for entry in changed:
        print(f"  - {entry}")
else:
    print(f"No markdown files required updates for {pkg_name}.")
PY
