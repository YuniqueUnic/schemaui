#!/usr/bin/env python3
"""Mirror published schemaui-cli releases to the Gitee mirror.

github.com is unreliable from mainland China, so a2ui-ask's installer falls back
to gitee.com/Credhat/schemaui. That fallback only works while the mirror carries
the same binaries under the same tag and file names, because Gitee serves
release assets from the same layout as GitHub:

    https://gitee.com/Credhat/schemaui/releases/download/<tag>/<asset>

cd.yml runs this right after upload-assets, so a release is mirrored as soon as
its binaries exist. --tag backfills the releases that predate the mirror.

Re-running is cheap and safe: an existing release is reused and an asset that is
already attached is skipped, so an interrupted backfill can simply be run again.
A dropped connection is retried, because a backfill makes a few hundred uploads
and one transient failure should not sink the run.
"""
from __future__ import annotations

import argparse
import http.client
import json
import os
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid

DEFAULT_REPO = "YuniqueUnic/schemaui"
DEFAULT_GITEE_REPO = "Credhat/schemaui"

GITHUB_API = "https://api.github.com"
GITEE_API = "https://gitee.com/api/v5"

# Gitee refuses an attachment larger than 100 MB. That is the mirror's limit
# rather than GitHub's, so stop before uploading instead of half-failing.
GITEE_MAX_ASSET_BYTES = 100 * 1024 * 1024

TIMEOUT = 120
UPLOAD_TIMEOUT = 600

RETRIES = 3
RETRY_BACKOFF_SECONDS = 3
TRANSIENT_HTTP_CODES = {408, 425, 429, 500, 502, 503, 504}


class SyncError(RuntimeError):
    pass


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--tag",
        action="append",
        default=[],
        metavar="TAG",
        help=(
            "release tag to mirror; repeatable, and each value may list several "
            "tags separated by spaces or commas"
        ),
    )
    parser.add_argument("--repo", default=DEFAULT_REPO, help="GitHub repo in owner/name form")
    parser.add_argument(
        "--gitee-repo",
        default=DEFAULT_GITEE_REPO,
        help="Gitee mirror in owner/name form",
    )
    parser.add_argument(
        "--target-commitish",
        default="main",
        help=(
            "branch the Gitee release points at; Gitee defaults to master, "
            "which this mirror does not have"
        ),
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="report what would change without writing to Gitee",
    )
    return parser.parse_args()


def requested_tags(values: list[str]) -> list[str]:
    """Split --tag values on commas and whitespace, keeping the caller's order.

    cd.yml passes a single tag; a backfill passes a list, which is easier to
    keep on one line in a workflow input than a repeated flag.
    """
    tags: list[str] = []
    for value in values:
        tags += [tag for tag in re.split(r"[,\s]+", value) if tag]
    return list(dict.fromkeys(tags))


def request_bytes(
    url: str,
    *,
    method: str = "GET",
    data: bytes | None = None,
    headers: dict[str, str] | None = None,
    timeout: int = TIMEOUT,
    attempts: int = RETRIES,
) -> bytes:
    """Perform a request, keeping the access token out of any error message.

    Gitee takes its token as a query parameter, so reporting the whole URL on
    failure would write it to the CI log; only the path is ever reported.

    A dropped connection or a 5xx is retried, since every caller but the asset
    upload is a read and those failures are transient. The upload opts out with
    attempts=1: resending a body whose first copy may already have been stored
    would attach the same asset twice, so attach_asset verifies instead.
    """
    request = urllib.request.Request(url, data=data, method=method, headers=headers or {})
    path = url.split("?")[0]
    for attempt in range(1, attempts + 1):
        try:
            with urllib.request.urlopen(request, timeout=timeout) as response:
                return response.read()
        except urllib.error.HTTPError as error:
            detail = error.read()[:300].decode(errors="replace").strip()
            if error.code not in TRANSIENT_HTTP_CODES or attempt == attempts:
                raise SyncError(f"{method} {path} -> HTTP {error.code}: {detail}") from error
            reason = f"HTTP {error.code}"
        except (urllib.error.URLError, http.client.HTTPException, OSError) as error:
            # A dropped connection arrives as RemoteDisconnected (HTTPException),
            # a timeout as URLError wrapping socket.timeout, a reset as OSError.
            if attempt == attempts:
                cause = getattr(error, "reason", error)
                raise SyncError(f"{method} {path} -> {cause}") from error
            reason = str(getattr(error, "reason", error))
        delay = RETRY_BACKOFF_SECONDS * attempt
        print(f"  {path}: {reason}; retrying in {delay}s", file=sys.stderr)
        time.sleep(delay)
    raise AssertionError("retry loop exited without returning or raising")


