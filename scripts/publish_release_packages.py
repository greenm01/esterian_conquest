#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

from build_release_bundle import SUPPORTED_TARGETS
from upsert_release_note import merge_body
from write_release_checksums import file_sha256


REPO_ROOT = Path(__file__).resolve().parents[1]
RELEASES_DIR = REPO_ROOT / "releases"
BUILD_RELEASE_PACKAGES = REPO_ROOT / "scripts" / "build_release_packages.py"
BUILD_RELEASE_BUNDLE = REPO_ROOT / "scripts" / "build_release_bundle.py"
CHECKSUM_PATH = RELEASES_DIR / "SHA256SUMS.txt"
SIGNATURE_PATH = RELEASES_DIR / "SHA256SUMS.txt.asc"
RELEASE_NOTE_PATH = RELEASES_DIR / "nc-connect-release-note.md"
RELEASE_NOTE_URL = (
    "https://github.com/greenm01/nostrian-conquest/blob/main/docs/release-signing.md"
)
EC_CONNECT_ARCHIVE_RE = re.compile(r"^nc-connect-v.*\.(?:zip|tar\.gz)$")
SYSOP_ARCHIVE_RE = re.compile(r"^nc-sysop-v.*\.(?:zip|tar\.gz)$")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Build selected release assets under releases/ and upload them to an "
            "existing GitHub Release with gh release upload --clobber."
        )
    )
    parser.add_argument(
        "--tag",
        default="release-artifacts",
        help="GitHub release tag to update. Default: release-artifacts",
    )
    parser.add_argument(
        "--variant",
        action="append",
        choices=("classic", "unlocked"),
        help=(
            "DOS package variant to build/upload. Defaults to both classic and "
            "unlocked when no other asset selection flags are passed."
        ),
    )
    parser.add_argument(
        "--nc-connect-target",
        action="append",
        choices=sorted(SUPPORTED_TARGETS),
        help="Build and upload a public nc-connect archive for the selected target.",
    )
    parser.add_argument(
        "--sysop-target",
        action="append",
        choices=(
            "x86_64-unknown-linux-gnu",
            "x86_64-pc-windows-msvc",
            "i686-pc-windows-msvc",
            "i686-win7-windows-msvc",
        ),
        help=(
            "Build and upload a public BBS/sysop archive for the "
            "selected target."
        ),
    )
    parser.add_argument(
        "--gpg-key",
        help=(
            "GPG key fingerprint or key ID used to sign the shared public Rust "
            "checksum manifest."
        ),
    )
    return parser.parse_args()


def run(
    argv: list[str],
    *,
    cwd: Path | None = None,
    capture_output: bool = False,
) -> subprocess.CompletedProcess[str]:
    env = dict(os.environ)
    env["RUSTC_WRAPPER"] = ""
    return subprocess.run(
        argv,
        cwd=cwd or REPO_ROOT,
        check=True,
        text=True,
        capture_output=capture_output,
        env=env,
    )


def capture(argv: list[str], *, cwd: Path | None = None) -> str:
    return run(argv, cwd=cwd, capture_output=True).stdout


def load_version() -> str:
    cargo_toml = (REPO_ROOT / "rust" / "nc-game" / "Cargo.toml").read_text(encoding="utf-8")
    match = re.search(r'(?m)^version = "([^"]+)"$', cargo_toml)
    if match is None:
        raise SystemExit("could not parse version from rust/nc-game/Cargo.toml")
    return match.group(1)


def list_release_assets(release_tag: str) -> list[str]:
    output = capture(
        ["gh", "release", "view", release_tag, "--json", "assets", "--jq", ".assets[].name"]
    )
    return [line.strip() for line in output.splitlines() if line.strip()]


def selected_dos_variants(args: argparse.Namespace) -> list[str]:
    variants = list(args.variant or [])
    if variants or args.nc_connect_target or args.sysop_target:
        return variants
    return ["classic", "unlocked"]


