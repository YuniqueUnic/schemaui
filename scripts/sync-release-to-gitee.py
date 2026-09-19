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

Tags are processed oldest version first. Gitee orders a repo's release list by
when the mirror created each record, newest first, so a backfill that creates the
newest release first leaves it stranded at the bottom of the list. The newest
release must be the one a visitor sees first, which is only true if the records
were created in ascending version order.

--check reports the same differences without writing and exits non-zero when it
finds any, so CI can assert the mirror still matches GitHub.
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
        "--dry-run",
        action="store_true",
        help="report what would change without writing to Gitee",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="report what would change and exit non-zero if anything would",
    )
    return parser.parse_args()


def requested_tags(values: list[str]) -> list[str]:
    """Split --tag values on commas and whitespace, ordered oldest version first.

    cd.yml passes a single tag; a backfill passes a list, which is easier to keep
    on one line in a workflow input than a repeated flag.

    The order matters even though every tag gets mirrored either way. Gitee lists
    a repo's releases newest-*record*-first, and a record is stamped when the
    mirror creates it, so creating the newest release first buries it at the
    bottom of the list where nobody browsing the mirror will find it. Sorting by
    version rather than trusting the caller keeps the page readable.
    """
    tags: list[str] = []
    for value in values:
        tags += [tag for tag in re.split(r"[,\s]+", value) if tag]
    return sorted(dict.fromkeys(tags), key=version_key)


def version_key(tag: str) -> tuple[tuple[int, ...], str]:
    """Order a tag by the version it carries, falling back to its text."""
    match = re.search(r"(\d+(?:\.\d+)*)$", tag)
    numbers = tuple(int(part) for part in match.group(1).split(".")) if match else ()
    return (numbers, tag)


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


def github_headers() -> dict[str, str]:
    headers = {"Accept": "application/vnd.github+json"}
    # Optional, and only useful in CI where the runner IP is shared and the
    # unauthenticated rate limit is easy to hit. A backfill definitely hits it.
    token = os.environ.get("GITHUB_TOKEN")
    if token:
        headers["Authorization"] = f"Bearer {token}"
    return headers


def github_release(repo: str, tag: str) -> dict:
    url = f"{GITHUB_API}/repos/{repo}/releases/tags/{urllib.parse.quote(tag)}"
    release = request_json(url, headers=github_headers())
    if not isinstance(release, dict):
        raise SyncError(f"{repo} has no GitHub release tagged {tag}")
    return release


def github_tag_commit(repo: str, tag: str) -> str:
    """Return the commit a tag points at, dereferencing an annotated tag.

    Gitee wants a commitish when it creates the release record, and it creates
    the tag at that commitish if the tag is missing there. A branch name would
    therefore drop an old release's tag onto today's tip, so resolve the commit
    the tag actually names. GitHub's own release.target_commitish is no help: it
    stores the branch the release was cut from, not a sha.
    """
    headers = github_headers()
    ref = request_json(
        f"{GITHUB_API}/repos/{repo}/git/ref/tags/{urllib.parse.quote(tag)}",
        headers=headers,
    )
    target = ref["object"]
    if target["type"] == "tag":
        # Annotated tag: the ref names a tag object, which names the commit.
        annotated = request_json(
            f"{GITHUB_API}/repos/{repo}/git/tags/{target['sha']}", headers=headers
        )
        return annotated["object"]["sha"]
    return target["sha"]


def gitee_url(path: str, token: str, **params: str) -> str:
    """Build a Gitee API URL, carrying the token only when there is one.

    Gitee reads an empty `access_token` as an invalid one and answers 401, so an
    absent token has to be left out of the query rather than sent blank.
    """
    if token:
        params["access_token"] = token
    query = urllib.parse.urlencode(params)
    return f"{GITEE_API}{path}?{query}" if query else f"{GITEE_API}{path}"


def gitee_release(gitee_repo: str, tag: str, token: str) -> dict | None:
    """Return the mirror's release for tag, or None when it does not exist yet."""
    path = f"/repos/{gitee_repo}/releases/tags/{urllib.parse.quote(tag)}"
    release = request_json(gitee_url(path, token))
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


def gitee_attachments(gitee_repo: str, release_id: int, token: str) -> list[dict]:
    """List a release's uploaded attachments, each with an id and a size.

    Preferred over the release's own `assets` array, which reports neither: a
    name-only view cannot tell a duplicate from a single copy, and the set it
    invites you to build silently swallows exactly that difference. `assets`
    also mixes in the two source archives Gitee generates, which are not
    attachments and cannot be deleted.
    """
    attachments = request_json(gitee_url(f"/repos/{gitee_repo}/releases/{release_id}/attach_files", token))
    return attachments if isinstance(attachments, list) else []


