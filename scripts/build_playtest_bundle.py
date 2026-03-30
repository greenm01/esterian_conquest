#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import tarfile
import tempfile
import zipfile
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
import re


REPO_ROOT = Path(__file__).resolve().parents[1]
RUST_ROOT = REPO_ROOT / "rust"
RELEASES_DIR = REPO_ROOT / "releases"
PLAYER_MANUAL = REPO_ROOT / "docs" / "manuals" / "ec_player_manual.pdf"
SYSOP_MANUAL = REPO_ROOT / "docs" / "manuals" / "ec_sysop_manual.pdf"


@dataclass(frozen=True)
class TargetPlatform:
    target_triple: str
    slug: str
    display_name: str
    issue_platform_label: str


SUPPORTED_TARGETS = {
    "x86_64-unknown-linux-gnu": TargetPlatform(
        target_triple="x86_64-unknown-linux-gnu",
        slug="linux-x64",
        display_name="Linux x64",
        issue_platform_label="Linux distro",
    ),
    "aarch64-apple-darwin": TargetPlatform(
        target_triple="aarch64-apple-darwin",
        slug="macos-arm64",
        display_name="macOS Apple Silicon",
        issue_platform_label="macOS version",
    ),
    "x86_64-apple-darwin": TargetPlatform(
        target_triple="x86_64-apple-darwin",
        slug="macos-x64",
        display_name="macOS Intel",
        issue_platform_label="macOS version",
    ),
    "x86_64-pc-windows-gnu": TargetPlatform(
        target_triple="x86_64-pc-windows-gnu",
        slug="windows-x64",
        display_name="Windows x64",
        issue_platform_label="Windows version",
    ),
}


@dataclass(frozen=True)
class BundleSpec:
    version: str
    platform: TargetPlatform
    artifact: str

    @property
    def bundle_root_name(self) -> str:
        if self.artifact == "ec-connect":
            return f"ec-connect-v{self.version}-{self.platform.slug}"
        return f"esterian-conquest-v{self.version}-{self.platform.slug}"

    @property
    def is_windows(self) -> bool:
        return self.platform.target_triple.startswith("x86_64-pc-windows")

    @property
    def archive_name(self) -> str:
        ext = "zip" if self.is_windows else "tar.gz"
        return f"{self.bundle_root_name}.{ext}"


def parse_args(default_target: str | None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Build a public beta ec-game/ec-sysop/ec-connect bundle for Linux or macOS."
        )
    )
    parser.add_argument(
        "--artifact",
        choices=("public-beta", "ec-connect"),
        default="public-beta",
        help=(
            "Artifact type to package. `public-beta` keeps the internal combined "
            "bundle; `ec-connect` builds the public player archive."
        ),
    )
    parser.add_argument(
        "--target",
        choices=sorted(SUPPORTED_TARGETS),
        default=default_target,
        help=(
            "Rust target triple to package. Defaults to the current host target "
            "when omitted."
        ),
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=RELEASES_DIR,
        help="Directory where the generated tar.gz will be written.",
    )
    parser.add_argument(
        "--verify",
        action="store_true",
        help="Unpack and verify the generated public beta bundle after building it.",
    )
    return parser.parse_args()


def load_version() -> str:
    cargo_toml = (RUST_ROOT / "ec-game" / "Cargo.toml").read_text(encoding="utf-8")
    match = re.search(r'(?m)^version = "([^"]+)"$', cargo_toml)
    if match is None:
        raise SystemExit("could not parse version from rust/ec-game/Cargo.toml")
    return match.group(1)


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
    return run(argv, cwd=cwd, capture_output=True).stdout.strip()


def detect_host_target() -> str:
    rustc_verbose = capture(["rustc", "-vV"])
    for line in rustc_verbose.splitlines():
        if line.startswith("host: "):
            return line.removeprefix("host: ").strip()
    raise SystemExit("unable to determine rustc host target from `rustc -vV`")


def resolve_target(target_triple: str | None) -> TargetPlatform:
    selected = target_triple or detect_host_target()
    try:
        return SUPPORTED_TARGETS[selected]
    except KeyError as err:
        supported = ", ".join(sorted(SUPPORTED_TARGETS))
        raise SystemExit(
            f"unsupported target '{selected}'. supported targets: {supported}"
        ) from err


def artifact_binaries(spec: BundleSpec) -> tuple[str, ...]:
    if spec.artifact == "ec-connect":
        if spec.is_windows:
            return ("ec-connect", "ec-connect-cli")
        return ("ec-connect",)
    return ("ec-game", "ec-sysop", "ec-connect")


def build_binaries(spec: BundleSpec) -> dict[str, Path]:
    binaries = artifact_binaries(spec)
    run(
        ["cargo", "build", "--release", "--target", spec.platform.target_triple]
        + [arg for name in binaries for arg in ("-p", name)],
        cwd=RUST_ROOT,
    )

    target_dir = RUST_ROOT / "target" / spec.platform.target_triple / "release"
    ext = ".exe" if spec.is_windows else ""
    return {name: target_dir / f"{name}{ext}" for name in binaries}


