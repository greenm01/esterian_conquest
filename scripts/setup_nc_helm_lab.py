#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import shutil
import subprocess
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
RUST_DIR = REPO_ROOT / "rust"
DEFAULT_ROOT = Path("/tmp/nc-helm-lab")
DEFAULT_SEED_BASE = 1515

LAB_PROFILES = [
    ("map18-p4", 4, 18),
    ("map27-p9", 9, 27),
    ("map36-p16", 16, 36),
    ("map45-p25", 25, 45),
]


def cargo_env() -> dict[str, str]:
    env = os.environ.copy()
    env["RUSTC_WRAPPER"] = ""
    return env


def run_nc_cli(*args: str) -> None:
    subprocess.run(
        ["cargo", "run", "-q", "-p", "nc-cli", "--", *args],
        cwd=RUST_DIR,
        check=True,
        env=cargo_env(),
        text=True,
        capture_output=True,
    )


def run_nc_helm(game_dir: Path, release: bool) -> None:
    cmd = ["cargo", "run", "-q"]
    if release:
        cmd.append("--release")
    cmd.extend(["-p", "nc-helm", "--", "--dir", str(game_dir)])
    subprocess.run(cmd, cwd=RUST_DIR, check=True, env=cargo_env())


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Seed a four-map nc-helm stress lab and print launch commands."
    )
    parser.add_argument(
        "--root",
        default=str(DEFAULT_ROOT),
        help=f"Lab root to create. Default: {DEFAULT_ROOT}.",
    )
    parser.add_argument(
        "--seed-base",
        type=int,
        default=DEFAULT_SEED_BASE,
        help=f"Base seed forwarded to nc-cli harness seed-nc-helm-lab. Default: {DEFAULT_SEED_BASE}.",
    )
    parser.add_argument(
        "--force",
        action="store_true",
        help="Remove the lab root first if it already exists.",
    )
    parser.add_argument(
        "--launch",
        choices=[slug for slug, _, _ in LAB_PROFILES],
        help="Launch one seeded nc-helm campaign immediately after setup.",
    )
    parser.add_argument(
        "--release",
        action="store_true",
        help="When used with --launch, run nc-helm with cargo --release.",
    )
    return parser.parse_args()


def ensure_root(root: Path, force: bool) -> None:
    if root.exists() and force:
        shutil.rmtree(root)


def print_lab_summary(root: Path) -> None:
    print()
    print(f"Seeded nc-helm lab at {root}")
    print(f"Manifest: {root / 'README.txt'}")
    print("Maps:")
    for slug, players, map_size in LAB_PROFILES:
        game_dir = root / slug
        print(f"  {slug}: players={players} map={map_size}x{map_size}")
        print(f"    cargo run -q -p nc-helm -- --dir {game_dir}")


def main() -> None:
    args = parse_args()
    root = Path(args.root).resolve()
    ensure_root(root, args.force)
    run_nc_cli(
        "harness",
        "seed-nc-helm-lab",
        "--root",
        str(root),
        "--seed-base",
        str(args.seed_base),
    )
    print_lab_summary(root)

    if args.launch:
        print()
        print(f"Launching nc-helm for {args.launch}...")
        run_nc_helm(root / args.launch, args.release)


if __name__ == "__main__":
    main()
