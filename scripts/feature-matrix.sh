#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-smoke}"
shift || true

PYTEST_TARGET="tests/e2e/tests/test_features_matrix.py"
PYTEST_ARGS=("${PYTEST_TARGET}" "-ra" "-s")

case "${MODE}" in
  smoke)
    unset SCHEMAUI_EXHAUSTIVE_FEATURE_MATRIX
    ;;
  exhaustive)
    export SCHEMAUI_EXHAUSTIVE_FEATURE_MATRIX=1
    ;;
  *)
    echo "usage: $0 [smoke|exhaustive] [additional pytest args...]" >&2
    exit 2
    ;;
esac

echo "[feature-matrix] mode=${MODE}"
echo "[feature-matrix] note: pytest will exit automatically after the matrix finishes;"
echo "[feature-matrix]       long waits come from repeated cargo checks, so live progress is enabled."

if command -v uv >/dev/null 2>&1; then
  exec uv run --with pytest python -m pytest "${PYTEST_ARGS[@]}" "$@"
fi

if python3 -m pytest --version >/dev/null 2>&1; then
  exec python3 -m pytest "${PYTEST_ARGS[@]}" "$@"
fi

cat >&2 <<'EOF'
feature-matrix test requires one of:
  - uv
  - python3 with pytest installed
EOF
exit 2