def build_dos_variants(variants: list[str]) -> list[Path]:
    if not variants:
        return []
    command = [sys.executable, str(BUILD_RELEASE_PACKAGES), "--verify"]
    for variant in variants:
        command.extend(["--variant", variant])
    run(command)
    archive_names = {
        "classic": "ec-v1.5-classic.zip",
        "unlocked": "ec-v1.5-unlocked.zip",
    }
    return [RELEASES_DIR / archive_names[variant] for variant in variants]


def build_nc_connect_archive(target: str) -> Path:
    output = capture(
        [
            sys.executable,
            str(BUILD_RELEASE_BUNDLE),
            "--artifact",
            "nc-connect",
            "--target",
            target,
            "--verify",
        ]
    )
    lines = [line.strip() for line in output.splitlines() if line.strip()]
    if not lines:
        raise SystemExit(f"build_release_bundle.py did not print an archive path for {target}")
    return Path(lines[-1])


def build_sysop_archive(target: str) -> Path:
    output = capture(
        [
            sys.executable,
            str(BUILD_RELEASE_BUNDLE),
            "--artifact",
            "sysop",
            "--target",
            target,
            "--verify",
        ]
    )
    lines = [line.strip() for line in output.splitlines() if line.strip()]
    if not lines:
        raise SystemExit(f"build_release_bundle.py did not print an archive path for {target}")
    return Path(lines[-1])


def download_existing_nc_connect_assets(
    release_tag: str,
    selected_names: set[str],
    download_dir: Path,
) -> list[Path]:
    output = capture(
        ["gh", "release", "view", release_tag, "--json", "assets", "--jq", ".assets[].name"]
    )
    asset_names = [line.strip() for line in output.splitlines() if line.strip()]
    downloaded: list[Path] = []
    for asset_name in asset_names:
        if asset_name in selected_names or not EC_CONNECT_ARCHIVE_RE.match(asset_name):
            continue
        run(
            [
                "gh",
                "release",
                "download",
                release_tag,
                "--pattern",
                asset_name,
                "--dir",
                str(download_dir),
            ]
        )
        downloaded.append(download_dir / asset_name)
    return downloaded


def download_existing_public_rust_assets(
    release_tag: str,
    current_version: str,
    selected_names: set[str],
    download_dir: Path,
) -> list[Path]:
    downloaded: list[Path] = []
    current_marker = f"-v{current_version}-"
    for asset_name in list_release_assets(release_tag):
        matches_public_rust = EC_CONNECT_ARCHIVE_RE.match(asset_name) or SYSOP_ARCHIVE_RE.match(asset_name)
        if (
            asset_name in selected_names
            or not matches_public_rust
            or current_marker not in asset_name
        ):
            continue
        run(
            [
                "gh",
                "release",
                "download",
                release_tag,
                "--pattern",
                asset_name,
                "--dir",
                str(download_dir),
            ]
        )
        downloaded.append(download_dir / asset_name)
    return downloaded


def prune_stale_public_rust_assets(release_tag: str, current_version: str) -> None:
    current_marker = f"-v{current_version}-"
    for asset_name in list_release_assets(release_tag):
        matches_public_rust = EC_CONNECT_ARCHIVE_RE.match(asset_name) or SYSOP_ARCHIVE_RE.match(asset_name)
        if not matches_public_rust or current_marker in asset_name:
            continue
        run(["gh", "release", "delete-asset", release_tag, asset_name, "--yes"])


def write_checksum_manifest(output_path: Path, assets: list[Path]) -> None:
    rows = [(asset.name, file_sha256(asset)) for asset in assets]
    rows.sort(key=lambda row: row[0])
    manifest = "".join(f"{digest}  {name}\n" for name, digest in rows)
    output_path.write_text(manifest, encoding="utf-8")


def resolve_fingerprint(gpg_key: str) -> str:
    output = capture(["gpg", "--batch", "--with-colons", "--fingerprint", gpg_key])
    for line in output.splitlines():
        parts = line.split(":")
        if parts and parts[0] == "fpr" and len(parts) > 9:
            return parts[9]
    raise SystemExit(f"unable to resolve a full fingerprint for GPG key: {gpg_key}")


