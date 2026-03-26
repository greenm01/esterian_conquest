#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import tarfile
import tempfile
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
import tomllib


REPO_ROOT = Path(__file__).resolve().parents[1]
RUST_ROOT = REPO_ROOT / "rust"
RELEASES_DIR = REPO_ROOT / "releases"
TARGET_TRIPLE = "x86_64-unknown-linux-gnu"
BINARIES = ("ec-game", "ec-sysop")
THEMES_DIR = RUST_ROOT / "ec-game" / "config" / "themes"
CONFIG_TEMPLATE = RUST_ROOT / "ec-data" / "config" / "config.kdl"
PLAYER_MANUAL = REPO_ROOT / "docs" / "manuals" / "ec_player_manual.pdf"
SYSOP_MANUAL = REPO_ROOT / "docs" / "manuals" / "ec_sysop_manual.pdf"


@dataclass(frozen=True)
class BundleSpec:
    version: str
    target_triple: str

    @property
    def bundle_root_name(self) -> str:
        return f"esterian-conquest-v{self.version}-linux-x64"

    @property
    def archive_name(self) -> str:
        return f"{self.bundle_root_name}.tar.gz"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Build a Linux x64 playtest bundle containing ec-game, ec-sysop, "
            "PDF manuals, themes, and a config.kdl template."
        )
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
        help="Unpack and smoke-test the generated bundle after building it.",
    )
    return parser.parse_args()


def load_version() -> str:
    cargo_toml = tomllib.loads((RUST_ROOT / "ec-game" / "Cargo.toml").read_text(encoding="utf-8"))
    return cargo_toml["package"]["version"]


def run(
    argv: list[str],
    *,
    cwd: Path | None = None,
    capture_output: bool = False,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        argv,
        cwd=cwd or REPO_ROOT,
        check=True,
        text=True,
        capture_output=capture_output,
    )


def capture(argv: list[str], *, cwd: Path | None = None) -> str:
    return run(argv, cwd=cwd, capture_output=True).stdout.strip()


def build_binaries(spec: BundleSpec) -> dict[str, Path]:
    run(
        [
            "cargo",
            "build",
            "--release",
            "--target",
            spec.target_triple,
            "-p",
            "ec-game",
            "-p",
            "ec-sysop",
        ],
        cwd=RUST_ROOT,
    )

    target_dir = RUST_ROOT / "target" / spec.target_triple / "release"
    return {name: target_dir / name for name in BINARIES}


def build_info_text(spec: BundleSpec) -> str:
    commit = capture(["git", "rev-parse", "HEAD"])
    short_commit = capture(["git", "rev-parse", "--short", "HEAD"])
    rustc = capture(["rustc", "-V"])
    built_at = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    lines = [
        f"version={spec.version}",
        f"git_commit={commit}",
        f"git_commit_short={short_commit}",
        f"target={spec.target_triple}",
        f"built_at_utc={built_at}",
        f"rustc={rustc}",
    ]
    return "\n".join(lines) + "\n"


def package_readme(spec: BundleSpec) -> str:
    return f"""# Esterian Conquest Linux x64 Playtest Bundle

This bundle contains the public Rust playtest binaries for Linux x64:

- `bin/ec-game`
- `bin/ec-sysop`

It also includes:

- `docs/ec_player_manual.pdf`
- `docs/ec_sysop_manual.pdf`
- `themes/` with the bundled theme KDL files
- `config.kdl`, a sample sysop config template
- `BUILD-INFO.txt` with version/build metadata for bug reports

## Quick Start

Create a fresh campaign:

```bash
./bin/ec-sysop new-game /tmp/ec-game --players 4 --seed 1515
```

Launch the player client:

```bash
./bin/ec-game --dir /tmp/ec-game --player 1
```

Run maintenance:

```bash
./bin/ec-sysop maint /tmp/ec-game 1
```

## config.kdl

The bundled `config.kdl` is a template for sysops to review or copy. The live
`config.kdl` for actual play belongs inside each campaign directory, and the
tools will bootstrap one automatically when needed.

## Themes

The files under `themes/` are the bundled theme KDLs shipped with `ec-game`.
Campaigns normally keep their active theme files under each game directory's
own `themes/` subdirectory.

## Bug Reports

When reporting a playtest issue, include:

- the version and commit from `BUILD-INFO.txt`
- your Linux distro and terminal emulator
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
    docs_dir = bundle_root / "docs"
    themes_dest = bundle_root / "themes"
    bin_dir = bundle_root / "bin"

    for name, path in binary_paths.items():
        copy_file(path, bin_dir / name, executable=True)

    copy_file(PLAYER_MANUAL, docs_dir / PLAYER_MANUAL.name)
    copy_file(SYSOP_MANUAL, docs_dir / SYSOP_MANUAL.name)
    copy_file(CONFIG_TEMPLATE, bundle_root / "config.kdl")
    shutil.copytree(THEMES_DIR, themes_dest, dirs_exist_ok=True)

    (bundle_root / "README.md").write_text(package_readme(spec), encoding="utf-8")
    (bundle_root / "BUILD-INFO.txt").write_text(build_info_text(spec), encoding="utf-8")
    return bundle_root


def write_archive(bundle_root: Path, archive_path: Path) -> None:
    archive_path.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(archive_path, "w:gz") as tf:
        tf.add(bundle_root, arcname=bundle_root.name)


def verify_archive(spec: BundleSpec, archive_path: Path) -> None:
    with tempfile.TemporaryDirectory(prefix="ec-linux-playtest-verify-") as temp_dir:
        temp_root = Path(temp_dir)
        with tarfile.open(archive_path, "r:gz") as tf:
            tf.extractall(temp_root)

        bundle_root = temp_root / spec.bundle_root_name
        if not bundle_root.exists():
            raise SystemExit(f"{archive_path.name}: missing bundle root {spec.bundle_root_name}")

        for relative in (
            "README.md",
            "BUILD-INFO.txt",
            "config.kdl",
            "docs/ec_player_manual.pdf",
            "docs/ec_sysop_manual.pdf",
            "bin/ec-game",
            "bin/ec-sysop",
            "themes/tokyo_night.kdl",
        ):
            path = bundle_root / relative
            if not path.exists():
                raise SystemExit(f"{archive_path.name}: missing {relative}")

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


def main() -> None:
    args = parse_args()
    spec = BundleSpec(version=load_version(), target_triple=TARGET_TRIPLE)
    binary_paths = build_binaries(spec)

    with tempfile.TemporaryDirectory(prefix="ec-linux-playtest-build-") as temp_dir:
        bundle_root = stage_bundle(spec, binary_paths, Path(temp_dir))
        archive_path = args.output_dir / spec.archive_name
        write_archive(bundle_root, archive_path)
        if args.verify:
            verify_archive(spec, archive_path)
        print(archive_path)


if __name__ == "__main__":
    main()
