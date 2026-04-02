#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
import textwrap
import tomllib
import urllib.error
import urllib.request
from pathlib import Path


DEFAULT_REPO = "YuniqueUnic/schemaui"
ROOT = Path(__file__).resolve().parent.parent
CLI_MANIFEST = ROOT / "schemaui-cli" / "Cargo.toml"
FORMULA_PATH = ROOT / "Formula" / "schemaui.rb"
SCOOP_PATH = ROOT / "packaging" / "scoop" / "schemaui-cli.json"
WINGET_DIR = (
    ROOT
    / "packaging"
    / "winget"
    / "manifests"
    / "y"
    / "YuniqueUnic"
    / "schemaui-cli"
)

PACKAGE_IDENTIFIER = "YuniqueUnic.schemaui-cli"
WINGET_MANIFEST_VERSION = "1.12.0"

WINDOWS_TARGETS = {
    "64bit": "x86_64-pc-windows-msvc",
    "arm64": "aarch64-pc-windows-msvc",
}


class SyncError(RuntimeError):
    pass


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Sync Homebrew / Scoop / winget manifests from a published "
            "schemaui-cli GitHub release."
        )
    )
    parser.add_argument("--repo", default=DEFAULT_REPO, help="GitHub repo in owner/name form")
    parser.add_argument("--version", help="CLI version, defaults to schemaui-cli/Cargo.toml")
    parser.add_argument("--tag", help="Release tag, defaults to schemaui-cli-v<version>")
    parser.add_argument(
        "--check",
        action="store_true",
        help="Verify files are up to date without rewriting them",
    )
    return parser.parse_args()


def cli_version() -> str:
    with CLI_MANIFEST.open("rb") as handle:
        manifest = tomllib.load(handle)
    return manifest["package"]["version"]


def github_headers(token: str | None) -> dict[str, str]:
    headers = {
        "Accept": "application/vnd.github+json",
        "User-Agent": "schemaui-package-sync",
    }
    if token:
        headers["Authorization"] = f"Bearer {token}"
    return headers


def fetch_json(url: str, token: str | None) -> object:
    request = urllib.request.Request(url, headers=github_headers(token))
    try:
        with urllib.request.urlopen(request) as response:
            return json.load(response)
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8", errors="replace")
        raise SyncError(f"request {url} failed with {exc.code}: {body}") from exc


def sha256_url(url: str, token: str | None) -> str:
    request = urllib.request.Request(url, headers=github_headers(token))
    digest = hashlib.sha256()
    try:
        with urllib.request.urlopen(request) as response:
            while True:
                chunk = response.read(1024 * 1024)
                if not chunk:
                    break
                digest.update(chunk)
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8", errors="replace")
        raise SyncError(f"download {url} failed with {exc.code}: {body}") from exc
    return digest.hexdigest()


def release_url(repo: str, tag: str) -> str:
    return f"https://api.github.com/repos/{repo}/releases/tags/{tag}"


def tag_archive_url(repo: str, tag: str) -> str:
    return f"https://api.github.com/repos/{repo}/tarball/{tag}"


def asset_sha256(asset: dict[str, object], token: str | None) -> str:
    digest = asset.get("digest")
    if isinstance(digest, str) and digest.startswith("sha256:"):
        return digest.split(":", 1)[1]
    url = str(asset["browser_download_url"])
    return sha256_url(url, token)


def require_asset(assets: dict[str, dict[str, object]], name: str) -> dict[str, object]:
    asset = assets.get(name)
    if asset is None:
        raise SyncError(f"release asset {name} is missing")
    return asset


def render_formula(repo: str, version: str, source_sha256: str) -> str:
    repo_url = f"https://github.com/{repo}"
    return textwrap.dedent(
        f"""\
        class Schemaui < Formula
          desc "Render JSON Schemas as TUIs and embedded web editors"
          homepage "{repo_url}"
          url "{repo_url}/archive/refs/tags/schemaui-cli-v{version}.tar.gz"
          sha256 "{source_sha256}"
          license "MIT OR Apache-2.0"
          head "{repo_url}.git", branch: "main"

          depends_on "rust" => :build

          def install
            system "cargo", "install", *std_cargo_args(path: "schemaui-cli"), "--features", "full"
          end

          test do
            assert_equal "schemaui #{{version}}\\n", shell_output("#{{bin}}/schemaui --version")
          end
        end
        """
    )


def render_scoop(repo: str, version: str, windows_assets: dict[str, dict[str, str]]) -> str:
    repo_url = f"https://github.com/{repo}"
    manifest = {
        "version": version,
        "description": "CLI wrapper for schemaui, rendering JSON Schemas as TUIs",
        "homepage": repo_url,
        "license": "MIT OR Apache-2.0",
        "architecture": {
            arch: {
                "url": asset["url"],
                "hash": asset["sha256"],
            }
            for arch, asset in windows_assets.items()
        },
        "bin": "schemaui.exe",
        "checkver": {
            "url": f"{repo_url}/releases",
            "regex": r"schemaui-cli-v([\d.]+)",
        },
        "autoupdate": {
            "architecture": {
                "64bit": {
                    "url": f"{repo_url}/releases/download/schemaui-cli-v$version/schemaui-{WINDOWS_TARGETS['64bit']}.zip"
                },
                "arm64": {
                    "url": f"{repo_url}/releases/download/schemaui-cli-v$version/schemaui-{WINDOWS_TARGETS['arm64']}.zip"
                },
            }
        },
    }
    return json.dumps(manifest, indent=4, ensure_ascii=False) + "\n"