def gitee_delete_attachment(gitee_repo: str, release_id: int, attachment_id: int, token: str) -> None:
    path = f"/repos/{gitee_repo}/releases/{release_id}/attach_files/{attachment_id}"
    request_bytes(gitee_url(path, token), method="DELETE")


def prune_duplicates(
    gitee_repo: str,
    release_id: int,
    name: str,
    copies: list[dict],
    size: int,
    token: str,
    dry_run: bool,
) -> int:
    """Reduce an asset to a single attachment, keeping the copy worth keeping.

    Two runs that overlap both read the release before either uploads, so both
    upload the same name and the release ends up listing it twice. Nothing else
    can clean that up: deleting the whole release to rebuild it would also push
    it to the top of Gitee's release list, which is ordered by record age.

    The copy whose size matches GitHub's is kept; a shorter one is a truncated
    upload. Ties go to the earliest, so repeated runs converge.

    Returns the number of surplus copies, whether or not they were removed.
    """
    if len(copies) < 2:
        return 0
    surplus = len(copies) - 1
    if dry_run:
        print(f"  {name}: would drop {surplus} duplicate copy(ies)")
        return surplus
    keep = next(
        (copy for copy in copies if copy["size"] == size), min(copies, key=lambda a: a["id"])
    )
    for copy in copies:
        if copy["id"] != keep["id"]:
            gitee_delete_attachment(gitee_repo, release_id, copy["id"], token)
    print(f"  {name}: dropped {surplus} duplicate copy(ies)")
    return surplus


def attach_asset(gitee_repo: str, release_id: int, name: str, blob: bytes, token: str) -> None:
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
            held = gitee_attachments(gitee_repo, release_id, token)
            if any(attachment["name"] == name for attachment in held):
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
        commit = github_tag_commit(repo, tag)
        release = gitee_create_release(gitee_repo, source, commit, token)
        print(f"{tag}: created Gitee release {release['id']} at {commit[:7]}")
    else:
        print(f"{tag}: reusing Gitee release {release['id']}")

    held: dict[str, list[dict]] = {}
    for attachment in gitee_attachments(gitee_repo, release["id"], token):
        held.setdefault(attachment["name"], []).append(attachment)

    differences = 0
    for asset in assets:
        name = asset["name"]
        size = int(asset["size"])
        copies = held.get(name, [])
        if copies:
            if len(copies) == 1:
                print(f"  {name}: already mirrored")
            differences += prune_duplicates(
                gitee_repo, release["id"], name, copies, size, token, dry_run
            )
            continue
        if size > GITEE_MAX_ASSET_BYTES:
            raise SyncError(
                f"{name} is {size} bytes, over Gitee's {GITEE_MAX_ASSET_BYTES} byte limit"
            )
        differences += 1
        if dry_run:
            print(f"  {name}: would upload {size} bytes")
            continue
        blob = download_asset(asset["browser_download_url"])
        attach_asset(gitee_repo, release["id"], name, blob, token)
        print(f"  {name}: uploaded {len(blob)} bytes")
    return differences


def main() -> int:
    args = parse_args()
    tags = requested_tags(args.tag)
    if not tags:
        raise SyncError("pass at least one --tag")

    token = os.environ.get("GITEE_TOKEN", "")
    # Every mode reads the Gitee API, including --check and --dry-run. Gitee
    # rate-limits anonymous callers hard enough that a run dies partway through
    # with a 403, so demand the token up front instead of failing mid-way.
    if not token:
        raise SyncError("GITEE_TOKEN is not set")

    # --check is --dry-run that also fails, so a CI job can assert the mirror
    # matches GitHub without ever writing to it.
    dry_run = args.dry_run or args.check

    differences = 0
    for tag in tags:
        differences += mirror_tag(
            tag,
            repo=args.repo,
            gitee_repo=args.gitee_repo,
            token=token,
            dry_run=dry_run,
        )

    if args.check:
        if differences:
            print(f"{len(tags)} release(s) checked, {differences} difference(s) from GitHub")
            return 1
        print(f"{len(tags)} release(s) checked, mirror matches GitHub")
        return 0
    if dry_run:
        print(f"would mirror {len(tags)} release(s), {differences} change(s)")
    else:
        print(f"mirrored {len(tags)} release(s), {differences} change(s)")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except SyncError as exc:
        print(f"error: {exc}", file=sys.stderr)
        raise SystemExit(1)
