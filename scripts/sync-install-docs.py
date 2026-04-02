#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
DATA_PATH = ROOT / "packaging" / "install" / "install-methods.json"

QUICKLINK_BEGIN = "<!-- AUTO-GENERATED:CLI-QUICKLINK:BEGIN -->"
QUICKLINK_END = "<!-- AUTO-GENERATED:CLI-QUICKLINK:END -->"
INSTALL_BEGIN = "<!-- AUTO-GENERATED:CLI-INSTALL:BEGIN -->"
INSTALL_END = "<!-- AUTO-GENERATED:CLI-INSTALL:END -->"


class SyncError(RuntimeError):
    pass


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Sync README installation sections from install-methods.json"
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Verify generated blocks are up to date without rewriting files",
    )
    return parser.parse_args()


def load_data() -> dict:
    return json.loads(DATA_PATH.read_text())


def update_marked_block(path: Path, begin: str, end: str, generated: str, check: bool) -> None:
    original = path.read_text()
    begin_matches = [match.start() for match in re.finditer(re.escape(begin), original)]
    end_matches = [match.start() for match in re.finditer(re.escape(end), original)]
    if len(begin_matches) != 1 or len(end_matches) != 1:
        raise SyncError(
            f"{path.relative_to(ROOT)} must contain exactly one {begin} and one {end}"
        )
    start = begin_matches[0] + len(begin)
    finish = end_matches[0]
    if finish <= start:
        raise SyncError(f"{path.relative_to(ROOT)} has invalid marker ordering for {begin}")
    replacement = f"\n{generated}\n"
    updated = f"{original[:start]}{replacement}{original[finish:]}"
    if updated == original:
        print(f"unchanged {path.relative_to(ROOT)}")
        return
    if check:
        if compare_markdown_equivalent(path, updated, original):
            print(f"unchanged {path.relative_to(ROOT)}")
            return
        raise SyncError(f"{path.relative_to(ROOT)} is out of date")
    path.write_text(updated)
    print(f"updated {path.relative_to(ROOT)}")


def compare_markdown_equivalent(path: Path, updated: str, original: str) -> bool:
    deno = shutil.which("deno")
    if deno is not None:
        return format_markdown_text(path, updated, deno) == original
    return normalize_markdown_for_compare(updated) == normalize_markdown_for_compare(original)


def format_markdown_text(path: Path, text: str, deno: str) -> str:
    with tempfile.TemporaryDirectory(prefix="schemaui-install-docs-") as tmpdir:
        temp_path = Path(tmpdir) / path.name
        temp_path.write_text(text)
        subprocess.run([deno, "fmt", str(temp_path)], check=True, capture_output=True)
        return temp_path.read_text()


def normalize_markdown_for_compare(text: str) -> str:
    normalized_blocks: list[str] = []
    in_code_fence = False
    current_block: list[str] = []

    def flush_block() -> None:
        if not current_block:
            return
        block = normalize_markdown_block(current_block)
        if block:
            normalized_blocks.append(block)
        current_block.clear()

    for raw_line in text.splitlines():
        line = raw_line.rstrip()
        if line.startswith("```"):
            if in_code_fence:
                current_block.append(line)
                flush_block()
                in_code_fence = False
            else:
                flush_block()
                in_code_fence = True
                current_block.append(line)
            continue

        if in_code_fence:
            current_block.append(line)
            continue

        if line.startswith("<!--") or line.startswith("#"):
            flush_block()
            current_block.append(line)
            flush_block()
            continue

        if not line.strip():
            flush_block()
            continue

        current_block.append(line)

    flush_block()
    return "\n\n".join(normalized_blocks)


def normalize_markdown_block(lines: list[str]) -> str:
    stripped = [line.strip() for line in lines if line.strip()]
    if not stripped:
        return ""

    first = stripped[0]
    if first.startswith("```"):
        return "\n".join(lines)
    if first.startswith("<!--"):
        return "\n".join(stripped)
    if first.startswith("#"):
        return "\n".join(stripped)
    if all(line.startswith(">") for line in stripped):
        content = " ".join(line[1:].strip() for line in stripped)
        return f"> {collapse_inline_whitespace(content)}"
    if first.startswith("#### "):
        return "\n".join(stripped)

    joined = " ".join(stripped)
    return collapse_inline_whitespace(joined)


def collapse_inline_whitespace(text: str) -> str:
    return " ".join(text.split())


def locale_text(mapping: dict[str, str], locale: str) -> str:
    return mapping[locale]


def render_quicklink(data: dict, locale: str) -> str:
    package_name = data["packageName"]
    binary_name = data["binaryName"]
    link_text = data["cliQuickLink"][locale].replace("#cli-schemaui-cli", data["cliSectionAnchor"])
    if locale == "en":
        return (
            f"> CLI available: `{package_name}` installs the `{binary_name}` binary. "
            f"{link_text}"
        )
    return (
        f"> CLI 可用：`{package_name}` 会安装 `{binary_name}` 可执行文件。"
        f"{link_text}"
    )


def install_heading(locale: str, with_heading: bool) -> list[str]:
    if not with_heading:
        return []
    return ["### Install" if locale == "en" else "### 安装", ""]


def render_install_block(data: dict, locale: str, with_heading: bool) -> str:
    lines: list[str] = []
    lines.extend(install_heading(locale, with_heading))
    lines.append(locale_text(data["intro"], locale))
    lines.append("")
    lines.append(
        "Choose one of the supported channels:"
        if locale == "en"
        else "选择下面任意一种支持的分发方式："
    )
    lines.append("")

    for channel in data["channels"]:
        lines.append(f"#### {locale_text(channel['title'], locale)}")
        lines.append(locale_text(channel["summary"], locale))
        commands = channel.get("commands", [])
        if commands:
            lines.append("")
            lines.append("```bash")
            lines.extend(commands)
            lines.append("```")
        lines.append("")

    return "\n".join(lines).rstrip()


def sync_file(path: Path, *, data: dict, locale: str, quicklink: bool, install_heading: bool, check: bool) -> None:
    if quicklink:
        update_marked_block(
            path,
            QUICKLINK_BEGIN,
            QUICKLINK_END,
            render_quicklink(data, locale),
            check,
        )
    update_marked_block(
        path,
        INSTALL_BEGIN,
        INSTALL_END,
        render_install_block(data, locale, install_heading),
        check,
    )


def main() -> int:
    args = parse_args()
    data = load_data()
    targets = [
        ROOT / "README.md",
        ROOT / "README.ZH.md",
        ROOT / "docs" / "en" / "cli_usage.md",
    ]
    sync_file(targets[0], data=data, locale="en", quicklink=True, install_heading=True, check=args.check)
    sync_file(targets[1], data=data, locale="zh", quicklink=True, install_heading=True, check=args.check)
    sync_file(
        targets[2],
        data=data,
        locale="en",
        quicklink=False,
        install_heading=False,
        check=args.check,
    )
    if not args.check:
        deno = shutil.which("deno")
        if deno is None:
            raise SyncError(
                "sync-install-docs.py requires `deno` when writing docs so the generated "
                "markdown matches repository formatting"
            )
        subprocess.run([deno, "fmt", *[str(path) for path in targets]], check=True)
    if args.check:
        print("installation docs are up to date")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except SyncError as exc:
        print(f"error: {exc}", file=sys.stderr)
        raise SystemExit(1)
