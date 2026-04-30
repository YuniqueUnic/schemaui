"""Feature-matrix compilation coverage for schemaui and schemaui-cli.

Reference inspiration:
https://github.com/YuniqueCore/multiio/blob/main/e2e/tests/test_features_matrix.py

This suite keeps a small always-on smoke matrix, and an opt-in exhaustive matrix
for every valid feature subset. Invalid frontend-only combinations are asserted
to fail with the expected document-format compile error.
"""

from __future__ import annotations

import itertools
import os
import subprocess
import time
from pathlib import Path

import pytest


SCHEMAUI_BASE_FEATURES = (
    "json",
    "yaml",
    "toml",
    "tui",
    "web",
    "web-types",
    "precompile",
    "debug",
)
SCHEMAUI_CLI_BASE_FEATURES = (
    "json",
    "yaml",
    "toml",
    "completion",
    "tui",
    "web",
    "web-types",
    "remote-schema",
)
FORMAT_FEATURES = frozenset({"json", "yaml", "toml"})
DOCUMENT_FORMAT_ERROR = (
    "schemaui requires at least one document format feature: json, yaml, or toml"
)
EXHAUSTIVE_ENV = "SCHEMAUI_EXHAUSTIVE_FEATURE_MATRIX"


def project_root() -> Path:
    return Path(__file__).resolve().parents[3]


def target_dir(package: str) -> Path:
    return project_root() / "target" / "feature-matrix-e2e" / package


def feature_flags(features: tuple[str, ...]) -> list[str]:
    if not features:
        return []
    return ["--features", ",".join(features)]


def feature_label(features: tuple[str, ...]) -> str:
    if not features:
        return "<none>"
    return ",".join(features)


def progress_label(index: int, total: int, package: str, features: tuple[str, ...]) -> str:
    return f"[{index}/{total}] {package} {feature_label(features)}"


def env_with_warnings_denied(package: str) -> dict[str, str]:
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(target_dir(package))
    rustflags = env.get("RUSTFLAGS", "").strip()
    env["RUSTFLAGS"] = f"{rustflags} -D warnings".strip()
    return env


def run_check(
    package: str,
    features: tuple[str, ...],
    *,
    no_default: bool,
    expect_failure: bool = False,
    progress: str | None = None,
) -> subprocess.CompletedProcess[str]:
    cmd = [
        "cargo",
        "check",
        "--quiet",
        "--all-targets",
        "--package",
        package,
    ]
    if no_default:
        cmd.append("--no-default-features")
    cmd.extend(feature_flags(features))
    label = progress or f"{package} {feature_label(features)}"
    print(f"[feature-matrix] {label} checking", flush=True)
    started_at = time.perf_counter()
    result = subprocess.run(
        cmd,
        cwd=project_root(),
        capture_output=True,
        text=True,
        env=env_with_warnings_denied(package),
    )
    elapsed = time.perf_counter() - started_at
    if expect_failure:
        status = (
            f"expected-failure({result.returncode})"
            if result.returncode != 0
            else "unexpected-success"
        )
    else:
        status = "ok" if result.returncode == 0 else f"failed({result.returncode})"
    print(
        f"[feature-matrix] {label} finished status={status} duration={elapsed:.2f}s",
        flush=True,
    )
    return result


def assert_check_ok(
    package: str,
    features: tuple[str, ...],
    *,
    no_default: bool,
    progress: str | None = None,
) -> None:
    result = run_check(package, features, no_default=no_default, progress=progress)
    assert result.returncode == 0, (
        f"cargo check failed for package={package}, no_default={no_default}, features={features}\n"
        f"stdout:\n{result.stdout}\n\n"
        f"stderr:\n{result.stderr}"
    )