def request_json(url: str, **kwargs) -> object:
    body = request_bytes(url, **kwargs)
    if not body:
        return None
    try:
        return json.loads(body)
    except json.JSONDecodeError as error:
        raise SyncError(f"{url.split('?')[0]} -> not JSON: {body[:200]!r}") from error


def github_release(repo: str, tag: str) -> dict:
    headers = {"Accept": "application/vnd.github+json"}
    # Optional, and only useful in CI where the runner IP is shared and the
    # unauthenticated rate limit is easy to hit.
    token = os.environ.get("GITHUB_TOKEN")
    if token:
        headers["Authorization"] = f"Bearer {token}"
    url = f"{GITHUB_API}/repos/{repo}/releases/tags/{urllib.parse.quote(tag)}"
    release = request_json(url, headers=headers)
    if not isinstance(release, dict):
        raise SyncError(f"{repo} has no GitHub release tagged {tag}")
    return release


def gitee_release(gitee_repo: str, tag: str, token: str) -> dict | None:
    """Return the mirror's release for tag, or None when it does not exist yet."""
    query = urllib.parse.urlencode({"access_token": token})
    url = f"{GITEE_API}/repos/{gitee_repo}/releases/tags/{urllib.parse.quote(tag)}?{query}"
    release = request_json(url)
    return release if isinstance(release, dict) else None


def gitee_create_release(gitee_repo: str, source: dict, target_commitish: str, token: str) -> dict:
    """Create the mirror's release from the GitHub one, notes included."""
    form = {
        "access_token": token,
        "tag_name": source["tag_name"],
        "name": source["name"] or source["tag_name"],
        "body": source["body"] or "",
        "target_commitish": target_commitish,
        "prerelease": "true" if source.get("prerelease") else "false",
    }
    url = f"{GITEE_API}/repos/{gitee_repo}/releases"
    release = request_json(url, method="POST", data=urllib.parse.urlencode(form).encode())
    if not isinstance(release, dict) or "id" not in release:
        raise SyncError(f"Gitee did not return a release for {source['tag_name']}")
    return release


def gitee_attach_asset(gitee_repo: str, release_id: int, name: str, blob: bytes, token: str) -> None:
    """Upload one asset, using the release asset name as the filename.

    The name is what a2ui-ask's installer asks for, so it has to survive the
    trip unchanged.
    """
    boundary = "----schemaui-gitee-sync-" + uuid.uuid4().hex
    preamble = (
        f"--{boundary}\r\n"
        f'Content-Disposition: form-data; name="access_token"\r\n\r\n{token}\r\n'
        f"--{boundary}\r\n"
        f'Content-Disposition: form-data; name="file"; filename="{name}"\r\n'
        "Content-Type: application/octet-stream\r\n\r\n"
    ).encode()
    url = f"{GITEE_API}/repos/{gitee_repo}/releases/{release_id}/attach_files"
    request_bytes(
        url,
        method="POST",
        data=preamble + blob + f"\r\n--{boundary}--\r\n".encode(),
        headers={"Content-Type": f"multipart/form-data; boundary={boundary}"},
        timeout=UPLOAD_TIMEOUT,
        # attach_asset owns retrying this one; a blind resend here could store
        # the same bytes twice.
        attempts=1,
    )


