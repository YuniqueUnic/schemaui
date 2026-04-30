#!/usr/bin/env bash
set -euo pipefail

exec cargo tree --duplicates --workspace --all-features "$@"