def sign_checksum_manifest(gpg_key: str) -> str:
    SIGNATURE_PATH.unlink(missing_ok=True)
    run(
        [
            "gpg",
            "--batch",
            "--yes",
            "--armor",
            "--local-user",
            gpg_key,
            "--output",
            str(SIGNATURE_PATH),
            "--detach-sign",
            str(CHECKSUM_PATH),
        ]
    )
    return resolve_fingerprint(gpg_key)


def write_release_note(fingerprint: str) -> None:
    body = f"""<!-- NC-RUST-VERIFY:START -->
## Verify Rust downloads

The Rust-built public downloads in this release can be verified with the signed `SHA256SUMS.txt` manifest.

`gpg --verify SHA256SUMS.txt.asc SHA256SUMS.txt`
`shasum -a 256 -c SHA256SUMS.txt`

Full instructions and public key: {RELEASE_NOTE_URL}
Signing key fingerprint: `{fingerprint}`

The signed manifest covers the public Rust download archives on this page, including `nc-connect` player packages and `nc-sysop` BBS/sysop packages, but not the DOS compatibility bundles.
<!-- NC-RUST-VERIFY:END -->
"""
    RELEASE_NOTE_PATH.write_text(body, encoding="utf-8")


def update_release_body(release_tag: str) -> None:
    existing_body = capture(["gh", "release", "view", release_tag, "--json", "body", "--jq", ".body"])
    release_note = RELEASE_NOTE_PATH.read_text(encoding="utf-8")
    with tempfile.NamedTemporaryFile(
        prefix="ec-release-body-",
        suffix=".md",
        mode="w",
        encoding="utf-8",
        delete=False,
    ) as handle:
        handle.write(merge_body(existing_body, release_note))
        merged_path = Path(handle.name)
    try:
        run(["gh", "release", "edit", release_tag, "--notes-file", str(merged_path)])
    finally:
        merged_path.unlink(missing_ok=True)


def upload_assets(release_tag: str, assets: list[Path]) -> None:
    command = ["gh", "release", "upload", release_tag]
    command.extend(str(asset) for asset in assets)
    command.append("--clobber")
    run(command)


def main() -> None:
    args = parse_args()
    current_version = load_version()
    dos_variants = selected_dos_variants(args)
    nc_connect_targets = list(args.nc_connect_target or [])
    sysop_targets = list(args.sysop_target or [])

    if (nc_connect_targets or sysop_targets) and not args.gpg_key:
        raise SystemExit("--gpg-key is required when publishing public Rust release assets.")

    assets = build_dos_variants(dos_variants)
    nc_connect_assets = [build_nc_connect_archive(target) for target in nc_connect_targets]
    sysop_assets = [build_sysop_archive(target) for target in sysop_targets]
    assets.extend(nc_connect_assets)
    assets.extend(sysop_assets)

    if not assets:
        raise SystemExit("no release assets selected")

    public_rust_assets = nc_connect_assets + sysop_assets
    if public_rust_assets:
        with tempfile.TemporaryDirectory(prefix="ec-release-download-") as temp_dir:
            download_dir = Path(temp_dir)
            selected_names = {asset.name for asset in public_rust_assets}
            manifest_assets = public_rust_assets + download_existing_public_rust_assets(
                args.tag,
                current_version,
                selected_names,
                download_dir,
            )
            write_checksum_manifest(CHECKSUM_PATH, manifest_assets)
            fingerprint = sign_checksum_manifest(args.gpg_key)
            write_release_note(fingerprint)
            assets.extend([CHECKSUM_PATH, SIGNATURE_PATH])

        upload_assets(args.tag, assets)
        prune_stale_public_rust_assets(args.tag, current_version)
        update_release_body(args.tag)
        print(f"Updated release assets on tag: {args.tag}")
        print("Updated the release-body verification notice automatically.")
        return

    upload_assets(args.tag, assets)
    print(f"Updated release assets on tag: {args.tag}")


if __name__ == "__main__":
    main()