def assert_missing_format_error(package: str, features: tuple[str, ...]) -> None:
    result = run_check(
        package,
        features,
        no_default=True,
        expect_failure=True,
    )
    combined = f"{result.stdout}\n{result.stderr}"
    assert result.returncode != 0, (
        f"expected missing-format failure for package={package}, features={features}, but cargo check succeeded"
    )
    assert DOCUMENT_FORMAT_ERROR in combined, (
        f"unexpected error for package={package}, features={features}\n{combined}"
    )


def has_document_format(features: tuple[str, ...]) -> bool:
    return any(feature in FORMAT_FEATURES for feature in features)


def valid_feature_subsets(base_features: tuple[str, ...]) -> list[tuple[str, ...]]:
    subsets: list[tuple[str, ...]] = []
    for size in range(len(base_features) + 1):
        for combo in itertools.combinations(base_features, size):
            if has_document_format(combo):
                subsets.append(combo)
    return subsets


@pytest.mark.parametrize(
    ("package", "features"),
    [
        ("schemaui", ("tui",)),
        ("schemaui", ("web",)),
        ("schemaui-cli", ("tui",)),
        ("schemaui-cli", ("web",)),
    ],
)
def test_frontend_only_builds_require_document_formats(
    package: str, features: tuple[str, ...]
) -> None:
    assert_missing_format_error(package, features)


def test_smoke_feature_matrix_compiles() -> None:
    print("[feature-matrix] running smoke matrix", flush=True)
    assert_check_ok("schemaui", (), no_default=False)
    assert_check_ok("schemaui", ("json",), no_default=True)
    assert_check_ok("schemaui", ("json", "tui"), no_default=True)
    assert_check_ok("schemaui", ("json", "web"), no_default=True)
    assert_check_ok("schemaui", ("json", "web", "web-types"), no_default=True)
    assert_check_ok("schemaui", ("json", "precompile"), no_default=True)
    assert_check_ok("schemaui", ("all_formats",), no_default=True)
    assert_check_ok("schemaui", ("full",), no_default=True)

    assert_check_ok("schemaui-cli", (), no_default=False)
    assert_check_ok("schemaui-cli", ("json",), no_default=True)
    assert_check_ok("schemaui-cli", ("json", "completion"), no_default=True)
    assert_check_ok("schemaui-cli", ("json", "tui"), no_default=True)
    assert_check_ok("schemaui-cli", ("json", "web"), no_default=True)
    assert_check_ok("schemaui-cli", ("json", "web", "web-types"), no_default=True)
    assert_check_ok("schemaui-cli", ("json", "remote-schema"), no_default=True)
    assert_check_ok("schemaui-cli", ("full",), no_default=True)


def test_exhaustive_feature_matrix_compiles() -> None:
    if os.environ.get(EXHAUSTIVE_ENV) != "1":
        pytest.skip(
            f"set {EXHAUSTIVE_ENV}=1 to enable exhaustive feature-matrix checks"
        )

    schemaui_subsets = valid_feature_subsets(SCHEMAUI_BASE_FEATURES)
    cli_subsets = valid_feature_subsets(SCHEMAUI_CLI_BASE_FEATURES)
    print(
        "[feature-matrix] running exhaustive matrix "
        f"(schemaui={len(schemaui_subsets)} combos, "
        f"schemaui-cli={len(cli_subsets)} combos)",
        flush=True,
    )
    total_progress = len(schemaui_subsets) + len(cli_subsets)

    assert_check_ok("schemaui", (), no_default=False)
    for index, combo in enumerate(schemaui_subsets, start=1):
        assert_check_ok(
            "schemaui",
            combo,
            no_default=True,
            progress=progress_label(index, total_progress, "schemaui", combo),
        )
    for index, combo in enumerate(cli_subsets, start=len(schemaui_subsets) + 1):
        assert_check_ok(
            "schemaui-cli",
            combo,
            no_default=True,
            progress=progress_label(index, total_progress, "schemaui-cli", combo),
        )

    assert_check_ok("schemaui", ("all_formats",), no_default=True)
    assert_check_ok("schemaui", ("full",), no_default=True)
    assert_check_ok("schemaui-cli", ("full",), no_default=True)