def gitee_asset_names(gitee_repo: str, tag: str, token: str) -> set[str]:
    """Names the mirror's release carries, its own source archives included."""
    release = gitee_release(gitee_repo, tag, token)
    return {asset.get("name") for asset in (release or {}).get("assets") or []}


def attach_asset(
    gitee_repo: str, tag: str, release_id: int, name: str, blob: bytes, token: str
) -> None:
    """Attach one asset, tolerating a connection Gitee dropped mid-upload.

    Gitee can close the connection before it answers, which leaves no way to
    tell from the failure alone whether the bytes were stored. Attaching the
    same name twice would leave the release carrying two copies, so a retry
    asks Gitee what it holds rather than resending blind.
    """
    for attempt in range(1, RETRIES + 1):
        try:
            gitee_attach_asset(gitee_repo, release_id, name, blob, token)
            return
        except SyncError as error:
            if attempt == RETRIES:
                raise
            if name in gitee_asset_names(gitee_repo, tag, token):
                print(f"  {name}: stored before the connection dropped", file=sys.stderr)
                return
            delay = RETRY_BACKOFF_SECONDS * attempt
            print(f"  {name}: {error}; retrying in {delay}s", file=sys.stderr)
            time.sleep(delay)


def download_asset(url: str) -> bytes:
    """Fetch a release asset from GitHub; the repo is public, so no token."""
    return request_bytes(url, headers={"Accept": "application/octet-stream"})


def mirror_tag(
    tag: str,
    *,
    repo: str,
    gitee_repo: str,
    target_commitish: str,
    token: str,
    dry_run: bool,
) -> int:
    source = github_release(repo, tag)
    assets = source.get("assets") or []
    if not assets:
        print(f"{tag}: GitHub release carries no assets", file=sys.stderr)

    release = gitee_release(gitee_repo, tag, token)
    if release is None:
        if dry_run:
            print(f"{tag}: would create the Gitee release with {len(assets)} asset(s)")
            return len(assets)
        release = gitee_create_release(gitee_repo, source, target_commitish, token)
        print(f"{tag}: created Gitee release {release['id']}")
    else:
        print(f"{tag}: reusing Gitee release {release['id']}")

    # Gitee adds the tag's two source archives by itself, so the names to
    # compare against are whatever the release already carries.
    attached = {asset.get("name") for asset in release.get("assets") or []}
    uploaded = 0
    for asset in assets:
        name = asset["name"]
        if name in attached:
            print(f"  {name}: already mirrored")
            continue
        size = int(asset["size"])
        if size > GITEE_MAX_ASSET_BYTES:
            raise SyncError(
                f"{name} is {size} bytes, over Gitee's {GITEE_MAX_ASSET_BYTES} byte limit"
            )
        if dry_run:
            print(f"  {name}: would upload {size} bytes")
            uploaded += 1
            continue
        blob = download_asset(asset["browser_download_url"])
        attach_asset(gitee_repo, tag, release["id"], name, blob, token)
        uploaded += 1
        print(f"  {name}: uploaded {len(blob)} bytes")
    return uploaded


def main() -> int:
    args = parse_args()
    tags = requested_tags(args.tag)
    if not tags:
        raise SyncError("pass at least one --tag")

    token = os.environ.get("GITEE_TOKEN", "")
    if not token and not args.dry_run:
        raise SyncError("GITEE_TOKEN is not set")

    uploaded = 0
    for tag in tags:
        uploaded += mirror_tag(
            tag,
            repo=args.repo,
            gitee_repo=args.gitee_repo,
            target_commitish=args.target_commitish,
            token=token,
            dry_run=args.dry_run,
        )

    if args.dry_run:
        print(f"would mirror {len(tags)} release(s), {uploaded} asset(s)")
    else:
        print(f"mirrored {len(tags)} release(s), uploaded {uploaded} asset(s)")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except SyncError as exc:
        print(f"error: {exc}", file=sys.stderr)
        raise SystemExit(1)