def build_info_text(spec: BundleSpec) -> str:
    commit = capture(["git", "rev-parse", "HEAD"])
    short_commit = capture(["git", "rev-parse", "--short", "HEAD"])
    rustc = capture(["rustc", "-V"])
    built_at = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    lines = [
        f"artifact={spec.artifact}",
        f"version={spec.version}",
        f"git_commit={commit}",
        f"git_commit_short={short_commit}",
        f"target={spec.platform.target_triple}",
        f"built_at_utc={built_at}",
        f"rustc={rustc}",
    ]
    return "\n".join(lines) + "\n"


def package_readme(spec: BundleSpec) -> str:
    windows_note = ""
    if spec.is_windows:
        windows_note = """
## Windows

Extract the `.zip` archive to a folder of your choice. Double-click `ec-connect.exe`
to launch. No installation required.

If Windows Defender flags the binary, click "More info" → "Run anyway".
""".rstrip()

    macos_quarantine_note = ""
    if spec.platform.target_triple.endswith("-apple-darwin"):
        binary_list = "./bin/ec-connect"
        if spec.artifact == "public-beta":
            binary_list = "./bin/ec-game ./bin/ec-sysop ./bin/ec-connect"
        macos_quarantine_note = f"""
## macOS First Run Note

These are command-line binaries, not bundled `.app` applications. If macOS
blocks them after download, remove the quarantine attribute from the unpacked
bundle root:

```bash
xattr -d com.apple.quarantine {binary_list}
```
""".rstrip()

    if spec.artifact == "ec-connect":
        return f"""# ec-connect {spec.platform.display_name}

This archive contains the public player client for {spec.platform.display_name}.

It contains:

- `bin/ec-connect`
- `ec-connect-cli.exe` (Windows only)
- `docs/ec_player_manual.pdf`
- `BUILD-INFO.txt` with version/build metadata

## Quick Start

Join a hosted game with the invite code from your sysop:

```bash
./bin/ec-connect --join amber-river@relay.example.com
```

The player manual PDF in `docs/` is the companion manual for this binary.
{macos_quarantine_note}{windows_note}

## Bug Reports

When reporting a player-client issue, include:

- the version and commit from `BUILD-INFO.txt`
- your {spec.platform.issue_platform_label} and terminal emulator
- the exact command you ran
- any stderr output
- a screenshot if the issue is visual
"""

    return f"""# Esterian Conquest {spec.platform.display_name} Public Beta Bundle

This bundle is for public beta testing on {spec.platform.display_name}.

It contains:

- `bin/ec-game`
- `bin/ec-sysop`
- `bin/ec-connect`

It also includes:

- `docs/ec_player_manual.pdf`
- `docs/ec_sysop_manual.pdf`
- `BUILD-INFO.txt` with version/build metadata for bug reports

This is not a public release package. Public GitHub Releases currently keep
only the DOS compatibility bundles while the hosted Rust path is still under
live playtest.

## Quick Start

Create a fresh campaign:

```bash
./bin/ec-sysop new-game /srv/ec/games/friday-night --name "Friday Night EC" --players 4 --seed 1515
```

Initialize and run the Nostr hosting daemon:

```bash
./bin/ec-sysop nostr init
./bin/ec-sysop nostr serve
```

The hosted-player join path is `ec-connect`:

```bash
./bin/ec-connect --join amber-river@relay.example.com
```

Run maintenance:

```bash
./bin/ec-sysop maint-all --config /etc/ec-gate/config.kdl
```

For localhost or hotseat play, you can still launch the game client directly:

```bash
./bin/ec-game --dir /tmp/ec-game --player 1
```

## BBS Door Note

If you host `ec-game` as a BBS door, the current stable door-mode controls are:

- `HJKL` for movement
- `Ctrl-U` / `Ctrl-D` for paging
- `Q` or `Esc` for back/quit

Arrow keys and `PgUp` / `PgDn` are not part of the primary door-mode contract.

Hosted Rust campaigns are DB-only. `ec-sysop new-game` creates just
`<game_dir>/ecgame.db`.
{macos_quarantine_note}

## Bug Reports

When reporting a playtest issue, include:

- the version and commit from `BUILD-INFO.txt`
- your {spec.platform.issue_platform_label} and terminal emulator
- the exact command you ran
- any stderr output
- a screenshot if the issue is visual
"""


def copy_file(src: Path, dest: Path, *, executable: bool = False) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, dest)
    if executable:
        mode = dest.stat().st_mode
        dest.chmod(mode | 0o755)