def render_winget_version(version: str) -> str:
    return textwrap.dedent(
        f"""\
        PackageIdentifier: {PACKAGE_IDENTIFIER}
        PackageVersion: {version}
        DefaultLocale: en-US
        ManifestType: version
        ManifestVersion: {WINGET_MANIFEST_VERSION}
        """
    )


def render_winget_default_locale(repo: str, version: str) -> str:
    repo_url = f"https://github.com/{repo}"
    return textwrap.dedent(
        f"""\
        PackageIdentifier: {PACKAGE_IDENTIFIER}
        PackageVersion: {version}
        PackageLocale: en-US
        Publisher: YuniqueUnic
        PublisherUrl: https://github.com/YuniqueUnic
        PublisherSupportUrl: {repo_url}/issues
        PackageName: schemaui-cli
        PackageUrl: {repo_url}
        Moniker: schemaui
        License: MIT OR Apache-2.0
        ShortDescription: CLI wrapper for schemaui, rendering JSON Schemas as TUIs
        Description: schemaui-cli wraps the schemaui workspace and ships the schemaui binary for terminal and embedded web editing flows.
        Tags:
          - json-schema
          - tui
          - config
          - web
        ManifestType: defaultLocale
        ManifestVersion: {WINGET_MANIFEST_VERSION}
        """
    )


def render_winget_installer(version: str, windows_assets: dict[str, dict[str, str]]) -> str:
    lines = [
        f"PackageIdentifier: {PACKAGE_IDENTIFIER}",
        f"PackageVersion: {version}",
        "Installers:",
    ]
    for arch, winget_arch in (("64bit", "x64"), ("arm64", "arm64")):
        asset = windows_assets[arch]
        lines.extend(
            [
                f"  - Architecture: {winget_arch}",
                "    InstallerType: zip",
                "    NestedInstallerType: portable",
                "    NestedInstallerFiles:",
                "      - RelativeFilePath: schemaui.exe",
                "        PortableCommandAlias: schemaui",
                "    Commands:",
                "      - schemaui",
                f"    InstallerUrl: {asset['url']}",
                f"    InstallerSha256: {asset['sha256'].upper()}",
            ]
        )
    lines.extend(
        [
            "ManifestType: installer",
            f"ManifestVersion: {WINGET_MANIFEST_VERSION}",
        ]
    )
    return "\n".join(lines) + "\n"


def write_text(path: Path, content: str, check: bool) -> bool:
    if path.exists():
        current = path.read_text()
        if current == content:
            print(f"unchanged {path.relative_to(ROOT)}")
            return False
    if check:
        raise SyncError(f"{path.relative_to(ROOT)} is out of date")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content)
    print(f"updated {path.relative_to(ROOT)}")
    return True


def main() -> int:
    args = parse_args()
    token = os.getenv("GITHUB_TOKEN") or os.getenv("GH_TOKEN")
    version = args.version or cli_version()
    tag = args.tag or f"schemaui-cli-v{version}"

    release = fetch_json(release_url(args.repo, tag), token)
    if not isinstance(release, dict):
        raise SyncError(f"unexpected release payload type: {type(release)!r}")

    assets_list = release.get("assets")
    if not isinstance(assets_list, list):
        raise SyncError("release payload is missing assets")
    assets = {
        asset["name"]: asset
        for asset in assets_list
        if isinstance(asset, dict) and isinstance(asset.get("name"), str)
    }

    windows_assets: dict[str, dict[str, str]] = {}
    for arch, target in WINDOWS_TARGETS.items():
        asset = require_asset(assets, f"schemaui-{target}.zip")
        windows_assets[arch] = {
            "url": str(asset["browser_download_url"]),
            "sha256": asset_sha256(asset, token),
        }

    source_sha256 = sha256_url(tag_archive_url(args.repo, tag), token)

    changed = False
    changed |= write_text(FORMULA_PATH, render_formula(args.repo, version, source_sha256), args.check)
    changed |= write_text(SCOOP_PATH, render_scoop(args.repo, version, windows_assets), args.check)

    winget_version_dir = WINGET_DIR / version
    changed |= write_text(
        winget_version_dir / f"{PACKAGE_IDENTIFIER}.yaml",
        render_winget_version(version),
        args.check,
    )
    changed |= write_text(
        winget_version_dir / f"{PACKAGE_IDENTIFIER}.locale.en-US.yaml",
        render_winget_default_locale(args.repo, version),
        args.check,
    )
    changed |= write_text(
        winget_version_dir / f"{PACKAGE_IDENTIFIER}.installer.yaml",
        render_winget_installer(version, windows_assets),
        args.check,
    )

    if args.check:
        print("package manager manifests are up to date")
    elif not changed:
        print("package manager manifests already in sync")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except SyncError as exc:
        print(f"error: {exc}", file=sys.stderr)
        raise SystemExit(1)