def stage_bundle(spec: BundleSpec, binary_paths: dict[str, Path], workspace_root: Path) -> Path:
    bundle_root = workspace_root / spec.bundle_root_name

    if spec.is_windows:
        for path in binary_paths.values():
            copy_file(path, bundle_root / path.name)
        copy_file(PLAYER_MANUAL, bundle_root / PLAYER_MANUAL.name)
        if spec.artifact == "public-beta":
            copy_file(SYSOP_MANUAL, bundle_root / SYSOP_MANUAL.name)
    else:
        docs_dir = bundle_root / "docs"
        bin_dir = bundle_root / "bin"
        for name, path in binary_paths.items():
            copy_file(path, bin_dir / name, executable=True)
        copy_file(PLAYER_MANUAL, docs_dir / PLAYER_MANUAL.name)
        if spec.artifact == "public-beta":
            copy_file(SYSOP_MANUAL, docs_dir / SYSOP_MANUAL.name)

    (bundle_root / "README.md").write_text(package_readme(spec), encoding="utf-8")
    (bundle_root / "BUILD-INFO.txt").write_text(build_info_text(spec), encoding="utf-8")
    return bundle_root


def write_archive(bundle_root: Path, archive_path: Path) -> None:
    archive_path.parent.mkdir(parents=True, exist_ok=True)
    if archive_path.suffix == ".zip":
        with zipfile.ZipFile(archive_path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
            for file in bundle_root.rglob("*"):
                if file.is_file():
                    zf.write(file, file.relative_to(bundle_root.parent))
    else:
        with tarfile.open(archive_path, "w:gz") as tf:
            tf.add(bundle_root, arcname=bundle_root.name)


def verify_archive(spec: BundleSpec, archive_path: Path, *, run_smoke: bool) -> None:
    with tempfile.TemporaryDirectory(prefix="ec-playtest-verify-") as temp_dir:
        temp_root = Path(temp_dir)
        with tarfile.open(archive_path, "r:gz") as tf:
            tf.extractall(temp_root)

        bundle_root = temp_root / spec.bundle_root_name
        if not bundle_root.exists():
            raise SystemExit(f"{archive_path.name}: missing bundle root {spec.bundle_root_name}")

        required_files = ["README.md", "BUILD-INFO.txt", "docs/ec_player_manual.pdf"]
        if spec.artifact == "public-beta":
            required_files.extend(
                ("docs/ec_sysop_manual.pdf", "bin/ec-game", "bin/ec-sysop", "bin/ec-connect")
            )
        else:
            required_files.append("bin/ec-connect")
        if spec.is_windows and spec.artifact == "ec-connect":
            required_files.append("ec-connect-cli.exe")

        for relative in required_files:
            path = bundle_root / relative
            if not path.exists():
                raise SystemExit(f"{archive_path.name}: missing {relative}")

        if not run_smoke:
            print(
                f"{archive_path.name}: verified archive contents; skipped binary smoke run "
                f"because target {spec.platform.target_triple} is not the current host."
            )
            return

        run([str(bundle_root / "bin" / "ec-connect"), "--help"], cwd=bundle_root)
        if spec.artifact == "public-beta":
            run([str(bundle_root / "bin" / "ec-game"), "--help"], cwd=bundle_root)
            run([str(bundle_root / "bin" / "ec-sysop"), "--help"], cwd=bundle_root)

            campaign_dir = temp_root / "playtest-campaign"
            run(
                [
                    str(bundle_root / "bin" / "ec-sysop"),
                    "new-game",
                    str(campaign_dir),
                    "--players",
                    "4",
                    "--seed",
                    "1515",
                ],
                cwd=bundle_root,
            )
            if not (campaign_dir / "ecgame.db").exists():
                raise SystemExit(f"{archive_path.name}: ec-sysop did not create ecgame.db")


def update_sha256sums(archive_path: Path) -> None:
    sums_path = archive_path.parent / "SHA256SUMS.txt"
    sig_path = archive_path.parent / "SHA256SUMS.txt.asc"

    # Compute sha256 of new archive
    import hashlib
    digest = hashlib.sha256(archive_path.read_bytes()).hexdigest()
    new_line = f"{digest}  {archive_path.name}\n"

    # Read existing entries, replace or append this archive's line
    existing = sums_path.read_text(encoding="utf-8") if sums_path.exists() else ""
    lines = [l for l in existing.splitlines(keepends=True) if not l.endswith(f"  {archive_path.name}\n")]
    lines.append(new_line)
    lines.sort(key=lambda l: l.split("  ", 1)[-1])
    sums_path.write_text("".join(lines), encoding="utf-8")

    # Re-sign the manifest
    sig_path.unlink(missing_ok=True)
    run(["gpg", "--armor", "--detach-sign", str(sums_path)])
    print(f"updated: {sums_path}")
    print(f"signed:  {sig_path}")


def main(*, default_target: str | None = None) -> None:
    args = parse_args(default_target)
    platform = resolve_target(args.target)
    spec = BundleSpec(version=load_version(), platform=platform, artifact=args.artifact)
    host_target = detect_host_target()
    binary_paths = build_binaries(spec)

    with tempfile.TemporaryDirectory(prefix="ec-playtest-build-") as temp_dir:
        bundle_root = stage_bundle(spec, binary_paths, Path(temp_dir))
        archive_path = args.output_dir / spec.archive_name
        write_archive(bundle_root, archive_path)
        if args.verify:
            verify_archive(spec, archive_path, run_smoke=platform.target_triple == host_target)
        update_sha256sums(archive_path)
        print(archive_path)


if __name__ == "__main__":
    main()
